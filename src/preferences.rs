use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Preferences {
    #[serde(default)]
    pub favorite_cars: BTreeSet<String>,
    #[serde(default)]
    pub favorite_tracks: BTreeSet<String>,
    #[serde(default)]
    pub presets: Vec<SessionPreset>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionPreset {
    pub id: String,
    pub name: String,
    pub car_id: String,
    pub skin_id: String,
    pub track_id: String,
    pub track_layout: String,
    pub conditions: Conditions,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Conditions {
    pub session_mode: String,
    pub session_duration: i32,
    pub race_laps: i32,
    pub opponents: i32,
    pub ai_level: i32,
    pub penalties: bool,
    pub weather_id: String,
    pub sun_angle: f64,
    pub time_multiplier: f64,
    pub cloud_speed: f64,
    pub ambient_temperature: i32,
    pub road_temperature: i32,
    pub wind_speed_min: i32,
    pub wind_speed_max: i32,
    pub wind_direction: i32,
    pub session_start_grip: i32,
    pub session_transfer: i32,
    pub randomness: i32,
    pub lap_gain: i32,
    pub virtual_laps: i32,
    pub max_laps: i32,
    pub starting_laps: i32,
    pub weather_controller: String,
    pub weather_type: i32,
}

impl Default for Conditions {
    fn default() -> Self {
        Self {
            session_mode: "practice".to_owned(),
            session_duration: 0,
            race_laps: 2,
            opponents: 0,
            ai_level: 98,
            penalties: false,
            weather_id: "3_clear".to_owned(),
            sun_angle: 0.0,
            time_multiplier: 0.0,
            cloud_speed: 0.2,
            ambient_temperature: 22,
            road_temperature: 28,
            wind_speed_min: 0,
            wind_speed_max: 0,
            wind_direction: 0,
            session_start_grip: 98,
            session_transfer: 80,
            randomness: 2,
            lap_gain: 700,
            virtual_laps: 10,
            max_laps: 30,
            starting_laps: 0,
            weather_controller: "base".to_owned(),
            weather_type: 15,
        }
    }
}

impl Preferences {
    pub fn load() -> Result<Self> {
        let path = path()?;
        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents)
                .with_context(|| format!("could not parse {}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error).with_context(|| format!("could not read {}", path.display())),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = path()?;
        let parent = path
            .parent()
            .context("preferences path has no parent directory")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("could not create {}", parent.display()))?;
        let temporary = path.with_extension("json.tmp");
        let contents = serde_json::to_string_pretty(self)
            .context("could not serialize application preferences")?;
        fs::write(&temporary, contents)
            .with_context(|| format!("could not write {}", temporary.display()))?;
        fs::rename(&temporary, &path)
            .with_context(|| format!("could not replace {}", path.display()))
    }

    pub fn toggle_favorite(&mut self, kind: &str, id: &str) -> Result<bool> {
        let favorites = match kind {
            "car" => &mut self.favorite_cars,
            "track" => &mut self.favorite_tracks,
            _ => anyhow::bail!("unknown favorite type: {kind}"),
        };
        let favorite = if favorites.remove(id) {
            false
        } else {
            favorites.insert(id.to_owned());
            true
        };
        self.save()?;
        Ok(favorite)
    }
}

fn path() -> Result<PathBuf> {
    if let Some(root) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(root)
            .join("ac-linux-manager")
            .join("preferences.json"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| {
            home.join(".config")
                .join("ac-linux-manager")
                .join("preferences.json")
        })
        .context("neither XDG_CONFIG_HOME nor HOME is set")
}

pub fn atomic_write(path: &Path, contents: &str) -> Result<()> {
    let parent = path
        .parent()
        .context("configuration path has no parent directory")?;
    fs::create_dir_all(parent).with_context(|| format!("could not create {}", parent.display()))?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, contents)
        .with_context(|| format!("could not write {}", temporary.display()))?;
    fs::rename(&temporary, path).with_context(|| format!("could not replace {}", path.display()))
}
