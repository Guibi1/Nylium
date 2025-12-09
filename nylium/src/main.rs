use std::str::FromStr;

use async_trait::async_trait;
use gpui::Global;
use nylium::{Nylium, NyliumLogger};
use nylium_adapter::fields::{FieldOptions, FieldValue};
use nylium_adapter::{GameRuleKeys, NyliumServer, Player, PlayerMap};
use uuid::Uuid;

fn main() {
    let logger = NyliumLogger::init();
    Nylium::new(DummyServer, logger).run();
}

#[derive(Clone)]
struct DummyServer;

impl Global for DummyServer {}

#[async_trait]
impl NyliumServer<ConfigKeys, GameRuleKeys> for DummyServer {
    async fn start(&self) {
        println!("Server started");
    }

    async fn stop(&self) {
        println!("Server stopped");
    }

    fn get_config(&self) -> Box<[FieldOptions<ConfigKeys>]> {
        Box::new([
            FieldOptions::new_number(ConfigKeys::Port, "Port", Some(1024), Some(65535)),
            FieldOptions::new_string(ConfigKeys::Seed, "Seed"),
            FieldOptions::new_number(ConfigKeys::MaxPlayers, "Max Players", Some(1), Some(100)),
            FieldOptions::new_bool(ConfigKeys::OnlineMode, "Online Mode", "online-mode"),
        ])
    }

    fn get_config_value(&self, key: ConfigKeys) -> FieldValue {
        match key {
            ConfigKeys::Port => FieldValue::Number(25565),
            ConfigKeys::Seed => FieldValue::String("".to_string()),
            ConfigKeys::MaxPlayers => FieldValue::Number(20),
            ConfigKeys::OnlineMode => FieldValue::Boolean(true),
        }
    }

    fn set_config_value(&self, _key: ConfigKeys, _value: FieldValue) {}

    fn get_gamerules(&self) -> Box<[FieldOptions<GameRuleKeys>]> {
        GameRuleKeys::get_gamerules()
    }

    fn get_gamerule_value(&self, key: GameRuleKeys) -> FieldValue {
        match key {
            GameRuleKeys::AnnounceAdvancements => FieldValue::Boolean(true),
            GameRuleKeys::BlockExplosionDropDecay => FieldValue::Boolean(true),
            GameRuleKeys::CommandBlockOutput => FieldValue::Boolean(true),
            GameRuleKeys::CommandModificationBlockLimit => FieldValue::Number(32768),
            GameRuleKeys::DisableElytraMovementCheck => FieldValue::Boolean(false),
            GameRuleKeys::DisableRaids => FieldValue::Boolean(false),
            GameRuleKeys::DoDaylightCycle => FieldValue::Boolean(true),
            GameRuleKeys::DoEntityDrops => FieldValue::Boolean(true),
            GameRuleKeys::DoFireTick => FieldValue::Boolean(true),
            GameRuleKeys::DoImmediateRespawn => FieldValue::Boolean(false),
            GameRuleKeys::DoInsomnia => FieldValue::Boolean(true),
            GameRuleKeys::DoLimitedCrafting => FieldValue::Boolean(false),
            GameRuleKeys::DoMobLoot => FieldValue::Boolean(true),
            GameRuleKeys::DoMobSpawning => FieldValue::Boolean(true),
            GameRuleKeys::DoPatrolSpawning => FieldValue::Boolean(true),
            GameRuleKeys::DoTileDrops => FieldValue::Boolean(true),
            GameRuleKeys::DoTraderSpawning => FieldValue::Boolean(true),
            GameRuleKeys::DoVinesSpread => FieldValue::Boolean(true),
            GameRuleKeys::DoWardenSpawning => FieldValue::Boolean(true),
            GameRuleKeys::DoWeatherCycle => FieldValue::Boolean(true),
            GameRuleKeys::DrowningDamage => FieldValue::Boolean(true),
            GameRuleKeys::EnderPearlsVanishOnDeath => FieldValue::Boolean(true),
            GameRuleKeys::FallDamage => FieldValue::Boolean(true),
            GameRuleKeys::FireDamage => FieldValue::Boolean(true),
            GameRuleKeys::ForgiveDeadPlayers => FieldValue::Boolean(true),
            GameRuleKeys::FreezeDamage => FieldValue::Boolean(true),
            GameRuleKeys::GlobalSoundEvents => FieldValue::Boolean(true),
            GameRuleKeys::KeepInventory => FieldValue::Boolean(false),
            GameRuleKeys::LavaSourceConversion => FieldValue::Boolean(false),
            GameRuleKeys::LogAdminCommands => FieldValue::Boolean(true),
            GameRuleKeys::MaxCommandChainLength => FieldValue::Number(65536),
            GameRuleKeys::MaxCommandForkCount => FieldValue::Number(65536),
            GameRuleKeys::MaxEntityCramming => FieldValue::Number(24),
            GameRuleKeys::MobExplosionDropDecay => FieldValue::Boolean(true),
            GameRuleKeys::MobGriefing => FieldValue::Boolean(true),
            GameRuleKeys::NaturalRegeneration => FieldValue::Boolean(true),
            GameRuleKeys::PlayersNetherPortalCreativeDelay => FieldValue::Number(1),
            GameRuleKeys::PlayersNetherPortalDefaultDelay => FieldValue::Number(80),
            GameRuleKeys::PlayersSleepingPercentage => FieldValue::Number(100),
            GameRuleKeys::ProjectilesCanBreakBlocks => FieldValue::Boolean(true),
            GameRuleKeys::RandomTickSpeed => FieldValue::Number(3),
            GameRuleKeys::ReducedDebugInfo => FieldValue::Boolean(false),
            GameRuleKeys::SendCommandFeedback => FieldValue::Boolean(true),
            GameRuleKeys::ShowDeathMessages => FieldValue::Boolean(true),
            GameRuleKeys::SnowAccumulationHeight => FieldValue::Number(1),
            GameRuleKeys::SpawnChunkRadius => FieldValue::Number(2),
            GameRuleKeys::SpawnRadius => FieldValue::Number(10),
            GameRuleKeys::SpectatorsGenerateChunks => FieldValue::Boolean(true),
            GameRuleKeys::TntExplosionDropDecay => FieldValue::Boolean(false),
            GameRuleKeys::UniversalAnger => FieldValue::Boolean(false),
            GameRuleKeys::WaterSourceConversion => FieldValue::Boolean(true),
        }
    }

    fn set_gamerule_value(&self, _key: GameRuleKeys, _value: FieldValue) {}

    async fn run_command(&self, command: &str) {
        println!("Command received: {}", command);
    }

    async fn get_players(&self) -> Vec<Player> {
        vec![
            Player::new(
                Uuid::from_str("0939003b-c550-4914-a461-09b5bb0c80ea").unwrap(),
                "Guibi1",
                PlayerMap::Overworld,
                true,
            ),
            Player::new(
                Uuid::from_str("829fcbb6-8375-4f38-adcf-6959e1835743").unwrap(),
                "LMF0906",
                PlayerMap::Nether,
                false,
            ),
            Player::new(
                Uuid::from_str("b876ec32-e396-476b-a115-8438d83c67d4").unwrap(),
                "Technoblade",
                PlayerMap::Custom("Skyblock".into()),
                true,
            ),
        ]
    }
}

#[derive(Clone, Copy)]
pub enum ConfigKeys {
    Port,
    Seed,
    MaxPlayers,
    OnlineMode,
}
