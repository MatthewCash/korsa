#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        include!("ac-linux-manager/src/widgets.h");
        type QString = cxx_qt_lib::QString;

        fn run_aclm_widgets() -> i32;
    }

    extern "RustQt" {
        #[qobject]
        #[qproperty(bool, scanning)]
        #[qproperty(bool, installation_found)]
        #[qproperty(bool, launching)]
        #[qproperty(bool, race_running)]
        #[qproperty(bool, control_capture_active)]
        #[qproperty(bool, control_monitoring)]
        #[qproperty(bool, content_busy)]
        #[qproperty(bool, online_busy)]
        #[qproperty(QString, status)]
        #[qproperty(QString, error_message)]
        #[qproperty(QString, control_capture_target)]
        #[qproperty(QString, control_input_json)]
        #[qproperty(QString, content_preview_json)]
        #[qproperty(QString, content_history_json)]
        #[qproperty(QString, servers_json)]
        #[qproperty(QString, setups_json)]
        #[qproperty(QString, csp_releases_json)]
        #[qproperty(QString, steam_root)]
        #[qproperty(QString, library_root)]
        #[qproperty(QString, game_root)]
        #[qproperty(QString, proton_prefix)]
        #[qproperty(QString, documents_root)]
        #[qproperty(QString, proton_command)]
        #[qproperty(bool, session_loaded)]
        #[qproperty(bool, tyre_blankets)]
        #[qproperty(QString, car_name)]
        #[qproperty(QString, car_brand)]
        #[qproperty(QString, car_id)]
        #[qproperty(QString, skin_id)]
        #[qproperty(QString, skin_name)]
        #[qproperty(QString, track_name)]
        #[qproperty(QString, track_location)]
        #[qproperty(QString, track_id)]
        #[qproperty(QString, track_layout_id)]
        #[qproperty(QString, track_layout)]
        #[qproperty(QString, weather_name)]
        #[qproperty(QString, session_type)]
        #[qproperty(QString, car_preview)]
        #[qproperty(QString, track_preview)]
        #[qproperty(QString, track_outline)]
        #[qproperty(QString, csp_status)]
        #[qproperty(QString, ac_settings_json)]
        #[qproperty(QString, app_layout_json)]
        #[qproperty(QString, controls_json)]
        #[qproperty(QString, csp_settings_json)]
        #[qproperty(QString, cars_json)]
        #[qproperty(QString, tracks_json)]
        #[qproperty(QString, replays_json)]
        #[qproperty(QString, weather_json)]
        #[qproperty(QString, weather_controllers_json)]
        #[qproperty(QString, conditions_json)]
        #[qproperty(QString, presets_json)]
        #[qproperty(i32, car_count)]
        #[qproperty(i32, track_count)]
        #[qproperty(i32, replay_count)]
        #[qproperty(i32, preset_count)]
        #[qproperty(i32, ac_module_count)]
        #[qproperty(i32, csp_module_count)]
        #[qproperty(i32, setup_count)]
        type Backend = super::BackendRust;

        #[qinvokable]
        fn discover(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "refreshRaceState"]
        fn refresh_race_state(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "stopRace"]
        fn stop_race(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "launchExistingSession"]
        fn launch_existing_session(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "launchConfiguration"]
        fn launch_configuration(
            self: Pin<&mut Self>,
            car_id: &QString,
            skin_id: &QString,
            track_key: &QString,
            conditions_json: &QString,
        );

        #[qinvokable]
        #[cxx_name = "toggleFavorite"]
        fn toggle_favorite(self: Pin<&mut Self>, kind: &QString, id: &QString);

        #[qinvokable]
        #[cxx_name = "toggleDashboardItem"]
        fn toggle_dashboard_item(self: Pin<&mut Self>, kind: &QString, id: &QString);

        #[qinvokable]
        #[cxx_name = "savePreset"]
        fn save_preset(
            self: Pin<&mut Self>,
            name: &QString,
            car_id: &QString,
            skin_id: &QString,
            track_key: &QString,
            conditions_json: &QString,
        );

        #[qinvokable]
        #[cxx_name = "deletePreset"]
        fn delete_preset(self: Pin<&mut Self>, id: &QString);

        #[qinvokable]
        #[cxx_name = "applyPreset"]
        fn apply_preset(self: Pin<&mut Self>, id: &QString);

        #[qinvokable]
        #[cxx_name = "applyConfiguration"]
        fn apply_configuration(
            self: Pin<&mut Self>,
            car_id: &QString,
            skin_id: &QString,
            track_key: &QString,
            conditions_json: &QString,
        );

        #[qinvokable]
        #[cxx_name = "setCspOption"]
        fn set_csp_option(
            self: Pin<&mut Self>,
            file: &QString,
            section: &QString,
            key: &QString,
            value: &QString,
        );

        #[qinvokable]
        #[cxx_name = "setAcOption"]
        fn set_ac_option(
            self: Pin<&mut Self>,
            file: &QString,
            section: &QString,
            key: &QString,
            value: &QString,
        );

        #[qinvokable]
        #[cxx_name = "setAppLayoutOption"]
        fn set_app_layout_option(
            self: Pin<&mut Self>,
            section: &QString,
            key: &QString,
            value: &QString,
        );

        #[qinvokable]
        #[cxx_name = "moveAppWindow"]
        fn move_app_window(self: Pin<&mut Self>, section: &QString, x: i32, y: i32);

        #[qinvokable]
        #[cxx_name = "setControlOption"]
        fn set_control_option(
            self: Pin<&mut Self>,
            section: &QString,
            key: &QString,
            value: &QString,
        );

        #[qinvokable]
        #[cxx_name = "captureControl"]
        fn capture_control(self: Pin<&mut Self>, section: &QString, key: &QString, label: &QString);

        #[qinvokable]
        #[cxx_name = "cancelControlCapture"]
        fn cancel_control_capture(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "setControlMonitoring"]
        fn update_control_monitoring(self: Pin<&mut Self>, enabled: bool);

        #[qinvokable]
        #[cxx_name = "clearControlBinding"]
        fn clear_control_binding(self: Pin<&mut Self>, section: &QString, key: &QString);

        #[qinvokable]
        #[cxx_name = "setControlAxisInverted"]
        fn set_control_axis_inverted(self: Pin<&mut Self>, section: &QString, inverted: bool);

        #[qinvokable]
        #[cxx_name = "saveControlPreset"]
        fn save_control_preset(self: Pin<&mut Self>, name: &QString);

        #[qinvokable]
        #[cxx_name = "loadControlPreset"]
        fn load_control_preset(self: Pin<&mut Self>, source: &QString, name: &QString);

        #[qinvokable]
        #[cxx_name = "deleteControlPreset"]
        fn delete_control_preset(self: Pin<&mut Self>, name: &QString);

        #[qinvokable]
        #[cxx_name = "refreshOnlineServers"]
        fn refresh_online_servers(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "joinOnlineServer"]
        fn join_online_server(self: Pin<&mut Self>, index: i32, password: &QString, car: &QString);

        #[qinvokable]
        #[cxx_name = "launchShowroom"]
        fn launch_showroom(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "saveCurrentSetupAs"]
        fn save_current_setup_as(self: Pin<&mut Self>, track: &QString, name: &QString);

        #[qinvokable]
        #[cxx_name = "applySetup"]
        fn apply_setup(self: Pin<&mut Self>, id: &QString);

        #[qinvokable]
        #[cxx_name = "deleteSetup"]
        fn delete_setup(self: Pin<&mut Self>, id: &QString);

        #[qinvokable]
        #[cxx_name = "refreshCspReleases"]
        fn refresh_csp_releases(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "installCspRelease"]
        fn install_csp_release(self: Pin<&mut Self>, version: &QString);

        #[qinvokable]
        #[cxx_name = "inspectContentArchive"]
        fn inspect_content_archive(self: Pin<&mut Self>, path: &QString);

        #[qinvokable]
        #[cxx_name = "installInspectedArchive"]
        fn install_inspected_archive(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "rollbackContentInstall"]
        fn rollback_content_install(self: Pin<&mut Self>, id: &QString);

        #[qinvokable]
        #[cxx_name = "setTyreBlankets"]
        fn update_tyre_blankets(self: Pin<&mut Self>, enabled: bool);
    }

    impl cxx_qt::Threading for Backend {}
}

use crate::{
    ac_settings, app_layout, configuration, content, controls, csp, csp_versions, discovery,
    installer, launcher, online, preferences, runtime, session, setups,
};
use anyhow::Context;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub struct BackendRust {
    scanning: bool,
    installation_found: bool,
    launching: bool,
    race_running: bool,
    race_state_busy: bool,
    control_capture_active: bool,
    control_monitoring: bool,
    content_busy: bool,
    online_busy: bool,
    status: QString,
    error_message: QString,
    control_capture_target: QString,
    control_input_json: QString,
    content_preview_json: QString,
    content_history_json: QString,
    servers_json: QString,
    setups_json: QString,
    csp_releases_json: QString,
    steam_root: QString,
    library_root: QString,
    game_root: QString,
    proton_prefix: QString,
    documents_root: QString,
    proton_command: QString,
    session_loaded: bool,
    tyre_blankets: bool,
    car_name: QString,
    car_brand: QString,
    car_id: QString,
    skin_id: QString,
    skin_name: QString,
    track_name: QString,
    track_location: QString,
    track_id: QString,
    track_layout_id: QString,
    track_layout: QString,
    weather_name: QString,
    session_type: QString,
    car_preview: QString,
    track_preview: QString,
    track_outline: QString,
    csp_status: QString,
    ac_settings_json: QString,
    app_layout_json: QString,
    controls_json: QString,
    csp_settings_json: QString,
    cars_json: QString,
    tracks_json: QString,
    replays_json: QString,
    weather_json: QString,
    weather_controllers_json: QString,
    conditions_json: QString,
    presets_json: QString,
    car_count: i32,
    track_count: i32,
    replay_count: i32,
    preset_count: i32,
    ac_module_count: i32,
    csp_module_count: i32,
    setup_count: i32,
    installation: Option<discovery::Installation>,
    catalog: content::Catalog,
    preferences: preferences::Preferences,
    control_capture_generation: Arc<AtomicU64>,
    control_monitor_generation: Arc<AtomicU64>,
    pending_archive: Option<installer::PendingArchive>,
    online_servers: Vec<online::Server>,
}

impl Default for BackendRust {
    fn default() -> Self {
        Self {
            scanning: false,
            installation_found: false,
            launching: false,
            race_running: false,
            race_state_busy: false,
            control_capture_active: false,
            control_monitoring: false,
            content_busy: false,
            online_busy: false,
            status: QString::from("Ready to scan"),
            error_message: QString::default(),
            control_capture_target: QString::default(),
            control_input_json: QString::from("{\"devices\":[]}"),
            content_preview_json: QString::from("{}"),
            content_history_json: QString::from("[]"),
            servers_json: QString::from("[]"),
            setups_json: QString::from("[]"),
            csp_releases_json: QString::from("[]"),
            steam_root: QString::default(),
            library_root: QString::default(),
            game_root: QString::default(),
            proton_prefix: QString::default(),
            documents_root: QString::default(),
            proton_command: QString::default(),
            session_loaded: false,
            tyre_blankets: false,
            car_name: QString::default(),
            car_brand: QString::default(),
            car_id: QString::default(),
            skin_id: QString::default(),
            skin_name: QString::default(),
            track_name: QString::default(),
            track_location: QString::default(),
            track_id: QString::default(),
            track_layout_id: QString::default(),
            track_layout: QString::default(),
            weather_name: QString::default(),
            session_type: QString::default(),
            car_preview: QString::default(),
            track_preview: QString::default(),
            track_outline: QString::default(),
            csp_status: QString::default(),
            ac_settings_json: QString::from("[]"),
            app_layout_json: QString::from("{}"),
            controls_json: QString::from("{}"),
            csp_settings_json: QString::from("[]"),
            cars_json: QString::from("[]"),
            tracks_json: QString::from("[]"),
            replays_json: QString::from("[]"),
            weather_json: QString::from("[]"),
            weather_controllers_json: QString::from("[]"),
            conditions_json: QString::from("{}"),
            presets_json: QString::from("[]"),
            car_count: 0,
            track_count: 0,
            replay_count: 0,
            preset_count: 0,
            ac_module_count: 0,
            csp_module_count: 0,
            setup_count: 0,
            installation: None,
            catalog: content::Catalog::default(),
            preferences: preferences::Preferences::default(),
            control_capture_generation: Arc::new(AtomicU64::new(0)),
            control_monitor_generation: Arc::new(AtomicU64::new(0)),
            pending_archive: None,
            online_servers: Vec::new(),
        }
    }
}

impl ffi::Backend {
    pub fn discover(mut self: Pin<&mut Self>) {
        if *self.scanning() {
            return;
        }

        log::info!("scanning Steam libraries and installed content");
        self.as_mut().set_scanning(true);
        self.as_mut().set_installation_found(false);
        self.as_mut().set_session_loaded(false);
        self.as_mut().set_car_count(0);
        self.as_mut().set_track_count(0);
        self.as_mut().set_replay_count(0);
        self.as_mut().set_error_message(QString::default());
        self.as_mut().rust_mut().get_mut().installation = None;
        self.as_mut()
            .set_status(QString::from("Scanning Steam libraries..."));

        let qt_thread = self.qt_thread();
        if let Err(error) = runtime::spawn(async move {
            let result = tokio::task::spawn_blocking(|| {
                let installation = discovery::discover()?;
                let (session, session_error) = match session::load(&installation) {
                    Ok(session) => (Some(session), None),
                    Err(error) => {
                        log::warn!("could not load the current race session: {error:#}");
                        (None, Some(format!("Session data unavailable: {error:#}")))
                    }
                };
                let preferences = preferences::Preferences::load().unwrap_or_else(|error| {
                    log::warn!("could not load application preferences: {error:#}");
                    preferences::Preferences::default()
                });
                let catalog = match content::load(&installation, &preferences) {
                    Ok(catalog) => catalog,
                    Err(error) => {
                        log::warn!("could not index installed content: {error:#}");
                        content::Catalog::default()
                    }
                };
                let weather_json =
                    configuration::load_weather_json(&installation).unwrap_or_else(|error| {
                        log::warn!("could not index weather presets: {error:#}");
                        "[]".to_owned()
                    });
                let weather_controllers_json = configuration::load_weather_controllers_json(
                    &installation,
                )
                .unwrap_or_else(|error| {
                    log::warn!("could not index weather controllers: {error:#}");
                    "[]".to_owned()
                });
                let conditions =
                    configuration::load_conditions(&installation).unwrap_or_else(|error| {
                        log::warn!("could not load track conditions: {error:#}");
                        preferences::Conditions::default()
                    });
                let csp_settings_json = csp::load_json(&installation).unwrap_or_else(|error| {
                    log::warn!("could not index CSP settings: {error:#}");
                    "[]".to_owned()
                });
                let ac_settings_json =
                    ac_settings::load_json(&installation).unwrap_or_else(|error| {
                        log::warn!("could not index Assetto Corsa settings: {error:#}");
                        "[]".to_owned()
                    });
                let controls_json = controls::load_json(&installation).unwrap_or_else(|error| {
                    log::warn!("could not load controls configuration: {error:#}");
                    "{}".to_owned()
                });
                let app_layout_json =
                    app_layout::load_json(&installation).unwrap_or_else(|error| {
                        log::warn!("could not load in-game app layout: {error:#}");
                        "{}".to_owned()
                    });
                let assists =
                    session::read_optional(&installation.documents_root.join("cfg/assists.ini"));
                let tyre_blankets = session::ini_value(&assists, "ASSISTS", "TYRE_BLANKETS")
                    .is_some_and(|value| value == "1");
                Ok::<_, anyhow::Error>((
                    installation,
                    session,
                    session_error,
                    catalog,
                    preferences,
                    weather_json,
                    weather_controllers_json,
                    conditions,
                    ac_settings_json,
                    controls_json,
                    app_layout_json,
                    csp_settings_json,
                    tyre_blankets,
                ))
            })
            .await;
            let result = match result {
                Ok(result) => result,
                Err(error) => Err(anyhow::anyhow!("discovery worker failed: {error}")),
            };

            if let Ok((installation, ..)) = &result {
                let installation = installation.clone();
                if let Err(error) = runtime::spawn(async move {
                    if let Err(error) = launcher::ensure_steam_service(&installation).await {
                        log::warn!("could not start Steam in the background: {error:#}");
                    }
                }) {
                    log::warn!("could not schedule Steam background startup: {error:#}");
                }
            }

            if let Err(error) = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_scanning(false);

                match result {
                    Ok((
                        installation,
                        session,
                        session_error,
                        catalog,
                        preferences,
                        weather_json,
                        weather_controllers_json,
                        conditions,
                        ac_settings_json,
                        controls_json,
                        app_layout_json,
                        csp_settings_json,
                        tyre_blankets,
                    )) => {
                        let documents_ready =
                            discovery::validate_documents_root(&installation).is_ok();
                        backend.as_mut().set_installation_found(true);
                        backend.as_mut().set_status(QString::from(
                            if documents_ready && session.is_some() {
                                "Ready to drive"
                            } else if documents_ready {
                                "Assetto Corsa is ready"
                            } else {
                                "Game found; Proton setup is incomplete"
                            },
                        ));
                        backend
                            .as_mut()
                            .set_steam_root(path_string(&installation.steam_root));
                        backend
                            .as_mut()
                            .set_library_root(path_string(&installation.library_root));
                        backend
                            .as_mut()
                            .set_game_root(path_string(&installation.game_root));
                        backend
                            .as_mut()
                            .set_proton_prefix(path_string(&installation.proton_prefix));
                        backend
                            .as_mut()
                            .set_documents_root(path_string(&installation.documents_root));
                        backend
                            .as_mut()
                            .set_proton_command(path_string(&installation.proton_command));
                        apply_catalog(backend.as_mut(), &catalog);
                        backend.as_mut().set_weather_json(text(&weather_json));
                        backend
                            .as_mut()
                            .set_weather_controllers_json(text(&weather_controllers_json));
                        backend.as_mut().set_conditions_json(text(
                            &configuration::conditions_json(&conditions)
                                .unwrap_or_else(|_| "{}".to_owned()),
                        ));
                        backend.as_mut().set_presets_json(text(
                            &configuration::presets_json(&preferences)
                                .unwrap_or_else(|_| "[]".to_owned()),
                        ));
                        backend.as_mut().set_preset_count(
                            i32::try_from(preferences.presets.len()).unwrap_or(i32::MAX),
                        );
                        backend
                            .as_mut()
                            .set_csp_settings_json(text(&csp_settings_json));
                        backend
                            .as_mut()
                            .set_ac_settings_json(text(&ac_settings_json));
                        backend.as_mut().set_controls_json(text(&controls_json));
                        backend
                            .as_mut()
                            .set_app_layout_json(text(&app_layout_json));
                        backend.as_mut().set_ac_module_count(
                            serde_json::from_str::<Vec<serde_json::Value>>(&ac_settings_json)
                                .ok()
                                .and_then(|modules| i32::try_from(modules.len()).ok())
                                .unwrap_or(0),
                        );
                        backend.as_mut().set_tyre_blankets(tyre_blankets);
                        backend.as_mut().set_csp_module_count(
                            serde_json::from_str::<Vec<serde_json::Value>>(&csp_settings_json)
                                .ok()
                                .and_then(|modules| i32::try_from(modules.len()).ok())
                                .unwrap_or(0),
                        );
                        backend.as_mut().set_content_history_json(text(
                            &installer::history_json(&installation)
                                .unwrap_or_else(|_| "[]".to_owned()),
                        ));
                        if let Some(session) = session {
                            let setups_json = setups::load_json(&installation, &session.car_id)
                                .unwrap_or_else(|_| "[]".to_owned());
                            backend.as_mut().set_setup_count(
                                serde_json::from_str::<Vec<serde_json::Value>>(&setups_json)
                                    .ok()
                                    .and_then(|items| i32::try_from(items.len()).ok())
                                    .unwrap_or(0),
                            );
                            backend.as_mut().set_setups_json(text(&setups_json));
                            log::info!(
                                "loaded session: {} at {} ({})",
                                session.car_name,
                                session.track_name,
                                session.csp_status
                            );
                            apply_session(backend.as_mut(), &session);
                        }
                        if let Some(error) = session_error {
                            backend.as_mut().set_error_message(QString::from(error));
                        }
                        log::info!(
                            "found Assetto Corsa at {} with {} cars, {} track layouts, and {} replays",
                            installation.game_root.display(),
                            catalog.car_count,
                            catalog.track_count,
                            catalog.replay_count
                        );
                        backend.as_mut().rust_mut().get_mut().installation =
                            Some(installation.clone());
                        backend.as_mut().rust_mut().get_mut().catalog = catalog;
                        backend.as_mut().rust_mut().get_mut().preferences = preferences;
                        crate::ipc::notify_dashboard_options_changed();
                    }
                    Err(error) => {
                        log::warn!("Assetto Corsa discovery failed: {error:#}");
                        backend
                            .as_mut()
                            .set_status(QString::from("Installation not found"));
                        backend
                            .as_mut()
                            .set_error_message(QString::from(error.to_string()));
                    }
                }
            }) {
                log::error!("could not return discovery result to Qt: {error}");
            }
        }) {
            self.as_mut().set_scanning(false);
            self.as_mut().set_status(QString::from("Discovery failed"));
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn refresh_online_servers(mut self: Pin<&mut Self>) {
        if *self.online_busy() || *self.scanning() {
            return;
        }
        let Some(installation) = self.rust().installation.clone() else {
            return;
        };
        self.as_mut().set_online_busy(true);
        self.as_mut().set_error_message(QString::default());
        self.as_mut()
            .set_status(QString::from("Loading official Assetto Corsa lobby..."));
        let qt_thread = self.qt_thread();
        if let Err(error) = runtime::spawn(async move {
            let result = online::fetch(&installation).await;
            if let Err(error) = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_online_busy(false);
                match result {
                    Ok(servers) => {
                        let json =
                            online::servers_json(&servers).unwrap_or_else(|_| "[]".to_owned());
                        let count = servers.len();
                        backend.as_mut().rust_mut().get_mut().online_servers = servers;
                        backend.as_mut().set_servers_json(text(&json));
                        backend
                            .as_mut()
                            .set_status(QString::from(format!("Loaded {count} online servers")));
                    }
                    Err(error) => set_operation_error(
                        backend.as_mut(),
                        "Could not load online servers",
                        error,
                    ),
                }
            }) {
                log::error!("could not return online servers to Qt: {error}");
            }
        }) {
            self.as_mut().set_online_busy(false);
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn join_online_server(
        mut self: Pin<&mut Self>,
        index: i32,
        password: &QString,
        car: &QString,
    ) {
        if *self.launching() {
            return;
        }
        let Some(installation) = self.rust().installation.clone() else {
            return;
        };
        let Ok(index) = usize::try_from(index) else {
            self.as_mut()
                .set_error_message(QString::from("Invalid online server selection"));
            return;
        };
        let Some(server) = self.rust().online_servers.get(index).cloned() else {
            self.as_mut()
                .set_error_message(QString::from("Online server is no longer available"));
            return;
        };
        let car = car.to_string();
        let password = password.to_string();
        self.as_mut().set_launching(true);
        self.as_mut().set_error_message(QString::default());
        self.as_mut()
            .set_status(QString::from("Configuring and joining server..."));
        let qt_thread = self.qt_thread();
        if let Err(error) = runtime::spawn(async move {
            let locked = match tokio::task::spawn_blocking(move || {
                let session_lock = configuration::lock_session(&installation)?;
                online::configure_join(&installation, &server, &car, &password)?;
                Ok::<_, anyhow::Error>((installation, session_lock))
            })
            .await
            {
                Ok(result) => result,
                Err(error) => Err(anyhow::anyhow!(
                    "online configuration worker failed: {error}"
                )),
            };
            let result = match locked {
                Ok((installation, session_lock)) => {
                    let result = launcher::launch_existing_session(&installation).await;
                    drop(session_lock);
                    result
                }
                Err(error) => Err(error),
            };
            if let Err(error) = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_launching(false);
                match result {
                    Ok(_) => backend
                        .as_mut()
                        .set_status(QString::from("Assetto Corsa is running")),
                    Err(error) => {
                        set_operation_error(backend.as_mut(), "Could not join online server", error)
                    }
                }
            }) {
                log::error!("could not return online launch result to Qt: {error}");
            }
        }) {
            self.as_mut().set_launching(false);
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn launch_showroom(mut self: Pin<&mut Self>) {
        if *self.launching() {
            return;
        }
        let Some(installation) = self.rust().installation.clone() else {
            return;
        };
        let car = self.car_id().to_string();
        let skin = self.skin_id().to_string();
        self.as_mut().set_launching(true);
        self.as_mut().set_error_message(QString::default());
        self.as_mut()
            .set_status(QString::from("Starting showroom..."));
        let qt_thread = self.qt_thread();
        if let Err(error) = runtime::spawn(async move {
            let locked = match tokio::task::spawn_blocking(move || {
                let session_lock = configuration::lock_session(&installation)?;
                Ok::<_, anyhow::Error>((installation, session_lock))
            })
            .await
            {
                Ok(result) => result,
                Err(error) => Err(anyhow::anyhow!("showroom launch worker failed: {error}")),
            };
            let result = match locked {
                Ok((installation, session_lock)) => {
                    let result = launcher::launch_showroom(&installation, &car, &skin).await;
                    drop(session_lock);
                    result
                }
                Err(error) => Err(error),
            };
            if let Err(error) = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_launching(false);
                match result {
                    Ok(_) => backend
                        .as_mut()
                        .set_status(QString::from("Showroom is running")),
                    Err(error) => {
                        set_operation_error(backend.as_mut(), "Could not launch showroom", error)
                    }
                }
            }) {
                log::error!("could not return showroom result to Qt: {error}");
            }
        }) {
            self.as_mut().set_launching(false);
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn save_current_setup_as(mut self: Pin<&mut Self>, track: &QString, name: &QString) {
        let Some(installation) = self.rust().installation.as_ref() else {
            return;
        };
        match setups::save_last_as(
            installation,
            &self.car_id().to_string(),
            &track.to_string(),
            &name.to_string(),
        ) {
            Ok(()) => {
                refresh_setups(self.as_mut());
                self.as_mut().set_status(QString::from("Setup saved"));
            }
            Err(error) => set_operation_error(self.as_mut(), "Could not save setup", error),
        }
    }

    pub fn apply_setup(mut self: Pin<&mut Self>, id: &QString) {
        let Some(installation) = self.rust().installation.as_ref() else {
            return;
        };
        match setups::apply(installation, &self.car_id().to_string(), &id.to_string()) {
            Ok(()) => {
                refresh_setups(self.as_mut());
                self.as_mut().set_status(QString::from("Setup selected"));
            }
            Err(error) => set_operation_error(self.as_mut(), "Could not select setup", error),
        }
    }

    pub fn delete_setup(mut self: Pin<&mut Self>, id: &QString) {
        let Some(installation) = self.rust().installation.as_ref() else {
            return;
        };
        match setups::delete(installation, &self.car_id().to_string(), &id.to_string()) {
            Ok(()) => {
                refresh_setups(self.as_mut());
                self.as_mut().set_status(QString::from("Setup deleted"));
            }
            Err(error) => set_operation_error(self.as_mut(), "Could not delete setup", error),
        }
    }

    pub fn refresh_csp_releases(mut self: Pin<&mut Self>) {
        if *self.content_busy() {
            return;
        }
        self.as_mut().set_content_busy(true);
        self.as_mut().set_error_message(QString::default());
        self.as_mut()
            .set_status(QString::from("Loading official CSP releases..."));
        let qt_thread = self.qt_thread();
        if let Err(error) = runtime::spawn(async move {
            let result = csp_versions::fetch().await;
            if let Err(error) = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_content_busy(false);
                match result {
                    Ok(releases) => {
                        let json = csp_versions::releases_json(&releases)
                            .unwrap_or_else(|_| "[]".to_owned());
                        backend.as_mut().set_csp_releases_json(text(&json));
                        backend.as_mut().set_status(QString::from(format!(
                            "Loaded {} public CSP releases",
                            releases.len()
                        )));
                    }
                    Err(error) => {
                        set_operation_error(backend.as_mut(), "Could not load CSP releases", error)
                    }
                }
            }) {
                log::error!("could not return CSP releases to Qt: {error}");
            }
        }) {
            self.as_mut().set_content_busy(false);
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn install_csp_release(mut self: Pin<&mut Self>, version: &QString) {
        if *self.content_busy() || *self.scanning() {
            return;
        }
        let Some(installation) = self.rust().installation.clone() else {
            return;
        };
        let version = version.to_string();
        self.as_mut().set_content_busy(true);
        self.as_mut().set_error_message(QString::default());
        self.as_mut()
            .set_status(QString::from(format!("Downloading CSP v{version}...")));
        let qt_thread = self.qt_thread();
        if let Err(error) = runtime::spawn(async move {
            let result = async {
                let archive = csp_versions::download(&installation, &version).await?;
                let (inspection, pending) = installer::inspect(&installation, &archive)?;
                anyhow::ensure!(
                    inspection.kind == installer::ArchiveKind::Csp,
                    "downloaded archive is not CSP"
                );
                installer::install(&installation, &pending)
            }
            .await;
            if let Err(error) = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_content_busy(false);
                match result {
                    Ok(record) => {
                        backend.as_mut().set_content_history_json(text(
                            &installer::history_json(&installation)
                                .unwrap_or_else(|_| "[]".to_owned()),
                        ));
                        backend.as_mut().set_status(QString::from(format!(
                            "Installed {}",
                            record.archive_name
                        )));
                        backend.as_mut().discover();
                    }
                    Err(error) => set_operation_error(
                        backend.as_mut(),
                        "Could not install CSP release",
                        error,
                    ),
                }
            }) {
                log::error!("could not return CSP installation to Qt: {error}");
            }
        }) {
            self.as_mut().set_content_busy(false);
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn inspect_content_archive(mut self: Pin<&mut Self>, path: &QString) {
        if *self.content_busy() || *self.scanning() {
            return;
        }
        let Some(installation) = self.rust().installation.clone() else {
            self.as_mut()
                .set_error_message(QString::from("No Assetto Corsa installation is selected"));
            return;
        };
        let path = std::path::PathBuf::from(path.to_string());
        self.as_mut().set_content_busy(true);
        self.as_mut().set_content_preview_json(QString::from("{}"));
        self.as_mut().set_error_message(QString::default());
        self.as_mut()
            .set_status(QString::from("Inspecting content archive..."));
        let qt_thread = self.qt_thread();
        if let Err(error) = runtime::spawn(async move {
            let result =
                tokio::task::spawn_blocking(move || installer::inspect(&installation, &path)).await;
            if let Err(error) = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_content_busy(false);
                match result {
                    Ok(Ok((inspection, pending))) => {
                        let preview =
                            serde_json::to_string(&inspection).unwrap_or_else(|_| "{}".to_owned());
                        backend.as_mut().rust_mut().get_mut().pending_archive = Some(pending);
                        backend.as_mut().set_content_preview_json(text(&preview));
                        backend
                            .as_mut()
                            .set_status(QString::from("Archive is ready to install"));
                    }
                    Ok(Err(error)) => {
                        set_operation_error(backend.as_mut(), "Could not inspect archive", error)
                    }
                    Err(error) => backend.as_mut().set_error_message(QString::from(format!(
                        "Archive worker failed: {error}"
                    ))),
                }
            }) {
                log::error!("could not return archive inspection to Qt: {error}");
            }
        }) {
            self.as_mut().set_content_busy(false);
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn install_inspected_archive(mut self: Pin<&mut Self>) {
        if *self.content_busy() || *self.scanning() {
            return;
        }
        let Some(installation) = self.rust().installation.clone() else {
            return;
        };
        let pending = self.as_mut().rust_mut().get_mut().pending_archive.take();
        let Some(pending) = pending else {
            self.as_mut()
                .set_error_message(QString::from("Choose and inspect an archive first"));
            return;
        };
        self.as_mut().set_content_busy(true);
        self.as_mut().set_error_message(QString::default());
        self.as_mut()
            .set_status(QString::from("Installing content with rollback backup..."));
        let qt_thread = self.qt_thread();
        if let Err(error) = runtime::spawn(async move {
            let worker_installation = installation.clone();
            let result = tokio::task::spawn_blocking(move || {
                installer::install(&worker_installation, &pending)
            })
            .await;
            if let Err(error) = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_content_busy(false);
                match result {
                    Ok(Ok(record)) => {
                        backend
                            .as_mut()
                            .set_content_preview_json(QString::from("{}"));
                        backend.as_mut().set_content_history_json(text(
                            &installer::history_json(&installation)
                                .unwrap_or_else(|_| "[]".to_owned()),
                        ));
                        backend.as_mut().set_status(QString::from(format!(
                            "Installed {}",
                            record.archive_name
                        )));
                        backend.as_mut().discover();
                    }
                    Ok(Err(error)) => {
                        set_operation_error(backend.as_mut(), "Could not install archive", error)
                    }
                    Err(error) => backend.as_mut().set_error_message(QString::from(format!(
                        "Archive worker failed: {error}"
                    ))),
                }
            }) {
                log::error!("could not return content installation to Qt: {error}");
            }
        }) {
            self.as_mut().set_content_busy(false);
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn rollback_content_install(mut self: Pin<&mut Self>, id: &QString) {
        if *self.content_busy() || *self.scanning() {
            return;
        }
        let Some(installation) = self.rust().installation.clone() else {
            return;
        };
        let id = id.to_string();
        self.as_mut().set_content_busy(true);
        self.as_mut().set_error_message(QString::default());
        self.as_mut()
            .set_status(QString::from("Rolling back content installation..."));
        let qt_thread = self.qt_thread();
        if let Err(error) = runtime::spawn(async move {
            let worker_installation = installation.clone();
            let result =
                tokio::task::spawn_blocking(move || installer::rollback(&worker_installation, &id))
                    .await;
            if let Err(error) = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_content_busy(false);
                match result {
                    Ok(Ok(())) => {
                        backend.as_mut().set_content_history_json(text(
                            &installer::history_json(&installation)
                                .unwrap_or_else(|_| "[]".to_owned()),
                        ));
                        backend
                            .as_mut()
                            .set_status(QString::from("Content installation rolled back"));
                        backend.as_mut().discover();
                    }
                    Ok(Err(error)) => {
                        set_operation_error(backend.as_mut(), "Could not roll back content", error)
                    }
                    Err(error) => backend.as_mut().set_error_message(QString::from(format!(
                        "Archive worker failed: {error}"
                    ))),
                }
            }) {
                log::error!("could not return content rollback to Qt: {error}");
            }
        }) {
            self.as_mut().set_content_busy(false);
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn refresh_race_state(mut self: Pin<&mut Self>) {
        if self.rust().race_state_busy {
            return;
        }
        self.as_mut().rust_mut().get_mut().race_state_busy = true;
        let qt_thread = self.qt_thread();
        if let Err(error) = runtime::spawn(async move {
            let result = tokio::task::spawn_blocking(launcher::race_is_running).await;
            if let Err(error) = qt_thread.queue(move |mut backend| {
                backend.as_mut().rust_mut().get_mut().race_state_busy = false;
                match result {
                    Ok(Ok(running)) => backend.as_mut().set_race_running(running),
                    Ok(Err(error)) => {
                        log::warn!("could not inspect Assetto Corsa process state: {error:#}")
                    }
                    Err(error) => log::warn!("race state worker failed: {error}"),
                }
            }) {
                log::error!("could not return race process state to Qt: {error}");
            }
        }) {
            self.as_mut().rust_mut().get_mut().race_state_busy = false;
            log::warn!("could not start race state worker: {error:#}");
        }
    }

    pub fn stop_race(mut self: Pin<&mut Self>) {
        if *self.launching() {
            return;
        }
        let Some(installation) = self.rust().installation.clone() else {
            self.as_mut()
                .set_error_message(QString::from("No Assetto Corsa installation is selected"));
            return;
        };
        self.as_mut().set_launching(true);
        self.as_mut()
            .set_status(QString::from("Stopping Assetto Corsa..."));
        self.as_mut().set_error_message(QString::default());
        let qt_thread = self.qt_thread();
        if let Err(error) = runtime::spawn(async move {
            let locked = match tokio::task::spawn_blocking(move || {
                let session_lock = configuration::lock_session(&installation)?;
                Ok::<_, anyhow::Error>(session_lock)
            })
            .await
            {
                Ok(result) => result,
                Err(error) => Err(anyhow::anyhow!("stop worker failed: {error}")),
            };
            let result = match locked {
                Ok(session_lock) => {
                    let result = launcher::stop_race().await;
                    drop(session_lock);
                    result
                }
                Err(error) => Err(error),
            };
            if let Err(error) = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_launching(false);
                match result {
                    Ok(()) => {
                        backend.as_mut().set_race_running(false);
                        backend
                            .as_mut()
                            .set_status(QString::from("Assetto Corsa stopped"));
                    }
                    Err(error) => {
                        backend.as_mut().set_status(QString::from("Stop failed"));
                        backend
                            .as_mut()
                            .set_error_message(QString::from(format!("{error:#}")));
                    }
                }
            }) {
                log::error!("could not return stop result to Qt: {error}");
            }
        }) {
            self.as_mut().set_launching(false);
            self.as_mut().set_status(QString::from("Stop failed"));
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn launch_existing_session(mut self: Pin<&mut Self>) {
        if *self.launching() {
            return;
        }

        let Some(installation) = self.rust().installation.clone() else {
            self.as_mut()
                .set_error_message(QString::from("No Assetto Corsa installation is selected"));
            return;
        };

        self.as_mut().set_launching(true);
        self.as_mut().set_error_message(QString::default());
        self.as_mut()
            .set_status(QString::from("Starting the last configured session..."));

        let qt_thread = self.qt_thread();
        if let Err(error) = runtime::spawn(async move {
            let locked = match tokio::task::spawn_blocking(move || {
                let session_lock = configuration::lock_session(&installation)?;
                Ok::<_, anyhow::Error>((installation, session_lock))
            })
            .await
            {
                Ok(result) => result,
                Err(error) => Err(anyhow::anyhow!("session launch worker failed: {error}")),
            };
            let result = match locked {
                Ok((installation, session_lock)) => {
                    let result = launcher::launch_existing_session(&installation).await;
                    drop(session_lock);
                    result
                }
                Err(error) => Err(error),
            };
            if let Err(error) = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_launching(false);
                match result {
                    Ok(process_id) => {
                        backend.as_mut().set_race_running(true);
                        backend
                            .as_mut()
                            .set_status(QString::from("Assetto Corsa is running"));
                        log::info!("started Steam runtime launcher process {process_id}");
                    }
                    Err(error) => {
                        backend.as_mut().set_status(QString::from("Launch failed"));
                        backend
                            .as_mut()
                            .set_error_message(QString::from(format!("{error:#}")));
                        log::error!("could not launch Assetto Corsa: {error:#}");
                    }
                }
            }) {
                log::error!("could not return launch result to Qt: {error}");
            }
        }) {
            self.as_mut().set_launching(false);
            self.as_mut().set_status(QString::from("Launch failed"));
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn launch_configuration(
        mut self: Pin<&mut Self>,
        car_id: &QString,
        skin_id: &QString,
        track_key: &QString,
        conditions_json: &QString,
    ) {
        if *self.launching() {
            return;
        }
        let Some(installation) = self.rust().installation.clone() else {
            self.as_mut()
                .set_error_message(QString::from("No Assetto Corsa installation is selected"));
            return;
        };
        let car_id = car_id.to_string();
        let skin_id = skin_id.to_string();
        let track_key = track_key.to_string();
        let conditions_json = conditions_json.to_string();

        self.as_mut().set_launching(true);
        self.as_mut().set_error_message(QString::default());
        self.as_mut()
            .set_status(QString::from("Configuring and starting session..."));

        let qt_thread = self.qt_thread();
        if let Err(error) = runtime::spawn(async move {
            let locked = match tokio::task::spawn_blocking(move || {
                let session_lock = configuration::lock_session(&installation)?;
                configuration::apply_configuration_json_unlocked(
                    &installation,
                    &car_id,
                    &skin_id,
                    &track_key,
                    &conditions_json,
                )?;
                Ok::<_, anyhow::Error>((installation, session_lock))
            })
            .await
            {
                Ok(result) => result,
                Err(error) => Err(anyhow::anyhow!(
                    "session configuration worker failed: {error}"
                )),
            };
            let result = match locked {
                Ok((installation, session_lock)) => {
                    let result = launcher::launch_existing_session(&installation).await;
                    drop(session_lock);
                    result
                }
                Err(error) => Err(error),
            };
            if let Err(error) = qt_thread.queue(move |mut backend| {
                backend.as_mut().set_launching(false);
                match result {
                    Ok(process_id) => {
                        backend.as_mut().set_race_running(true);
                        backend
                            .as_mut()
                            .set_status(QString::from("Assetto Corsa is running"));
                        log::info!("started Steam runtime launcher process {process_id}");
                    }
                    Err(error) => {
                        backend.as_mut().set_status(QString::from("Launch failed"));
                        backend
                            .as_mut()
                            .set_error_message(QString::from(format!("{error:#}")));
                        log::error!("could not launch Assetto Corsa: {error:#}");
                    }
                }
            }) {
                log::error!("could not return launch result to Qt: {error}");
            }
        }) {
            self.as_mut().set_launching(false);
            self.as_mut().set_status(QString::from("Launch failed"));
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn toggle_favorite(mut self: Pin<&mut Self>, kind: &QString, id: &QString) {
        let kind = kind.to_string();
        let id = id.to_string();
        let result = {
            let rust = self.as_mut().rust_mut();
            let rust = rust.get_mut();
            rust.preferences
                .toggle_favorite(&kind, &id)
                .and_then(|favorite| rust.catalog.set_favorite(&kind, &id, favorite))
        };

        match result {
            Ok(()) => {
                let rust = self.rust();
                let cars_json = text(&rust.catalog.cars_json);
                let tracks_json = text(&rust.catalog.tracks_json);
                self.as_mut().set_cars_json(cars_json);
                self.as_mut().set_tracks_json(tracks_json);
            }
            Err(error) => {
                log::error!("could not update favorite: {error:#}");
                self.as_mut()
                    .set_error_message(QString::from(format!("Could not save favorite: {error}")));
            }
        }
    }

    pub fn toggle_dashboard_item(mut self: Pin<&mut Self>, kind: &QString, id: &QString) {
        let kind = kind.to_string();
        let id = id.to_string();
        let result = {
            let rust = self.as_mut().rust_mut();
            let rust = rust.get_mut();
            if rust.installation.is_none() {
                Err(anyhow::anyhow!("Assetto Corsa discovery is not complete"))
            } else if !rust.catalog.contains(&kind, &id) {
                Err(anyhow::anyhow!("catalog item not found: {id}"))
            } else {
                rust.preferences
                    .toggle_dashboard_item(&kind, &id)
                    .and_then(|selected| {
                        rust.catalog.set_dashboard(&kind, &id, selected)?;
                        Ok(selected)
                    })
            }
        };

        match result {
            Ok(selected) => {
                let (cars_json, tracks_json) = {
                    let rust = self.rust();
                    (
                        text(&rust.catalog.cars_json),
                        text(&rust.catalog.tracks_json),
                    )
                };
                self.as_mut().set_cars_json(cars_json);
                self.as_mut().set_tracks_json(tracks_json);
                self.as_mut().set_status(QString::from(if selected {
                    "Added to race dashboard"
                } else {
                    "Removed from race dashboard"
                }));
                self.as_mut().set_error_message(QString::default());
                crate::ipc::notify_dashboard_options_changed();
            }
            Err(error) => {
                self.as_mut()
                    .set_status(QString::from("Dashboard choices unchanged"));
                self.as_mut().set_error_message(QString::from(format!(
                    "Could not update dashboard choices: {error}"
                )));
            }
        }
    }

    pub fn save_preset(
        mut self: Pin<&mut Self>,
        name: &QString,
        car_id: &QString,
        skin_id: &QString,
        track_key: &QString,
        conditions_json: &QString,
    ) {
        let result = {
            let rust = self.as_mut().rust_mut();
            configuration::save_preset(
                &mut rust.get_mut().preferences,
                &name.to_string(),
                &car_id.to_string(),
                &skin_id.to_string(),
                &track_key.to_string(),
                &conditions_json.to_string(),
            )
        };
        match result {
            Ok(()) => {
                refresh_presets(self.as_mut());
                self.as_mut().set_status(QString::from("Preset saved"));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => set_operation_error(self.as_mut(), "Could not save preset", error),
        }
    }

    pub fn delete_preset(mut self: Pin<&mut Self>, id: &QString) {
        let result = {
            let rust = self.as_mut().rust_mut();
            configuration::delete_preset(&mut rust.get_mut().preferences, &id.to_string())
        };
        match result {
            Ok(()) => {
                refresh_presets(self.as_mut());
                self.as_mut().set_status(QString::from("Preset deleted"));
            }
            Err(error) => set_operation_error(self.as_mut(), "Could not delete preset", error),
        }
    }

    pub fn apply_preset(mut self: Pin<&mut Self>, id: &QString) {
        let result = {
            let rust = self.rust();
            let installation = rust
                .installation
                .as_ref()
                .context("Assetto Corsa installation is unavailable");
            installation.and_then(|installation| {
                let conditions =
                    configuration::apply_preset(installation, &rust.preferences, &id.to_string())?;
                let session = session::load(installation)?;
                Ok::<_, anyhow::Error>((conditions, session))
            })
        };
        match result {
            Ok((conditions, session)) => {
                apply_session(self.as_mut(), &session);
                refresh_setups(self.as_mut());
                if let Ok(json) = configuration::conditions_json(&conditions) {
                    self.as_mut().set_conditions_json(text(&json));
                }
                self.as_mut()
                    .set_status(QString::from("Preset applied to race.ini"));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => set_operation_error(self.as_mut(), "Could not apply preset", error),
        }
    }

    pub fn apply_configuration(
        mut self: Pin<&mut Self>,
        car_id: &QString,
        skin_id: &QString,
        track_key: &QString,
        conditions_json: &QString,
    ) {
        let result = {
            let rust = self.rust();
            rust.installation
                .as_ref()
                .context("Assetto Corsa installation is unavailable")
                .and_then(|installation| {
                    let conditions = configuration::apply_configuration_json(
                        installation,
                        &car_id.to_string(),
                        &skin_id.to_string(),
                        &track_key.to_string(),
                        &conditions_json.to_string(),
                    )?;
                    let session = session::load(installation)?;
                    Ok::<_, anyhow::Error>((conditions, session))
                })
        };
        match result {
            Ok((conditions, session)) => {
                apply_session(self.as_mut(), &session);
                refresh_setups(self.as_mut());
                if let Ok(json) = configuration::conditions_json(&conditions) {
                    self.as_mut().set_conditions_json(text(&json));
                }
                self.as_mut()
                    .set_status(QString::from("Race setup applied to race.ini"));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => set_operation_error(self.as_mut(), "Could not apply race setup", error),
        }
    }

    pub fn set_csp_option(
        mut self: Pin<&mut Self>,
        file: &QString,
        section: &QString,
        key: &QString,
        value: &QString,
    ) {
        let result = {
            let rust = self.rust();
            rust.installation
                .as_ref()
                .context("Assetto Corsa installation is unavailable")
                .and_then(|installation| {
                    csp::set_option(
                        installation,
                        &file.to_string(),
                        &section.to_string(),
                        &key.to_string(),
                        &value.to_string(),
                    )?;
                    csp::load_json(installation)
                })
        };
        match result {
            Ok(json) => {
                self.as_mut().set_csp_settings_json(text(&json));
                self.as_mut().set_status(QString::from("CSP setting saved"));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => set_operation_error(self.as_mut(), "Could not save CSP setting", error),
        }
    }

    pub fn set_ac_option(
        mut self: Pin<&mut Self>,
        file: &QString,
        section: &QString,
        key: &QString,
        value: &QString,
    ) {
        let result = {
            let rust = self.rust();
            rust.installation
                .as_ref()
                .context("Assetto Corsa installation is unavailable")
                .and_then(|installation| {
                    ac_settings::set_option(
                        installation,
                        &file.to_string(),
                        &section.to_string(),
                        &key.to_string(),
                        &value.to_string(),
                    )?;
                    ac_settings::load_json(installation)
                })
        };
        match result {
            Ok(json) => {
                self.as_mut().set_ac_settings_json(text(&json));
                self.as_mut()
                    .set_status(QString::from("Assetto Corsa setting saved"));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => {
                set_operation_error(self.as_mut(), "Could not save Assetto Corsa setting", error)
            }
        }
    }

    pub fn set_app_layout_option(
        mut self: Pin<&mut Self>,
        section: &QString,
        key: &QString,
        value: &QString,
    ) {
        let result = {
            let rust = self.rust();
            rust.installation
                .as_ref()
                .context("Assetto Corsa installation is unavailable")
                .and_then(|installation| {
                    app_layout::set_option(
                        installation,
                        &section.to_string(),
                        &key.to_string(),
                        &value.to_string(),
                    )?;
                    load_app_layout_payload(installation)
                })
        };
        match result {
            Ok((app_layout_json, ac_settings_json)) => {
                self.as_mut().set_app_layout_json(text(&app_layout_json));
                self.as_mut().set_ac_settings_json(text(&ac_settings_json));
                self.as_mut()
                    .set_status(QString::from("In-game app layout saved"));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => set_operation_error(self.as_mut(), "Could not save app layout", error),
        }
    }

    pub fn move_app_window(mut self: Pin<&mut Self>, section: &QString, x: i32, y: i32) {
        let result = {
            let rust = self.rust();
            rust.installation
                .as_ref()
                .context("Assetto Corsa installation is unavailable")
                .and_then(|installation| {
                    app_layout::move_window(installation, &section.to_string(), x, y)?;
                    load_app_layout_payload(installation)
                })
        };
        match result {
            Ok((app_layout_json, ac_settings_json)) => {
                self.as_mut().set_app_layout_json(text(&app_layout_json));
                self.as_mut().set_ac_settings_json(text(&ac_settings_json));
                self.as_mut()
                    .set_status(QString::from("In-game app position saved"));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => set_operation_error(self.as_mut(), "Could not move in-game app", error),
        }
    }

    pub fn set_control_option(
        mut self: Pin<&mut Self>,
        section: &QString,
        key: &QString,
        value: &QString,
    ) {
        let result = {
            let rust = self.rust();
            rust.installation
                .as_ref()
                .context("Assetto Corsa installation is unavailable")
                .and_then(|installation| {
                    controls::set_option(
                        installation,
                        &section.to_string(),
                        &key.to_string(),
                        &value.to_string(),
                    )?;
                    load_control_payload(installation)
                })
        };
        match result {
            Ok((controls_json, ac_settings_json)) => {
                self.as_mut().set_controls_json(text(&controls_json));
                self.as_mut().set_ac_settings_json(text(&ac_settings_json));
                self.as_mut()
                    .set_status(QString::from("Control setting saved"));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => {
                set_operation_error(self.as_mut(), "Could not save control setting", error)
            }
        }
    }

    pub fn capture_control(
        mut self: Pin<&mut Self>,
        section: &QString,
        key: &QString,
        label: &QString,
    ) {
        let Some(installation) = self.rust().installation.clone() else {
            self.as_mut()
                .set_error_message(QString::from("Assetto Corsa installation is unavailable"));
            return;
        };
        let section = section.to_string();
        let key = key.to_string();
        let label = label.to_string();
        let kind = if key == "AXLE" {
            controls::CaptureKind::Axis
        } else {
            controls::CaptureKind::Button
        };
        let generation = self.rust().control_capture_generation.clone();
        let token = generation.fetch_add(1, Ordering::Relaxed) + 1;
        self.as_mut().set_control_capture_active(true);
        self.as_mut().set_control_capture_target(text(&label));
        self.as_mut().set_error_message(QString::default());
        self.as_mut()
            .set_status(QString::from(format!("Listening for {label} input...")));

        let qt_thread = self.qt_thread();
        let worker_generation = generation.clone();
        if let Err(error) = runtime::spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                let captured =
                    controls::capture(&installation, kind, worker_generation.clone(), token)?;
                let Some(captured) = captured else {
                    return Ok::<_, anyhow::Error>(None);
                };
                if worker_generation.load(Ordering::Relaxed) != token {
                    return Ok(None);
                }
                controls::assign(&installation, &section, &key, captured)?;
                let payload = load_control_payload(&installation)?;
                Ok(Some((captured, payload)))
            })
            .await;
            let result = match result {
                Ok(result) => result,
                Err(error) => Err(anyhow::anyhow!("controls capture worker failed: {error}")),
            };

            if let Err(error) = qt_thread.queue(move |mut backend| {
                if backend
                    .rust()
                    .control_capture_generation
                    .load(Ordering::Relaxed)
                    != token
                {
                    return;
                }
                backend.as_mut().set_control_capture_active(false);
                backend
                    .as_mut()
                    .set_control_capture_target(QString::default());
                match result {
                    Ok(Some((captured, (controls_json, ac_settings_json)))) => {
                        backend.as_mut().set_controls_json(text(&controls_json));
                        backend
                            .as_mut()
                            .set_ac_settings_json(text(&ac_settings_json));
                        backend.as_mut().set_status(QString::from(format!(
                            "Assigned {label} to controller {}, input {}",
                            captured.controller + 1,
                            captured.input + 1
                        )));
                        backend.as_mut().set_error_message(QString::default());
                    }
                    Ok(None) => {
                        backend
                            .as_mut()
                            .set_status(QString::from("Control assignment timed out"));
                    }
                    Err(error) => {
                        set_operation_error(
                            backend.as_mut(),
                            "Could not assign control input",
                            error,
                        );
                    }
                }
            }) {
                log::error!("could not return controls capture result to Qt: {error}");
            }
        }) {
            self.as_mut().set_control_capture_active(false);
            self.as_mut().set_control_capture_target(QString::default());
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn cancel_control_capture(mut self: Pin<&mut Self>) {
        self.rust()
            .control_capture_generation
            .fetch_add(1, Ordering::Relaxed);
        self.as_mut().set_control_capture_active(false);
        self.as_mut().set_control_capture_target(QString::default());
        self.as_mut()
            .set_status(QString::from("Control assignment cancelled"));
    }

    pub fn update_control_monitoring(mut self: Pin<&mut Self>, enabled: bool) {
        if *self.control_monitoring() == enabled {
            return;
        }
        let generation = self.rust().control_monitor_generation.clone();
        let token = generation.fetch_add(1, Ordering::Relaxed) + 1;
        self.as_mut().set_control_monitoring(enabled);
        if !enabled {
            self.as_mut()
                .set_control_input_json(QString::from("{\"devices\":[]}"));
            return;
        }

        let Some(installation) = self.rust().installation.clone() else {
            self.as_mut().set_control_monitoring(false);
            return;
        };
        let (updates, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let qt_thread = self.qt_thread();
        let worker_generation = generation.clone();
        if let Err(error) = runtime::spawn(async move {
            let worker = tokio::task::spawn_blocking(move || {
                controls::monitor(&installation, worker_generation, token, updates)
            });
            while let Some(json) = receiver.recv().await {
                if generation.load(Ordering::Relaxed) != token {
                    break;
                }
                if let Err(error) = qt_thread.queue(move |mut backend| {
                    if backend
                        .rust()
                        .control_monitor_generation
                        .load(Ordering::Relaxed)
                        == token
                    {
                        backend.as_mut().set_control_input_json(text(&json));
                    }
                }) {
                    log::error!("could not publish live controller input to Qt: {error}");
                    generation.fetch_add(1, Ordering::Relaxed);
                    break;
                }
            }
            let result = match worker.await {
                Ok(result) => result,
                Err(error) => Err(anyhow::anyhow!("controls monitor worker failed: {error}")),
            };
            if generation.load(Ordering::Relaxed) == token {
                let _ = qt_thread.queue(move |mut backend| {
                    backend.as_mut().set_control_monitoring(false);
                    if let Err(error) = result {
                        log::warn!("live controller preview stopped: {error:#}");
                        backend.as_mut().set_error_message(QString::from(format!(
                            "Live controller preview is unavailable: {error}"
                        )));
                    }
                });
            }
        }) {
            self.as_mut().set_control_monitoring(false);
            self.as_mut()
                .set_error_message(QString::from(error.to_string()));
        }
    }

    pub fn clear_control_binding(mut self: Pin<&mut Self>, section: &QString, key: &QString) {
        let result = {
            let rust = self.rust();
            rust.installation
                .as_ref()
                .context("Assetto Corsa installation is unavailable")
                .and_then(|installation| {
                    controls::clear_binding(installation, &section.to_string(), &key.to_string())?;
                    load_control_payload(installation)
                })
        };
        match result {
            Ok((controls_json, ac_settings_json)) => {
                self.as_mut().set_controls_json(text(&controls_json));
                self.as_mut().set_ac_settings_json(text(&ac_settings_json));
                self.as_mut()
                    .set_status(QString::from("Control binding cleared"));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => {
                set_operation_error(self.as_mut(), "Could not clear control binding", error)
            }
        }
    }

    pub fn set_control_axis_inverted(mut self: Pin<&mut Self>, section: &QString, inverted: bool) {
        let result = {
            let rust = self.rust();
            rust.installation
                .as_ref()
                .context("Assetto Corsa installation is unavailable")
                .and_then(|installation| {
                    controls::set_axis_inverted(installation, &section.to_string(), inverted)?;
                    load_control_payload(installation)
                })
        };
        match result {
            Ok((controls_json, ac_settings_json)) => {
                self.as_mut().set_controls_json(text(&controls_json));
                self.as_mut().set_ac_settings_json(text(&ac_settings_json));
                self.as_mut()
                    .set_status(QString::from("Axis direction saved"));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => {
                set_operation_error(self.as_mut(), "Could not invert control axis", error)
            }
        }
    }

    pub fn save_control_preset(mut self: Pin<&mut Self>, name: &QString) {
        let result = {
            let rust = self.rust();
            rust.installation
                .as_ref()
                .context("Assetto Corsa installation is unavailable")
                .and_then(|installation| {
                    controls::save_preset(installation, &name.to_string())?;
                    controls::load_json(installation)
                })
        };
        match result {
            Ok(json) => {
                self.as_mut().set_controls_json(text(&json));
                self.as_mut()
                    .set_status(QString::from("Controls preset saved"));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => {
                set_operation_error(self.as_mut(), "Could not save controls preset", error)
            }
        }
    }

    pub fn load_control_preset(mut self: Pin<&mut Self>, source: &QString, name: &QString) {
        let result = {
            let rust = self.rust();
            rust.installation
                .as_ref()
                .context("Assetto Corsa installation is unavailable")
                .and_then(|installation| {
                    controls::load_preset(installation, &source.to_string(), &name.to_string())?;
                    load_control_payload(installation)
                })
        };
        match result {
            Ok((controls_json, ac_settings_json)) => {
                self.as_mut().set_controls_json(text(&controls_json));
                self.as_mut().set_ac_settings_json(text(&ac_settings_json));
                self.as_mut()
                    .set_status(QString::from("Controls preset loaded"));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => {
                set_operation_error(self.as_mut(), "Could not load controls preset", error)
            }
        }
    }

    pub fn delete_control_preset(mut self: Pin<&mut Self>, name: &QString) {
        let result = {
            let rust = self.rust();
            rust.installation
                .as_ref()
                .context("Assetto Corsa installation is unavailable")
                .and_then(|installation| {
                    controls::delete_preset(installation, &name.to_string())?;
                    controls::load_json(installation)
                })
        };
        match result {
            Ok(json) => {
                self.as_mut().set_controls_json(text(&json));
                self.as_mut()
                    .set_status(QString::from("Controls preset deleted"));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => {
                set_operation_error(self.as_mut(), "Could not delete controls preset", error)
            }
        }
    }

    pub fn update_tyre_blankets(mut self: Pin<&mut Self>, enabled: bool) {
        let result = {
            let rust = self.rust();
            rust.installation
                .as_ref()
                .context("Assetto Corsa installation is unavailable")
                .and_then(|installation| {
                    ac_settings::set_option(
                        installation,
                        "assists.ini",
                        "ASSISTS",
                        "TYRE_BLANKETS",
                        if enabled { "1" } else { "0" },
                    )?;
                    ac_settings::load_json(installation)
                })
        };
        match result {
            Ok(json) => {
                self.as_mut().set_tyre_blankets(enabled);
                self.as_mut().set_ac_settings_json(text(&json));
                self.as_mut().set_status(QString::from(if enabled {
                    "Tyre warmers enabled"
                } else {
                    "Tyre warmers disabled"
                }));
                self.as_mut().set_error_message(QString::default());
            }
            Err(error) => set_operation_error(self.as_mut(), "Could not save tyre warmers", error),
        }
    }
}

fn load_control_payload(
    installation: &discovery::Installation,
) -> anyhow::Result<(String, String)> {
    Ok((
        controls::load_json(installation)?,
        ac_settings::load_json(installation)?,
    ))
}

fn load_app_layout_payload(
    installation: &discovery::Installation,
) -> anyhow::Result<(String, String)> {
    Ok((
        app_layout::load_json(installation)?,
        ac_settings::load_json(installation)?,
    ))
}

fn path_string(path: &std::path::Path) -> QString {
    QString::from(path.to_string_lossy().as_ref())
}

fn apply_session(mut backend: Pin<&mut ffi::Backend>, session: &session::SessionInfo) {
    backend.as_mut().set_session_loaded(true);
    backend.as_mut().set_car_name(text(&session.car_name));
    backend.as_mut().set_car_brand(text(&session.car_brand));
    backend.as_mut().set_car_id(text(&session.car_id));
    backend.as_mut().set_skin_id(text(&session.skin_id));
    backend.as_mut().set_skin_name(text(&session.skin_name));
    backend.as_mut().set_track_name(text(&session.track_name));
    backend
        .as_mut()
        .set_track_location(text(&session.track_location));
    backend.as_mut().set_track_id(text(&session.track_id));
    backend
        .as_mut()
        .set_track_layout_id(text(&session.track_layout_id));
    backend
        .as_mut()
        .set_track_layout(text(&session.track_layout));
    backend
        .as_mut()
        .set_weather_name(text(&session.weather_name));
    backend
        .as_mut()
        .set_session_type(text(&session.session_type));
    backend.as_mut().set_car_preview(text(&session.car_preview));
    backend
        .as_mut()
        .set_track_preview(text(&session.track_preview));
    backend
        .as_mut()
        .set_track_outline(text(&session.track_outline));
    backend.as_mut().set_csp_status(text(&session.csp_status));
}

fn text(value: &str) -> QString {
    QString::from(value)
}

fn apply_catalog(mut backend: Pin<&mut ffi::Backend>, catalog: &content::Catalog) {
    backend.as_mut().set_cars_json(text(&catalog.cars_json));
    backend.as_mut().set_tracks_json(text(&catalog.tracks_json));
    backend
        .as_mut()
        .set_replays_json(text(&catalog.replays_json));
    backend.as_mut().set_car_count(catalog.car_count);
    backend.as_mut().set_track_count(catalog.track_count);
    backend.as_mut().set_replay_count(catalog.replay_count);
}

fn refresh_presets(mut backend: Pin<&mut ffi::Backend>) {
    let (json, count) = {
        let preferences = &backend.rust().preferences;
        (
            configuration::presets_json(preferences).unwrap_or_else(|_| "[]".to_owned()),
            i32::try_from(preferences.presets.len()).unwrap_or(i32::MAX),
        )
    };
    backend.as_mut().set_presets_json(text(&json));
    backend.as_mut().set_preset_count(count);
}

fn refresh_setups(mut backend: Pin<&mut ffi::Backend>) {
    let result = backend.rust().installation.as_ref().map(|installation| {
        setups::load_json(installation, &backend.car_id().to_string())
            .unwrap_or_else(|_| "[]".to_owned())
    });
    if let Some(json) = result {
        let count = serde_json::from_str::<Vec<serde_json::Value>>(&json)
            .ok()
            .and_then(|items| i32::try_from(items.len()).ok())
            .unwrap_or(0);
        backend.as_mut().set_setups_json(text(&json));
        backend.as_mut().set_setup_count(count);
    }
}

fn set_operation_error(mut backend: Pin<&mut ffi::Backend>, context: &str, error: anyhow::Error) {
    log::error!("{context}: {error:#}");
    backend.as_mut().set_status(QString::from(context));
    backend
        .as_mut()
        .set_error_message(QString::from(format!("{context}: {error:#}")));
}

pub fn run_widgets_application() -> i32 {
    ffi::run_aclm_widgets()
}
