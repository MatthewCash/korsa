use crate::discovery::Installation;
use anyhow::{Context, Result};
use std::{fs, path::Path};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionInfo {
    pub car_name: String,
    pub car_brand: String,
    pub car_id: String,
    pub skin_id: String,
    pub skin_name: String,
    pub track_name: String,
    pub track_location: String,
    pub track_id: String,
    pub track_layout_id: String,
    pub track_layout: String,
    pub weather_name: String,
    pub session_type: String,
    pub car_preview: String,
    pub track_preview: String,
    pub track_outline: String,
    pub csp_status: String,
}

pub fn load(installation: &Installation) -> Result<SessionInfo> {
    let race_path = installation.documents_root.join("cfg/race.ini");
    let race = fs::read_to_string(&race_path)
        .with_context(|| format!("could not read {}", race_path.display()))?;

    let car_id = ini_value(&race, "RACE", "MODEL").unwrap_or_default();
    let skin_id = configured_skin(&race);
    let track_id = ini_value(&race, "RACE", "TRACK").unwrap_or_default();
    let track_layout = ini_value(&race, "RACE", "CONFIG_TRACK").unwrap_or_default();
    let weather_id = ini_value(&race, "WEATHER", "NAME").unwrap_or_default();
    let session_type = ini_value(&race, "SESSION_0", "NAME")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Session".to_owned());

    let car_root = installation.game_root.join("content/cars").join(&car_id);
    let car_ui = read_optional(&car_root.join("ui/ui_car.json"));
    let car_name = json_string(&car_ui, "name").unwrap_or_else(|| humanize(&car_id));
    let car_brand = json_string(&car_ui, "brand").unwrap_or_default();
    let car_preview = file_url(&car_root.join("skins").join(&skin_id).join("preview.jpg"));

    let track_root = installation
        .game_root
        .join("content/tracks")
        .join(&track_id);
    let base_ui = track_root.join("ui");
    let selected_ui = if track_layout.is_empty() {
        base_ui.clone()
    } else {
        base_ui.join(&track_layout)
    };
    let track_ui_path = if selected_ui.join("ui_track.json").is_file() {
        selected_ui.join("ui_track.json")
    } else {
        base_ui.join("ui_track.json")
    };
    let track_ui = read_optional(&track_ui_path);
    let track_name = json_string(&track_ui, "name").unwrap_or_else(|| humanize(&track_id));
    let track_location = [
        json_string(&track_ui, "city"),
        json_string(&track_ui, "country"),
    ]
    .into_iter()
    .flatten()
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>()
    .join(", ");

    let manifest = read_optional(
        &installation
            .game_root
            .join("extension/config/data_manifest.ini"),
    );
    let csp_version = ini_value(&manifest, "VERSION", "SHADERS_PATCH");
    let csp_build = ini_value(&manifest, "VERSION", "SHADERS_PATCH_BUILD");
    let csp_status = match (csp_version, csp_build) {
        (Some(version), Some(build)) => format!("Installed - v{version} (build {build})"),
        (Some(version), None) => format!("Installed - v{version}"),
        _ if installation.game_root.join("dwrite.dll").is_file() => "Installed".to_owned(),
        _ => "Not detected".to_owned(),
    };

    Ok(SessionInfo {
        car_name,
        car_brand,
        car_id,
        skin_id: skin_id.clone(),
        skin_name: humanize(&skin_id),
        track_name,
        track_location,
        track_id,
        track_layout_id: track_layout.clone(),
        track_layout: humanize(&track_layout),
        weather_name: humanize(&weather_id),
        session_type,
        car_preview,
        track_preview: first_existing(&[
            selected_ui.join("preview.png"),
            base_ui.join("preview.png"),
        ]),
        track_outline: first_existing(&[
            selected_ui.join("outline_cropped.png"),
            selected_ui.join("outline.png"),
        ]),
        csp_status,
    })
}

pub(crate) fn read_optional(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

pub(crate) fn file_url(path: &Path) -> String {
    if path.is_file() {
        url::Url::from_file_path(path)
            .map(|url| url.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    }
}

fn first_existing(paths: &[std::path::PathBuf]) -> String {
    paths
        .iter()
        .find(|path| path.is_file())
        .map(|path| file_url(path))
        .unwrap_or_default()
}

pub(crate) fn ini_value(contents: &str, section: &str, key: &str) -> Option<String> {
    let mut current_section = "";

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            current_section = name;
            continue;
        }
        if current_section != section || line.starts_with(';') {
            continue;
        }

        let Some((candidate, value)) = line.split_once('=') else {
            continue;
        };
        if candidate.trim() == key {
            return Some(value.trim().to_owned());
        }
    }

    None
}

fn configured_skin(race: &str) -> String {
    ini_value(race, "RACE", "SKIN")
        .filter(|value| !value.is_empty())
        .or_else(|| ini_value(race, "CAR_0", "SKIN").filter(|value| !value.is_empty()))
        .unwrap_or_default()
}

pub(crate) fn json_string(contents: &str, key: &str) -> Option<String> {
    let (_, after_key) = contents.split_once(&format!("\"{key}\""))?;
    let (_, value) = after_key.split_once(':')?;
    let mut characters = value.trim_start().strip_prefix('"')?.chars();
    let mut parsed = String::new();
    let mut escaped = false;

    for character in characters.by_ref() {
        if escaped {
            parsed.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some(parsed);
        } else {
            parsed.push(character);
        }
    }

    None
}

pub(crate) fn humanize(identifier: &str) -> String {
    identifier
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            if part.chars().all(|character| character.is_ascii_digit()) || part.len() <= 2 {
                part.to_ascii_uppercase()
            } else {
                let mut characters = part.chars();
                characters
                    .next()
                    .map(|first| first.to_ascii_uppercase().to_string() + characters.as_str())
                    .unwrap_or_default()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_values_from_the_requested_ini_section() {
        let input = "[RACE]\nMODEL=car_one\n[CAR_0]\nMODEL=-\n";

        assert_eq!(
            ini_value(input, "RACE", "MODEL").as_deref(),
            Some("car_one")
        );
    }

    #[test]
    fn falls_back_to_player_car_skin_when_race_skin_is_missing() {
        let input = "[RACE]\nMODEL=car_one\nSKIN=\n[CAR_0]\nSKIN=blue\n";

        assert_eq!(configured_skin(input), "blue");
    }

    #[test]
    fn reads_json_strings_without_requiring_valid_full_json() {
        let input = "{\n  \"name\": \"Corvette C7\",\n  \"description\": \"line one\nline two\"\n}";

        assert_eq!(json_string(input, "name").as_deref(), Some("Corvette C7"));
    }

    #[test]
    fn humanizes_asset_identifiers() {
        assert_eq!(humanize("01_torch_red"), "01 Torch Red");
        assert_eq!(humanize("tatsumi_pa"), "Tatsumi PA");
    }
}
