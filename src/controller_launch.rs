//! File-based handshake because Steam's runtime service does not forward stdout.
//! The Windows helper holds DirectInput open, validates hotplug, then starts AC.
use crate::{controls, discovery::Installation};
use anyhow::{Context, Result, bail};
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::{process::Child, time::sleep};

const HELPER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/korsa-controller-helper.exe"));
static NEXT_LAUNCH: AtomicU64 = AtomicU64::new(0);

pub(crate) struct ControllerLaunch {
    directory: PathBuf,
}

impl ControllerLaunch {
    pub(crate) fn prepare(installation: &Installation) -> Result<Option<Self>> {
        if !controls::needs_controller_refresh(installation)? {
            return Ok(None);
        }
        // Keep files under the Steam prefix, not /tmp: pressure-vessel has its
        // own /tmp. create_dir (not create_dir_all) rejects collisions/symlinks.
        let root = installation.proton_prefix.join("drive_c/korsa-launch");
        fs::create_dir_all(&root).context("could not create controller validation directory")?;
        let directory = root.join(format!(
            "{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos(),
            NEXT_LAUNCH.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .context("could not create a private controller validation directory")?;
        let launch = Self { directory };
        fs::write(launch.executable(), HELPER)
            .context("could not extract the Wine controller helper")?;
        Ok(Some(launch))
    }

    pub(crate) fn executable(&self) -> PathBuf {
        self.directory.join("korsa-controller-helper.exe")
    }

    pub(crate) fn arguments(&self, installation: &Installation) -> Result<[String; 2]> {
        Ok([
            wine_path(&self.directory)?,
            wine_path(&installation.game_root.join("acs.exe"))?,
        ])
    }

    pub(crate) async fn validate_and_approve(
        &self,
        installation: &Installation,
        child: &mut Child,
    ) -> Result<()> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
        let path = self.directory.join("devices.json");
        loop {
            self.check_error()?;
            if path.is_file() {
                let manifest =
                    fs::read_to_string(&path).context("could not read DirectInput devices")?;
                controls::refresh_controller_ids(installation, &manifest)
                    .context("controller validation failed; Assetto Corsa was not started")?;
                fs::write(self.directory.join("continue"), b"")
                    .context("could not approve the validated controller map")?;
                return Ok(());
            }
            if let Some(status) = child.try_wait()? {
                self.check_error()?;
                bail!("Wine controller helper exited before enumerating devices ({status})");
            }
            if tokio::time::Instant::now() >= deadline {
                bail!("Wine controller enumeration timed out; Assetto Corsa was not started");
            }
            sleep(Duration::from_millis(50)).await;
        }
    }

    pub(crate) fn check_error(&self) -> Result<()> {
        let path = self.directory.join("error.txt");
        if path.is_file() {
            bail!(
                "{}",
                fs::read_to_string(path).context("could not read controller helper error")?
            );
        }
        Ok(())
    }
}

impl Drop for ControllerLaunch {
    fn drop(&mut self) {
        // Removing approval/the directory also cancels helpers still waiting.
        // Do not recursively delete anything supplied by the game or user.
        for name in [
            "continue",
            "abort",
            "devices.json",
            "devices.pending",
            "devices.check",
            "error.txt",
            "korsa-controller-helper.exe",
        ] {
            let _ = fs::remove_file(self.directory.join(name));
        }
        let _ = fs::remove_dir(&self.directory);
    }
}

fn wine_path(path: &Path) -> Result<String> {
    anyhow::ensure!(path.is_absolute(), "Wine launch paths must be absolute");
    let path = path
        .to_str()
        .context("Wine launch paths must be valid UTF-8")?;
    anyhow::ensure!(
        !path.contains(['\\', '"', '\r', '\n']),
        "unsupported character in Wine launch path"
    );
    Ok(format!("Z:{}", path.replace('/', "\\")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Installation {
        let root = std::env::temp_dir().join(format!(
            "korsa-launch-test-{}-{}",
            std::process::id(),
            NEXT_LAUNCH.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("cfg")).unwrap();
        fs::create_dir_all(root.join("drive_c")).unwrap();
        fs::write(
            root.join("cfg/controls.ini"),
            "[HEADER]\nINPUT_METHOD=WHEEL\n\
            [CONTROLLERS]\nCON0=Wheel\nPGUID0=0002346E-0000-0000-0000-504944564944\n\
            [STEER]\nJOY=0\nAXLE=0\n",
        )
        .unwrap();
        Installation {
            steam_root: root.clone(),
            library_root: root.clone(),
            game_root: root.clone(),
            proton_prefix: root.clone(),
            documents_root: root.clone(),
            proton_command: root.clone(),
            proton_config: None,
            runtime_root: root.clone(),
            runtime_client: root,
        }
    }

    #[test]
    fn maps_absolute_paths_without_shell_quoting() {
        assert_eq!(
            wine_path(Path::new("/games/Steam Library/acs.exe")).unwrap(),
            "Z:\\games\\Steam Library\\acs.exe"
        );
        assert!(wine_path(Path::new("relative/acs.exe")).is_err());
        assert!(wine_path(Path::new("/games/quoted\"/acs.exe")).is_err());
    }

    #[test]
    fn bundled_helper_uses_the_windowless_gui_subsystem() {
        // IMAGE_OPTIONAL_HEADER64.Subsystem is 68 bytes into the optional header.
        let pe = u32::from_le_bytes(HELPER[0x3c..0x40].try_into().unwrap()) as usize;
        assert_eq!(&HELPER[pe..pe + 4], b"PE\0\0");
        let subsystem = pe + 24 + 68;
        assert_eq!(
            u16::from_le_bytes(HELPER[subsystem..subsystem + 2].try_into().unwrap()),
            2,
            "the helper must not allocate a Wine console window"
        );
    }

    #[test]
    fn extracts_helper_privately_and_cleans_up_on_cancellation() {
        use std::os::unix::fs::PermissionsExt;
        let installation = fixture();
        let launch = ControllerLaunch::prepare(&installation).unwrap().unwrap();
        let directory = launch.directory.clone();
        assert_eq!(
            fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert!(fs::read(launch.executable()).unwrap().starts_with(b"MZ"));
        assert!(!directory.join("continue").exists());
        drop(launch);
        assert!(!directory.exists());
        fs::write(
            installation.documents_root.join("cfg/controls.ini"),
            "[HEADER]\nINPUT_METHOD=KEYBOARD\n",
        )
        .unwrap();
        assert!(ControllerLaunch::prepare(&installation).unwrap().is_none());
        fs::remove_dir_all(installation.documents_root).unwrap();
    }

    #[test]
    fn approval_requires_a_valid_complete_device_map() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let installation = fixture();
            let launch = ControllerLaunch::prepare(&installation).unwrap().unwrap();
            let path = installation.documents_root.join("cfg/controls.ini");
            let original = fs::read_to_string(&path).unwrap();
            let mut child = tokio::process::Command::new("true")
                .kill_on_drop(true)
                .spawn()
                .unwrap();
            fs::write(launch.directory.join("devices.json"), "[]").unwrap();
            assert!(
                launch
                    .validate_and_approve(&installation, &mut child)
                    .await
                    .is_err()
            );
            assert!(!launch.directory.join("continue").exists());
            assert_eq!(fs::read_to_string(&path).unwrap(), original);
            fs::write(
                launch.directory.join("devices.json"),
                r#"[{
                "name":"Wheel", "product_guid":"0002346E-0000-0000-0000-504944564944",
                "instance_guid":"9E573EDD-7734-11D2-8D4A-23903FB6BDF7", "path":"wheel-path"
            }]"#,
            )
            .unwrap();
            launch
                .validate_and_approve(&installation, &mut child)
                .await
                .unwrap();
            assert!(launch.directory.join("continue").exists());
            assert!(
                fs::read_to_string(&path)
                    .unwrap()
                    .contains("__IGUID0=9E573EDD")
            );
            child.wait().await.unwrap();
            drop(launch);
            fs::remove_dir_all(installation.documents_root).unwrap();
        });
    }

    #[test]
    fn reports_windows_helper_errors() {
        let installation = fixture();
        let launch = ControllerLaunch::prepare(&installation).unwrap().unwrap();
        fs::write(
            launch.directory.join("error.txt"),
            "Controllers changed during launch",
        )
        .unwrap();
        assert!(
            launch
                .check_error()
                .unwrap_err()
                .to_string()
                .contains("Controllers changed")
        );
        drop(launch);
        fs::remove_dir_all(installation.documents_root).unwrap();
    }
}
