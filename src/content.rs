use crate::{discovery::Installation, preferences::Preferences, session};
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
    let cars = load_cars(
        &installation.game_root.join("content/cars"),
        &preferences.favorite_cars,
    )?;
    let tracks = load_tracks(
        &installation.game_root.join("content/tracks"),
        &preferences.favorite_tracks,
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

impl Catalog {
    pub fn set_favorite(&mut self, kind: &str, id: &str, favorite: bool) -> Result<()> {
        let items = match kind {
            "car" => &mut self.cars,
            "track" => &mut self.tracks,
            _ => anyhow::bail!("unknown favorite type: {kind}"),
        };
        let mut matched = false;
        for item in items.iter_mut() {
            if item.id == id || (kind == "track" && item.id.starts_with(&format!("{id}/"))) {
                item.favorite = favorite;
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
                        preview: session::file_url(&skin_root.join("preview.jpg")),
                    })
                })
                .collect::<Vec<_>>();
            let skin = skins
                .first()
                .map(|skin| skin.id.clone())
                .unwrap_or_default();
            let preview = skins
                .iter()
                .find(|skin| !skin.preview.is_empty())
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
                favorite: favorites.contains(&id) || favorites.contains(&track_id),
                id,
                name,
                subtitle,
                preview: session::file_url(&layout_root.join("preview.png")),
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
                outline: session::file_url(&layout_root.join("outline.png")),
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
    fn base_track_favorite_updates_every_layout() {
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
        catalog.set_favorite("track", "track", true).unwrap();
        assert!(catalog.tracks.iter().all(|track| track.favorite));
    }
}
