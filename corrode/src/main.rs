use std::net::SocketAddr;

use corrode::Corrode;
use corrode_adapter::{CorrodeConfig, CorrodeServer};

fn main() {
    Corrode::new(DummyServer).run();
}

struct DummyServer;

impl CorrodeServer<DummyConfig> for DummyServer {
    fn start(&self) {
        println!("Server started");
    }

    fn stop(&self) {
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

#[derive(CorrodeConfig)]
pub struct DummyConfig {
    pub server_address: SocketAddr,
    pub seed: String,
    pub max_players: u32,
    pub online_mode: bool,
}
