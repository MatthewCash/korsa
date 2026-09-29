use crate::discovery::Installation;
use anyhow::{Context, Result, bail, ensure};
use reqwest::Client;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

const INDEX_URL: &str = "https://acstuff.club/patch/";
const MAX_DOWNLOAD_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
pub struct Release {
    pub version: String,
    pub status: String,
    pub size: String,
    pub recommended: bool,
}

pub async fn fetch() -> Result<Vec<Release>> {
    let response = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?
        .get(INDEX_URL)
        .send()
        .await?
        .error_for_status()?;
    let html = response.text().await?;
    let releases = parse_releases(&html);
    ensure!(
        !releases.is_empty(),
        "official CSP page did not contain public releases"
    );
    Ok(releases)
}

pub fn releases_json(releases: &[Release]) -> Result<String> {
    serde_json::to_string(releases).context("could not serialize CSP releases")
}

pub async fn download(installation: &Installation, version: &str) -> Result<PathBuf> {
    validate_version(version)?;
    let root = installation.game_root.join(".korsa/downloads");
    fs::create_dir_all(&root)?;
    let destination = root.join(format!("lights-patch-v{version}.zip"));
    let temporary = destination.with_extension("zip.part");
    let url = format!("https://acstuff.club/patch/?get={version}");
    let mut response = Client::builder()
        .timeout(Duration::from_secs(600))
        .build()?
        .get(&url)
        .send()
        .await?
        .error_for_status()?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_DOWNLOAD_BYTES)
    {
        bail!("CSP download exceeds the 1 GiB safety limit");
    }
    let mut output = File::create(&temporary)?;
    let mut written = 0_u64;
    while let Some(chunk) = response.chunk().await? {
        written = written
            .checked_add(chunk.len() as u64)
            .context("CSP download size overflow")?;
        ensure!(
            written <= MAX_DOWNLOAD_BYTES,
            "CSP download exceeds the 1 GiB safety limit"
        );
        output.write_all(&chunk)?;
    }
    output.flush()?;
    drop(output);
    let mut signature = [0_u8; 4];
    File::open(&temporary)?.read_exact(&mut signature)?;
    ensure!(
        signature.starts_with(b"PK"),
        "official CSP download was not a ZIP archive"
    );
    fs::rename(&temporary, &destination)?;
    Ok(destination)
}

fn parse_releases(html: &str) -> Vec<Release> {
    let mut releases = BTreeMap::new();
    for segment in html.split("href=\"?info=").skip(1) {
        let Some((version, remainder)) = segment.split_once('"') else {
            continue;
        };
        if validate_version(version).is_err() {
            continue;
        }
        let item = remainder.split("</li>").next().unwrap_or_default();
        let size = item
            .split('(')
            .nth(1)
            .and_then(|value| value.split(',').next())
            .unwrap_or_default()
            .trim()
            .to_owned();
        let status = item
            .split("tag tag-")
            .nth(1)
            .and_then(|value| value.split('>').nth(1))
            .and_then(|value| value.split('<').next())
            .unwrap_or("unknown")
            .trim()
            .to_owned();
        releases.insert(
            version.to_owned(),
            Release {
                version: version.to_owned(),
                recommended: status == "recommended",
                status,
                size,
            },
        );
    }
    let mut releases = releases.into_values().collect::<Vec<_>>();
    releases.sort_by_key(|release| std::cmp::Reverse(version_parts(&release.version)));
    releases
}

fn validate_version(version: &str) -> Result<()> {
    ensure!(
        !version.is_empty()
            && version.len() <= 20
            && version
                .split('.')
                .all(|part| !part.is_empty()
                    && part.chars().all(|character| character.is_ascii_digit())),
        "invalid CSP version"
    );
    Ok(())
}

fn version_parts(version: &str) -> Vec<u32> {
    version
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_public_release_rows() {
        let html = r#"<li><a href="?info=0.2.10">v0.2.10</a> (119 MB, <span class="tag tag-buggy">buggy</span>)</li><li><a href="?info=0.2.11">v0.2.11</a> (120 MB, <span class="tag tag-recommended">recommended</span>)</li>"#;
        let releases = parse_releases(html);
        assert_eq!(releases[0].version, "0.2.11");
        assert!(releases[0].recommended);
        assert_eq!(releases[1].status, "buggy");
    }
}
