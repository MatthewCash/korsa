use crate::discovery::Installation;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashSet},
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use zip::ZipArchive;

const MAX_ENTRIES: usize = 100_000;
const MAX_EXPANDED_BYTES: u64 = 8 * 1024 * 1024 * 1024;
static NEXT_TRANSACTION: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveKind {
    Cars,
    Tracks,
    Csp,
}

#[derive(Clone, Debug, Serialize)]
pub struct ArchiveInspection {
    pub file_name: String,
    pub kind: ArchiveKind,
    pub components: Vec<String>,
    pub file_count: usize,
    pub expanded_bytes: u64,
    pub replacements: usize,
}

#[derive(Clone, Debug)]
pub struct PendingArchive {
    source: PathBuf,
    fingerprint: [u8; 32],
    kind: ArchiveKind,
    entries: Vec<PlannedEntry>,
    components: Vec<String>,
}

#[derive(Clone, Debug)]
struct PlannedEntry {
    archive_index: usize,
    destination: PathBuf,
    size: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstallRecord {
    pub id: String,
    pub archive_name: String,
    pub kind: ArchiveKind,
    pub components: Vec<String>,
    pub created: Vec<String>,
    pub replaced: Vec<String>,
}

pub fn inspect(
    installation: &Installation,
    source: &Path,
) -> Result<(ArchiveInspection, PendingArchive)> {
    ensure!(
        source.is_file(),
        "archive does not exist: {}",
        source.display()
    );
    ensure!(
        source
            .extension()
            .is_some_and(|value| value.eq_ignore_ascii_case("zip")),
        "only ZIP archives are supported"
    );
    let fingerprint = file_hash(source)?;
    let file =
        File::open(source).with_context(|| format!("could not open {}", source.display()))?;
    let mut archive = ZipArchive::new(file).context("archive is not a valid ZIP file")?;
    ensure!(
        archive.len() <= MAX_ENTRIES,
        "archive contains too many entries"
    );

    let mut raw = Vec::new();
    let mut expanded_bytes = 0_u64;
    let mut names = HashSet::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .context("could not read ZIP entry")?;
        let name = entry.name().to_owned();
        ensure!(
            !name.contains('\\'),
            "archive path uses backslashes: {name}"
        );
        let path = entry
            .enclosed_name()
            .context("archive contains an unsafe path")?;
        validate_relative_path(&path)?;
        if let Some(mode) = entry.unix_mode() {
            ensure!(
                mode & 0o170000 != 0o120000,
                "archive contains a symbolic link: {name}"
            );
        }
        if entry.is_dir() {
            continue;
        }
        expanded_bytes = expanded_bytes
            .checked_add(entry.size())
            .context("archive size overflow")?;
        ensure!(
            expanded_bytes <= MAX_EXPANDED_BYTES,
            "archive expands beyond the 8 GiB safety limit"
        );
        let folded = path.to_string_lossy().to_ascii_lowercase();
        ensure!(
            names.insert(folded),
            "archive contains duplicate destination paths"
        );
        raw.push((index, path, entry.size()));
    }
    ensure!(!raw.is_empty(), "archive is empty");

    let wrapper = common_wrapper(raw.iter().map(|(_, path, _)| path.as_path()));
    let stripped = raw
        .iter()
        .map(|(index, path, size)| {
            let path = wrapper.as_ref().map_or(path.clone(), |prefix| {
                path.strip_prefix(prefix).unwrap_or(path).to_path_buf()
            });
            (*index, path, *size)
        })
        .collect::<Vec<_>>();
    let normalized = if classify(&stripped).is_ok() {
        stripped
    } else {
        raw
    };
    let (kind, roots) = classify(&normalized)?;
    let mut entries = Vec::new();
    let mut components = BTreeSet::new();
    for (archive_index, path, size) in normalized {
        let Some((destination, component)) = destination_for(&kind, &roots, &path) else {
            continue;
        };
        validate_relative_path(&destination)?;
        ensure!(
            !destination.starts_with(".korsa"),
            "archive targets manager metadata"
        );
        components.insert(component);
        entries.push(PlannedEntry {
            archive_index,
            destination,
            size,
        });
    }
    ensure!(!entries.is_empty(), "archive contains no installable files");
    let replacements = entries
        .iter()
        .filter(|entry| installation.game_root.join(&entry.destination).exists())
        .count();
    let components = components.into_iter().collect::<Vec<_>>();
    let inspection = ArchiveInspection {
        file_name: source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        kind: kind.clone(),
        components: components.clone(),
        file_count: entries.len(),
        expanded_bytes,
        replacements,
    };
    Ok((
        inspection,
        PendingArchive {
            source: source.to_path_buf(),
            fingerprint,
            kind,
            entries,
            components,
        },
    ))
}

pub fn install(installation: &Installation, pending: &PendingArchive) -> Result<InstallRecord> {
    ensure!(
        file_hash(&pending.source)? == pending.fingerprint,
        "archive changed after inspection"
    );
    let id = transaction_id()?;
    let transaction = transaction_root(installation).join(&id);
    let stage = transaction.join("stage");
    let backup = transaction.join("backup");
    fs::create_dir_all(&stage).with_context(|| format!("could not create {}", stage.display()))?;
    fs::create_dir_all(&backup)
        .with_context(|| format!("could not create {}", backup.display()))?;

    let file = File::open(&pending.source)?;
    let mut archive = ZipArchive::new(file)?;
    for planned in &pending.entries {
        let mut entry = archive.by_index(planned.archive_index)?;
        ensure!(
            entry.size() == planned.size,
            "archive changed during extraction"
        );
        let output = stage.join(&planned.destination);
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut destination = File::create(&output)?;
        std::io::copy(&mut entry, &mut destination)?;
        destination.flush()?;
    }

    let mut record = InstallRecord {
        id,
        archive_name: pending
            .source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        kind: pending.kind.clone(),
        components: pending.components.clone(),
        created: Vec::new(),
        replaced: Vec::new(),
    };
    let commit = (|| -> Result<()> {
        for planned in &pending.entries {
            let relative = planned.destination.to_string_lossy().into_owned();
            let target = installation.game_root.join(&planned.destination);
            if target.exists() {
                let saved = backup.join(&planned.destination);
                if let Some(parent) = saved.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&target, &saved)?;
                record.replaced.push(relative);
            } else {
                record.created.push(relative);
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(stage.join(&planned.destination), target)?;
        }
        write_record(&transaction, &record)
    })();
    if let Err(error) = commit {
        restore(installation, &transaction, &record);
        return Err(error).context("installation failed and was rolled back");
    }
    fs::remove_dir_all(stage).ok();
    Ok(record)
}

pub fn history_json(installation: &Installation) -> Result<String> {
    serde_json::to_string(&history(installation)?).context("could not serialize install history")
}

pub fn rollback(installation: &Installation, id: &str) -> Result<()> {
    ensure!(
        !id.is_empty()
            && id
                .chars()
                .all(|character| character.is_ascii_digit() || character == '-'),
        "invalid transaction ID"
    );
    let transaction = transaction_root(installation).join(id);
    let record: InstallRecord =
        serde_json::from_str(&fs::read_to_string(transaction.join("manifest.json"))?)?;
    restore(installation, &transaction, &record);
    fs::remove_dir_all(transaction)?;
    Ok(())
}

fn restore(installation: &Installation, transaction: &Path, record: &InstallRecord) {
    for relative in record.created.iter().rev() {
        fs::remove_file(installation.game_root.join(relative)).ok();
    }
    for relative in record.replaced.iter().rev() {
        let target = installation.game_root.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::copy(transaction.join("backup").join(relative), target).ok();
    }
}

fn history(installation: &Installation) -> Result<Vec<InstallRecord>> {
    let root = transaction_root(installation);
    let mut records = fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| fs::read_to_string(entry.path().join("manifest.json")).ok())
        .filter_map(|contents| serde_json::from_str(&contents).ok())
        .collect::<Vec<_>>();
    records.sort_by(|left: &InstallRecord, right: &InstallRecord| right.id.cmp(&left.id));
    Ok(records)
}

fn write_record(transaction: &Path, record: &InstallRecord) -> Result<()> {
    fs::write(
        transaction.join("manifest.json"),
        serde_json::to_vec_pretty(record)?,
    )?;
    Ok(())
}

fn transaction_root(installation: &Installation) -> PathBuf {
    installation.game_root.join(".korsa/transactions")
}

fn transaction_id() -> Result<String> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let sequence = NEXT_TRANSACTION.fetch_add(1, Ordering::Relaxed);
    Ok(format!("{nanos}-{}-{sequence}", std::process::id()))
}

fn file_hash(path: &Path) -> Result<[u8; 32]> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(hash.finalize().into())
}

fn validate_relative_path(path: &Path) -> Result<()> {
    ensure!(
        !path.as_os_str().is_empty() && !path.is_absolute(),
        "archive contains an invalid path"
    );
    ensure!(
        path.components()
            .all(|component| matches!(component, Component::Normal(_))),
        "archive path contains traversal components"
    );
    Ok(())
}

fn common_wrapper<'a>(paths: impl Iterator<Item = &'a Path>) -> Option<PathBuf> {
    let paths = paths.collect::<Vec<_>>();
    let first = paths.first()?.components().next()?;
    let Component::Normal(first) = first else {
        return None;
    };
    let prefix = PathBuf::from(first);
    (paths.iter().all(|path| path.starts_with(&prefix))
        && !matches!(
            first.to_string_lossy().as_ref(),
            "content" | "extension" | "dwrite.dll"
        ))
    .then_some(prefix)
}

fn classify(entries: &[(usize, PathBuf, u64)]) -> Result<(ArchiveKind, Vec<PathBuf>)> {
    let paths = entries.iter().map(|(_, path, _)| path).collect::<Vec<_>>();
    let csp = paths.iter().any(|path| {
        path == &&PathBuf::from("dwrite.dll")
            || path == &&PathBuf::from("extension/config/data_manifest.ini")
    });
    let car_roots = marker_roots(&paths, &["ui", "ui_car.json"]);
    let track_roots = paths
        .iter()
        .filter_map(|path| track_root(path))
        .collect::<BTreeSet<_>>();
    let kinds = usize::from(csp)
        + usize::from(!car_roots.is_empty())
        + usize::from(!track_roots.is_empty());
    ensure!(kinds == 1, "archive is unknown, mixed, or ambiguous");
    if csp {
        ensure!(
            paths
                .iter()
                .all(|path| path == &&PathBuf::from("dwrite.dll") || path.starts_with("extension")),
            "CSP archive contains files outside dwrite.dll and extension/"
        );
        return Ok((ArchiveKind::Csp, Vec::new()));
    }
    if !car_roots.is_empty() {
        return Ok((ArchiveKind::Cars, car_roots));
    }
    Ok((ArchiveKind::Tracks, track_roots.into_iter().collect()))
}

fn marker_roots(paths: &[&PathBuf], suffix: &[&str]) -> Vec<PathBuf> {
    paths
        .iter()
        .filter_map(|path| {
            let components = path
                .components()
                .map(|value| value.as_os_str().to_string_lossy())
                .collect::<Vec<_>>();
            (components.len() > suffix.len()
                && components[components.len() - suffix.len()..]
                    .iter()
                    .zip(suffix)
                    .all(|(left, right)| left == right))
            .then(|| {
                let mut root = PathBuf::new();
                for component in path.components().take(components.len() - suffix.len()) {
                    root.push(component.as_os_str());
                }
                root
            })
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn track_root(path: &Path) -> Option<PathBuf> {
    let parts = path
        .components()
        .map(|value| value.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let ui = parts.iter().position(|part| part == "ui")?;
    if parts.last().is_some_and(|part| part == "ui_track.json")
        && (parts.len() == ui + 2 || parts.len() == ui + 3)
    {
        return Some(parts[..ui].iter().collect());
    }
    None
}

fn destination_for(
    kind: &ArchiveKind,
    roots: &[PathBuf],
    path: &Path,
) -> Option<(PathBuf, String)> {
    if *kind == ArchiveKind::Csp {
        return Some((path.to_path_buf(), "Custom Shaders Patch".to_owned()));
    }
    let root = roots
        .iter()
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count())?;
    let id = root.file_name()?.to_string_lossy().into_owned();
    let rooted = root.starts_with("content/cars") || root.starts_with("content/tracks");
    let destination = if rooted {
        path.to_path_buf()
    } else {
        let category = if *kind == ArchiveKind::Cars {
            "cars"
        } else {
            "tracks"
        };
        Path::new("content").join(category).join(path)
    };
    Some((destination, id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn archive(entries: &[(&str, &[u8])]) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "aclm-installer-{}-{}.zip",
            std::process::id(),
            transaction_id().unwrap()
        ));
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, contents) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(contents).unwrap();
        }
        fs::write(&path, writer.finish().unwrap().into_inner()).unwrap();
        path
    }

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
    fn classifies_and_installs_a_wrapped_car() {
        let zip = archive(&[
            ("mod/my_car/ui/ui_car.json", b"{}"),
            ("mod/my_car/data.acd", b"data"),
        ]);
        let root = std::env::temp_dir().join(format!("aclm-install-{}", transaction_id().unwrap()));
        fs::create_dir_all(&root).unwrap();
        let (info, pending) = inspect(&installation(&root), &zip).unwrap();
        assert_eq!(info.kind, ArchiveKind::Cars);
        install(&installation(&root), &pending).unwrap();
        assert!(root.join("content/cars/my_car/ui/ui_car.json").is_file());
        fs::remove_file(zip).ok();
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn rejects_path_traversal() {
        let zip = archive(&[("../outside/ui/ui_car.json", b"{}")]);
        let root = std::env::temp_dir();
        assert!(inspect(&installation(&root), &zip).is_err());
        fs::remove_file(zip).ok();
    }

    #[test]
    fn classifies_csp_and_rejects_unrelated_files() {
        let good = archive(&[
            ("dwrite.dll", b"dll"),
            ("extension/config/data_manifest.ini", b"[VERSION]"),
        ]);
        assert_eq!(
            inspect(&installation(&std::env::temp_dir()), &good)
                .unwrap()
                .0
                .kind,
            ArchiveKind::Csp
        );
        let bad = archive(&[("dwrite.dll", b"dll"), ("evil.exe", b"bad")]);
        assert!(inspect(&installation(&std::env::temp_dir()), &bad).is_err());
        fs::remove_file(good).ok();
        fs::remove_file(bad).ok();
    }

    #[test]
    fn rollback_restores_replaced_files() {
        let zip = archive(&[
            ("my_car/ui/ui_car.json", b"new"),
            ("my_car/data.acd", b"data"),
        ]);
        let root =
            std::env::temp_dir().join(format!("aclm-rollback-{}", transaction_id().unwrap()));
        let original = root.join("content/cars/my_car/ui/ui_car.json");
        fs::create_dir_all(original.parent().unwrap()).unwrap();
        fs::write(&original, b"original").unwrap();
        let install = installation(&root);
        let (_, pending) = inspect(&install, &zip).unwrap();
        let record = super::install(&install, &pending).unwrap();
        assert_eq!(fs::read(&original).unwrap(), b"new");
        rollback(&install, &record.id).unwrap();
        assert_eq!(fs::read(&original).unwrap(), b"original");
        assert!(!root.join("content/cars/my_car/data.acd").exists());
        fs::remove_file(zip).ok();
        fs::remove_dir_all(root).ok();
    }
}
