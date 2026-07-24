use anyhow::{Context, Result};
use std::{future::Future, sync::OnceLock};
use tokio::runtime::Handle;

static HANDLE: OnceLock<Handle> = OnceLock::new();

pub fn initialize(handle: Handle) -> Result<()> {
    HANDLE
        .set(handle)
        .map_err(|_| anyhow::anyhow!("Tokio runtime was already initialized"))
}

pub fn spawn<F>(future: F) -> Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    HANDLE
        .get()
        .context("Tokio runtime is not initialized")?
        .spawn(future);
    Ok(())
}
