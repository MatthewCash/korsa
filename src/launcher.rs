use crate::{configuration, discovery::Installation, preferences};
use anyhow::{Context, Result, bail, ensure};
use std::{ffi::OsString, fs, path::PathBuf, process::Stdio, time::Duration};
use tokio::{process::Command, time::sleep};

const APP_ID: &str = "244210";
const START_TIMEOUT: Duration = Duration::from_secs(60);
const STABILITY_WINDOW: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const LAUNCH_SERVICE: &str = "com.steampowered.PressureVessel.LaunchAlongsideSteam";
const RACE_PROCESS_NAME: &str = "acs.exe";

pub fn race_is_running() -> Result<bool> {
    Ok(!race_process_ids()?.is_empty())
}

pub async fn stop_race() -> Result<()> {
    if !race_is_running()? {
        return Ok(());
    }
    signal_race_processes(libc::SIGTERM)?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while tokio::time::Instant::now() < deadline {
        if !race_is_running()? {
            return Ok(());
        }
        sleep(POLL_INTERVAL).await;
    }

    signal_race_processes(libc::SIGKILL)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        if !race_is_running()? {
            return Ok(());
        }
        sleep(POLL_INTERVAL).await;
    }
    bail!("Assetto Corsa is still running after SIGKILL")
}

pub async fn launch_existing_session(installation: &Installation) -> Result<u32> {
    validate(installation)?;

    let compatibility_data = installation
        .proton_prefix
        .parent()
        .context("invalid Proton prefix path")?
        .to_path_buf();
    ensure_steam_service(installation).await?;

    log::info!(
        "launching the existing race through Steam Linux Runtime with {}",
        installation.proton_command.display()
    );

    let log_path = installation.documents_root.join("logs/log.txt");
    let previous_log_modified = modified_time(&log_path);
    let mut command = Command::new(&installation.runtime_client);
    command.args(launch_arguments(
        installation,
        &compatibility_data,
        "acs.exe",
    ));
    command
        .current_dir(&installation.game_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(false);

    let mut child = command
        .spawn()
        .context("failed to contact the Steam runtime launcher")?;
    let process_id = child
        .id()
        .context("Steam runtime launcher started without a process ID")?;

    let deadline = tokio::time::Instant::now() + START_TIMEOUT;
    let mut running_since = None;
    loop {
        if process_is_running()? {
            let since = running_since.get_or_insert_with(tokio::time::Instant::now);
            if since.elapsed() >= STABILITY_WINDOW {
                ensure_log_has_no_crash(&log_path, previous_log_modified)?;
                return Ok(process_id);
            }
        } else {
            running_since = None;
        }

        if let Some(status) = child
            .try_wait()
            .context("failed to inspect the Steam runtime launcher")?
        {
            ensure_log_has_no_crash(&log_path, previous_log_modified)?;
            bail!("Steam runtime launcher exited before acs.exe started ({status})");
        }

        if tokio::time::Instant::now() >= deadline {
            ensure_log_has_no_crash(&log_path, previous_log_modified)?;
            bail!("Steam did not start acs.exe within 60 seconds");
        }
        sleep(POLL_INTERVAL).await;
    }
}

pub async fn launch_showroom(installation: &Installation, car: &str, skin: &str) -> Result<u32> {
    validate_identifier(car, "car")?;
    validate_identifier(skin, "skin")?;
    ensure!(
        installation.game_root.join("dwrite.dll").is_file(),
        "the CSP showroom requires an installed Custom Shaders Patch"
    );
    let path = installation.documents_root.join("cfg/race.ini");
    let original =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    preferences::atomic_write(&path, &showroom_configuration(&original, car, skin))?;
    let result = launch_existing_session(installation).await;
    let restore = preferences::atomic_write(&path, &original);
    match (result, restore) {
        (Ok(process_id), Ok(())) => Ok(process_id),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => {
            Err(error).context("showroom started but race.ini could not be restored")
        }
        (Err(launch_error), Err(restore_error)) => Err(launch_error).context(format!(
            "race.ini also could not be restored: {restore_error:#}"
        )),
    }
}

fn showroom_configuration(original: &str, car: &str, skin: &str) -> String {
    let mut contents = original.to_owned();
    for (section, key, value) in [
        ("HEADER", "VERSION", "2"),
        ("HEADER", "__CM_FEATURE_SET", "2"),
        ("RACE", "MODEL", car),
        ("RACE", "MODEL_CONFIG", ""),
        ("RACE", "SKIN", skin),
        ("RACE", "TRACK", "../showroom/showroom"),
        ("RACE", "CONFIG_TRACK", ""),
        ("RACE", "CARS", "1"),
        ("RACE", "PENALTIES", "0"),
        ("RACE", "FIXED_SETUP", "0"),
        ("CAR_0", "MODEL", car),
        ("CAR_0", "MODEL_CONFIG", ""),
        ("CAR_0", "SKIN", skin),
        ("SESSION_0", "NAME", "Showroom"),
        ("SESSION_0", "TYPE", "1"),
        ("SESSION_0", "DURATION_MINUTES", "0"),
        ("SESSION_0", "SPAWN_SET", "PIT"),
        ("TEMPERATURE", "AMBIENT", "26"),
        ("TEMPERATURE", "ROAD", "32"),
        ("LIGHTING", "SUN_ANGLE", "0"),
        ("LIGHTING", "TIME_MULT", "1"),
        ("LIGHTING", "CLOUD_SPEED", "1"),
        ("WIND", "DIRECTION_DEG", "0"),
        ("WIND", "SPEED_KMH_MIN", "0"),
        ("WIND", "SPEED_KMH_MAX", "0"),
        ("REMOTE", "ACTIVE", "0"),
    ] {
        contents = configuration::set_ini_value(&contents, section, key, value);
    }
    contents
}

pub(crate) async fn ensure_steam_service(installation: &Installation) -> Result<()> {
    if steam_service_available(installation).await? {
        return Ok(());
    }

    log::info!("starting Steam for the runtime launcher service");
    Command::new("steam")
        .arg("-silent")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(false)
        .spawn()
        .context("Steam is not running and could not be started")?;

    let deadline = tokio::time::Instant::now() + START_TIMEOUT;
    while tokio::time::Instant::now() < deadline {
        if steam_service_available(installation).await? {
            return Ok(());
        }
        sleep(POLL_INTERVAL).await;
    }

    bail!("Steam runtime launcher service did not become available")
}

async fn steam_service_available(installation: &Installation) -> Result<bool> {
    let output = Command::new(&installation.runtime_client)
        .arg("--list")
        .output()
        .await
        .context("failed to inspect Steam runtime launcher services")?;

    Ok(output.status.success()
        && String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line == format!("--bus-name={LAUNCH_SERVICE}")))
}

fn launch_arguments(
    installation: &Installation,
    compatibility_data: &std::path::Path,
    executable_name: &str,
) -> Vec<OsString> {
    let runtime_entry = installation.runtime_root.join("_v2-entry-point");
    let executable = installation.game_root.join(executable_name);
    let proton_root = installation
        .proton_command
        .parent()
        .expect("validated Proton path must have a parent");
    let shader_path = installation
        .library_root
        .join(format!("steamapps/shadercache/{APP_ID}"));
    let steamworks = installation
        .library_root
        .join("steamapps/common/Steamworks Shared");

    let mut arguments: Vec<OsString> = [
        "--alongside-steam".into(),
        format!("--directory={}", installation.game_root.display()).into(),
        format!("--env=SteamAppId={APP_ID}").into(),
        format!("--env=SteamGameId={APP_ID}").into(),
        format!("--env=STEAM_COMPAT_APP_ID={APP_ID}").into(),
        format!(
            "--env=STEAM_COMPAT_CLIENT_INSTALL_PATH={}",
            installation.steam_root.display()
        )
        .into(),
        format!(
            "--env=STEAM_COMPAT_INSTALL_PATH={}",
            installation.game_root.display()
        )
        .into(),
        format!(
            "--env=STEAM_COMPAT_DATA_PATH={}",
            compatibility_data.display()
        )
        .into(),
        format!("--env=STEAM_COMPAT_SHADER_PATH={}", shader_path.display()).into(),
        format!(
            "--env=STEAM_COMPAT_TOOL_PATHS={}:{}",
            proton_root.display(),
            installation.runtime_root.display()
        )
        .into(),
        format!(
            "--env=STEAM_COMPAT_MOUNTS={}:{}",
            steamworks.display(),
            installation.runtime_root.display()
        )
        .into(),
        "--env=STEAM_COMPAT_PROTON=1".into(),
        "--env=STEAM_COMPAT_FLAGS=search-cwd".into(),
        "--".into(),
        runtime_entry.into_os_string(),
        "--verb=run".into(),
        "--".into(),
        installation.proton_command.clone().into_os_string(),
        "run".into(),
        executable.into_os_string(),
    ]
    .into();

    if let Some(config) = &installation.proton_config {
        arguments.insert(13, format!("--env=STEAM_COMPAT_CONFIG={config}").into());
    }

    if let Ok(sink) = std::env::var("PULSE_SINK") {
        let command_separator = arguments
            .iter()
            .position(|argument| argument == "--")
            .expect("launch arguments must contain a command separator");
        arguments.insert(command_separator, format!("--env=PULSE_SINK={sink}").into());
    }

    arguments
}

fn modified_time(path: &std::path::Path) -> Option<std::time::SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

fn ensure_log_has_no_crash(
    path: &std::path::Path,
    previous_modified: Option<std::time::SystemTime>,
) -> Result<()> {
    let current_modified = modified_time(path);
    if current_modified.is_some() && current_modified != previous_modified {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("could not read Assetto Corsa log at {}", path.display()))?;
        if contents.lines().any(|line| line.trim() == "CRASH in:") {
            bail!(
                "Assetto Corsa crashed during startup; inspect {}",
                path.display()
            );
        }
    }

    Ok(())
}

fn validate(installation: &Installation) -> Result<()> {
    if process_is_running()? {
        bail!("Assetto Corsa is already running");
    }
    if process_matches("AssettoCorsa\\.exe")? {
        bail!("Content Manager is running; close it before launching a race");
    }

    for (description, path) in required_paths(installation) {
        if !path.is_file() {
            bail!("{description} does not exist: {}", path.display());
        }
    }

    Ok(())
}

fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        bail!("invalid showroom {label} ID");
    }
    Ok(())
}

fn required_paths(installation: &Installation) -> [(&'static str, PathBuf); 6] {
    [
        (
            "Assetto Corsa executable",
            installation.game_root.join("acs.exe"),
        ),
        ("Proton executable", installation.proton_command.clone()),
        ("race.ini", installation.documents_root.join("cfg/race.ini")),
        (
            "assists.ini",
            installation.documents_root.join("cfg/assists.ini"),
        ),
        (
            "Steam Linux Runtime entry point",
            installation.runtime_root.join("_v2-entry-point"),
        ),
        (
            "Steam runtime launcher client",
            installation.runtime_client.clone(),
        ),
    ]
}

fn process_is_running() -> Result<bool> {
    race_is_running()
}

fn process_matches(pattern: &str) -> Result<bool> {
    let output = std::process::Command::new("pgrep")
        .args(["-f", pattern])
        .output()
        .context("failed to inspect running processes")?;

    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => bail!(
            "pgrep failed while inspecting running processes: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
    }
}

fn race_process_ids() -> Result<Vec<i32>> {
    let processes = fs::read_dir("/proc").context("failed to inspect running processes")?;
    Ok(processes
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_string_lossy().parse::<i32>().ok())
        .filter(|pid| is_assetto_corsa_process(*pid))
        .collect())
}

fn is_assetto_corsa_process(pid: i32) -> bool {
    let Ok(command_line) = fs::read(format!("/proc/{pid}/cmdline")) else {
        return false;
    };
    if !is_race_command(&command_line) {
        return false;
    }

    let Ok(environment) = fs::read(format!("/proc/{pid}/environ")) else {
        return false;
    };
    environment.split(|byte| *byte == 0).any(|variable| {
        variable == format!("STEAM_COMPAT_APP_ID={APP_ID}").as_bytes()
            || variable == format!("SteamAppId={APP_ID}").as_bytes()
    })
}

fn is_race_command(command_line: &[u8]) -> bool {
    let executable = command_line
        .split(|byte| *byte == 0)
        .next()
        .unwrap_or_default();
    executable
        .rsplit(|byte| matches!(byte, b'/' | b'\\'))
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case(RACE_PROCESS_NAME.as_bytes()))
}

fn signal_race_processes(signal: i32) -> Result<()> {
    for pid in race_process_ids()? {
        let result = unsafe { libc::kill(pid, signal) };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error)
                    .with_context(|| format!("could not signal Assetto Corsa PID {pid}"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn installation() -> Installation {
        Installation {
            steam_root: "/steam".into(),
            library_root: "/library".into(),
            game_root: "/library/steamapps/common/assettocorsa".into(),
            proton_prefix: "/library/steamapps/compatdata/244210/pfx".into(),
            documents_root: "/documents/Assetto Corsa".into(),
            proton_command: "/steam/compatibilitytools.d/GE-Proton/proton".into(),
            proton_config: Some("noxalia".into()),
            runtime_root: "/steam/steamapps/common/SteamLinuxRuntime_sniper".into(),
            runtime_client: "/steam/steamapps/common/SteamLinuxRuntime_sniper/pressure-vessel/bin/steam-runtime-launch-client".into(),
        }
    }

    #[test]
    fn launches_acs_through_runtime_alongside_steam() {
        let arguments =
            launch_arguments(&installation(), Path::new("/compatdata/244210"), "acs.exe");
        let arguments = arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(
            arguments.first().map(String::as_str),
            Some("--alongside-steam")
        );
        assert!(arguments.contains(&"--env=STEAM_COMPAT_APP_ID=244210".to_owned()));
        assert!(arguments.contains(&"--env=STEAM_COMPAT_CONFIG=noxalia".to_owned()));
        assert!(arguments.contains(&"--verb=run".to_owned()));
        assert_eq!(
            arguments.last().map(String::as_str),
            Some("/library/steamapps/common/assettocorsa/acs.exe")
        );
        assert!(
            !arguments
                .iter()
                .any(|argument| argument.contains("AssettoCorsa.exe"))
        );
    }

    #[test]
    fn recognizes_renamed_wine_race_from_its_command_line() {
        assert!(is_race_command(b"Z:\\home\\user\\assettocorsa\\acs.exe\0"));
        assert!(is_race_command(b"/games/assettocorsa/ACS.EXE\0"));
        assert!(!is_race_command(b"/games/assettocorsa/AssettoCorsa.exe\0"));
    }

    #[test]
    fn reports_a_new_startup_crash_log() {
        let path = std::env::temp_dir().join(format!("aclm-crash-log-{}", std::process::id()));
        fs::write(&path, "startup\nCRASH in:\nstack\n").unwrap();

        let error = ensure_log_has_no_crash(&path, None).unwrap_err();

        assert!(error.to_string().contains("crashed during startup"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn configures_csp_showroom_as_a_special_race() {
        let contents = showroom_configuration("[REMOTE]\nACTIVE=1\n", "test_car", "red");
        assert_eq!(
            crate::session::ini_value(&contents, "RACE", "TRACK").as_deref(),
            Some("../showroom/showroom")
        );
        assert_eq!(
            crate::session::ini_value(&contents, "RACE", "MODEL").as_deref(),
            Some("test_car")
        );
        assert_eq!(
            crate::session::ini_value(&contents, "CAR_0", "MODEL").as_deref(),
            Some("test_car")
        );
        assert_eq!(
            crate::session::ini_value(&contents, "REMOTE", "ACTIVE").as_deref(),
            Some("0")
        );
    }
}
