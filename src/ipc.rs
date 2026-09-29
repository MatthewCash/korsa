use crate::{configuration, content, discovery, launcher, preferences, runtime};
use anyhow::{Context, Result};
use std::{sync::OnceLock, time::Duration};
use zbus::{
    Connection,
    connection::Builder,
    fdo, interface,
    object_server::{InterfaceRef, SignalEmitter},
};

pub const SERVICE_NAME: &str = "com.matthewcash.ACLinuxManager";
pub const OBJECT_PATH: &str = "/com/matthewcash/ACLinuxManager";
const PROTOCOL_VERSION: u32 = 1;
const RACE_STATUS_INTERVAL: Duration = Duration::from_millis(500);

static CONNECTION: OnceLock<Connection> = OnceLock::new();

pub struct AcLinuxManager;

#[interface(name = "com.matthewcash.ACLinuxManager")]
impl AcLinuxManager {
    fn get_protocol_version(&self) -> u32 {
        PROTOCOL_VERSION
    }

    async fn get_dashboard_options(&self) -> fdo::Result<String> {
        blocking(|| {
            let installation = discovery::discover()?;
            let preferences = preferences::Preferences::load()?;
            content::dashboard_json(&installation, &preferences)
        })
        .await
    }

    fn get_race_running(&self) -> fdo::Result<bool> {
        launcher::race_is_running().map_err(method_error)
    }

    async fn launch_dashboard_session(
        &self,
        car_id: String,
        track_key: String,
        start_minutes: i32,
    ) -> fdo::Result<()> {
        let (installation, session_lock) = blocking(move || {
            let installation = discovery::discover()?;
            let session_lock = configuration::lock_session(&installation)?;
            configuration::apply_dashboard_configuration(
                &installation,
                &car_id,
                &track_key,
                start_minutes,
            )?;
            Ok((installation, session_lock))
        })
        .await?;

        let process_id = launcher::launch_existing_session(&installation)
            .await
            .map_err(method_error)?;
        drop(session_lock);
        log::info!("started dashboard session as process {process_id}");
        Ok(())
    }

    async fn stop_race(&self) -> fdo::Result<()> {
        let session_lock = blocking(|| {
            let installation = discovery::discover()?;
            configuration::lock_session(&installation)
        })
        .await?;
        let result = launcher::stop_race().await.map_err(method_error);
        drop(session_lock);
        result
    }

    #[zbus(signal)]
    async fn dashboard_options_changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn race_running_changed(emitter: &SignalEmitter<'_>, running: bool) -> zbus::Result<()>;
}

pub async fn start() -> Result<Connection> {
    let connection = Builder::session()?
        .name(SERVICE_NAME)?
        .serve_at(OBJECT_PATH, AcLinuxManager)?
        .build()
        .await
        .context("could not start AC Linux Manager D-Bus service")?;
    CONNECTION
        .set(connection.clone())
        .map_err(|_| anyhow::anyhow!("AC Linux Manager D-Bus service was already initialized"))?;

    let monitor_connection = connection.clone();
    runtime::spawn(async move { monitor_race_state(monitor_connection).await })?;
    Ok(connection)
}

pub fn notify_dashboard_options_changed() {
    let Some(connection) = CONNECTION.get().cloned() else {
        return;
    };
    if let Err(error) = runtime::spawn(async move {
        match interface(&connection).await {
            Ok(interface) => {
                if let Err(error) = interface.dashboard_options_changed().await {
                    log::warn!("could not notify race dashboard: {error}");
                }
            }
            Err(error) => log::warn!("could not access D-Bus interface: {error}"),
        }
    }) {
        log::warn!("could not queue race dashboard notification: {error}");
    }
}

async fn monitor_race_state(connection: Connection) {
    let mut previous = None;
    loop {
        match launcher::race_is_running() {
            Ok(running) if previous != Some(running) => {
                previous = Some(running);
                match interface(&connection).await {
                    Ok(interface) => {
                        if let Err(error) = interface.race_running_changed(running).await {
                            log::warn!("could not publish race state: {error}");
                        }
                    }
                    Err(error) => log::warn!("could not access D-Bus interface: {error}"),
                }
            }
            Ok(_) => {}
            Err(error) => log::warn!("could not inspect race state: {error}"),
        }
        tokio::time::sleep(RACE_STATUS_INTERVAL).await;
    }
}

async fn interface(connection: &Connection) -> zbus::Result<InterfaceRef<AcLinuxManager>> {
    connection.object_server().interface(OBJECT_PATH).await
}

async fn blocking<T, F>(operation: F) -> fdo::Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| fdo::Error::Failed(format!("worker task failed: {error}")))?
        .map_err(method_error)
}

fn method_error(error: anyhow::Error) -> fdo::Error {
    fdo::Error::Failed(format!("{error:#}"))
}
