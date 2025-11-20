use std::net::SocketAddr;
use std::str::FromStr;

use async_trait::async_trait;
use gpui::Global;
use nylium::{Nylium, NyliumLogger};
use nylium_adapter::{NyliumConfig, NyliumServer, Player};
use uuid::Uuid;

fn main() {
    let logger = NyliumLogger::init();
    Nylium::new(DummyServer, logger).run();
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

    async fn send_command(&self, command: &str) {
        println!("Command received: {}", command);
    }

    async fn get_players(&self) -> Vec<Player> {
        vec![Player {
            id: Uuid::from_str("0939003b-c550-4914-a461-09b5bb0c80ea").unwrap(),
            name: "Guibi1".to_string(),
        }]
    }
}

#[derive(NyliumConfig)]
pub struct DummyConfig {
    pub server_address: SocketAddr,
    pub seed: String,
    pub max_players: u32,
    pub online_mode: bool,
}
