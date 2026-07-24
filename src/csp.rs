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
    unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    multiplier: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
struct NumericRange {
    minimum: f64,
    maximum: f64,
    step: f64,
    decimals: u8,
    unit: String,
    multiplier: f64,
}

pub fn load_json(installation: &Installation) -> Result<String> {
    let defaults_root = installation.game_root.join("extension/config");
    let overrides_root = installation.documents_root.join("cfg/extension");
    let mut paths = fs::read_dir(&defaults_root)
        .with_context(|| format!("could not read {}", defaults_root.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ini"))
        .collect::<Vec<_>>();
    paths.sort();

    let mut modules = paths
        .into_iter()
        .filter_map(|path| {
            let file = path.file_name()?.to_string_lossy().into_owned();
            let defaults = fs::read_to_string(&path).ok()?;
            let name = session::ini_value(&defaults, "ℹ", "FULLNAME")?;
            let description = session::ini_value(&defaults, "ℹ", "DESCRIPTION")
                .or_else(|| session::ini_value(&defaults, "ℹ", "SHORT_DESCRIPTION"))
                .unwrap_or_default();
            let overrides = session::read_optional(&overrides_root.join(&file));
            let values = parse_values(&overrides);
            Some(Module {
                file,
                name,
                description,
                sections: parse_sections(&defaults, &values),
            })
        })
        .collect::<Vec<_>>();
    append_weather_implementation_modules(installation, &mut modules);
    modules.sort_by_key(|module| module.name.to_lowercase());
    serde_json::to_string(&modules).context("could not serialize CSP settings")
}

pub fn set_option(
    installation: &Installation,
    file: &str,
    section: &str,
    key: &str,
    value: &str,
) -> Result<()> {
    for (label, candidate) in [("section", section), ("key", key), ("value", value)] {
        anyhow::ensure!(
            !candidate.contains(['\n', '\r']),
            "CSP setting {label} contains a newline"
        );
    }
    let (default_path, path) = if let Some(id) = file.strip_prefix("wfx_impl:") {
        anyhow::ensure!(safe_id(id), "invalid Weather FX implementation ID");
        (
            installation
                .game_root
                .join("extension/weather")
                .join(id)
                .join("settings.ini"),
            installation
                .documents_root
                .join("cfg/extension/state/lua/wfx_impl")
                .join(format!("{id}__settings.ini")),
        )
    } else {
        anyhow::ensure!(
            !file.is_empty()
                && !file.contains('/')
                && !file.contains('\\')
                && file.ends_with(".ini"),
            "invalid CSP settings filename"
        );
        (
            installation.game_root.join("extension/config").join(file),
            installation.documents_root.join("cfg/extension").join(file),
        )
    };
    anyhow::ensure!(
        default_path.is_file(),
        "unknown CSP settings module: {file}"
    );
    let defaults = fs::read_to_string(&default_path)
        .with_context(|| format!("could not read {}", default_path.display()))?;
    if let Some(option) = parse_sections(&defaults, &BTreeMap::new())
        .into_iter()
        .find(|candidate| candidate.id.eq_ignore_ascii_case(section))
        .and_then(|candidate| {
            candidate
                .options
                .into_iter()
                .find(|option| option.key.eq_ignore_ascii_case(key))
        })
        && let (Some(minimum), Some(maximum)) = (option.minimum, option.maximum)
    {
        let number = value
            .parse::<f64>()
            .with_context(|| format!("{key} must be a number"))?;
        anyhow::ensure!(
            (minimum..=maximum).contains(&number),
            "{key} must be between {minimum} and {maximum}"
        );
    }
    let contents = session::read_optional(&path);
    let updated = configuration::set_ini_value(&contents, section, key, value);
    preferences::atomic_write(&path, &updated)
}

fn append_weather_implementation_modules(installation: &Installation, modules: &mut Vec<Module>) {
    let root = installation.game_root.join("extension/weather");
    let overrides_root = installation
        .documents_root
        .join("cfg/extension/state/lua/wfx_impl");
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        let Some(id) = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };
        let defaults = session::read_optional(&path.join("settings.ini"));
        if defaults.is_empty() {
            continue;
        }
        let manifest = session::read_optional(&path.join("manifest.ini"));
        let name = session::ini_value(&manifest, "ABOUT", "NAME")
            .unwrap_or_else(|| session::humanize(&id));
        let description = session::ini_value(&manifest, "ABOUT", "DESCRIPTION").unwrap_or_default();
        let overrides = session::read_optional(&overrides_root.join(format!("{id}__settings.ini")));
        modules.push(Module {
            file: format!("wfx_impl:{id}"),
            name: format!("Weather FX: {name}"),
            description,
            sections: parse_sections(&defaults, &parse_values(&overrides)),
        });
    }
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn parse_sections(contents: &str, overrides: &BTreeMap<(String, String), String>) -> Vec<Section> {
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
        if current_section.is_empty() || current_section == "ℹ" || line.starts_with(';') {
            continue;
        }
        let (assignment, comment) = split_comment(line);
        let Some((key, default_value)) = assignment.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || key.starts_with("__") {
            continue;
        }
        let metadata = comment
            .split(';')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if metadata
            .iter()
            .any(|part| part.eq_ignore_ascii_case("hidden"))
        {
            continue;
        }
        let range = numeric_range(&metadata);
        let kind = if metadata
            .iter()
            .any(|part| part.eq_ignore_ascii_case("1 or 0"))
        {
            "bool"
        } else if range.is_some() {
            "slider"
        } else if metadata.iter().any(|part| {
            part.to_ascii_lowercase().starts_with("from ")
                || part.to_ascii_lowercase().contains(" values from ")
        }) {
            "number"
        } else {
            "text"
        };
        let label = metadata
            .iter()
            .find(|part| !is_type_metadata(part))
            .map(|part| (*part).to_owned())
            .unwrap_or_else(|| session::humanize(key));
        let default_value = default_value.trim().to_owned();
        let value = overrides
            .get(&(
                current_section.to_ascii_lowercase(),
                key.to_ascii_lowercase(),
            ))
            .cloned()
            .unwrap_or_else(|| default_value.clone());
        let option = OptionItem {
            key: key.to_owned(),
            label,
            description: metadata.join("; "),
            value,
            default_value,
            kind,
            minimum: range.as_ref().map(|range| range.minimum),
            maximum: range.as_ref().map(|range| range.maximum),
            step: range.as_ref().map(|range| range.step),
            decimals: range.as_ref().map(|range| range.decimals),
            unit: range.as_ref().map(|range| range.unit.clone()),
            multiplier: range.as_ref().map(|range| range.multiplier),
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
    sections.retain(|section| !section.options.is_empty());
    sections
}

fn numeric_range(metadata: &[&str]) -> Option<NumericRange> {
    let range_text = metadata.iter().find_map(|part| {
        let normalized = part.to_ascii_lowercase();
        let start = normalized.find("from ")? + "from ".len();
        Some(&part[start..])
    })?;
    let normalized = range_text.to_ascii_lowercase();
    let split = normalized.find(" to ")?;
    let minimum_text = &range_text[..split];
    let maximum_text = &range_text[split + " to ".len()..];
    let (minimum, unit_hint) = first_number(minimum_text)?;
    let (maximum, _) = first_number(maximum_text)?;
    if minimum >= maximum {
        return None;
    }
    let all_metadata = metadata.join(" ").to_ascii_lowercase();
    let percentage = all_metadata.contains("perc");
    let explicit_step = all_metadata
        .find("round to ")
        .and_then(|start| first_number(&all_metadata[start + "round to ".len()..]))
        .map(|(value, _)| value)
        .filter(|value| *value > 0.0);
    let step = explicit_step.unwrap_or({
        if percentage || maximum - minimum <= 2.0 {
            0.01
        } else if maximum - minimum <= 10.0 {
            0.1
        } else {
            1.0
        }
    });
    Some(NumericRange {
        minimum,
        maximum,
        step,
        decimals: decimals_for_step(step),
        unit: if percentage {
            "%".to_owned()
        } else {
            unit_hint
        },
        multiplier: if percentage { 100.0 } else { 1.0 },
    })
}

fn first_number(value: &str) -> Option<(f64, String)> {
    let start = value
        .char_indices()
        .find(|(_, character)| character.is_ascii_digit() || matches!(character, '-' | '+' | '.'))?
        .0;
    let end = value[start..]
        .char_indices()
        .find(|(_, character)| {
            !character.is_ascii_digit() && !matches!(character, '-' | '+' | '.' | 'e' | 'E')
        })
        .map_or(value.len(), |(offset, _)| start + offset);
    let number = value[start..end].parse::<f64>().ok()?;
    let unit = value[end..]
        .trim()
        .trim_matches(|character: char| character == ',' || character == '.')
        .trim()
        .to_owned();
    Some((number, unit))
}

fn decimals_for_step(step: f64) -> u8 {
    for decimals in 0..=6 {
        if (step * 10_f64.powi(decimals)).fract().abs() < 1e-8 {
            return decimals as u8;
        }
    }
    6
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

fn is_type_metadata(part: &str) -> bool {
    let normalized = part.to_ascii_lowercase();
    normalized == "1 or 0"
        || normalized == "hidden"
        || normalized.starts_with("from ")
        || normalized.contains(" values from ")
        || normalized.starts_with("visible with")
        || normalized.starts_with("hidden with")
        || normalized.starts_with("only with")
        || normalized.starts_with("not available with")
        || matches!(
            normalized.as_str(),
            "color" | "color with alpha" | "text" | "two numbers" | "3d-offset"
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_semicolons_inside_quoted_values() {
        assert_eq!(
            split_comment("SEPARATOR=\";\" ; Character; text"),
            ("SEPARATOR=\";\" ", " Character; text")
        );
    }

    #[test]
    fn parses_boolean_metadata_and_ignores_hidden_options() {
        let input = "[BASIC]\nENABLED=1 ; Active; 1 or 0\nSECRET=1 ; hidden\n";
        let sections = parse_sections(input, &BTreeMap::new());

        assert_eq!(sections[0].options.len(), 1);
        assert_eq!(sections[0].options[0].kind, "bool");
    }

    #[test]
    fn parses_bounded_numeric_metadata_for_sliders() {
        let sections = parse_sections(
            "[BASIC]\nGAIN=0.8 ; Gain; from 0 to 2, perc.; round to 0.1\n",
            &BTreeMap::new(),
        );
        let option = &sections[0].options[0];

        assert_eq!(option.kind, "slider");
        assert_eq!(option.minimum, Some(0.0));
        assert_eq!(option.maximum, Some(2.0));
        assert_eq!(option.step, Some(0.1));
        assert_eq!(option.unit.as_deref(), Some("%"));
        assert_eq!(option.multiplier, Some(100.0));
    }

    #[test]
    fn preserves_units_from_numeric_metadata() {
        let range = numeric_range(&["Full blur speed", "from 40 km/h to 120"]).unwrap();

        assert_eq!(range.minimum, 40.0);
        assert_eq!(range.maximum, 120.0);
        assert_eq!(range.step, 1.0);
        assert_eq!(range.unit, "km/h");
    }
}
