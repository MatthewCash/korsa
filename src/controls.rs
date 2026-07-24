use crate::{configuration, discovery::Installation, preferences, session};
use anyhow::{Context, Result};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{ErrorKind, Read},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::UnboundedSender;

const AXIS_SECTIONS: [(&str, &str); 5] = [
    ("STEER", "Steering"),
    ("THROTTLE", "Throttle"),
    ("BRAKES", "Brake"),
    ("CLUTCH", "Clutch"),
    ("HANDBRAKE", "Handbrake"),
];

#[derive(Debug, Serialize)]
struct ControlsModel {
    input_method: String,
    devices: Vec<Controller>,
    system_devices: Vec<SystemDevice>,
    axes: Vec<AxisBinding>,
    bindings: Vec<ButtonBinding>,
    shifter: Shifter,
    steering: BTreeMap<String, String>,
    force_feedback: BTreeMap<String, String>,
    gamepad: BTreeMap<String, String>,
    keyboard: BTreeMap<String, String>,
    presets: Vec<Preset>,
}

#[derive(Clone, Debug, Serialize)]
struct Controller {
    index: i32,
    name: String,
    guid: String,
    connected: bool,
}

#[derive(Clone, Debug, Serialize)]
struct SystemDevice {
    path: String,
    name: String,
    controller_index: i32,
    readable: bool,
}

#[derive(Debug, Serialize)]
struct AxisBinding {
    section: String,
    label: String,
    joy: i32,
    axis: i32,
    device_name: String,
    assignment: String,
    min: String,
    max: String,
    gamma: String,
    inverted: bool,
}

#[derive(Debug, Serialize)]
struct ButtonBinding {
    section: String,
    label: String,
    category: &'static str,
    joy: i32,
    button: i32,
    device_name: String,
    assignment: String,
    key: String,
    key_assignment: String,
    xbox_button: String,
}

#[derive(Debug, Serialize)]
struct Shifter {
    active: bool,
    joy: i32,
    device_name: String,
    gears: Vec<GearBinding>,
}

#[derive(Debug, Serialize)]
struct GearBinding {
    key: &'static str,
    label: &'static str,
    button: i32,
    assignment: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Preset {
    pub name: String,
    pub source: &'static str,
}

#[derive(Clone, Debug)]
struct IniSection {
    name: String,
    values: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
struct Joystick {
    path: PathBuf,
    name: String,
    controller_index: i32,
}

struct JoystickReader {
    joystick: Joystick,
    file: fs::File,
    axes: Vec<f64>,
    buttons: Vec<bool>,
}

#[derive(Debug, Serialize)]
struct LiveInputModel<'a> {
    devices: Vec<LiveDevice<'a>>,
}

#[derive(Debug, Serialize)]
struct LiveDevice<'a> {
    controller_index: i32,
    name: &'a str,
    axes: &'a [f64],
    buttons: &'a [bool],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureKind {
    Axis,
    Button,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapturedInput {
    pub controller: i32,
    pub input: i32,
}

pub fn load_json(installation: &Installation) -> Result<String> {
    let path = controls_path(installation);
    let contents =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    let sections = parse_sections(&contents);
    let mut devices = controllers(&sections);
    let system_devices = system_devices(&devices);
    for device in &mut devices {
        device.connected = system_devices
            .iter()
            .any(|candidate| candidate.controller_index == device.index);
    }

    let axes = AXIS_SECTIONS
        .iter()
        .map(|(section, label)| axis_binding(&sections, &devices, section, label))
        .collect();
    let bindings = sections
        .iter()
        .filter(|section| is_button_binding(section))
        .map(|section| button_binding(section, &devices))
        .collect();

    let model = ControlsModel {
        input_method: value(&sections, "HEADER", "INPUT_METHOD", "KEYBOARD"),
        axes,
        bindings,
        shifter: shifter(&sections, &devices),
        steering: selected_values(
            &sections,
            "STEER",
            &[
                "LOCK",
                "SCALE",
                "FF_GAIN",
                "FILTER_FF",
                "STEER_GAMMA",
                "STEER_FILTER",
                "SPEED_SENSITIVITY",
                "DEBOUNCING_MS",
            ],
        ),
        force_feedback: force_feedback(&sections),
        gamepad: section_values(&sections, "X360"),
        keyboard: section_values(&sections, "KEYBOARD"),
        presets: list_presets(installation),
        devices,
        system_devices,
    };
    serde_json::to_string(&model).context("could not serialize controls configuration")
}

pub fn set_option(
    installation: &Installation,
    section: &str,
    key: &str,
    value: &str,
) -> Result<()> {
    crate::ac_settings::set_option(installation, "controls.ini", section, key, value)
}

pub fn assign(
    installation: &Installation,
    section: &str,
    input_key: &str,
    captured: CapturedInput,
) -> Result<()> {
    let path = controls_path(installation);
    let contents =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    let sections = parse_sections(&contents);
    anyhow::ensure!(
        sections
            .iter()
            .any(|candidate| candidate.name.eq_ignore_ascii_case(section)),
        "unknown controls binding: {section}"
    );
    anyhow::ensure!(
        input_key == "AXLE"
            || input_key == "BUTTON"
            || (section.eq_ignore_ascii_case("SHIFTER")
                && matches!(
                    input_key,
                    "GEAR_1"
                        | "GEAR_2"
                        | "GEAR_3"
                        | "GEAR_4"
                        | "GEAR_5"
                        | "GEAR_6"
                        | "GEAR_7"
                        | "GEAR_R"
                )),
        "invalid controls input key"
    );
    let updated = configuration::set_ini_value(
        &configuration::set_ini_value(&contents, section, "JOY", &captured.controller.to_string()),
        section,
        input_key,
        &captured.input.to_string(),
    );
    preferences::atomic_write(&path, &updated)
}

pub fn clear_binding(installation: &Installation, section: &str, input_key: &str) -> Result<()> {
    if section.eq_ignore_ascii_case("SHIFTER") && input_key.starts_with("GEAR_") {
        let path = controls_path(installation);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        let updated = configuration::set_ini_value(&contents, section, input_key, "-1");
        return preferences::atomic_write(&path, &updated);
    }
    assign(
        installation,
        section,
        input_key,
        CapturedInput {
            controller: -1,
            input: -1,
        },
    )
}

pub fn set_axis_inverted(installation: &Installation, section: &str, inverted: bool) -> Result<()> {
    anyhow::ensure!(
        AXIS_SECTIONS
            .iter()
            .any(|(candidate, _)| candidate.eq_ignore_ascii_case(section)),
        "unknown controls axis: {section}"
    );
    let path = controls_path(installation);
    let contents =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    let sections = parse_sections(&contents);
    let min = number_value(&sections, section, "MIN", -1.0);
    let max = number_value(&sections, section, "MAX", 1.0);
    if (min > max) == inverted {
        return Ok(());
    }
    let updated = configuration::set_ini_value(
        &configuration::set_ini_value(&contents, section, "MIN", &max.to_string()),
        section,
        "MAX",
        &min.to_string(),
    );
    preferences::atomic_write(&path, &updated)
}

pub fn capture(
    installation: &Installation,
    kind: CaptureKind,
    generation: Arc<AtomicU64>,
    token: u64,
) -> Result<Option<CapturedInput>> {
    let contents = fs::read_to_string(controls_path(installation))
        .context("could not read controls before listening for input")?;
    let controllers = controllers(&parse_sections(&contents));
    let joysticks = joysticks(&controllers);
    let mut readers = joysticks
        .into_iter()
        .filter(|joystick| joystick.controller_index >= 0)
        .filter_map(|joystick| {
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&joystick.path)
                .ok()
                .map(|file| (joystick, file))
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        !readers.is_empty(),
        "no readable Linux joystick matches the controllers in controls.ini"
    );

    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(12) {
        if generation.load(Ordering::Relaxed) != token {
            return Ok(None);
        }
        for (joystick, reader) in &mut readers {
            let mut event = [0_u8; 8];
            loop {
                match reader.read(&mut event) {
                    Ok(8) => {
                        let event_type = event[6];
                        if event_type & 0x80 != 0 {
                            continue;
                        }
                        let base_type = event_type & !0x80;
                        let input = i32::from(event[7]);
                        let event_value = i16::from_ne_bytes([event[4], event[5]]);
                        let accepted = match kind {
                            CaptureKind::Axis => {
                                base_type == 0x02 && event_value.unsigned_abs() > 8_000
                            }
                            CaptureKind::Button => base_type == 0x01 && event_value != 0,
                        };
                        if accepted {
                            return Ok(Some(CapturedInput {
                                controller: joystick.controller_index,
                                input,
                            }));
                        }
                    }
                    Ok(_) => break,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                    Err(error) => {
                        log::warn!("could not read {}: {error}", joystick.path.display());
                        break;
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(8));
    }
    Ok(None)
}

pub fn monitor(
    installation: &Installation,
    generation: Arc<AtomicU64>,
    token: u64,
    updates: UnboundedSender<String>,
) -> Result<()> {
    let contents = fs::read_to_string(controls_path(installation))
        .context("could not read controls before monitoring input")?;
    let controllers = controllers(&parse_sections(&contents));
    let mut readers = joystick_readers(&controllers);
    anyhow::ensure!(
        !readers.is_empty(),
        "no readable Linux joystick matches the controllers in controls.ini"
    );

    let mut dirty = true;
    let mut last_update = Instant::now() - Duration::from_millis(40);
    while generation.load(Ordering::Relaxed) == token {
        for reader in &mut readers {
            let mut event = [0_u8; 8];
            loop {
                match reader.file.read(&mut event) {
                    Ok(8) => {
                        dirty |= apply_live_event(reader, &event);
                    }
                    Ok(_) => break,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                    Err(error) => {
                        log::warn!("could not read {}: {error}", reader.joystick.path.display());
                        break;
                    }
                }
            }
        }
        if dirty && last_update.elapsed() >= Duration::from_millis(33) {
            if updates.send(live_input_json(&readers)?).is_err() {
                return Ok(());
            }
            dirty = false;
            last_update = Instant::now();
        }
        thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

pub fn save_preset(installation: &Installation, name: &str) -> Result<()> {
    let name = validate_preset_name(name)?;
    let source = controls_path(installation);
    let contents = fs::read_to_string(&source)
        .with_context(|| format!("could not read {}", source.display()))?;
    let path = user_presets_root(installation).join(format!("{name}.ini"));
    preferences::atomic_write(&path, &contents)
}

pub fn load_preset(installation: &Installation, source: &str, name: &str) -> Result<()> {
    let name = validate_preset_name(name)?;
    let path = preset_root(installation, source)?.join(format!("{name}.ini"));
    anyhow::ensure!(path.is_file(), "controls preset does not exist: {name}");
    let contents =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    preferences::atomic_write(&controls_path(installation), &contents)
}

pub fn delete_preset(installation: &Installation, name: &str) -> Result<()> {
    let name = validate_preset_name(name)?;
    let path = user_presets_root(installation).join(format!("{name}.ini"));
    anyhow::ensure!(
        path.is_file(),
        "saved controls preset does not exist: {name}"
    );
    fs::remove_file(&path).with_context(|| format!("could not delete {}", path.display()))
}

fn controls_path(installation: &Installation) -> PathBuf {
    installation.documents_root.join("cfg/controls.ini")
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
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        let assignment = line.split(';').next().unwrap_or(line);
        if let Some((key, value)) = assignment.split_once('=') {
            section
                .values
                .insert(key.trim().to_ascii_uppercase(), value.trim().to_owned());
        }
    }
    sections
}

fn section<'a>(sections: &'a [IniSection], name: &str) -> Option<&'a IniSection> {
    sections
        .iter()
        .find(|section| section.name.eq_ignore_ascii_case(name))
}

fn value(sections: &[IniSection], section_name: &str, key: &str, default: &str) -> String {
    section(sections, section_name)
        .and_then(|section| section.values.get(&key.to_ascii_uppercase()))
        .cloned()
        .unwrap_or_else(|| default.to_owned())
}

fn number(sections: &[IniSection], section: &str, key: &str) -> i32 {
    value(sections, section, key, "-1").parse().unwrap_or(-1)
}

fn controllers(sections: &[IniSection]) -> Vec<Controller> {
    let Some(section) = section(sections, "CONTROLLERS") else {
        return Vec::new();
    };
    let mut devices = section
        .values
        .iter()
        .filter_map(|(key, name)| {
            let index = key.strip_prefix("CON")?.parse::<i32>().ok()?;
            Some(Controller {
                index,
                name: name.clone(),
                guid: section
                    .values
                    .get(&format!("PGUID{index}"))
                    .cloned()
                    .unwrap_or_default(),
                connected: false,
            })
        })
        .collect::<Vec<_>>();
    devices.sort_by_key(|device| device.index);
    devices
}

fn system_devices(controllers: &[Controller]) -> Vec<SystemDevice> {
    joysticks(controllers)
        .into_iter()
        .map(|joystick| SystemDevice {
            readable: OpenOptions::new().read(true).open(&joystick.path).is_ok(),
            path: joystick.path.to_string_lossy().into_owned(),
            name: joystick.name,
            controller_index: joystick.controller_index,
        })
        .collect()
}

fn joysticks(controllers: &[Controller]) -> Vec<Joystick> {
    let Ok(entries) = fs::read_dir("/dev/input") else {
        return Vec::new();
    };
    let mut devices = entries
        .flatten()
        .filter_map(|entry| {
            let filename = entry.file_name().to_string_lossy().into_owned();
            let index = filename.strip_prefix("js")?.parse::<u32>().ok()?;
            let name = session::read_optional(
                &Path::new("/sys/class/input")
                    .join(&filename)
                    .join("device/name"),
            )
            .trim()
            .to_owned();
            if name.is_empty() {
                return None;
            }
            let normalized = normalize_name(&name);
            let controller_index = controllers
                .iter()
                .find(|controller| normalize_name(&controller.name) == normalized)
                .map_or(-1, |controller| controller.index);
            Some((
                index,
                Joystick {
                    path: entry.path(),
                    name,
                    controller_index,
                },
            ))
        })
        .collect::<Vec<_>>();
    devices.sort_by_key(|(index, _)| *index);
    devices.into_iter().map(|(_, device)| device).collect()
}

fn joystick_readers(controllers: &[Controller]) -> Vec<JoystickReader> {
    joysticks(controllers)
        .into_iter()
        .filter(|joystick| joystick.controller_index >= 0)
        .filter_map(|joystick| {
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&joystick.path)
                .ok()
                .map(|file| JoystickReader {
                    joystick,
                    file,
                    axes: vec![0.0; 16],
                    buttons: vec![false; 128],
                })
        })
        .collect()
}

fn apply_live_event(reader: &mut JoystickReader, event: &[u8; 8]) -> bool {
    let event_type = event[6] & !0x80;
    let input = usize::from(event[7]);
    let value = i16::from_ne_bytes([event[4], event[5]]);
    match event_type {
        0x01 if input < reader.buttons.len() => {
            let pressed = value != 0;
            let changed = reader.buttons[input] != pressed;
            reader.buttons[input] = pressed;
            changed
        }
        0x02 if input < reader.axes.len() => {
            let position = if value < 0 {
                f64::from(value) / 32768.0
            } else {
                f64::from(value) / 32767.0
            };
            let changed = (reader.axes[input] - position).abs() > 0.0001;
            reader.axes[input] = position.clamp(-1.0, 1.0);
            changed
        }
        _ => false,
    }
}

fn live_input_json(readers: &[JoystickReader]) -> Result<String> {
    serde_json::to_string(&LiveInputModel {
        devices: readers
            .iter()
            .map(|reader| LiveDevice {
                controller_index: reader.joystick.controller_index,
                name: &reader.joystick.name,
                axes: &reader.axes,
                buttons: &reader.buttons,
            })
            .collect(),
    })
    .context("could not serialize live controller input")
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn device_name(devices: &[Controller], joy: i32) -> String {
    devices
        .iter()
        .find(|device| device.index == joy)
        .map(|device| device.name.clone())
        .unwrap_or_default()
}

fn axis_binding(
    sections: &[IniSection],
    devices: &[Controller],
    section: &str,
    label: &str,
) -> AxisBinding {
    let joy = number(sections, section, "JOY");
    let axis = number(sections, section, "AXLE");
    let device_name = device_name(devices, joy);
    AxisBinding {
        section: section.to_owned(),
        label: label.to_owned(),
        joy,
        axis,
        assignment: if joy >= 0 && axis >= 0 {
            format!("{device_name} · Axis {}", axis + 1)
        } else {
            "Not assigned".to_owned()
        },
        min: value(sections, section, "MIN", "-1"),
        max: value(sections, section, "MAX", "1"),
        gamma: value(sections, section, "GAMMA", "1"),
        inverted: number_value(sections, section, "MIN", -1.0)
            > number_value(sections, section, "MAX", 1.0),
        device_name,
    }
}

fn is_button_binding(section: &IniSection) -> bool {
    section.values.contains_key("BUTTON")
        && section.values.contains_key("JOY")
        && !AXIS_SECTIONS
            .iter()
            .any(|(name, _)| section.name.eq_ignore_ascii_case(name))
        && !section.name.eq_ignore_ascii_case("SHIFTER")
}

fn button_binding(section: &IniSection, devices: &[Controller]) -> ButtonBinding {
    let joy = section
        .values
        .get("JOY")
        .and_then(|value| value.parse().ok())
        .unwrap_or(-1);
    let button = section
        .values
        .get("BUTTON")
        .and_then(|value| value.parse().ok())
        .unwrap_or(-1);
    let device_name = device_name(devices, joy);
    let key = section.values.get("KEY").cloned().unwrap_or_default();
    ButtonBinding {
        label: binding_label(&section.name),
        category: binding_category(&section.name),
        section: section.name.clone(),
        joy,
        button,
        assignment: if joy >= 0 && button >= 0 {
            format!("{device_name} · Button {}", button + 1)
        } else {
            "Not assigned".to_owned()
        },
        key_assignment: key_label(&key),
        xbox_button: section
            .values
            .get("XBOXBUTTON")
            .filter(|value| value.as_str() != "-1")
            .cloned()
            .unwrap_or_default(),
        key,
        device_name,
    }
}

fn binding_category(section: &str) -> &'static str {
    if section.starts_with("__EXT_") || section.starts_with("__CM_") {
        "Patch"
    } else if matches!(
        section,
        "ACTION_HEADLIGHTS"
            | "ACTION_HEADLIGHTS_FLASH"
            | "ACTION_HORN"
            | "ACTION_CHANGE_CAMERA"
            | "GLANCELEFT"
            | "GLANCERIGHT"
            | "GLANCEBACK"
            | "HIDE_APPS"
            | "SHOW_DAMAGE"
            | "HIDE_DAMAGE"
    ) {
        "View"
    } else if section.starts_with("ACTION_")
        || matches!(
            section,
            "NEXT_CAR"
                | "PREVIOUS_CAR"
                | "PLAYER_CAR"
                | "NEXT_LAP"
                | "PREVIOUS_LAP"
                | "PAUSE_REPLAY"
                | "START_REPLAY"
                | "SLOWMO"
                | "FFWD"
                | "RESET_RACE"
        )
    {
        "System"
    } else {
        "Driving"
    }
}

fn binding_label(section: &str) -> String {
    match section {
        "GEARUP" => "Shift up".to_owned(),
        "GEARDN" => "Shift down".to_owned(),
        "BALANCEUP" => "Brake bias up".to_owned(),
        "BALANCEDN" => "Brake bias down".to_owned(),
        "TCUP" => "Traction control up".to_owned(),
        "TCDN" => "Traction control down".to_owned(),
        "ABSUP" => "ABS up".to_owned(),
        "ABSDN" => "ABS down".to_owned(),
        "GLANCELEFT" => "Look left".to_owned(),
        "GLANCERIGHT" => "Look right".to_owned(),
        "GLANCEBACK" => "Look back".to_owned(),
        "ACTION_HEADLIGHTS" => "Headlights".to_owned(),
        "ACTION_HEADLIGHTS_FLASH" => "Flash headlights".to_owned(),
        "ACTION_HORN" => "Horn".to_owned(),
        "ACTION_CHANGE_CAMERA" => "Change camera".to_owned(),
        _ => session::humanize(
            section
                .trim_start_matches("__EXT_")
                .trim_start_matches("__CM_")
                .trim_start_matches("ACTION_"),
        ),
    }
}

fn key_label(value: &str) -> String {
    if value.is_empty() || value == "-1" {
        return "Not assigned".to_owned();
    }
    let Some(hex) = value.strip_prefix("0x") else {
        return value.to_owned();
    };
    let Ok(code) = u32::from_str_radix(hex, 16) else {
        return value.to_owned();
    };
    match code {
        0x20 => "Space".to_owned(),
        0x25 => "Left Arrow".to_owned(),
        0x26 => "Up Arrow".to_owned(),
        0x27 => "Right Arrow".to_owned(),
        0x28 => "Down Arrow".to_owned(),
        0x2E => "Delete".to_owned(),
        0x60..=0x69 => format!("Numpad {}", code - 0x60),
        0xA0 => "Left Shift".to_owned(),
        0xA2 => "Left Control".to_owned(),
        0x30..=0x39 | 0x41..=0x5A => {
            char::from_u32(code).map_or_else(|| value.to_owned(), |character| character.to_string())
        }
        _ => value.to_owned(),
    }
}

fn shifter(sections: &[IniSection], devices: &[Controller]) -> Shifter {
    let joy = number(sections, "SHIFTER", "JOY");
    let device_name = device_name(devices, joy);
    let gears = [
        ("GEAR_1", "1st"),
        ("GEAR_2", "2nd"),
        ("GEAR_3", "3rd"),
        ("GEAR_4", "4th"),
        ("GEAR_5", "5th"),
        ("GEAR_6", "6th"),
        ("GEAR_7", "7th"),
        ("GEAR_R", "Reverse"),
    ]
    .into_iter()
    .map(|(key, label)| {
        let button = number(sections, "SHIFTER", key);
        GearBinding {
            key,
            label,
            button,
            assignment: if joy >= 0 && button >= 0 {
                format!("{device_name} · Button {}", button + 1)
            } else {
                "Not assigned".to_owned()
            },
        }
    })
    .collect();
    Shifter {
        active: value(sections, "SHIFTER", "ACTIVE", "0") == "1",
        joy,
        device_name,
        gears,
    }
}

fn force_feedback(sections: &[IniSection]) -> BTreeMap<String, String> {
    let mut values = selected_values(sections, "STEER", &["FF_GAIN", "FILTER_FF"]);
    for (section, keys) in [
        (
            "FF_TWEAKS",
            &["MIN_FF", "CENTER_BOOST_GAIN", "CENTER_BOOST_RANGE"][..],
        ),
        ("FF_ENHANCEMENT", &["CURBS", "ROAD", "SLIPS", "ABS"][..]),
        ("FF_ENHANCEMENT_2", &["UNDERSTEER"][..]),
        ("FF_SKIP_STEPS", &["VALUE"][..]),
    ] {
        for key in keys {
            values.insert(
                format!("{section}.{key}"),
                value(sections, section, key, "0"),
            );
        }
    }
    values
}

fn selected_values(
    sections: &[IniSection],
    section_name: &str,
    keys: &[&str],
) -> BTreeMap<String, String> {
    keys.iter()
        .map(|key| ((*key).to_owned(), value(sections, section_name, key, "0")))
        .collect()
}

fn section_values(sections: &[IniSection], name: &str) -> BTreeMap<String, String> {
    section(sections, name)
        .map(|section| section.values.clone())
        .unwrap_or_default()
}

fn number_value(sections: &[IniSection], section: &str, key: &str, default: f64) -> f64 {
    value(sections, section, key, "").parse().unwrap_or(default)
}

fn list_presets(installation: &Installation) -> Vec<Preset> {
    let mut presets = presets_in(&user_presets_root(installation), "user");
    presets.extend(presets_in(
        &installation.game_root.join("cfg/controllers/presets"),
        "built-in",
    ));
    presets
}

fn presets_in(root: &Path, source: &'static str) -> Vec<Preset> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut presets = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().is_none_or(|extension| extension != "ini") {
                return None;
            }
            Some(Preset {
                name: path.file_stem()?.to_string_lossy().into_owned(),
                source,
            })
        })
        .collect::<Vec<_>>();
    presets.sort_by_key(|preset| preset.name.to_ascii_lowercase());
    presets
}

fn user_presets_root(installation: &Installation) -> PathBuf {
    installation
        .documents_root
        .join("cfg/controllers/savedsetups")
}

fn preset_root(installation: &Installation, source: &str) -> Result<PathBuf> {
    match source {
        "user" => Ok(user_presets_root(installation)),
        "built-in" => Ok(installation.game_root.join("cfg/controllers/presets")),
        _ => anyhow::bail!("unknown controls preset source"),
    }
}

fn validate_preset_name(name: &str) -> Result<&str> {
    let name = name.trim();
    anyhow::ensure!(!name.is_empty(), "preset name is empty");
    anyhow::ensure!(name.len() <= 80, "preset name is too long");
    anyhow::ensure!(
        !name.contains(['/', '\\', '\n', '\r']) && name != "." && name != "..",
        "preset name contains invalid characters"
    );
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    const SAMPLE: &str = "[HEADER]\nINPUT_METHOD=WHEEL\n\n[CONTROLLERS]\nCON0=Test Wheel\nPGUID0=123\n\n[STEER]\nJOY=0\nAXLE=1\nMIN=-1\nMAX=1\n\n[GEARUP]\nJOY=0\nBUTTON=3\nKEY=0x20\n";

    #[test]
    fn parses_devices_axes_and_buttons() {
        let sections = parse_sections(SAMPLE);
        let devices = controllers(&sections);
        let axis = axis_binding(&sections, &devices, "STEER", "Steering");
        let button = button_binding(section(&sections, "GEARUP").unwrap(), &devices);

        assert_eq!(devices[0].name, "Test Wheel");
        assert_eq!(axis.assignment, "Test Wheel · Axis 2");
        assert_eq!(button.assignment, "Test Wheel · Button 4");
        assert_eq!(button.key_assignment, "Space");
    }

    #[test]
    fn updates_a_binding_in_one_configuration() {
        let updated = configuration::set_ini_value(
            &configuration::set_ini_value(SAMPLE, "GEARUP", "JOY", "2"),
            "GEARUP",
            "BUTTON",
            "7",
        );

        assert!(updated.contains("JOY=2\nBUTTON=7"));
        assert!(updated.contains("INPUT_METHOD=WHEEL"));
    }

    #[test]
    fn rejects_unsafe_preset_names() {
        assert!(validate_preset_name("Road setup").is_ok());
        assert!(validate_preset_name("../controls").is_err());
        assert!(validate_preset_name("").is_err());
    }

    #[test]
    fn loads_and_round_trips_a_saved_controls_preset() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("aclm-controls-{unique}"));
        let documents_root = root.join("documents");
        let game_root = root.join("game");
        fs::create_dir_all(documents_root.join("cfg")).unwrap();
        fs::create_dir_all(game_root.join("cfg/controllers/presets")).unwrap();
        fs::write(documents_root.join("cfg/controls.ini"), SAMPLE).unwrap();
        let installation = Installation {
            steam_root: root.clone(),
            library_root: root.clone(),
            game_root,
            proton_prefix: root.clone(),
            documents_root,
            proton_command: root.clone(),
            proton_config: None,
            runtime_root: root.clone(),
            runtime_client: root.clone(),
        };

        let model: serde_json::Value =
            serde_json::from_str(&load_json(&installation).unwrap()).unwrap();
        assert_eq!(model["input_method"], "WHEEL");
        assert_eq!(model["axes"][0]["assignment"], "Test Wheel · Axis 2");

        save_preset(&installation, "Test setup").unwrap();
        set_option(&installation, "HEADER", "INPUT_METHOD", "KEYBOARD").unwrap();
        load_preset(&installation, "user", "Test setup").unwrap();
        let restored = fs::read_to_string(controls_path(&installation)).unwrap();
        assert!(restored.contains("INPUT_METHOD=WHEEL"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn updates_live_axis_and_button_state_from_joystick_events() {
        let mut reader = JoystickReader {
            joystick: Joystick {
                path: PathBuf::from("/dev/null"),
                name: "Test Wheel".to_owned(),
                controller_index: 0,
            },
            file: fs::File::open("/dev/null").unwrap(),
            axes: vec![0.0; 4],
            buttons: vec![false; 8],
        };
        let mut axis_event = [0_u8; 8];
        axis_event[4..6].copy_from_slice(&16_384_i16.to_ne_bytes());
        axis_event[6] = 0x82;
        axis_event[7] = 2;
        let mut button_event = [0_u8; 8];
        button_event[4..6].copy_from_slice(&1_i16.to_ne_bytes());
        button_event[6] = 0x01;
        button_event[7] = 5;

        assert!(apply_live_event(&mut reader, &axis_event));
        assert!(apply_live_event(&mut reader, &button_event));
        assert!((reader.axes[2] - 0.5).abs() < 0.001);
        assert!(reader.buttons[5]);

        let json = live_input_json(&[reader]).unwrap();
        assert!(json.contains("\"controller_index\":0"));
        assert!(json.contains("true"));
    }
}
