use crate::discovery::Installation;
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Serialize)]
struct Setup {
    id: String,
    name: String,
    track: String,
    modified: u64,
}

pub fn load_json(installation: &Installation, car: &str) -> Result<String> {
    validate_id(car, "car")?;
    let root = installation.documents_root.join("setups").join(car);
    let mut setups = Vec::new();
    for track in fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
    {
        let track_id = track.file_name().to_string_lossy().into_owned();
        for entry in fs::read_dir(track.path()).into_iter().flatten().flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|value| value.eq_ignore_ascii_case("ini"))
            {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let modified = entry
                    .metadata()
                    .ok()
                    .and_then(|metadata| metadata.modified().ok())
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |value| value.as_secs());
                setups.push(Setup {
                    id: format!("{track_id}/{name}"),
                    name,
                    track: track_id.clone(),
                    modified,
                });
            }
        }
    }
    setups.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| left.name.cmp(&right.name))
    });
    serde_json::to_string(&setups).context("could not serialize car setups")
}

pub fn save_last_as(installation: &Installation, car: &str, track: &str, name: &str) -> Result<()> {
    validate_id(car, "car")?;
    validate_id(track, "track")?;
    validate_name(name)?;
    let root = installation
        .documents_root
        .join("setups")
        .join(car)
        .join(track);
    let source = root.join("last.ini");
    ensure!(
        source.is_file(),
        "there is no current setup for this car and track"
    );
    fs::copy(&source, root.join(format!("{name}.ini")))?;
    Ok(())
}

pub fn apply(installation: &Installation, car: &str, id: &str) -> Result<()> {
    let source = setup_path(installation, car, id)?;
    ensure!(source.is_file(), "setup does not exist");
    let target = source
        .parent()
        .context("invalid setup path")?
        .join("last.ini");
    if source != target {
        fs::copy(source, target)?;
    }
    Ok(())
}

pub fn delete(installation: &Installation, car: &str, id: &str) -> Result<()> {
    let path = setup_path(installation, car, id)?;
    ensure!(
        path.file_name().is_some_and(|name| name != "last.ini"),
        "the active last.ini setup cannot be deleted"
    );
    fs::remove_file(path)?;
    Ok(())
}

fn setup_path(installation: &Installation, car: &str, id: &str) -> Result<PathBuf> {
    validate_id(car, "car")?;
    let relative = Path::new(id);
    ensure!(
        relative.components().count() == 2
            && relative
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "invalid setup ID"
    );
    Ok(installation
        .documents_root
        .join("setups")
        .join(car)
        .join(relative)
        .with_extension("ini"))
}

fn validate_id(value: &str, label: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.chars().all(
                |character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
            ),
        "invalid {label} ID"
    );
    Ok(())
}

fn validate_name(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= 80
            && !value.contains(['/', '\\', '\n', '\r'])
            && value != "."
            && value != "..",
        "invalid setup name"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installation(root: &Path) -> Installation {
        Installation {
            steam_root: root.into(),
            library_root: root.into(),
            game_root: root.into(),
            proton_prefix: root.into(),
            documents_root: root.into(),
            proton_command: root.into(),
            proton_config: None,
            runtime_root: root.into(),
            runtime_client: root.into(),
        }
    }

    #[test]
    fn saves_applies_and_deletes_named_setup() {
        let root = std::env::temp_dir().join(format!("aclm-setups-{}", std::process::id()));
        let setup_root = root.join("setups/test_car/generic");
        fs::create_dir_all(&setup_root).unwrap();
        fs::write(setup_root.join("last.ini"), "[FUEL]\nVALUE=20\n").unwrap();
        let installation = installation(&root);
        save_last_as(&installation, "test_car", "generic", "race").unwrap();
        fs::write(setup_root.join("last.ini"), "changed").unwrap();
        apply(&installation, "test_car", "generic/race").unwrap();
        assert!(
            fs::read_to_string(setup_root.join("last.ini"))
                .unwrap()
                .contains("VALUE=20")
        );
        delete(&installation, "test_car", "generic/race").unwrap();
        assert!(!setup_root.join("race.ini").exists());
        fs::remove_dir_all(root).ok();
    }
}
