use crate::{configuration, discovery::Installation, preferences, session};
use anyhow::{Context, Result};
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::PathBuf};

#[derive(Debug, Serialize)]
struct LayoutModel {
    selected_desktop: i32,
    screen_width: i32,
    screen_height: i32,
    apps: Vec<AppWindow>,
}

#[derive(Clone, Debug, Serialize)]
struct AppWindow {
    section: String,
    desktop: i32,
    id: String,
    name: String,
    x: i32,
    y: i32,
    visible: bool,
    blocked: bool,
    scale: f64,
}

#[derive(Clone, Debug)]
struct IniSection {
    name: String,
    values: BTreeMap<String, String>,
}

pub fn load_json(installation: &Installation) -> Result<String> {
    let path = layout_path(installation);
    let contents =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    let sections = parse_sections(&contents);
    let video = session::read_optional(&installation.documents_root.join("cfg/video.ini"));
    let screen_width = positive_dimension(&video, "VIDEO", "WIDTH", 1920);
    let screen_height = positive_dimension(&video, "VIDEO", "HEIGHT", 1080);
    let selected_desktop = value(&sections, "HEADER", "DESKTOP_SELECTED", "1")
        .parse::<i32>()
        .unwrap_or(1)
        .clamp(1, 4);

    let mut apps = sections
        .iter()
        .filter_map(app_window)
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect::<Vec<_>>();
    apps.sort_by_key(|app| (app.desktop, app.name.to_ascii_lowercase()));

    serde_json::to_string(&LayoutModel {
        selected_desktop,
        screen_width,
        screen_height,
        apps,
    })
    .context("could not serialize in-game app layout")
}

pub fn set_option(
    installation: &Installation,
    section: &str,
    key: &str,
    value: &str,
) -> Result<()> {
    anyhow::ensure!(
        key == "DESKTOP_SELECTED"
            || matches!(key, "VISIBLE" | "BLOCKED" | "SCALE" | "POSX" | "POSY"),
        "unsupported in-game app layout option"
    );
    if key == "DESKTOP_SELECTED" {
        anyhow::ensure!(section == "HEADER", "invalid desktop selection section");
        let desktop = value.parse::<i32>().context("desktop must be a number")?;
        anyhow::ensure!(
            (1..=4).contains(&desktop),
            "desktop must be between 1 and 4"
        );
    } else {
        anyhow::ensure!(
            section_parts(section).is_some(),
            "invalid app layout section"
        );
        match key {
            "VISIBLE" | "BLOCKED" => {
                anyhow::ensure!(matches!(value, "0" | "1"), "toggle must be 0 or 1");
            }
            "SCALE" => {
                let scale = value.parse::<f64>().context("scale must be a number")?;
                anyhow::ensure!((0.1..=4.0).contains(&scale), "scale is out of range");
            }
            "POSX" | "POSY" => {
                let position = value.parse::<i32>().context("position must be a number")?;
                anyhow::ensure!(
                    (-20_000..=20_000).contains(&position),
                    "position is out of range"
                );
            }
            _ => unreachable!(),
        }
    }
    let path = layout_path(installation);
    let contents =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    anyhow::ensure!(
        parse_sections(&contents)
            .iter()
            .any(|candidate| candidate.name.eq_ignore_ascii_case(section)),
        "unknown in-game app layout section"
    );
    let updated = configuration::set_ini_value(&contents, section, key, value);
    preferences::atomic_write(&path, &updated)
}

pub fn move_window(installation: &Installation, section: &str, x: i32, y: i32) -> Result<()> {
    anyhow::ensure!(
        section_parts(section).is_some(),
        "invalid app layout section"
    );
    anyhow::ensure!(
        (-20_000..=20_000).contains(&x),
        "horizontal position is out of range"
    );
    anyhow::ensure!(
        (-20_000..=20_000).contains(&y),
        "vertical position is out of range"
    );
    let path = layout_path(installation);
    let contents =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    anyhow::ensure!(
        parse_sections(&contents)
            .iter()
            .any(|candidate| candidate.name.eq_ignore_ascii_case(section)),
        "unknown in-game app layout section"
    );
    let updated = configuration::set_ini_value(
        &configuration::set_ini_value(&contents, section, "POSX", &x.to_string()),
        section,
        "POSY",
        &y.to_string(),
    );
    preferences::atomic_write(&path, &updated)
}

fn layout_path(installation: &Installation) -> PathBuf {
    installation.documents_root.join("cfg/acos.ini")
}

fn parse_sections(contents: &str) -> Vec<IniSection> {
    let mut sections = Vec::<IniSection>::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            sections.push(IniSection {
                name: name.to_owned(),
                values: BTreeMap::new(),
            });
            continue;
        }
        let Some(section) = sections.last_mut() else {
            continue;
        };
        if let Some((key, value)) = line.split_once('=') {
            section
                .values
                .insert(key.trim().to_ascii_uppercase(), value.trim().to_owned());
        }
    }
    sections
}

fn value(sections: &[IniSection], section_name: &str, key: &str, default: &str) -> String {
    sections
        .iter()
        .rev()
        .find(|section| section.name.eq_ignore_ascii_case(section_name))
        .and_then(|section| section.values.get(key))
        .cloned()
        .unwrap_or_else(|| default.to_owned())
}

fn app_window(section: &IniSection) -> Option<((i32, String), AppWindow)> {
    let (desktop, id) = section_parts(&section.name)?;
    let app = AppWindow {
        section: section.name.clone(),
        desktop,
        name: app_name(id),
        id: id.to_owned(),
        x: section
            .values
            .get("POSX")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        y: section
            .values
            .get("POSY")
            .and_then(|value| value.parse().ok())
            .unwrap_or(80),
        visible: section
            .values
            .get("VISIBLE")
            .is_some_and(|value| value == "1"),
        blocked: section
            .values
            .get("BLOCKED")
            .is_some_and(|value| value == "1"),
        scale: section
            .values
            .get("SCALE")
            .and_then(|value| value.parse().ok())
            .unwrap_or(1.0),
    };
    Some(((desktop, id.to_ascii_lowercase()), app))
}

fn section_parts(section: &str) -> Option<(i32, &str)> {
    let remainder = section.strip_prefix("DESK_")?;
    let (desktop, id) = remainder.split_once("_FORM_")?;
    let desktop = desktop.parse::<i32>().ok()?;
    ((0..4).contains(&desktop) && !id.is_empty()).then_some((desktop, id))
}

fn app_name(id: &str) -> String {
    let cleaned = id
        .strip_prefix("IMGUI_CSP_LUA_")
        .or_else(|| id.strip_prefix("IMGUI_LUA_"))
        .or_else(|| id.strip_prefix("IMGUI_CSP_"))
        .unwrap_or(id);
    let cleaned = cleaned.strip_suffix("_main").unwrap_or(cleaned);
    session::humanize(cleaned)
        .split_whitespace()
        .map(|word| {
            let mut characters = word.chars();
            characters
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + characters.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn positive_dimension(contents: &str, section: &str, key: &str, default: i32) -> i32 {
    session::ini_value(contents, section, key)
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_desktops_and_readable_app_names() {
        let sections = parse_sections(
            "[HEADER]\nDESKTOP_SELECTED=2\n[DESK_1_FORM_IMGUI_LUA_comfy map_main]\nPOSX=100\nPOSY=200\nVISIBLE=1\nBLOCKED=0\nSCALE=0.8\n",
        );
        let (_, app) = app_window(&sections[1]).unwrap();

        assert_eq!(app.desktop, 1);
        assert_eq!(app.name, "Comfy Map");
        assert_eq!((app.x, app.y), (100, 200));
        assert!(app.visible);
    }

    #[test]
    fn rejects_non_layout_sections() {
        assert_eq!(section_parts("DESK_0_FORM_PEDALS"), Some((0, "PEDALS")));
        assert_eq!(section_parts("HEADER"), None);
        assert_eq!(section_parts("DESK_8_FORM_BAD"), None);
    }
}
