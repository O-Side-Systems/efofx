//! Binary entry point. Everything meaningful lives in the library crate
//! (`efofx_core`) so integration tests can build an in-process router
//! without going through a real network listener.

use efofx_config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let cfg = AppConfig::load()?;
    efofx_core::run(cfg).await
}

fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,efofx_core=debug"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json().with_target(true))
        .init();
}
