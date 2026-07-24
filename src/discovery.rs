use anyhow::{Context, Result, bail};
use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};

const APP_ID: &str = "244210";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installation {
    pub steam_root: PathBuf,
    pub library_root: PathBuf,
    pub game_root: PathBuf,
    pub proton_prefix: PathBuf,
    pub documents_root: PathBuf,
    pub proton_command: PathBuf,
    pub proton_config: Option<String>,
    pub runtime_root: PathBuf,
    pub runtime_client: PathBuf,
}

pub fn discover() -> Result<Installation> {
    discover_from_roots(steam_roots())
}

fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(path) = env::var_os("STEAM_DIR") {
        roots.push(PathBuf::from(path));
    }

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        roots.extend([
            home.join(".local/share/Steam"),
            home.join(".steam/steam"),
            home.join(".var/app/com.valvesoftware.Steam/data/Steam"),
        ]);
    }

    deduplicate(roots)
}

fn discover_from_roots(roots: Vec<PathBuf>) -> Result<Installation> {
    for steam_root in roots.into_iter().filter(|path| path.is_dir()) {
        for library_root in libraries_for(&steam_root) {
            let steamapps = library_root.join("steamapps");
            if !steamapps
                .join(format!("appmanifest_{APP_ID}.acf"))
                .is_file()
            {
                continue;
            }

            let game_root = steamapps.join("common/assettocorsa");
            if !is_valid_game_root(&game_root) {
                log::warn!(
                    "ignoring incomplete Assetto Corsa installation at {}",
                    game_root.display()
                );
                continue;
            }

            let proton_prefix = steamapps.join(format!("compatdata/{APP_ID}/pfx"));
            let documents_root =
                proton_prefix.join("drive_c/users/steamuser/Documents/Assetto Corsa");
            let (proton_command, proton_config) = configured_proton(&steam_root)?;
            let runtime_root = steam_runtime(&steam_root)?;
            let runtime_client =
                runtime_root.join("pressure-vessel/bin/steam-runtime-launch-client");

            return Ok(Installation {
                steam_root,
                library_root,
                game_root,
                proton_prefix,
                documents_root,
                proton_command,
                proton_config,
                runtime_root,
                runtime_client,
            });
        }
    }

    bail!("Assetto Corsa was not found in any Steam library")
}

fn steam_runtime(steam_root: &Path) -> Result<PathBuf> {
    libraries_for(steam_root)
        .into_iter()
        .map(|library| library.join("steamapps/common/SteamLinuxRuntime_sniper"))
        .find(|runtime| {
            runtime.join("_v2-entry-point").is_file()
                && runtime
                    .join("pressure-vessel/bin/steam-runtime-launch-client")
                    .is_file()
        })
        .context("Steam Linux Runtime 3.0 (sniper) is not installed")
}

fn configured_proton(steam_root: &Path) -> Result<(PathBuf, Option<String>)> {
    let config_path = steam_root.join("config/config.vdf");
    let config = fs::read_to_string(&config_path)
        .with_context(|| format!("could not read {}", config_path.display()))?;
    let tool_name = parse_compat_tool(&config, APP_ID)
        .context("Assetto Corsa has no configured Steam compatibility tool")?;
    let tool_config = parse_compat_config(&config, APP_ID);
    let command = steam_root
        .join("compatibilitytools.d")
        .join(tool_name)
        .join("proton");

    command
        .is_file()
        .then_some((command, tool_config))
        .context("the configured Proton executable does not exist")
}

fn parse_compat_tool(contents: &str, app_id: &str) -> Option<String> {
    let mapping = contents.split_once("\"CompatToolMapping\"")?.1;
    let app = mapping.split_once(&format!("\"{app_id}\""))?.1;
    let name = app.split_once("\"name\"")?.1;
    quoted_fields(name).into_iter().next()
}

fn parse_compat_config(contents: &str, app_id: &str) -> Option<String> {
    let mapping = contents.split_once("\"CompatToolMapping\"")?.1;
    let app = mapping.split_once(&format!("\"{app_id}\""))?.1;
    let config = app.split_once("\"config\"")?.1;
    quoted_fields(config)
        .into_iter()
        .next()
        .filter(|value| !value.is_empty())
}

fn libraries_for(steam_root: &Path) -> Vec<PathBuf> {
    let mut libraries = vec![steam_root.to_path_buf()];
    let file = steam_root.join("steamapps/libraryfolders.vdf");

    match fs::read_to_string(&file) {
        Ok(contents) => libraries.extend(parse_library_paths(&contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => log::warn!("could not read {}: {error}", file.display()),
    }

    deduplicate(libraries)
}

fn parse_library_paths(contents: &str) -> Vec<PathBuf> {
    contents
        .lines()
        .filter_map(|line| {
            let fields = quoted_fields(line);
            fields
                .windows(2)
                .find(|pair| pair[0] == "path")
                .map(|pair| &pair[1])
                .map(|path| PathBuf::from(path.replace("\\\\", "\\")))
        })
        .collect()
}

fn quoted_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;

    for character in line.chars() {
        if !quoted {
            if character == '"' {
                quoted = true;
                current.clear();
            }
            continue;
        }

        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            current.push(character);
            escaped = true;
        } else if character == '"' {
            fields.push(current.clone());
            quoted = false;
        } else {
            current.push(character);
        }
    }

    fields
}

fn is_valid_game_root(path: &Path) -> bool {
    path.join("acs.exe").is_file()
        && path.join("content/cars").is_dir()
        && path.join("content/tracks").is_dir()
}

fn deduplicate(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

pub fn validate_documents_root(installation: &Installation) -> Result<()> {
    installation
        .proton_prefix
        .is_dir()
        .then_some(())
        .context("the Proton prefix has not been created yet")?;

    installation
        .documents_root
        .is_dir()
        .then_some(())
        .context("Assetto Corsa has not created its Documents directory yet")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modern_library_folders() {
        let input = r#"
            "libraryfolders"
            {
                "0" { "path" "/home/user/.local/share/Steam" }
                "1"
                {
                    "path" "/games/SteamLibrary"
                    "label" "Fast disk"
                }
            }
        "#;

        assert_eq!(
            parse_library_paths(input),
            [
                PathBuf::from("/home/user/.local/share/Steam"),
                PathBuf::from("/games/SteamLibrary")
            ]
        );
    }

    #[test]
    fn ignores_unrelated_vdf_values() {
        let input = r#"
            "label" "path"
            "contentid" "244210"
        "#;

        assert!(parse_library_paths(input).is_empty());
    }

    #[test]
    fn removes_duplicate_paths_without_reordering() {
        assert_eq!(
            deduplicate(vec![
                PathBuf::from("/a"),
                PathBuf::from("/b"),
                PathBuf::from("/a")
            ]),
            [PathBuf::from("/a"), PathBuf::from("/b")]
        );
    }

    #[test]
    fn parses_configured_compatibility_tool() {
        let input = r#"
            "CompatToolMapping"
            {
                "244210"
                {
                    "name" "GE-Proton9-20"
                    "config" ""
                }
            }
        "#;

        assert_eq!(
            parse_compat_tool(input, "244210").as_deref(),
            Some("GE-Proton9-20")
        );
        assert_eq!(parse_compat_config(input, "244210"), None);
    }

    #[test]
    fn parses_non_empty_compatibility_config() {
        let input = r#"
            "CompatToolMapping"
            {
                "244210"
                {
                    "name" "GE-Proton9-20"
                    "config" "noxalia"
                }
            }
        "#;

        assert_eq!(
            parse_compat_config(input, "244210").as_deref(),
            Some("noxalia")
        );
    }
}
