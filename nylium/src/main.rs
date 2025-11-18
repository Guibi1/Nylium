use std::net::SocketAddr;

use async_trait::async_trait;
use gpui::Global;
use nylium::Nylium;
use nylium_adapter::{NyliumConfig, NyliumServer};

fn main() {
    Nylium::new(DummyServer).run();
}

#[derive(Clone)]
struct DummyServer;

impl Global for DummyServer {}

#[async_trait]
impl NyliumServer<DummyConfig> for DummyServer {
    async fn start(&self) {
        println!("Server started");
    }

    async fn stop(&self) {
        println!("Server stopped");
    }

    fn send_command(&self, command: &str) {
        println!("Command received: {}", command);
    }

    fn get_config(&self) -> DummyConfig {
        DummyConfig {
            server_address: "127.0.0.1:25565".parse().unwrap(),
            seed: "ExampleSeed".into(),
            max_players: 20,
            online_mode: false,
        }
    }

    fn update_config(&self, _config: &DummyConfig) -> bool {
        println!("Config updated");
        true
    }
}

#[derive(NyliumConfig)]
pub struct DummyConfig {
    pub server_address: SocketAddr,
    pub seed: String,
    pub max_players: u32,
    pub online_mode: bool,
}
