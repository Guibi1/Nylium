use nylium::Nylium;
use nylium_pumpkin::PumpkinAdapter;
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, prelude::*};

fn main() {
    let config = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| ".".into());
    let result = PumpkinAdapter::new(config).and_then(|adapter| {
        tracing_subscriber::registry()
            .with(adapter.logging_layer())
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
            .try_init()
            .map_err(|error| error.to_string())?;
        adapter.install_panic_handler();
        Nylium::run(adapter);
        Ok(())
    });
    if let Err(error) = result {
        eprintln!("Cannot start Nylium for PumpkinMC: {error}");
        std::process::exit(1);
    }
}
