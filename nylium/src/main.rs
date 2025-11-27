use std::str::FromStr;

use async_trait::async_trait;
use gpui::Global;
use nylium::{Nylium, NyliumLogger};
use nylium_adapter::config::{FieldOptions, FieldValue};
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

    fn set_config_value(&self, key: ConfigKeys, value: FieldValue) {
        println!("Config updated {:?} {:?}", key, value);
    }

    fn get_gamerules(&self) -> Box<[FieldOptions<GameRuleKeys>]> {
        Box::new([
            FieldOptions::new_bool(
                GameRuleKeys::AnnounceAdvancements,
                "Announce Advancements",
                "announceAdvancements",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::BlockExplosionDropDecay,
                "Block Explosion Drop Decay",
                "blockExplosionDropDecay",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::CommandBlockOutput,
                "Command Block Output",
                "commandBlockOutput",
            ),
            FieldOptions::new_number(
                GameRuleKeys::CommandModificationBlockLimit,
                "Command Modification Block Limit",
                Some(0),
                Some(32768),
            ),
            FieldOptions::new_bool(
                GameRuleKeys::DisableElytraMovementCheck,
                "Disable Elytra Movement Check",
                "disableElytraMovementCheck",
            ),
            FieldOptions::new_bool(GameRuleKeys::DisableRaids, "Disable Raids", "disableRaids"),
            FieldOptions::new_bool(
                GameRuleKeys::DoDaylightCycle,
                "Do Daylight Cycle",
                "doDaylightCycle",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::DoEntityDrops,
                "Do Entity Drops",
                "doEntityDrops",
            ),
            FieldOptions::new_bool(GameRuleKeys::DoFireTick, "Do Fire Tick", "doFireTick"),
            FieldOptions::new_bool(
                GameRuleKeys::DoImmediateRespawn,
                "Do Immediate Respawn",
                "doImmediateRespawn",
            ),
            FieldOptions::new_bool(GameRuleKeys::DoInsomnia, "Do Insomnia", "doInsomnia"),
            FieldOptions::new_bool(
                GameRuleKeys::DoLimitedCrafting,
                "Do Limited Crafting",
                "doLimitedCrafting",
            ),
            FieldOptions::new_bool(GameRuleKeys::DoMobLoot, "Do Mob Loot", "doMobLoot"),
            FieldOptions::new_bool(
                GameRuleKeys::DoMobSpawning,
                "Do Mob Spawning",
                "doMobSpawning",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::DoPatrolSpawning,
                "Do Patrol Spawning",
                "doPatrolSpawning",
            ),
            FieldOptions::new_bool(GameRuleKeys::DoTileDrops, "Do Tile Drops", "doTileDrops"),
            FieldOptions::new_bool(
                GameRuleKeys::DoTraderSpawning,
                "Do Trader Spawning",
                "doTraderSpawning",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::DoVinesSpread,
                "Do Vines Spread",
                "doVinesSpread",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::DoWardenSpawning,
                "Do Warden Spawning",
                "doWardenSpawning",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::DoWeatherCycle,
                "Do Weather Cycle",
                "doWeatherCycle",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::DrowningDamage,
                "Drowning Damage",
                "drowningDamage",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::EnderPearlsVanishOnDeath,
                "Ender Pearls Vanish On Death",
                "enderPearlsVanishOnDeath",
            ),
            FieldOptions::new_bool(GameRuleKeys::FallDamage, "Fall Damage", "fallDamage"),
            FieldOptions::new_bool(GameRuleKeys::FireDamage, "Fire Damage", "fireDamage"),
            FieldOptions::new_bool(
                GameRuleKeys::ForgiveDeadPlayers,
                "Forgive Dead Players",
                "forgiveDeadPlayers",
            ),
            FieldOptions::new_bool(GameRuleKeys::FreezeDamage, "Freeze Damage", "freezeDamage"),
            FieldOptions::new_bool(
                GameRuleKeys::GlobalSoundEvents,
                "Global Sound Events",
                "globalSoundEvents",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::KeepInventory,
                "Keep Inventory",
                "keepInventory",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::LavaSourceConversion,
                "Lava Source Conversion",
                "lavaSourceConversion",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::LogAdminCommands,
                "Log Admin Commands",
                "logAdminCommands",
            ),
            FieldOptions::new_number(
                GameRuleKeys::MaxCommandChainLength,
                "Max Command Chain Length",
                Some(0),
                Some(16777215),
            ),
            FieldOptions::new_number(
                GameRuleKeys::MaxCommandForkCount,
                "Max Command Fork Count",
                Some(0),
                Some(16777215),
            ),
            FieldOptions::new_number(
                GameRuleKeys::MaxEntityCramming,
                "Max Entity Cramming",
                Some(0),
                Some(255),
            ),
            FieldOptions::new_bool(
                GameRuleKeys::MobExplosionDropDecay,
                "Mob Explosion Drop Decay",
                "mobExplosionDropDecay",
            ),
            FieldOptions::new_bool(GameRuleKeys::MobGriefing, "Mob Griefing", "mobGriefing"),
            FieldOptions::new_bool(
                GameRuleKeys::NaturalRegeneration,
                "Natural Regeneration",
                "naturalRegeneration",
            ),
            FieldOptions::new_number(
                GameRuleKeys::PlayersNetherPortalCreativeDelay,
                "Players Nether Portal Creative Delay",
                Some(0),
                Some(16777215),
            ),
            FieldOptions::new_number(
                GameRuleKeys::PlayersNetherPortalDefaultDelay,
                "Players Nether Portal Default Delay",
                Some(0),
                Some(16777215),
            ),
            FieldOptions::new_number(
                GameRuleKeys::PlayersSleepingPercentage,
                "Players Sleeping Percentage",
                Some(0),
                Some(100),
            ),
            FieldOptions::new_bool(
                GameRuleKeys::ProjectilesCanBreakBlocks,
                "Projectiles Can Break Blocks",
                "projectilesCanBreakBlocks",
            ),
            FieldOptions::new_number(
                GameRuleKeys::RandomTickSpeed,
                "Random Tick Speed",
                Some(0),
                Some(4096),
            ),
            FieldOptions::new_bool(
                GameRuleKeys::ReducedDebugInfo,
                "Reduced Debug Info",
                "reducedDebugInfo",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::SendCommandFeedback,
                "Send Command Feedback",
                "sendCommandFeedback",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::ShowDeathMessages,
                "Show Death Messages",
                "showDeathMessages",
            ),
            FieldOptions::new_number(
                GameRuleKeys::SnowAccumulationHeight,
                "Snow Accumulation Height",
                Some(1),
                Some(8),
            ),
            FieldOptions::new_number(
                GameRuleKeys::SpawnChunkRadius,
                "Spawn Chunk Radius",
                Some(0),
                Some(32),
            ),
            FieldOptions::new_number(
                GameRuleKeys::SpawnRadius,
                "Spawn Radius",
                Some(0),
                Some(65536),
            ),
            FieldOptions::new_bool(
                GameRuleKeys::SpectatorsGenerateChunks,
                "Spectators Generate Chunks",
                "spectatorsGenerateChunks",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::TntExplosionDropDecay,
                "TNT Explosion Drop Decay",
                "tntExplosionDropDecay",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::UniversalAnger,
                "Universal Anger",
                "universalAnger",
            ),
            FieldOptions::new_bool(
                GameRuleKeys::WaterSourceConversion,
                "Water Source Conversion",
                "waterSourceConversion",
            ),
        ])
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

    fn set_gamerule_value(&self, key: GameRuleKeys, value: FieldValue) {
        println!("Game rule updated {:?} {:?}", key, value);
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
pub enum ConfigKeys {
    Port,
    Seed,
    MaxPlayers,
    OnlineMode,
}

#[derive(Debug, Clone, Copy)]
pub enum GameRuleKeys {
    AnnounceAdvancements,
    BlockExplosionDropDecay,
    CommandBlockOutput,
    CommandModificationBlockLimit,
    DisableElytraMovementCheck,
    DisableRaids,
    DoDaylightCycle,
    DoEntityDrops,
    DoFireTick,
    DoImmediateRespawn,
    DoInsomnia,
    DoLimitedCrafting,
    DoMobLoot,
    DoMobSpawning,
    DoPatrolSpawning,
    DoTileDrops,
    DoTraderSpawning,
    DoVinesSpread,
    DoWardenSpawning,
    DoWeatherCycle,
    DrowningDamage,
    EnderPearlsVanishOnDeath,
    FallDamage,
    FireDamage,
    ForgiveDeadPlayers,
    FreezeDamage,
    GlobalSoundEvents,
    KeepInventory,
    LavaSourceConversion,
    LogAdminCommands,
    MaxCommandChainLength,
    MaxCommandForkCount,
    MaxEntityCramming,
    MobExplosionDropDecay,
    MobGriefing,
    NaturalRegeneration,
    PlayersNetherPortalCreativeDelay,
    PlayersNetherPortalDefaultDelay,
    PlayersSleepingPercentage,
    ProjectilesCanBreakBlocks,
    RandomTickSpeed,
    ReducedDebugInfo,
    SendCommandFeedback,
    ShowDeathMessages,
    SnowAccumulationHeight,
    SpawnChunkRadius,
    SpawnRadius,
    SpectatorsGenerateChunks,
    TntExplosionDropDecay,
    UniversalAnger,
    WaterSourceConversion,
}
