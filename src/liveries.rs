use crate::{discovery::Installation, preferences, session::SessionInfo};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs};

/// Kept separately from cached application preferences so dashboard/GUI edits
/// cannot overwrite newer livery history when saving favorites or presets.
#[derive(Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Liveries(BTreeMap<String, String>);

impl Liveries {
    pub fn load(installation: &Installation) -> Result<Self> {
        let path = installation.documents_root.join("cfg/korsa-liveries.json");
        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents)
                .with_context(|| format!("could not read livery history from {}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error).with_context(|| format!("could not read {}", path.display())),
        }
    }

    pub fn save(&self, installation: &Installation) -> Result<()> {
        let path = installation.documents_root.join("cfg/korsa-liveries.json");
        let contents = serde_json::to_string_pretty(self)?;
        if fs::read_to_string(&path).ok().as_deref() != Some(&contents) {
            preferences::atomic_write(&path, &contents).context("could not save livery history")?;
        }
        Ok(())
    }

    /// Explicit choices/presets win. Otherwise keep the current car's skin,
    /// restore this car's history, or use a deterministic installed fallback.
    pub fn resolve<'a>(
        &self,
        car_id: &str,
        installed: &'a [String],
        requested: Option<&str>,
        current: Option<&SessionInfo>,
    ) -> Option<&'a str> {
        let current = current
            .filter(|s| s.car_id == car_id)
            .map(|s| s.skin_id.as_str());
        for candidate in [requested, current, self.0.get(car_id).map(String::as_str)]
            .into_iter()
            .flatten()
        {
            if let Some(skin) = installed.iter().find(|skin| skin.as_str() == candidate) {
                return Some(skin);
            }
        }
        installed.first().map(String::as_str)
    }

    pub fn remember(&mut self, car_id: &str, skin_id: &str) {
        self.0.insert(car_id.to_owned(), skin_id.to_owned());
    }
}

pub fn installed_skins(installation: &Installation, car_id: &str) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_choices_current_session_and_history_have_defined_priority() {
        let installed = vec!["blue".into(), "green".into(), "red".into()];
        let mut liveries = Liveries::default();
        liveries.remember("car_a", "red");
        liveries.remember("car_b", "green");
        let current = SessionInfo {
            car_id: "car_a".into(),
            skin_id: "green".into(),
            ..Default::default()
        };
        assert_eq!(
            liveries.resolve("car_a", &installed, Some("blue"), Some(&current)),
            Some("blue")
        );
        assert_eq!(
            liveries.resolve("car_a", &installed, None, Some(&current)),
            Some("green")
        );
        assert_eq!(
            liveries.resolve("car_b", &installed, None, Some(&current)),
            Some("green")
        );
        assert_eq!(
            liveries.resolve("car_a", &installed, None, None),
            Some("red")
        );
        assert_eq!(
            liveries.resolve("unknown", &installed, None, None),
            Some("blue")
        );
    }

    #[test]
    fn missing_or_invalid_skins_never_resolve_outside_installed_choices() {
        let installed = vec!["blue".into(), "red".into()];
        let mut liveries = Liveries::default();
        liveries.remember("car", "../../elsewhere");
        let current = SessionInfo {
            car_id: "car".into(),
            skin_id: "removed".into(),
            ..Default::default()
        };
        assert_eq!(
            liveries.resolve("car", &installed, Some("missing"), Some(&current)),
            Some("blue")
        );
        assert_eq!(liveries.resolve("car", &[], None, None), None);
    }

    #[test]
    fn history_persists_independently_of_application_preferences() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("korsa-liveries-{unique}"));
        let installation = Installation {
            steam_root: root.clone(),
            library_root: root.clone(),
            game_root: root.clone(),
            proton_prefix: root.clone(),
            documents_root: root.clone(),
            proton_command: root.clone(),
            proton_config: None,
            runtime_root: root.clone(),
            runtime_client: root.clone(),
        };
        let mut liveries = Liveries::load(&installation).unwrap();
        liveries.remember("car_a", "red");
        liveries.remember("car_b", "blue");
        liveries.save(&installation).unwrap();
        let installed = vec!["blue".into(), "red".into()];
        let loaded = Liveries::load(&installation).unwrap();
        assert_eq!(loaded.resolve("car_a", &installed, None, None), Some("red"));
        assert_eq!(
            loaded.resolve("car_b", &installed, None, None),
            Some("blue")
        );
        let path = root.join("cfg/korsa-liveries.json");
        fs::write(&path, "invalid JSON").unwrap();
        assert!(Liveries::load(&installation).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "invalid JSON");
        fs::remove_dir_all(root).unwrap();
    }
}
