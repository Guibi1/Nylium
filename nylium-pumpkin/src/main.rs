use std::path::PathBuf;

use nylium_adapter::console;
use nylium_pumpkin::PumpkinAdapter;
use tokio::runtime::Builder;
use tracing_subscriber::{EnvFilter, prelude::*};

fn main() {
    if let Err(error) = run() {
        eprintln!("PumpkinMC: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| ".".into());
    let adapter = PumpkinAdapter::new(config)?;
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .with(adapter.logging_layer())
        .try_init()
        .map_err(|error| error.to_string())?;
    adapter.install_panic_handler();

    let adapter = std::sync::Arc::new(adapter);
    Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("Cannot create Tokio runtime: {error}"))?
        .block_on(console::run(adapter, async {
            tokio::signal::ctrl_c()
                .await
                .map_err(|error| format!("Cannot listen for Ctrl-C: {error}"))
        }))
}
