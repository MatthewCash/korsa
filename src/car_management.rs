//! Car operations are confined to a single, validated content/cars child.
use crate::{discovery::Installation, launcher, preferences::Preferences, session};
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn car_directory(root: &Path, id: &str) -> Result<PathBuf> {
    ensure!(
        !id.is_empty()
            && id != "."
            && id != ".."
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_- .".contains(&b)),
        "invalid car folder name"
    );
    let root = root
        .canonicalize()
        .context("could not resolve cars directory")?;
    let path = root.join(id);
    let metadata = fs::symlink_metadata(&path).context("car is no longer installed")?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "car folder must be a real directory, not a symbolic link"
    );
    ensure!(
        path.canonicalize()?.parent() == Some(root.as_path()),
        "car folder escapes cars directory"
    );
    Ok(path)
}

pub fn set_hidden(preferences: &mut Preferences, id: &str, hidden: bool) -> Result<()> {
    let previous = preferences.hidden_cars.clone();
    if hidden {
        preferences.hidden_cars.insert(id.into());
    } else {
        preferences.hidden_cars.remove(id);
    }
    if let Err(error) = preferences.save() {
        preferences.hidden_cars = previous;
        return Err(error);
    }
    Ok(())
}

pub fn delete(installation: &Installation, id: &str) -> Result<()> {
    ensure!(
        !launcher::race_is_running()?,
        "close Assetto Corsa before deleting cars"
    );
    ensure!(
        session::load(installation)?.car_id != id,
        "select a different session car before deleting this car"
    );
    let path = car_directory(&installation.game_root.join("content/cars"), id)?;
    fs::remove_dir_all(&path).with_context(|| {
        format!(
            "could not completely delete {}; rescan before retrying",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deletion_paths_reject_traversal_and_symlinks_and_preserve_other_cars() {
        let root = std::env::temp_dir().join(format!("korsa-car-delete-{}", std::process::id()));
        fs::create_dir_all(root.join("cars/test_car/skins/red")).unwrap();
        fs::create_dir_all(root.join("outside")).unwrap();
        fs::write(root.join("outside/keep"), "keep").unwrap();
        std::os::unix::fs::symlink(root.join("outside"), root.join("cars/link")).unwrap();
        for id in [
            "",
            ".",
            "..",
            "../outside",
            "/tmp",
            "link",
            "missing",
            "a\\b",
        ] {
            assert!(car_directory(&root.join("cars"), id).is_err(), "{id}");
        }
        std::os::unix::fs::symlink(root.join("outside"), root.join("cars/test_car/external"))
            .unwrap();
        fs::remove_dir_all(car_directory(&root.join("cars"), "test_car").unwrap()).unwrap();
        assert!(root.join("outside/keep").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
