use tracing_subscriber::{EnvFilter, prelude::*};
pub fn initialize() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer().json())
        .try_init()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}
