use crate::{
    discovery::Installation,
    preferences::{self, Conditions, Preferences, SessionPreset},
    session,
};
use anyhow::{Context, Result};
use serde::Serialize;
use std::{
    fs,
    fs::{File, OpenOptions},
    os::fd::AsRawFd,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct SessionLock(File);

pub fn lock_session(installation: &Installation) -> Result<SessionLock> {
    session_lock(installation, false)
}

fn try_lock_session(installation: &Installation) -> Result<SessionLock> {
    session_lock(installation, true)
}

fn session_lock(installation: &Installation, nonblocking: bool) -> Result<SessionLock> {
    let path = installation.documents_root.join("cfg/.aclm-session.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("could not open {}", path.display()))?;
    let operation = libc::LOCK_EX | if nonblocking { libc::LOCK_NB } else { 0 };
    let result = unsafe { libc::flock(file.as_raw_fd(), operation) };
    anyhow::ensure!(
        result == 0,
        "another Assetto Corsa session is being configured or launched"
    );
    Ok(SessionLock(file))
}

impl Drop for SessionLock {
    fn drop(&mut self) {
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

#[derive(Debug, Serialize)]
struct WeatherItem {
    id: String,
    name: String,
    weather_type: i32,
}

#[derive(Debug, Serialize)]
struct WeatherControllerItem {
    id: String,
    name: String,
    description: String,
}

pub fn load_weather_json(installation: &Installation) -> Result<String> {
    let root = installation.game_root.join("content/weather");
    let mut weather = fs::read_dir(&root)
        .with_context(|| format!("could not read {}", root.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|path| {
            let id = path.file_name()?.to_string_lossy().into_owned();
            let metadata = session::read_optional(&path.join("weather.ini"));
            let name = session::ini_value(&metadata, "LAUNCHER", "NAME")
                .unwrap_or_else(|| session::humanize(&id));
            let weather_type = session::ini_value(&metadata, "__LAUNCHER_CM", "WEATHER_TYPE")
                .and_then(|value| value.parse().ok())
                .unwrap_or(15);
            Some(WeatherItem {
                id,
                name,
                weather_type,
            })
        })
        .collect::<Vec<_>>();
    weather.sort_by_key(|item| item.name.to_lowercase());
    serde_json::to_string(&weather).context("could not serialize weather catalog")
}

pub fn load_weather_controllers_json(installation: &Installation) -> Result<String> {
    let root = installation.game_root.join("extension/weather-controllers");
    let mut controllers = fs::read_dir(&root)
        .with_context(|| format!("could not read {}", root.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|path| {
            let id = path.file_name()?.to_string_lossy().into_owned();
            let manifest = session::read_optional(&path.join("manifest.ini"));
            let name = session::ini_value(&manifest, "ABOUT", "NAME")
                .unwrap_or_else(|| session::humanize(&id));
            let description =
                session::ini_value(&manifest, "ABOUT", "DESCRIPTION").unwrap_or_default();
            Some(WeatherControllerItem {
                id,
                name,
                description,
            })
        })
        .collect::<Vec<_>>();
    controllers.sort_by_key(|item| item.name.to_lowercase());
    serde_json::to_string(&controllers).context("could not serialize weather controllers")
}

pub fn load_conditions(installation: &Installation) -> Result<Conditions> {
    let path = installation.documents_root.join("cfg/race.ini");
    let contents =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    let mut conditions = Conditions::default();
    conditions.session_mode = session_mode_from_type(number(&contents, "SESSION_0", "TYPE", 1));
    conditions.session_duration = number(
        &contents,
        "SESSION_0",
        "DURATION_MINUTES",
        conditions.session_duration,
    );
    conditions.race_laps = number(&contents, "RACE", "RACE_LAPS", conditions.race_laps);
    conditions.opponents = (number(&contents, "RACE", "CARS", 1) - 1).max(0);
    conditions.ai_level = number(&contents, "RACE", "AI_LEVEL", conditions.ai_level);
    conditions.penalties = value(&contents, "RACE", "PENALTIES", "0") == "1";
    conditions.weather_id = value(&contents, "WEATHER", "NAME", &conditions.weather_id);
    conditions.sun_angle = number(&contents, "LIGHTING", "SUN_ANGLE", conditions.sun_angle);
    conditions.time_multiplier = number(
        &contents,
        "LIGHTING",
        "TIME_MULT",
        conditions.time_multiplier,
    );
    conditions.cloud_speed = number(&contents, "LIGHTING", "CLOUD_SPEED", conditions.cloud_speed);
    conditions.ambient_temperature = number(
        &contents,
        "TEMPERATURE",
        "AMBIENT",
        conditions.ambient_temperature,
    );
    conditions.road_temperature = number(
        &contents,
        "TEMPERATURE",
        "ROAD",
        conditions.road_temperature,
    );
    conditions.wind_speed_min = number(
        &contents,
        "WIND",
        "SPEED_KMH_MIN",
        conditions.wind_speed_min,
    );
    conditions.wind_speed_max = number(
        &contents,
        "WIND",
        "SPEED_KMH_MAX",
        conditions.wind_speed_max,
    );
    conditions.wind_direction = number(
        &contents,
        "WIND",
        "DIRECTION_DEG",
        conditions.wind_direction,
    );
    conditions.session_start_grip = number(
        &contents,
        "DYNAMIC_TRACK",
        "SESSION_START",
        conditions.session_start_grip,
    );
    conditions.session_transfer = number(
        &contents,
        "DYNAMIC_TRACK",
        "SESSION_TRANSFER",
        conditions.session_transfer,
    );
    conditions.randomness = number(
        &contents,
        "DYNAMIC_TRACK",
        "RANDOMNESS",
        conditions.randomness,
    );
    conditions.lap_gain = number(&contents, "DYNAMIC_TRACK", "LAP_GAIN", conditions.lap_gain);
    conditions.virtual_laps = number(&contents, "GROOVE", "VIRTUAL_LAPS", conditions.virtual_laps);
    conditions.max_laps = number(&contents, "GROOVE", "MAX_LAPS", conditions.max_laps);
    conditions.starting_laps = number(
        &contents,
        "GROOVE",
        "STARTING_LAPS",
        conditions.starting_laps,
    );
    conditions.weather_controller = value(
        &contents,
        "LIGHTING",
        "__CM_WEATHER_CONTROLLER",
        &conditions.weather_controller,
    );
    conditions.weather_type = number(
        &contents,
        "LIGHTING",
        "__CM_WEATHER_TYPE",
        conditions.weather_type,
    );
    Ok(conditions)
}

pub fn conditions_json(conditions: &Conditions) -> Result<String> {
    serde_json::to_string(conditions).context("could not serialize track conditions")
}

pub fn presets_json(preferences: &Preferences) -> Result<String> {
    serde_json::to_string(&preferences.presets).context("could not serialize session presets")
}

pub fn save_preset(
    preferences: &mut Preferences,
    name: &str,
    car_id: &str,
    skin_id: &str,
    track_key: &str,
    conditions_json: &str,
) -> Result<()> {
    anyhow::ensure!(!name.trim().is_empty(), "preset name cannot be empty");
    anyhow::ensure!(!car_id.is_empty(), "a car must be selected");
    anyhow::ensure!(!track_key.is_empty(), "a track must be selected");
    let conditions = serde_json::from_str(conditions_json).context("invalid track conditions")?;
    let (track_id, track_layout) = split_track_key(track_key);
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_millis()
        .to_string();
    preferences.presets.push(SessionPreset {
        id,
        name: name.trim().to_owned(),
        car_id: car_id.to_owned(),
        skin_id: skin_id.to_owned(),
        track_id,
        track_layout,
        conditions,
    });
    preferences.save()
}

pub fn delete_preset(preferences: &mut Preferences, id: &str) -> Result<()> {
    let previous_len = preferences.presets.len();
    preferences.presets.retain(|preset| preset.id != id);
    anyhow::ensure!(
        preferences.presets.len() != previous_len,
        "preset not found: {id}"
    );
    preferences.save()
}

pub fn apply_preset(
    installation: &Installation,
    preferences: &Preferences,
    id: &str,
) -> Result<Conditions> {
    let _lock = try_lock_session(installation)?;
    let preset = preferences
        .presets
        .iter()
        .find(|preset| preset.id == id)
        .with_context(|| format!("preset not found: {id}"))?;
    apply_configuration(
        installation,
        &preset.car_id,
        &preset.skin_id,
        &track_key(&preset.track_id, &preset.track_layout),
        &preset.conditions,
    )?;
    Ok(preset.conditions.clone())
}

pub fn apply_configuration_json(
    installation: &Installation,
    car_id: &str,
    skin_id: &str,
    track_key: &str,
    conditions_json: &str,
) -> Result<Conditions> {
    let _lock = try_lock_session(installation)?;
    apply_configuration_json_unlocked(installation, car_id, skin_id, track_key, conditions_json)
}

pub fn apply_configuration_json_unlocked(
    installation: &Installation,
    car_id: &str,
    skin_id: &str,
    track_key: &str,
    conditions_json: &str,
) -> Result<Conditions> {
    anyhow::ensure!(!car_id.is_empty(), "a car must be selected");
    anyhow::ensure!(!track_key.is_empty(), "a track must be selected");
    let conditions = serde_json::from_str(conditions_json).context("invalid track conditions")?;
    apply_configuration(installation, car_id, skin_id, track_key, &conditions)?;
    Ok(conditions)
}

pub fn apply_dashboard_configuration(
    installation: &Installation,
    car_id: &str,
    track_key: &str,
    start_minutes: i32,
) -> Result<()> {
    anyhow::ensure!(
        (0..1440).contains(&start_minutes),
        "start time must be between 00:00 and 23:59"
    );
    anyhow::ensure!(
        installation
            .game_root
            .join("content/cars")
            .join(car_id)
            .is_dir(),
        "car is not installed: {car_id}"
    );
    let (track_id, _) = split_track_key(track_key);
    anyhow::ensure!(
        installation
            .game_root
            .join("content/tracks")
            .join(&track_id)
            .is_dir(),
        "track is not installed: {track_key}"
    );

    let session = session::load(installation)?;
    let skins = installed_skins(installation, car_id);
    let skin_id = if session.car_id == car_id && skins.contains(&session.skin_id) {
        session.skin_id
    } else {
        skins
            .first()
            .cloned()
            .context("selected car has no installed skins")?
    };
    let mut conditions = load_conditions(installation)?;
    conditions.sun_angle = (f64::from(start_minutes) / 60.0 - 13.0) * 16.0;
    apply_configuration(installation, car_id, &skin_id, track_key, &conditions)
}

fn apply_configuration(
    installation: &Installation,
    car_id: &str,
    skin_id: &str,
    selected_track: &str,
    conditions: &Conditions,
) -> Result<()> {
    let path = installation.documents_root.join("cfg/race.ini");
    let mut contents =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    let (track_id, track_layout) = split_track_key(selected_track);
    let (session_name, session_type, drift_mode) = session_mode(&conditions.session_mode)?;
    anyhow::ensure!(
        (0..=63).contains(&conditions.opponents),
        "opponent count must be between 0 and 63"
    );
    anyhow::ensure!(
        (70..=100).contains(&conditions.ai_level),
        "AI strength must be between 70 and 100"
    );
    anyhow::ensure!(
        (1..=999).contains(&conditions.race_laps),
        "race laps must be between 1 and 999"
    );
    anyhow::ensure!(
        (0..=1440).contains(&conditions.session_duration),
        "session duration must be between 0 and 1440 minutes"
    );
    let values = [
        ("RACE", "MODEL", car_id.to_owned()),
        ("RACE", "SKIN", skin_id.to_owned()),
        ("RACE", "DRIFT_MODE", drift_mode.to_string()),
        ("RACE", "CARS", (conditions.opponents + 1).to_string()),
        ("RACE", "AI_LEVEL", conditions.ai_level.to_string()),
        ("RACE", "RACE_LAPS", conditions.race_laps.to_string()),
        (
            "RACE",
            "PENALTIES",
            if conditions.penalties { "1" } else { "0" }.to_owned(),
        ),
        ("RACE", "TRACK", track_id),
        ("RACE", "CONFIG_TRACK", track_layout),
        ("CAR_0", "SKIN", skin_id.to_owned()),
        ("SESSION_0", "NAME", session_name.to_owned()),
        ("SESSION_0", "TYPE", session_type.to_string()),
        ("SESSION_0", "SPAWN_SET", "START".to_owned()),
        (
            "SESSION_0",
            "DURATION_MINUTES",
            conditions.session_duration.to_string(),
        ),
        ("WEATHER", "NAME", conditions.weather_id.clone()),
        ("LIGHTING", "SUN_ANGLE", conditions.sun_angle.to_string()),
        (
            "LIGHTING",
            "TIME_MULT",
            conditions.time_multiplier.to_string(),
        ),
        (
            "LIGHTING",
            "CLOUD_SPEED",
            conditions.cloud_speed.to_string(),
        ),
        (
            "LIGHTING",
            "__CM_WEATHER_CONTROLLER",
            conditions.weather_controller.clone(),
        ),
        (
            "LIGHTING",
            "__CM_WEATHER_TYPE",
            conditions.weather_type.to_string(),
        ),
        (
            "TEMPERATURE",
            "AMBIENT",
            conditions.ambient_temperature.to_string(),
        ),
        (
            "TEMPERATURE",
            "ROAD",
            conditions.road_temperature.to_string(),
        ),
        (
            "WIND",
            "SPEED_KMH_MIN",
            conditions.wind_speed_min.to_string(),
        ),
        (
            "WIND",
            "SPEED_KMH_MAX",
            conditions.wind_speed_max.to_string(),
        ),
        (
            "WIND",
            "DIRECTION_DEG",
            conditions.wind_direction.to_string(),
        ),
        (
            "DYNAMIC_TRACK",
            "SESSION_START",
            conditions.session_start_grip.to_string(),
        ),
        (
            "DYNAMIC_TRACK",
            "SESSION_TRANSFER",
            conditions.session_transfer.to_string(),
        ),
        (
            "DYNAMIC_TRACK",
            "RANDOMNESS",
            conditions.randomness.to_string(),
        ),
        ("DYNAMIC_TRACK", "LAP_GAIN", conditions.lap_gain.to_string()),
        (
            "GROOVE",
            "VIRTUAL_LAPS",
            conditions.virtual_laps.to_string(),
        ),
        ("GROOVE", "MAX_LAPS", conditions.max_laps.to_string()),
        (
            "GROOVE",
            "STARTING_LAPS",
            conditions.starting_laps.to_string(),
        ),
    ];
    for (section, key, value) in values {
        contents = set_ini_value(&contents, section, key, &value);
    }
    let skins = installed_skins(installation, car_id);
    for index in 1..=conditions.opponents {
        let section = format!("CAR_{index}");
        let skin = skins
            .get(index as usize % skins.len().max(1))
            .cloned()
            .unwrap_or_else(|| skin_id.to_owned());
        for (key, value) in [
            ("MODEL", car_id.to_owned()),
            ("MODEL_CONFIG", String::new()),
            ("SKIN", skin),
            ("AI_LEVEL", conditions.ai_level.to_string()),
        ] {
            contents = set_ini_value(&contents, &section, key, &value);
        }
    }
    preferences::atomic_write(&path, &contents)
}

fn installed_skins(installation: &Installation, car_id: &str) -> Vec<String> {
    let root = installation
        .game_root
        .join("content/cars")
        .join(car_id)
        .join("skins");
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut skins = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    skins.sort();
    skins
}

fn session_mode(mode: &str) -> Result<(&'static str, i32, i32)> {
    match mode {
        "practice" => Ok(("Practice", 1, 0)),
        "qualifying" => Ok(("Qualifying", 2, 0)),
        "race" => Ok(("Race", 3, 0)),
        "hotlap" => Ok(("Hotlap", 4, 0)),
        "time_attack" => Ok(("Time Attack", 5, 0)),
        "drift" => Ok(("Drift", 6, 1)),
        "drag" => Ok(("Drag", 7, 0)),
        _ => anyhow::bail!("unknown race mode: {mode}"),
    }
}

fn session_mode_from_type(session_type: i32) -> String {
    match session_type {
        2 => "qualifying",
        3 => "race",
        4 => "hotlap",
        5 => "time_attack",
        6 => "drift",
        7 => "drag",
        _ => "practice",
    }
    .to_owned()
}

fn split_track_key(key: &str) -> (String, String) {
    key.split_once('/')
        .map(|(track, layout)| (track.to_owned(), layout.to_owned()))
        .unwrap_or_else(|| (key.to_owned(), String::new()))
}

fn track_key(track: &str, layout: &str) -> String {
    if layout.is_empty() {
        track.to_owned()
    } else {
        format!("{track}/{layout}")
    }
}

fn value(contents: &str, section: &str, key: &str, default: &str) -> String {
    session::ini_value(contents, section, key).unwrap_or_else(|| default.to_owned())
}

fn number<T>(contents: &str, section: &str, key: &str, default: T) -> T
where
    T: std::str::FromStr,
{
    session::ini_value(contents, section, key)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

pub(crate) fn set_ini_value(contents: &str, section: &str, key: &str, value: &str) -> String {
    let newline = if contents.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let had_trailing_newline = contents.ends_with('\n');
    let mut lines = contents.lines().map(str::to_owned).collect::<Vec<_>>();
    let section_header = format!("[{section}]");
    let section_start = lines
        .iter()
        .rposition(|line| line.trim().eq_ignore_ascii_case(&section_header));

    if let Some(start) = section_start {
        let end = lines[start + 1..]
            .iter()
            .position(|line| {
                let line = line.trim();
                line.starts_with('[') && line.ends_with(']')
            })
            .map(|offset| start + 1 + offset)
            .unwrap_or(lines.len());
        if let Some(index) = (start + 1..end).find(|index| {
            lines[*index]
                .split_once('=')
                .is_some_and(|(candidate, _)| candidate.trim().eq_ignore_ascii_case(key))
        }) {
            lines[index] = format!("{key}={value}");
        } else {
            lines.insert(end, format!("{key}={value}"));
        }
    } else {
        if lines.last().is_some_and(|line| !line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(section_header);
        lines.push(format!("{key}={value}"));
    }

    let mut output = lines.join(newline);
    if had_trailing_newline {
        output.push_str(newline);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updates_existing_ini_values_without_removing_other_keys() {
        let input = "[RACE]\nMODEL=old\nTRACK=track\n\n[OTHER]\nKEEP=1\n";
        let output = set_ini_value(input, "RACE", "MODEL", "new");

        assert!(output.contains("MODEL=new"));
        assert!(output.contains("TRACK=track"));
        assert!(output.contains("[OTHER]\nKEEP=1"));
    }

    #[test]
    fn inserts_missing_ini_sections_and_keys() {
        let output = set_ini_value("[RACE]\nMODEL=car\n", "WIND", "DIRECTION_DEG", "90");

        assert!(output.contains("[WIND]\nDIRECTION_DEG=90"));
    }

    #[test]
    fn maps_supported_single_player_session_modes() {
        assert_eq!(session_mode("race").unwrap(), ("Race", 3, 0));
        assert_eq!(session_mode("drift").unwrap(), ("Drift", 6, 1));
        assert_eq!(session_mode_from_type(5), "time_attack");
        assert!(session_mode("unknown").is_err());
    }

    #[test]
    fn writes_race_details_and_same_car_opponents() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("aclm-race-{unique}"));
        let documents_root = root.join("documents");
        let game_root = root.join("game");
        fs::create_dir_all(documents_root.join("cfg")).unwrap();
        fs::create_dir_all(game_root.join("content/cars/test_car/skins/red")).unwrap();
        fs::create_dir_all(game_root.join("content/cars/test_car/skins/blue")).unwrap();
        fs::create_dir_all(game_root.join("content/tracks/test_track")).unwrap();
        fs::write(
            documents_root.join("cfg/race.ini"),
            "[RACE]\nCARS=1\n[CAR_0]\nMODEL=-\n[SESSION_0]\nTYPE=1\n",
        )
        .unwrap();
        let installation = Installation {
            steam_root: root.clone(),
            library_root: root.clone(),
            game_root,
            proton_prefix: root.clone(),
            documents_root,
            proton_command: root.clone(),
            proton_config: None,
            runtime_root: root.clone(),
            runtime_client: root.clone(),
        };
        let conditions = Conditions {
            session_mode: "race".to_owned(),
            opponents: 2,
            ai_level: 94,
            race_laps: 12,
            penalties: true,
            ..Conditions::default()
        };

        apply_configuration(
            &installation,
            "test_car",
            "red",
            "test_track/layout",
            &conditions,
        )
        .unwrap();
        let output = fs::read_to_string(installation.documents_root.join("cfg/race.ini")).unwrap();

        assert!(output.contains("CARS=3"));
        assert!(output.contains("RACE_LAPS=12"));
        assert!(output.contains("PENALTIES=1"));
        assert!(output.contains("[SESSION_0]\nTYPE=3"));
        assert!(output.contains("[CAR_1]"));
        assert!(output.contains("[CAR_2]"));
        assert!(output.matches("AI_LEVEL=94").count() >= 3);

        apply_dashboard_configuration(&installation, "test_car", "test_track/layout", 18 * 60)
            .unwrap();
        let output = fs::read_to_string(installation.documents_root.join("cfg/race.ini")).unwrap();
        assert!(output.contains("RACE_LAPS=12"));
        assert!(output.contains("SUN_ANGLE=80"));

        fs::remove_dir_all(root).unwrap();
    }
}
