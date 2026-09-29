mod ac_settings;
mod app_layout;
mod backend;
mod configuration;
mod content;
mod controls;
mod csp;
mod csp_versions;
mod discovery;
mod installer;
mod ipc;
mod launcher;
mod online;
mod preferences;
mod runtime;
mod session;
mod setups;

use anyhow::{Context, Result};
fn main() -> Result<()> {
    cxx_qt::init_crate!(ac_linux_manager);
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name("aclm-worker")
        .enable_all()
        .build()
        .context("failed to create Tokio runtime")?;
    runtime::initialize(tokio_runtime.handle().clone())?;

    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "--launch-last" => {
            let installation = discovery::discover()?;
            let _lock = configuration::lock_session(&installation)?;
            let process_id =
                tokio_runtime.block_on(launcher::launch_existing_session(&installation))?;
            log::info!("started the last configured session as process {process_id}");
            return Ok(());
        }
        [command] if command == "--dashboard-json" => {
            let installation = discovery::discover()?;
            let preferences = preferences::Preferences::load()?;
            println!("{}", content::dashboard_json(&installation, &preferences)?);
            return Ok(());
        }
        [command] if command == "--race-running" => {
            println!("{}", launcher::race_is_running()?);
            return Ok(());
        }
        [command] if command == "--stop-race" => {
            let installation = discovery::discover()?;
            let _lock = configuration::lock_session(&installation)?;
            tokio_runtime.block_on(launcher::stop_race())?;
            return Ok(());
        }
        [command, car_id, track_key, start_minutes] if command == "--dashboard-launch" => {
            let installation = discovery::discover()?;
            let _lock = configuration::lock_session(&installation)?;
            configuration::apply_dashboard_configuration(
                &installation,
                car_id,
                track_key,
                start_minutes
                    .parse()
                    .context("invalid dashboard start time")?,
            )?;
            let process_id =
                tokio_runtime.block_on(launcher::launch_existing_session(&installation))?;
            log::info!("started dashboard session as process {process_id}");
            return Ok(());
        }
        [] => {}
        _ => anyhow::bail!(
            "usage: ac-linux-manager [--launch-last | --dashboard-json | --race-running | --stop-race | --dashboard-launch CAR TRACK START_MINUTES]"
        ),
    }

    let _dbus_connection = tokio_runtime.block_on(ipc::start())?;
    log::info!("starting AC Linux Manager");
    let exit_code = backend::run_widgets_application();
    anyhow::ensure!(
        exit_code == 0,
        "Qt Widgets application exited with {exit_code}"
    );

    Ok(())
}
