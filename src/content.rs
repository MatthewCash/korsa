use crate::{
    configuration,
    discovery::Installation,
    liveries::Liveries,
    preferences::{DASHBOARD_CAPACITY, Preferences},
    session,
};
use anyhow::{Context, Result};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Default)]
pub struct Catalog {
    pub cars_json: String,
    pub tracks_json: String,
    pub replays_json: String,
    pub car_count: i32,
    pub track_count: i32,
    pub replay_count: i32,
    cars: Vec<CatalogItem>,
    tracks: Vec<CatalogItem>,
}

#[derive(Debug, Default, Serialize)]
struct CatalogItem {
    id: String,
    name: String,
    subtitle: String,
    preview: String,
    favorite: bool,
    dashboard: bool,
    parent: String,
    transmission: String,
    skin: String,
    author: String,
    description: String,
    tags: String,
    class_name: String,
    power: String,
    torque: String,
    weight: String,
    top_speed: String,
    acceleration: String,
    power_weight: String,
    drivetrain: String,
    version: String,
    country: String,
    city: String,
    length: String,
    width: String,
    pitboxes: String,
    direction: String,
    year: String,
    outline: String,
    skins: Vec<SkinItem>,
}

#[derive(Debug, Default, Serialize)]
struct SkinItem {
    id: String,
    name: String,
    preview: String,
}

pub fn load(installation: &Installation, preferences: &Preferences) -> Result<Catalog> {
    let liveries = Liveries::load(installation)?;
    let current = session::load(installation).ok();
    let cars = load_cars(
        &installation.game_root.join("content/cars"),
        &preferences.favorite_cars,
        &preferences.dashboard_cars,
        &liveries,
        current.as_ref(),
    )?;
    let tracks = load_tracks(
        &installation.game_root.join("content/tracks"),
        &preferences.favorite_tracks,
        &preferences.dashboard_tracks,
    )?;
    let replays = load_replays(&installation.documents_root.join("replay"));

    Ok(Catalog {
        car_count: count(&cars),
        track_count: count(&tracks),
        replay_count: count(&replays),
        cars_json: serde_json::to_string(&cars).context("could not serialize car catalog")?,
        tracks_json: serde_json::to_string(&tracks).context("could not serialize track catalog")?,
        replays_json: serde_json::to_string(&replays)
            .context("could not serialize replay catalog")?,
        cars,
        tracks,
    })
}

pub fn dashboard_json(installation: &Installation, preferences: &Preferences) -> Result<String> {
    #[derive(Serialize)]
    struct DashboardOptions<'a> {
        cars: Vec<DashboardItem<'a>>,
        tracks: Vec<DashboardItem<'a>>,
        times: [DashboardTime; 4],
        selection: configuration::DashboardSelection,
        capacity: usize,
    }

    #[derive(Serialize)]
    struct DashboardItem<'a> {
        id: &'a str,
        name: &'a str,
        preview: &'a str,
        outline: &'a str,
        skin: &'a str,
        skin_name: &'a str,
    }

    #[derive(Serialize)]
    struct DashboardTime {
        name: &'static str,
        minutes: i32,
    }

    let catalog = load(installation, preferences)?;
    serde_json::to_string(&DashboardOptions {
        selection: configuration::dashboard_selection(installation)?,
        capacity: DASHBOARD_CAPACITY,
        cars: catalog
            .cars
            .iter()
            .filter(|item| item.dashboard)
            .map(|item| DashboardItem {
                id: &item.id,
                name: &item.name,
                preview: &item.preview,
                outline: &item.outline,
                skin: &item.skin,
                skin_name: item
                    .skins
                    .iter()
                    .find(|skin| skin.id == item.skin)
                    .map_or("", |skin| skin.name.as_str()),
            })
            .collect(),
        tracks: catalog
            .tracks
            .iter()
            .filter(|item| item.dashboard)
            .map(|item| DashboardItem {
                id: &item.id,
                name: &item.name,
                preview: &item.preview,
                outline: &item.outline,
                skin: "",
                skin_name: "",
            })
            .collect(),
        times: [
            DashboardTime {
                name: "Dawn",
                minutes: 6 * 60,
            },
            DashboardTime {
                name: "Noon",
                minutes: 12 * 60,
            },
            DashboardTime {
                name: "Sunset",
                minutes: 18 * 60,
            },
            DashboardTime {
                name: "Midnight",
                minutes: 0,
            },
        ],
    })
    .context("could not serialize dashboard choices")
}

impl Catalog {
    pub fn set_livery(&mut self, car_id: &str, skin_id: &str) -> Result<bool> {
        let Some(car) = self.cars.iter_mut().find(|car| car.id == car_id) else {
            return Ok(false);
        };
        let Some(skin) = car.skins.iter().find(|skin| skin.id == skin_id) else {
            return Ok(false);
        };
        if car.skin == skin_id && car.preview == skin.preview {
            return Ok(false);
        }
        car.skin = skin.id.clone();
        car.preview = skin.preview.clone();
        self.cars_json = serde_json::to_string(&self.cars)?;
        Ok(true)
    }

    pub fn contains(&self, kind: &str, id: &str) -> bool {
        match kind {
            "car" => &self.cars,
            "track" => &self.tracks,
            _ => return false,
        }
        .iter()
        .any(|item| item.id == id)
    }

    pub fn set_favorite(&mut self, kind: &str, id: &str, favorite: bool) -> Result<()> {
        self.set_flag(kind, id, "favorite", favorite)
    }

    pub fn set_dashboard(&mut self, kind: &str, id: &str, selected: bool) -> Result<()> {
        self.set_flag(kind, id, "dashboard", selected)
    }

    fn set_flag(&mut self, kind: &str, id: &str, flag: &str, value: bool) -> Result<()> {
        let items = match kind {
            "car" => &mut self.cars,
            "track" => &mut self.tracks,
            _ => anyhow::bail!("unknown favorite type: {kind}"),
        };
        let mut matched = false;
        for item in items.iter_mut() {
            if item.id == id {
                match flag {
                    "favorite" => item.favorite = value,
                    "dashboard" => item.dashboard = value,
                    _ => unreachable!(),
                }
                matched = true;
            }
        }
        anyhow::ensure!(matched, "catalog item not found: {id}");
        sort_items(items);
        match kind {
            "car" => self.cars_json = serde_json::to_string(items)?,
            "track" => self.tracks_json = serde_json::to_string(items)?,
            _ => unreachable!(),
        }
        Ok(())
    }
}

fn load_cars(
    root: &Path,
    favorites: &std::collections::BTreeSet<String>,
    dashboard: &std::collections::BTreeSet<String>,
    liveries: &Liveries,
    current: Option<&session::SessionInfo>,
) -> Result<Vec<CatalogItem>> {
    let mut cars = directories(root)?
        .into_iter()
        .filter_map(|car_root| {
            let id = file_name(&car_root)?;
            let metadata = session::read_optional(&car_root.join("ui/ui_car.json"));
            let name =
                session::json_string(&metadata, "name").unwrap_or_else(|| session::humanize(&id));
            let brand = session::json_string(&metadata, "brand").unwrap_or_default();
            let skin_roots = directories(&car_root.join("skins")).unwrap_or_default();
            let skins = skin_roots
                .iter()
                .filter_map(|skin_root| {
                    let id = file_name(skin_root)?;
                    let metadata = session::read_optional(&skin_root.join("ui_skin.json"));
                    let name = session::json_string(&metadata, "skinname")
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or_else(|| session::humanize(&id));
                    Some(SkinItem {
                        id,
                        name,
                        preview: session::first_existing(&[
                            skin_root.join("preview.jpg"),
                            skin_root.join("preview.png"),
                        ]),
                    })
                })
                .collect::<Vec<_>>();
            let installed = skins.iter().map(|skin| skin.id.clone()).collect::<Vec<_>>();
            let skin = liveries
                .resolve(&id, &installed, None, current)
                .unwrap_or_default()
                .to_owned();
            let preview = skins
                .iter()
                .find(|candidate| candidate.id == skin)
                .map(|skin| skin.preview.clone())
                .unwrap_or_default();
            let tags = json_strings(&metadata, "tags");
            let transmission = ["manual", "semiautomatic", "automatic", "sequential"]
                .into_iter()
                .filter(|candidate| tags.iter().any(|tag| tag.eq_ignore_ascii_case(candidate)))
                .collect::<Vec<_>>()
                .join(",");
            let drivetrain = ["awd", "4wd", "rwd", "fwd"]
                .into_iter()
                .find(|candidate| tags.iter().any(|tag| tag.eq_ignore_ascii_case(candidate)))
                .map(str::to_ascii_uppercase)
                .unwrap_or_default();

            Some(CatalogItem {
                favorite: favorites.contains(&id),
                dashboard: dashboard.contains(&id),
                parent: session::json_string(&metadata, "parent").unwrap_or_default(),
                id,
                name,
                subtitle: brand,
                preview,
                transmission,
                skin,
                skins,
                author: session::json_string(&metadata, "author").unwrap_or_default(),
                description: plain_text(
                    &session::json_string(&metadata, "description").unwrap_or_default(),
                ),
                tags: tags.join(", "),
                class_name: session::json_string(&metadata, "class").unwrap_or_default(),
                power: session::json_string(&metadata, "bhp").unwrap_or_default(),
                torque: session::json_string(&metadata, "torque").unwrap_or_default(),
                weight: session::json_string(&metadata, "weight").unwrap_or_default(),
                top_speed: session::json_string(&metadata, "topspeed").unwrap_or_default(),
                acceleration: session::json_string(&metadata, "acceleration").unwrap_or_default(),
                power_weight: session::json_string(&metadata, "pwratio").unwrap_or_default(),
                drivetrain,
                ..CatalogItem::default()
            })
        })
        .collect::<Vec<_>>();
    sort_items(&mut cars);
    Ok(cars)
}

fn load_tracks(
    root: &Path,
    favorites: &std::collections::BTreeSet<String>,
    dashboard: &std::collections::BTreeSet<String>,
) -> Result<Vec<CatalogItem>> {
    let mut tracks = Vec::new();

    for track_root in directories(root)? {
        let Some(track_id) = file_name(&track_root) else {
            continue;
        };
        let ui_root = track_root.join("ui");
        let mut layouts = directories(&ui_root).unwrap_or_default();
        if ui_root.join("ui_track.json").is_file() {
            layouts.push(ui_root.clone());
        }

        for layout_root in layouts {
            let metadata_path = layout_root.join("ui_track.json");
            if !metadata_path.is_file() {
                continue;
            }

            let metadata = session::read_optional(&metadata_path);
            let layout_id = (layout_root != ui_root)
                .then(|| file_name(&layout_root))
                .flatten()
                .unwrap_or_default();
            let name = session::json_string(&metadata, "name")
                .unwrap_or_else(|| session::humanize(&track_id));
            let location = [
                session::json_string(&metadata, "city"),
                session::json_string(&metadata, "country"),
            ]
            .into_iter()
            .flatten()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
            let subtitle = if location.is_empty() {
                session::humanize(&layout_id)
            } else {
                location
            };

            let id = if layout_id.is_empty() {
                track_id.clone()
            } else {
                format!("{track_id}/{layout_id}")
            };
            tracks.push(CatalogItem {
                favorite: favorites.contains(&id),
                dashboard: dashboard.contains(&id),
                id,
                name,
                subtitle,
                preview: session::first_existing(&[
                    layout_root.join("preview.png"),
                    ui_root.join("preview.png"),
                ]),
                author: session::json_string(&metadata, "author").unwrap_or_default(),
                description: plain_text(
                    &session::json_string(&metadata, "description").unwrap_or_default(),
                ),
                tags: json_strings(&metadata, "tags").join(", "),
                version: session::json_string(&metadata, "version").unwrap_or_default(),
                country: session::json_string(&metadata, "country").unwrap_or_default(),
                city: session::json_string(&metadata, "city").unwrap_or_default(),
                length: session::json_string(&metadata, "length").unwrap_or_default(),
                width: session::json_string(&metadata, "width").unwrap_or_default(),
                pitboxes: session::json_string(&metadata, "pitboxes").unwrap_or_default(),
                direction: session::json_string(&metadata, "run").unwrap_or_default(),
                year: json_scalar(&metadata, "year"),
                outline: ["outline.png"]
                    .into_iter()
                    .map(|name| layout_root.join(name))
                    .find(|path| path.is_file())
                    .map(|path| session::file_url(&path))
                    .unwrap_or_default(),
                ..CatalogItem::default()
            });
        }
    }

    sort_items(&mut tracks);
    Ok(tracks)
}

fn load_replays(root: &Path) -> Vec<CatalogItem> {
    let mut files = Vec::new();
    collect_replays(root, &mut files);
    files.sort();
    files.reverse();

    files
        .into_iter()
        .filter_map(|path| {
            let file_name = path.file_name()?.to_string_lossy().into_owned();
            let name = path.file_stem()?.to_string_lossy().into_owned();
            Some(CatalogItem {
                id: path.to_string_lossy().into_owned(),
                name,
                subtitle: file_name,
                preview: String::new(),
                ..CatalogItem::default()
            })
        })
        .collect()
}

fn collect_replays(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_replays(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "acreplay")
        {
            files.push(path);
        }
    }
}

fn directories(root: &Path) -> Result<Vec<PathBuf>> {
    let mut directories = fs::read_dir(root)
        .with_context(|| format!("could not read {}", root.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    directories.sort();
    Ok(directories)
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

fn count(items: &[CatalogItem]) -> i32 {
    i32::try_from(items.len()).unwrap_or(i32::MAX)
}

fn sort_items(items: &mut [CatalogItem]) {
    items.sort_by_key(|item| (!item.favorite, item.name.to_lowercase()));
}

fn json_strings(contents: &str, key: &str) -> Vec<String> {
    let Some((_, after_key)) = contents.split_once(&format!("\"{key}\"")) else {
        return Vec::new();
    };
    let Some(start) = after_key.find('[') else {
        return Vec::new();
    };
    let Some(end) = after_key[start..].find(']') else {
        return Vec::new();
    };
    serde_json::from_str(&after_key[start..=start + end]).unwrap_or_default()
}

fn json_scalar(contents: &str, key: &str) -> String {
    let Some((_, after_key)) = contents.split_once(&format!("\"{key}\"")) else {
        return String::new();
    };
    let Some((_, value)) = after_key.split_once(':') else {
        return String::new();
    };
    value
        .trim_start()
        .split([',', '\n', '\r', '}'])
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches('"')
        .to_owned()
}

fn plain_text(html: &str) -> String {
    let html = html
        .replace("<br><br>", "\n\n")
        .replace("<br />", "\n")
        .replace("<br/>", "\n")
        .replace("<br>", "\n");
    let mut output = String::with_capacity(html.len());
    let mut in_tag = false;
    for character in html.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }
    output
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_catalog_items_for_frontend() {
        assert_eq!(count(&[]), 0);
    }

    #[test]
    fn track_outlines_ignore_cropped_files() {
        let root = std::env::temp_dir().join(format!("korsa-outlines-{}", std::process::id()));
        let ui = root.join("track/ui/normal");
        fs::create_dir_all(&ui).unwrap();
        fs::write(ui.join("ui_track.json"), r#"{"name":"Test Track"}"#).unwrap();
        fs::write(ui.join("outline_cropped.png"), "unused cropped image").unwrap();
        fs::write(ui.join("outline.png"), "full outline").unwrap();
        let tracks = load_tracks(&root, &Default::default(), &Default::default()).unwrap();
        assert_eq!(tracks.len(), 1);
        assert!(tracks[0].outline.ends_with("/outline.png"));
        fs::remove_file(ui.join("outline.png")).unwrap();
        let tracks = load_tracks(&root, &Default::default(), &Default::default()).unwrap();
        assert!(tracks[0].outline.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dashboard_and_catalog_previews_follow_each_cars_livery() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("korsa-livery-previews-{unique}"));
        fs::create_dir_all(root.join("cfg")).unwrap();
        fs::create_dir_all(root.join("content/tracks")).unwrap();
        for car in ["car_a", "car_b"] {
            for (skin, name, extension) in [
                ("blue", "Blue Racing", "jpg"),
                ("red", "Scarlet Racing", "png"),
            ] {
                let path = root.join(format!("content/cars/{car}/skins/{skin}"));
                fs::create_dir_all(&path).unwrap();
                fs::write(
                    path.join("ui_skin.json"),
                    format!(r#"{{"skinname":"{name}"}}"#),
                )
                .unwrap();
                fs::write(path.join(format!("preview.{extension}")), "test image").unwrap();
            }
        }
        fs::write(
            root.join("cfg/race.ini"),
            "[RACE]\nMODEL=car_b\nSKIN=blue\nTRACK=track\n",
        )
        .unwrap();
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
        let mut liveries = Liveries::default();
        liveries.remember("car_a", "red");
        liveries.remember("car_b", "red");
        liveries.save(&installation).unwrap();
        let preferences = Preferences {
            dashboard_cars: ["car_a".into(), "car_b".into()].into(),
            ..Default::default()
        };
        let options: serde_json::Value =
            serde_json::from_str(&dashboard_json(&installation, &preferences).unwrap()).unwrap();
        let cars = options["cars"].as_array().unwrap();
        let a = cars.iter().find(|car| car["id"] == "car_a").unwrap();
        let b = cars.iter().find(|car| car["id"] == "car_b").unwrap();
        assert_eq!(a["skin"], "red");
        assert_eq!(a["skin_name"], "Scarlet Racing");
        assert!(a["preview"].as_str().unwrap().ends_with("red/preview.png"));
        assert_eq!(
            b["skin"], "blue",
            "the active session takes precedence over old history"
        );
        assert!(b["preview"].as_str().unwrap().ends_with("blue/preview.jpg"));
        assert_eq!(
            session::load(&installation).unwrap().skin_name,
            "Blue Racing"
        );

        let mut catalog = load(&installation, &preferences).unwrap();
        assert!(catalog.set_livery("car_a", "blue").unwrap());
        let a = catalog.cars.iter().find(|car| car.id == "car_a").unwrap();
        assert_eq!(a.skin, "blue");
        assert!(a.preview.ends_with("blue/preview.jpg"));
        assert!(!catalog.set_livery("car_a", "blue").unwrap());
        fs::remove_file(root.join("content/cars/car_a/skins/red/preview.png")).unwrap();
        let catalog = load(&installation, &preferences).unwrap();
        assert!(
            catalog
                .cars
                .iter()
                .find(|car| car.id == "car_a")
                .unwrap()
                .preview
                .is_empty(),
            "do not display a different livery when the selected skin has no preview"
        );
        fs::remove_dir_all(root.join("content/cars/car_a/skins/red")).unwrap();
        let options: serde_json::Value =
            serde_json::from_str(&dashboard_json(&installation, &preferences).unwrap()).unwrap();
        let a = options["cars"]
            .as_array()
            .unwrap()
            .iter()
            .find(|car| car["id"] == "car_a")
            .unwrap();
        assert_eq!(a["skin"], "blue");
        assert!(a["preview"].as_str().unwrap().ends_with("blue/preview.jpg"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dashboard_options_include_six_choices_and_the_actual_session() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("korsa-dashboard-{unique}"));
        fs::create_dir_all(root.join("cfg")).unwrap();
        let mut preferences = Preferences::default();
        for index in 0..6 {
            let car = format!("car{index}");
            let track = format!("track{index}");
            let car_ui = root.join(format!("content/cars/{car}/ui"));
            let track_ui = root.join(format!("content/tracks/{track}/ui"));
            fs::create_dir_all(&car_ui).unwrap();
            fs::create_dir_all(&track_ui).unwrap();
            fs::write(
                car_ui.join("ui_car.json"),
                format!(r#"{{"name":"Car {index}"}}"#),
            )
            .unwrap();
            fs::write(
                track_ui.join("ui_track.json"),
                format!(r#"{{"name":"Track {index}"}}"#),
            )
            .unwrap();
            preferences.dashboard_cars.insert(car);
            preferences.dashboard_tracks.insert(track);
        }
        fs::write(root.join("cfg/race.ini"), "[RACE]\nMODEL=outside_pins\nTRACK=track4\nCONFIG_TRACK=layout\n[LIGHTING]\nSUN_ANGLE=-208\n").unwrap();
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
        let options: serde_json::Value =
            serde_json::from_str(&dashboard_json(&installation, &preferences).unwrap()).unwrap();
        assert_eq!(options["capacity"], 6);
        assert_eq!(options["cars"].as_array().unwrap().len(), 6);
        assert_eq!(options["tracks"].as_array().unwrap().len(), 6);
        assert_eq!(options["selection"]["car_id"], "outside_pins");
        assert_eq!(options["selection"]["track_key"], "track4/layout");
        assert_eq!(options["selection"]["start_minutes"], 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reads_transmission_tags_from_nonstandard_car_json() {
        let input =
            "{\n\"description\": \"line one\nline two\",\n\"tags\": [\"manual\", \"street\"]\n}";

        assert_eq!(json_strings(input, "tags"), ["manual", "street"]);
    }

    #[test]
    fn converts_content_descriptions_to_plain_text() {
        assert_eq!(
            plain_text("Fast &amp; loud<br><br><b>Enjoy</b>"),
            "Fast & loud\n\nEnjoy"
        );
    }

    #[test]
    fn track_favorite_updates_only_the_selected_layout() {
        let mut catalog = Catalog {
            tracks: vec![
                CatalogItem {
                    id: "track/layout_a".to_owned(),
                    ..CatalogItem::default()
                },
                CatalogItem {
                    id: "track/layout_b".to_owned(),
                    ..CatalogItem::default()
                },
            ],
            ..Catalog::default()
        };
        catalog
            .set_favorite("track", "track/layout_a", true)
            .unwrap();
        assert!(catalog.tracks[0].favorite);
        assert!(!catalog.tracks[1].favorite);
    }
}
