use crate::{configuration, discovery::Installation, preferences};
use anyhow::{Context, Result, bail, ensure};
use reqwest::header::{CACHE_CONTROL, USER_AGENT};
use reqwest::{Client, redirect::Policy};
use serde::Serialize;
use serde_json::Value;
use std::{
    fs,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const LOBBY_URLS: [&str; 2] = [
    "http://93.57.10.21/lobby.ashx/list",
    "http://lobby.assettocorsa.net/lobby.ashx/list",
];
const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
pub struct Server {
    pub name: String,
    pub ip: String,
    pub udp_port: u16,
    pub tcp_port: u16,
    pub http_port: u16,
    pub clients: u16,
    pub capacity: u16,
    pub track: String,
    pub cars: Vec<String>,
    pub passworded: bool,
    pub pickup: bool,
    pub country: String,
    pub session: String,
}

#[derive(Clone, Debug)]
pub struct SteamProfile {
    pub id: String,
    pub name: String,
}

pub async fn fetch(installation: &Installation) -> Result<Vec<Server>> {
    let profile = steam_profile(installation)?;
    let client = Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(15))
        .build()?;
    let mut failures = Vec::new();
    let cache_key = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 30
        % 1440;
    let cache_key = cache_key.to_string();
    for url in LOBBY_URLS {
        let response = match client
            .get(url)
            .header(USER_AGENT, "Assetto Corsa Launcher")
            .header(CACHE_CONTROL, "no-cache")
            .query(&[("guid", profile.id.as_str()), ("r", cache_key.as_str())])
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                failures.push(format!("{url}: {error}"));
                continue;
            }
        };
        if response.status().is_redirection() {
            failures.push(format!("{url}: redirected to the lobby home page"));
            continue;
        }
        let response = match response.error_for_status() {
            Ok(response) => response,
            Err(error) => {
                failures.push(format!("{url}: {error}"));
                continue;
            }
        };
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            failures.push(format!("{url}: response is too large"));
            continue;
        }
        let bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.push(format!("{url}: {error}"));
                continue;
            }
        };
        if bytes.len() > MAX_RESPONSE_BYTES {
            failures.push(format!("{url}: response is too large"));
            continue;
        }
        match serde_json::from_slice::<Vec<Value>>(&bytes) {
            Ok(values) => return Ok(values.into_iter().filter_map(parse_server).collect()),
            Err(_) => {
                let description = if bytes.is_empty() {
                    "empty response".to_owned()
                } else {
                    let preview = String::from_utf8_lossy(&bytes[..bytes.len().min(80)])
                        .replace(['\n', '\r'], " ");
                    format!("non-JSON response: {preview}")
                };
                failures.push(format!("{url}: {description}"));
            }
        }
    }
    bail!(
        "the official Assetto Corsa lobby did not provide a server list; no additional local configuration is required ({})",
        failures.join("; ")
    )
}

pub fn servers_json(servers: &[Server]) -> Result<String> {
    serde_json::to_string(servers).context("could not serialize online servers")
}

pub fn configure_join(
    installation: &Installation,
    server: &Server,
    car: &str,
    password: &str,
) -> Result<()> {
    ensure!(
        !car.is_empty() && !car.contains(['/', '\\', '\n', '\r']),
        "invalid online car ID"
    );
    ensure!(
        server.cars.iter().any(|candidate| candidate == car),
        "selected car is not allowed by this server"
    );
    ensure!(
        installation
            .game_root
            .join("content/cars")
            .join(car)
            .is_dir(),
        "selected server car is not installed"
    );
    ensure!(
        !password.contains(['\n', '\r']),
        "password contains a newline"
    );
    let profile = steam_profile(installation)?;
    let path = installation.documents_root.join("cfg/race.ini");
    let mut contents =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    for (section, key, value) in [
        ("REMOTE", "ACTIVE", "1".to_owned()),
        ("REMOTE", "SERVER_IP", server.ip.clone()),
        ("REMOTE", "SERVER_PORT", server.tcp_port.to_string()),
        ("REMOTE", "SERVER_HTTP_PORT", server.http_port.to_string()),
        ("REMOTE", "REQUESTED_CAR", car.to_owned()),
        ("REMOTE", "NAME", profile.name),
        ("REMOTE", "TEAM", String::new()),
        ("REMOTE", "GUID", profile.id),
        ("REMOTE", "PASSWORD", password.to_owned()),
    ] {
        contents = configuration::set_ini_value(&contents, section, key, &value);
    }
    preferences::atomic_write(&path, &contents)
}

pub fn steam_profile(installation: &Installation) -> Result<SteamProfile> {
    let contents = fs::read_to_string(installation.steam_root.join("config/loginusers.vdf"))
        .context("could not read Steam loginusers.vdf")?;
    let mut current_id = None;
    let mut current_name = None;
    let mut fallback = None;
    for line in contents.lines().map(str::trim) {
        let quoted = quoted_values(line);
        if quoted.len() == 1
            && quoted[0]
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            if let (Some(id), Some(name)) = (current_id.take(), current_name.take()) {
                fallback.get_or_insert(SteamProfile { id, name });
            }
            current_id = Some(quoted[0].clone());
        } else if quoted.len() >= 2 && quoted[0].eq_ignore_ascii_case("PersonaName") {
            current_name = Some(quoted[1].clone());
        } else if quoted.len() >= 2
            && quoted[0].eq_ignore_ascii_case("MostRecent")
            && quoted[1] == "1"
        {
            let id = current_id.clone().context("Steam profile has no ID")?;
            let name = current_name.clone().unwrap_or_else(|| "Player".to_owned());
            return Ok(SteamProfile { id, name });
        }
    }
    fallback
        .or_else(|| {
            current_id.map(|id| SteamProfile {
                id,
                name: current_name.unwrap_or_else(|| "Player".to_owned()),
            })
        })
        .context("no Steam account was found in loginusers.vdf")
}

fn parse_server(value: Value) -> Option<Server> {
    let object = value.as_object()?;
    let port = number(object.get("port"))?;
    let country = object
        .get("country")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    Some(Server {
        name: string(object.get("name")),
        ip: string(object.get("ip")),
        udp_port: u16::try_from(port).ok()?,
        tcp_port: u16::try_from(number(object.get("tport")).unwrap_or(port)).ok()?,
        http_port: u16::try_from(number(object.get("cport")).unwrap_or_default()).ok()?,
        clients: u16::try_from(number(object.get("clients")).unwrap_or_default())
            .unwrap_or(u16::MAX),
        capacity: u16::try_from(number(object.get("maxclients")).unwrap_or_default())
            .unwrap_or(u16::MAX),
        track: string(object.get("track")),
        cars: object
            .get("cars")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        passworded: object.get("pass").and_then(Value::as_bool).unwrap_or(false),
        pickup: object
            .get("pickup")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        country,
        session: session_name(number(object.get("session")).unwrap_or_default()).to_owned(),
    })
}

fn string(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or_default().to_owned()
}
fn number(value: Option<&Value>) -> Option<u64> {
    value.and_then(Value::as_u64)
}

fn session_name(value: u64) -> &'static str {
    match value {
        0 => "Booking",
        1 => "Practice",
        2 => "Qualifying",
        3 => "Race",
        4 => "Hotlap",
        5 => "Time Attack",
        6 => "Drift",
        7 => "Drag",
        _ => "Unknown",
    }
}

fn quoted_values(line: &str) -> Vec<String> {
    line.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lobby_server_shape() {
        let value = serde_json::json!({"name":"Server","ip":"127.0.0.1","port":9600,"tport":9601,"cport":8081,"clients":3,"maxclients":24,"track":"monza","cars":["car_a"],"pass":true,"pickup":true,"country":["Italy","IT"],"session":3});
        let server = parse_server(value).unwrap();
        assert_eq!(server.tcp_port, 9601);
        assert_eq!(server.session, "Race");
        assert!(server.passworded);
    }

    #[test]
    fn extracts_quoted_vdf_values() {
        assert_eq!(
            quoted_values("\"PersonaName\"  \"Driver\""),
            ["PersonaName", "Driver"]
        );
    }

    #[test]
    #[ignore = "contacts the live Kunos lobby"]
    fn live_lobby_returns_servers() {
        let installation = crate::discovery::discover().unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let servers = runtime.block_on(fetch(&installation)).unwrap();
        assert!(!servers.is_empty());
    }
}
