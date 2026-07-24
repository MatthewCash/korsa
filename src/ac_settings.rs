use crate::{configuration, discovery::Installation, preferences, session};
use anyhow::{Context, Result};
use serde::Serialize;
use std::{collections::BTreeMap, fs};

#[derive(Debug, Serialize)]
struct Module {
    file: String,
    name: String,
    description: String,
    sections: Vec<Section>,
}

#[derive(Debug, Serialize)]
struct Section {
    id: String,
    name: String,
    options: Vec<OptionItem>,
}

#[derive(Debug, Serialize)]
struct OptionItem {
    key: String,
    label: String,
    description: String,
    value: String,
    default_value: String,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    step: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decimals: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    multiplier: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct NumericRange {
    minimum: f64,
    maximum: f64,
    step: f64,
    decimals: u8,
    unit: &'static str,
    multiplier: f64,
}

pub fn load_json(installation: &Installation) -> Result<String> {
    let root = installation.documents_root.join("cfg");
    let mut paths = fs::read_dir(&root)
        .with_context(|| format!("could not read {}", root.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ini"))
        .filter(|path| path.file_name().is_some_and(|name| name != "race.ini"))
        .collect::<Vec<_>>();
    paths.sort_by_key(|path| {
        let file = path.file_name().unwrap_or_default().to_string_lossy();
        (module_order(&file), file.to_lowercase())
    });

    let modules = paths
        .into_iter()
        .filter_map(|path| {
            let file = path.file_name()?.to_string_lossy().into_owned();
            let contents = fs::read_to_string(&path).ok()?;
            let defaults = session::read_optional(&installation.game_root.join("cfg").join(&file));
            let sections = parse_sections(&file, &contents, &parse_values(&defaults));
            (!sections.is_empty()).then(|| Module {
                name: module_name(&file),
                description: format!("Assetto Corsa user settings from cfg/{file}"),
                file,
                sections,
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&modules).context("could not serialize Assetto Corsa settings")
}

pub fn set_option(
    installation: &Installation,
    file: &str,
    section: &str,
    key: &str,
    value: &str,
) -> Result<()> {
    anyhow::ensure!(
        !file.is_empty()
            && !file.contains('/')
            && !file.contains('\\')
            && file.ends_with(".ini")
            && file != "race.ini",
        "invalid Assetto Corsa settings filename"
    );
    for (label, candidate) in [("section", section), ("key", key), ("value", value)] {
        anyhow::ensure!(
            !candidate.contains(['\n', '\r']),
            "Assetto Corsa setting {label} contains a newline"
        );
    }
    if let Some(range) = numeric_range(file, section, key) {
        let number = value
            .parse::<f64>()
            .with_context(|| format!("{key} must be a number"))?;
        anyhow::ensure!(
            (range.minimum..=range.maximum).contains(&number),
            "{key} must be between {} and {}",
            range.minimum,
            range.maximum
        );
    }
    let path = installation.documents_root.join("cfg").join(file);
    anyhow::ensure!(
        path.is_file(),
        "unknown Assetto Corsa settings module: {file}"
    );
    let contents =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    let updated = configuration::set_ini_value(&contents, section, key, value);
    preferences::atomic_write(&path, &updated)
}

fn parse_sections(
    file: &str,
    contents: &str,
    defaults: &BTreeMap<(String, String), String>,
) -> Vec<Section> {
    let mut sections = Vec::<Section>::new();
    let mut current_section = String::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if let Some(section) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            current_section = section.to_owned();
            continue;
        }
        if current_section.is_empty() || line.starts_with(';') {
            continue;
        }
        let (assignment, comment) = split_comment(line);
        let Some((key, value)) = assignment.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value.trim().to_owned();
        let default_value = defaults
            .get(&(
                current_section.to_ascii_lowercase(),
                key.to_ascii_lowercase(),
            ))
            .cloned()
            .unwrap_or_else(|| value.clone());
        let range = numeric_range(file, &current_section, key);
        let option = OptionItem {
            key: key.to_owned(),
            label: session::humanize(key),
            description: comment.trim().to_owned(),
            kind: if range.is_some() {
                "slider"
            } else if is_boolean(key, &value) {
                "bool"
            } else {
                "text"
            },
            minimum: range.map(|range| range.minimum),
            maximum: range.map(|range| range.maximum),
            step: range.map(|range| range.step),
            decimals: range.map(|range| range.decimals),
            unit: range.map(|range| range.unit),
            multiplier: range.map(|range| range.multiplier),
            default_value,
            value,
        };
        if let Some(section) = sections
            .iter_mut()
            .find(|section| section.id == current_section)
        {
            section.options.push(option);
        } else {
            sections.push(Section {
                id: current_section.clone(),
                name: session::humanize(&current_section),
                options: vec![option],
            });
        }
    }
    sections
}

fn numeric_range(file: &str, section: &str, key: &str) -> Option<NumericRange> {
    let percent = |minimum, maximum, step, decimals| NumericRange {
        minimum,
        maximum,
        step,
        decimals,
        unit: "%",
        multiplier: if maximum <= 2.0 { 100.0 } else { 1.0 },
    };
    let plain = |minimum, maximum, step, decimals, unit| NumericRange {
        minimum,
        maximum,
        step,
        decimals,
        unit,
        multiplier: 1.0,
    };
    match (
        file,
        section.to_ascii_uppercase().as_str(),
        key.to_ascii_uppercase().as_str(),
    ) {
        ("audio.ini", "LEVELS" | "LEVELS_EXT", _) => Some(percent(0.0, 1.0, 0.01, 2)),
        ("audio.ini", "SKIDS", "ENTRY_POINT") => Some(percent(0.0, 200.0, 1.0, 0)),
        ("video.ini", "SATURATION", "LEVEL") => Some(percent(0.0, 200.0, 1.0, 0)),
        ("video.ini", "ASSETTOCORSA", "WORLD_DETAIL") => Some(plain(0.0, 5.0, 1.0, 0, "")),
        ("video.ini", "CUBEMAP", "FACES_PER_FRAME") => Some(plain(0.0, 6.0, 1.0, 0, "")),
        ("video.ini", "EFFECTS", "SMOKE") => Some(plain(0.0, 5.0, 1.0, 0, "")),
        ("assists.ini", "ASSISTS", "STABILITY_CONTROL") => Some(percent(0.0, 100.0, 1.0, 0)),
        ("camera_onboard.ini", "MODE", "FOV") => Some(plain(10.0, 120.0, 1.0, 0, "°")),
        ("camera_onboard.ini", "GFORCES", "LAG") | ("camera_onboard.ini", "SHAKE", "RANDOM") => {
            Some(percent(0.0, 1.0, 0.01, 2))
        }
        ("camera_onboard.ini", "ROTATION", "HEAD_MAX_DEGREES") => {
            Some(plain(0.0, 180.0, 1.0, 0, "°"))
        }
        _ => None,
    }
}

fn parse_values(contents: &str) -> BTreeMap<(String, String), String> {
    let mut values = BTreeMap::new();
    let mut section = String::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            section = name.to_ascii_lowercase();
            continue;
        }
        if section.is_empty() || line.starts_with(';') {
            continue;
        }
        let (assignment, _) = split_comment(line);
        if let Some((key, value)) = assignment.split_once('=') {
            values.insert(
                (section.clone(), key.trim().to_ascii_lowercase()),
                value.trim().to_owned(),
            );
        }
    }
    values
}

fn split_comment(line: &str) -> (&str, &str) {
    let mut quoted = false;
    for (index, character) in line.char_indices() {
        if character == '"' {
            quoted = !quoted;
        } else if character == ';' && !quoted {
            return (&line[..index], &line[index + 1..]);
        }
    }
    (line, "")
}

fn is_boolean(key: &str, value: &str) -> bool {
    if !matches!(value, "0" | "1") {
        return false;
    }
    let key = key.to_ascii_uppercase();
    key == "ACTIVE"
        || key == "ENABLED"
        || key == "IS_ACTIVE"
        || key == "VISIBLE"
        || key == "BLOCKED"
        || key == "FULLSCREEN"
        || key == "VSYNC"
        || key == "HQ"
        || key == "VISUALDAMAGE"
        || key == "SLIPSTREAM"
        || key == "TYRE_BLANKETS"
        || key.starts_with("USE_")
        || key.starts_with("HIDE_")
        || key.starts_with("SHOW_")
        || key.starts_with("ALLOW_")
        || key.starts_with("DISABLE_")
        || key.starts_with("ENABLE_")
        || key.starts_with("AUTO_")
        || key.starts_with("RENDER_")
        || key.starts_with("LOCK_")
        || key.ends_with("_ENABLED")
}

fn module_order(file: &str) -> usize {
    match file {
        "video.ini" => 0,
        "audio.ini" => 1,
        "controls.ini" => 2,
        "assists.ini" => 3,
        "gameplay.ini" => 4,
        "python.ini" => 5,
        "replay.ini" => 6,
        _ => 100,
    }
}

fn module_name(file: &str) -> String {
    match file {
        "acos.ini" => "In-game Apps Layout".to_owned(),
        "assists.ini" => "Driving Assists".to_owned(),
        "camera_manager.ini" => "Camera Manager".to_owned(),
        "camera_onboard.ini" => "Onboard Camera".to_owned(),
        "controls.ini" => "Controls and Force Feedback".to_owned(),
        "ff_post_process.ini" => "Force Feedback Post-processing".to_owned(),
        "openvr.ini" => "OpenVR".to_owned(),
        "python.ini" => "Python Apps".to_owned(),
        "trackir.ini" => "TrackIR".to_owned(),
        "triple_screen.ini" => "Triple Screen".to_owned(),
        _ => session::humanize(file.trim_end_matches(".ini")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_infers_unambiguous_boolean_values() {
        assert!(is_boolean("ENABLED", "1"));
        assert!(is_boolean("HIDE_ARMS", "0"));
        assert!(!is_boolean("MASTER", "1"));
        assert!(!is_boolean("DAMAGE", "0"));
        assert!(!is_boolean("ENABLED", "0.5"));
    }

    #[test]
    fn parses_all_active_ini_assignments() {
        let sections = parse_sections(
            "video.ini",
            "[VIDEO]\nFULLSCREEN=1\nWIDTH=2560\n; HIDDEN=1\n",
            &BTreeMap::new(),
        );

        assert_eq!(sections[0].options.len(), 2);
        assert_eq!(sections[0].options[0].kind, "bool");
        assert_eq!(sections[0].options[1].kind, "text");
    }

    #[test]
    fn exposes_audio_levels_as_bounded_percentage_sliders() {
        let sections = parse_sections(
            "audio.ini",
            "[LEVELS]\nMASTER=0.75\n[SETTINGS]\nDRIVER_NAME=Default\n",
            &BTreeMap::new(),
        );
        let volume = &sections[0].options[0];

        assert_eq!(volume.kind, "slider");
        assert_eq!(volume.minimum, Some(0.0));
        assert_eq!(volume.maximum, Some(1.0));
        assert_eq!(volume.multiplier, Some(100.0));
        assert_eq!(sections[1].options[0].kind, "text");
    }
}
