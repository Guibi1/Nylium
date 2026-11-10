//! In-memory demo adapter. It simulates controls and players; it does not run Minecraft.

use std::{
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use chrono::Utc;
use nylium_adapter::{
    Field, FieldKind, FieldValue, Lifecycle, LogEntry, LogLevel, Metric, Player, Section,
    ServerAdapter, ServerSnapshot, StatePublisher,
};

#[derive(Clone)]
pub struct DemoServer {
    state: Arc<StatePublisher>,
    sampler: Arc<AtomicU64>,
    lifecycle_lock: Arc<Mutex<()>>,
}

impl DemoServer {
    pub fn new() -> Self {
        let snapshot = ServerSnapshot {
            name: "Nylium Demo".into(),
            settings: vec![
                Field {
                    id: "port".into(),
                    label: "Port".into(),
                    description: "Demo server port".into(),
                    value: FieldValue::Number(25565),
                    kind: FieldKind::Number {
                        min: Some(1),
                        max: Some(65535),
                    },
                },
                Field {
                    id: "motd".into(),
                    label: "MOTD".into(),
                    description: "Message shown to players".into(),
                    value: FieldValue::Text("A Nylium demo server".into()),
                    kind: FieldKind::Text,
                },
                Field {
                    id: "seed".into(),
                    label: "Seed".into(),
                    description: "Demo world seed".into(),
                    value: FieldValue::Text(String::new()),
                    kind: FieldKind::Text,
                },
                Field {
                    id: "max_players".into(),
                    label: "Max players".into(),
                    description: "Maximum simulated players".into(),
                    value: FieldValue::Number(20),
                    kind: FieldKind::Number {
                        min: Some(1),
                        max: Some(100),
                    },
                },
                Field {
                    id: "online_mode".into(),
                    label: "Online mode".into(),
                    description: "Require authenticated players".into(),
                    value: FieldValue::Boolean(true),
                    kind: FieldKind::Boolean,
                },
            ],
            game_rules: vec![
                Field {
                    id: "keep_inventory".into(),
                    label: "Keep inventory".into(),
                    description: "Keep inventory after death".into(),
                    value: FieldValue::Boolean(false),
                    kind: FieldKind::Boolean,
                },
                Field {
                    id: "do_daylight_cycle".into(),
                    label: "Daylight cycle".into(),
                    description: "Advance the in-game time".into(),
                    value: FieldValue::Boolean(true),
                    kind: FieldKind::Boolean,
                },
                Field {
                    id: "random_tick_speed".into(),
                    label: "Random tick speed".into(),
                    description: "Block random tick rate".into(),
                    value: FieldValue::Number(3),
                    kind: FieldKind::Number {
                        min: Some(0),
                        max: Some(4096),
                    },
                },
            ],
            players: vec![
                Player {
                    id: uuid::Uuid::from_u128(1),
                    name: "Alex".into(),
                    online: true,
                    dimension: "minecraft:overworld".into(),
                    operator: true,
                },
                Player {
                    id: uuid::Uuid::from_u128(2),
                    name: "Steve".into(),
                    online: true,
                    dimension: "minecraft:overworld".into(),
                    operator: false,
                },
            ],
            ..Default::default()
        };
        Self {
            state: Arc::new(StatePublisher::new(snapshot)),
            sampler: Arc::new(AtomicU64::new(0)),
            lifecycle_lock: Arc::new(Mutex::new(())),
        }
    }

    fn log(&self, level: LogLevel, message: impl Into<String>) {
        let message = message.into();
        self.state.update(|snapshot| {
            let sequence = snapshot
                .logs
                .last()
                .map_or(1, |entry| entry.sequence.saturating_add(1));
            snapshot.logs.push(LogEntry {
                sequence,
                timestamp: Utc::now().to_rfc3339(),
                level,
                message,
            });
            if snapshot.logs.len() > 1000 {
                snapshot.logs.remove(0);
            }
        });
    }
}

impl Default for DemoServer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ServerAdapter for DemoServer {
    fn subscribe(&self) -> smol::channel::Receiver<()> {
        self.state.subscribe()
    }

    fn snapshot(&self) -> Arc<ServerSnapshot> {
        self.state.snapshot()
    }

    async fn start(&self) -> Result<(), String> {
        let _guard = self
            .lifecycle_lock
            .lock()
            .map_err(|_| "lifecycle lock poisoned".to_string())?;
        let lifecycle = self.snapshot().lifecycle.clone();
        if matches!(lifecycle, Lifecycle::Running | Lifecycle::Starting) {
            return Ok(());
        }
        let generation = self.sampler.fetch_add(1, Ordering::Relaxed) + 1;
        self.state.update(|snapshot| {
            snapshot.lifecycle = Lifecycle::Starting;
            snapshot.connected = true;
            snapshot.metrics.clear();
            snapshot.restart_required = false;
            snapshot
                .players
                .iter_mut()
                .for_each(|player| player.online = true);
        });
        self.log(LogLevel::Info, "Demo server started (in-memory simulation)");
        self.state
            .update(|snapshot| snapshot.lifecycle = Lifecycle::Running);

        let weak: Weak<StatePublisher> = Arc::downgrade(&self.state);
        let sampler = self.sampler.clone();
        smol::spawn(async move {
            let pid = sysinfo::get_current_pid().ok();
            let mut system = sysinfo::System::new();
            let started = std::time::Instant::now();
            loop {
                smol::Timer::after(Duration::from_secs(1)).await;
                let Some(state) = weak.upgrade() else { break };
                if sampler.load(Ordering::Relaxed) != generation {
                    break;
                }
                let snapshot = state.snapshot();
                if !matches!(&snapshot.lifecycle, Lifecycle::Running) {
                    break;
                }
                let mut cpu_percent = 0.0;
                let mut memory_bytes = 0;
                if let Some(pid) = pid {
                    system.refresh_processes_specifics(
                        sysinfo::ProcessesToUpdate::Some(&[pid]),
                        false,
                        sysinfo::ProcessRefreshKind::nothing()
                            .with_cpu()
                            .with_memory(),
                    );
                    if let Some(process) = system.process(pid) {
                        let cpu_count = std::thread::available_parallelism().map_or(1, usize::from);
                        cpu_percent = process.cpu_usage() / cpu_count as f32;
                        memory_bytes = process.memory();
                    }
                }
                state.update(|snapshot| {
                    if sampler.load(Ordering::Relaxed) == generation
                        && matches!(&snapshot.lifecycle, Lifecycle::Running)
                    {
                        snapshot.metrics.push(Metric {
                            elapsed_seconds: started.elapsed().as_secs(),
                            cpu_percent,
                            memory_bytes,
                        });
                        if snapshot.metrics.len() > 120 {
                            snapshot.metrics.remove(0);
                        }
                    }
                });
            }
        })
        .detach();
        Ok(())
    }

    async fn stop(&self) -> Result<(), String> {
        let _guard = self
            .lifecycle_lock
            .lock()
            .map_err(|_| "lifecycle lock poisoned".to_string())?;
        let lifecycle = self.snapshot().lifecycle.clone();
        if matches!(lifecycle, Lifecycle::Stopped | Lifecycle::Stopping) {
            return Ok(());
        }
        self.sampler.fetch_add(1, Ordering::Relaxed);
        self.state
            .update(|snapshot| snapshot.lifecycle = Lifecycle::Stopping);
        self.log(LogLevel::Info, "Demo server stopped");
        self.state.update(|snapshot| {
            snapshot.lifecycle = Lifecycle::Stopped;
            snapshot.connected = false;
            snapshot
                .players
                .iter_mut()
                .for_each(|player| player.online = false);
        });
        Ok(())
    }

    async fn command(&self, command: String) -> Result<(), String> {
        if !matches!(&self.snapshot().lifecycle, Lifecycle::Running) {
            return Err("Demo server is not running".into());
        }
        let mut words = command.split_whitespace();
        let Some(action) = words.next() else {
            return Err("Command cannot be empty".into());
        };
        match action.to_ascii_lowercase().as_str() {
            "help" => self.log(
                LogLevel::Info,
                "Commands: help, list, say, op, deop, kick, ban, stop",
            ),
            "list" => {
                let snapshot = self.snapshot();
                let players = snapshot
                    .players
                    .iter()
                    .filter(|player| player.online)
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>();
                self.log(
                    LogLevel::Info,
                    format!(
                        "There are {} players online: {}",
                        players.len(),
                        players.join(", ")
                    ),
                );
            }
            "say" => {
                let message = words.collect::<Vec<_>>().join(" ");
                if message.is_empty() {
                    return Err("Usage: say <message>".into());
                }
                self.log(LogLevel::Info, format!("[Server] {message}"));
            }
            "op" | "deop" => {
                let target = words
                    .next()
                    .ok_or_else(|| format!("Usage: {action} <player>"))?;
                let player_id = resolve_player(&self.snapshot(), target)
                    .ok_or_else(|| format!("Unknown player: {target}"))?;
                let operator = action.eq_ignore_ascii_case("op");
                if !self.snapshot().players.iter().any(|p| p.id == player_id) {
                    return Err(format!("Unknown player: {target}"));
                }
                self.state.update(|snapshot| {
                    if let Some(player) = snapshot.players.iter_mut().find(|p| p.id == player_id) {
                        player.operator = operator;
                    }
                });
                self.log(
                    LogLevel::Info,
                    format!(
                        "{target} is {}",
                        if operator {
                            "now an operator"
                        } else {
                            "no longer an operator"
                        }
                    ),
                );
            }
            "kick" | "ban" => {
                let target = words
                    .next()
                    .ok_or_else(|| format!("Usage: {action} <player> [reason]"))?;
                let reason = words.collect::<Vec<_>>().join(" ");
                let player_id = resolve_player(&self.snapshot(), target)
                    .filter(|id| {
                        action.eq_ignore_ascii_case("ban")
                            || self
                                .snapshot()
                                .players
                                .iter()
                                .any(|p| p.id == *id && p.online)
                    })
                    .ok_or_else(|| format!("Unknown online player: {target}"))?;
                if !self
                    .snapshot()
                    .players
                    .iter()
                    .any(|p| p.id == player_id && (action.eq_ignore_ascii_case("ban") || p.online))
                {
                    return Err(format!("Unknown online player: {target}"));
                }
                self.state.update(|snapshot| {
                    if let Some(player) = snapshot
                        .players
                        .iter_mut()
                        .find(|p| p.id == player_id && p.online)
                    {
                        player.online = false;
                    }
                });
                self.log(
                    LogLevel::Warn,
                    format!(
                        "{target} was {}{}",
                        action.to_ascii_lowercase(),
                        if reason.is_empty() {
                            String::new()
                        } else {
                            format!(": {reason}")
                        }
                    ),
                );
            }
            "stop" => return self.stop().await,
            _ => return Err(format!("Unknown demo command: {action}")),
        }
        Ok(())
    }

    async fn set_field(
        &self,
        section: Section,
        id: String,
        value: FieldValue,
    ) -> Result<(), String> {
        let snapshot = self.snapshot();
        let fields = match section {
            Section::Settings => &snapshot.settings,
            Section::GameRules => &snapshot.game_rules,
        };
        let field = fields
            .iter()
            .find(|field| field.id == id)
            .ok_or_else(|| format!("Unknown field: {id}"))?;
        field.validate(&value)?;
        self.state.update(|snapshot| {
            let fields = match section {
                Section::Settings => &mut snapshot.settings,
                Section::GameRules => &mut snapshot.game_rules,
            };
            if let Some(field) = fields.iter_mut().find(|field| field.id == id) {
                field.value = value;
            }
            if section == Section::Settings {
                snapshot.restart_required = true;
            }
        });
        Ok(())
    }
}

fn resolve_player(snapshot: &ServerSnapshot, target: &str) -> Option<uuid::Uuid> {
    if let Ok(id) = uuid::Uuid::parse_str(target) {
        return snapshot
            .players
            .iter()
            .find(|player| player.id == id)
            .map(|player| player.id);
    }
    snapshot
        .players
        .iter()
        .find(|player| player.name.eq_ignore_ascii_case(target))
        .map(|player| player.id)
}

use crate::{AdapterPage, UiAdapter};
use gpui::{App, AppContext, Context, IntoElement, ParentElement, Render, Styled, Window, div};
// A real backend can return arbitrary views through the same extension point.
impl UiAdapter for DemoServer {
    fn pages(&self, _window: &mut Window, cx: &mut App) -> Vec<AdapterPage> {
        let view = cx.new(|_| DemoPage {
            state: self.snapshot(),
        });
        vec![AdapterPage::new(
            "demo.adapter",
            "Adapter",
            view,
            |view, state, _, _| view.state = state,
        )]
    }
}

struct DemoPage {
    state: Arc<ServerSnapshot>,
}

impl Render for DemoPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use ely::theme::ActiveTheme;
        div().flex().flex_col().gap_3()
            .child(div().text_2xl().child("Demo adapter"))
            .child("This executable demonstrates Nylium without running Minecraft.")
            .child("Settings and game rules are held in memory. They reset when you close the application.")
            .child("Console commands: help, list, say <message>, op <name>, deop <name>, kick <name>, ban <name>, stop.")
            .child(div().text_color(cx.theme().colors.fg_muted).child(format!("Live state: {:?} · {} known players · {} log entries", self.state.lifecycle, self.state.players.len(), self.state.logs.len())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_commands_lifecycle_and_notifications_work() {
        smol::block_on(async {
            let server = DemoServer::new();
            let updates = server.subscribe();
            assert!(server.command("list".into()).await.is_err());
            server.start().await.unwrap();
            server
                .set_field(
                    Section::Settings,
                    "max_players".into(),
                    FieldValue::Number(30),
                )
                .await
                .unwrap();
            assert_eq!(
                server
                    .snapshot()
                    .settings
                    .iter()
                    .find(|field| field.id == "max_players")
                    .unwrap()
                    .value,
                FieldValue::Number(30)
            );
            assert!(server.snapshot().restart_required);
            assert!(
                server
                    .set_field(
                        Section::Settings,
                        "max_players".into(),
                        FieldValue::Text("bad".into())
                    )
                    .await
                    .is_err()
            );
            server.command("op Steve".into()).await.unwrap();
            assert!(server.snapshot().players[1].operator);
            let steve = server.snapshot().players[1].id;
            server.command(format!("deop {steve}")).await.unwrap();
            assert!(!server.snapshot().players[1].operator);
            server.command("kick Steve testing".into()).await.unwrap();
            assert!(!server.snapshot().players[1].online);
            server.command(format!("op {steve}")).await.unwrap();
            server
                .command(format!("ban {steve} testing"))
                .await
                .unwrap();
            assert!(server.snapshot().players[1].operator);
            assert!(server.command("invalid".into()).await.is_err());
            assert_eq!(updates.try_recv(), Ok(()));
            assert!(matches!(&server.snapshot().lifecycle, Lifecycle::Running));
            server.stop().await.unwrap();
            assert!(matches!(&server.snapshot().lifecycle, Lifecycle::Stopped));
            assert!(!server.snapshot().players.iter().any(|player| player.online));
            assert!(server.command("list".into()).await.is_err());
        });
    }
}
