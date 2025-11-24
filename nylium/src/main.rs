use std::str::FromStr;

use async_trait::async_trait;
use gpui::Global;
use nylium::{Nylium, NyliumLogger};
use nylium_adapter::config::{ConfigOptions, ConfigValue};
use nylium_adapter::{NyliumServer, Player};
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

    fn get_config(&self) -> Box<[ConfigOptions<DummyConfig>]> {
        Box::new([
            ConfigOptions::new_number(DummyConfig::Port, "Port", Some(1024), Some(65535)),
            ConfigOptions::new_string(DummyConfig::Seed, "Seed"),
            ConfigOptions::new_number(DummyConfig::MaxPlayers, "Max Players", Some(1), Some(100)),
            ConfigOptions::new_bool(DummyConfig::OnlineMode, "Online Mode", "online-mode"),
        ])
    }

    fn get_config_value(&self, key: DummyConfig) -> ConfigValue {
        match key {
            DummyConfig::Port => ConfigValue::Number(25565),
            DummyConfig::Seed => ConfigValue::String("".to_string()),
            DummyConfig::MaxPlayers => ConfigValue::Number(20),
            DummyConfig::OnlineMode => ConfigValue::Boolean(true),
        }
    }

    fn set_config_value(&self, key: DummyConfig, value: ConfigValue) {
        println!("Config updated {:?} {:?}", key, value);
    }

    async fn run_command(&self, command: &str) {
        println!("Command received: {}", command);
    }

    async fn get_players(&self) -> Vec<Player> {
        vec![Player::new(
            Uuid::from_str("0939003b-c550-4914-a461-09b5bb0c80ea").unwrap(),
            "Guibi1",
            true,
        )]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DummyConfig {
    Port,
    Seed,
    MaxPlayers,
    OnlineMode,
}
