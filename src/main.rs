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

    if std::env::args().any(|argument| argument == "--launch-last") {
        let installation = discovery::discover()?;
        let process_id =
            tokio_runtime.block_on(launcher::launch_existing_session(&installation))?;
        log::info!("started the last configured session as process {process_id}");
        return Ok(());
    }

    log::info!("starting AC Linux Manager");
    let exit_code = backend::run_widgets_application();
    anyhow::ensure!(
        exit_code == 0,
        "Qt Widgets application exited with {exit_code}"
    );

    Ok(())
}
