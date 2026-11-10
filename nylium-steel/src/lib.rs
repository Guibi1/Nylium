//! SteelMC embedded in Nylium. Steel's Tokio runtimes live on a dedicated OS thread.
mod config;
mod logging;
mod state;

use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use nylium_adapter::{
    FieldValue, Lifecycle, Metric, Section, ServerAdapter, ServerSnapshot, StatePublisher,
};
use smol::channel::{self, Receiver, Sender};
use steel::SteelServer;
use steel_core::{command::sender::CommandSender, server::Server};
use tokio::runtime::{Builder, Runtime};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use config::ConfigFile;
pub use logging::SteelLogLayer;

type Reply = Sender<Result<(), String>>;
enum Request {
    Start(Reply),
    Stop(Reply),
    Command(String, Reply),
    Set(Section, String, FieldValue, Reply),
}

/// Owns a SteelMC instance and publishes live state for the built-in Nylium pages.
#[derive(Clone)]
pub struct SteelAdapter {
    state: Arc<StatePublisher>,
    requests: Sender<Request>,
    cancellation: Arc<Mutex<Option<CancellationToken>>>,
    panicked: Arc<AtomicBool>,
}

impl SteelAdapter {
    /// Loads Steel's configuration; relative world paths keep Steel's working-directory semantics.
    pub fn new(config_path: impl Into<PathBuf>) -> Result<Self, String> {
        let config = ConfigFile::open(config_path.into())?;
        let state = Arc::new(StatePublisher::new(ServerSnapshot {
            name: "SteelMC".into(),
            settings: config.fields()?,
            ..Default::default()
        }));
        let cancellation = Arc::new(Mutex::new(None));
        let panicked = Arc::new(AtomicBool::new(false));
        let (requests, receiver) = channel::bounded(32);
        let actor = Actor {
            state: state.clone(),
            config,
            cancellation: cancellation.clone(),
            panicked: panicked.clone(),
            running: None,
            system: sysinfo::System::new(),
            last_metric: Instant::now(),
            last_config_error: None,
        };
        thread::Builder::new()
            .name("nylium-steel".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || actor.run(receiver))
            .map_err(|e| format!("Cannot create Steel thread: {e}"))?;
        Ok(Self {
            state,
            requests,
            cancellation,
            panicked,
        })
    }

    /// Add this layer to the host's tracing subscriber to receive Steel's logs and command feedback.
    pub fn logging_layer(&self) -> SteelLogLayer {
        SteelLogLayer::new(self.state.clone())
    }

    /// Chains the host's panic hook, cancelling Steel on worker panics like its standalone launcher.
    /// Hosts that already cancel their server on panics can keep their own hook instead.
    pub fn install_panic_handler(&self) {
        let cancellation = Arc::downgrade(&self.cancellation);
        let panicked = Arc::downgrade(&self.panicked);
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let current = thread::current();
            let name = current.name().unwrap_or("");
            if name == "nylium-steel"
                || ["steel-", "world-tick-", "rayon-gen-", "rayon-chunk-"]
                    .iter()
                    .any(|prefix| name.starts_with(prefix))
            {
                if let Some(panicked) = panicked.upgrade() {
                    panicked.store(true, Ordering::Release);
                }
                // Never wait on a mutex in a panic hook: the panicking worker may own it.
                if let Some(cancellation) = cancellation.upgrade()
                    && let Ok(guard) = cancellation.try_lock()
                    && let Some(token) = guard.as_ref()
                {
                    token.cancel();
                }
            }
            previous(info);
        }));
    }

    async fn request(&self, make: impl FnOnce(Reply) -> Request) -> Result<(), String> {
        let (reply, result) = channel::bounded(1);
        self.requests
            .send(make(reply))
            .await
            .map_err(|_| "Steel worker has stopped".to_string())?;
        result
            .recv()
            .await
            .map_err(|_| "Steel worker stopped before completing the operation".to_string())?
    }
}

#[cfg(feature = "ui")]
impl nylium::UiAdapter for SteelAdapter {}

#[async_trait]
impl ServerAdapter for SteelAdapter {
    fn subscribe(&self) -> Receiver<()> {
        self.state.subscribe()
    }
    fn snapshot(&self) -> Arc<ServerSnapshot> {
        self.state.snapshot()
    }
    async fn start(&self) -> Result<(), String> {
        self.request(Request::Start).await
    }
    async fn stop(&self) -> Result<(), String> {
        if let Some(token) = self
            .cancellation
            .lock()
            .map_err(|e| e.to_string())?
            .as_ref()
        {
            token.cancel();
        }
        self.request(Request::Stop).await
    }
    /// Steel acknowledges queue admission; syntax/execution feedback arrives through the log layer.
    async fn command(&self, command: String) -> Result<(), String> {
        self.request(|reply| Request::Command(command, reply)).await
    }
    async fn set_field(
        &self,
        section: Section,
        id: String,
        value: FieldValue,
    ) -> Result<(), String> {
        self.request(|reply| Request::Set(section, id, value, reply))
            .await
    }
}

// Field order keeps the chunk runtime alive while main-runtime tasks are dropped.
struct Runtimes {
    main: Runtime,
    chunks: Arc<Runtime>,
}

struct Running {
    runtimes: Runtimes,
    server: Arc<Server>,
    cancel: CancellationToken,
    tracker: TaskTracker,
    task: tokio::task::JoinHandle<()>,
    started: Instant,
    settings: Vec<nylium_adapter::Field>,
}

struct Actor {
    state: Arc<StatePublisher>,
    config: ConfigFile,
    cancellation: Arc<Mutex<Option<CancellationToken>>>,
    panicked: Arc<AtomicBool>,
    running: Option<Running>,
    system: sysinfo::System,
    last_metric: Instant,
    last_config_error: Option<String>,
}

impl Actor {
    fn run(mut self, requests: Receiver<Request>) {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.run_inner(requests)));
        if result.is_err() {
            let _ = self.stop();
            let error = "Steel worker panicked; inspect the server logs".to_string();
            logging::append(&self.state, nylium_adapter::LogLevel::Error, error.clone());
            self.state.update(|state| {
                state.lifecycle = Lifecycle::Failed(error);
                state.connected = false;
            });
        }
    }

    fn run_inner(&mut self, requests: Receiver<Request>) {
        loop {
            let event = smol::block_on(smol::future::or(
                async { Some(requests.recv().await) },
                async {
                    smol::Timer::after(Duration::from_millis(250)).await;
                    None
                },
            ));
            match event {
                Some(Ok(request)) => {
                    let (result, reply) = match request {
                        Request::Start(reply) => (self.start(), reply),
                        Request::Stop(reply) => (self.stop(), reply),
                        Request::Command(command, reply) => (self.command(command), reply),
                        Request::Set(section, id, value, reply) => {
                            (self.set(section, id, value), reply)
                        }
                    };
                    self.refresh();
                    let _ = reply.try_send(result);
                }
                Some(Err(_)) => {
                    let _ = self.stop();
                    return;
                }
                None => self.refresh(),
            }
        }
    }

    fn start(&mut self) -> Result<(), String> {
        if self.running.is_some() {
            return Ok(());
        }
        self.panicked.store(false, Ordering::Release);
        self.state.update(|state| {
            state.lifecycle = Lifecycle::Starting;
            state.connected = false;
        });
        let result = self.launch();
        match result {
            Ok(running) => {
                self.state.update(|state| {
                    state.lifecycle = Lifecycle::Running;
                    state.connected = true;
                    state.metrics.clear();
                    state.restart_required = false;
                });
                self.last_metric = Instant::now();
                self.running = Some(running);
                Ok(())
            }
            Err(error) => {
                self.state.update(|state| {
                    state.lifecycle = Lifecycle::Failed(error.clone());
                    state.connected = false;
                });
                if let Ok(mut cancellation) = self.cancellation.lock() {
                    *cancellation = None;
                }
                Err(error)
            }
        }
    }

    fn launch(&self) -> Result<Running, String> {
        let config = self.config.load()?;
        let settings = self.config.fields()?;
        let workers = steel_utils::threading::available_worker_threads();
        let main = Builder::new_multi_thread()
            .worker_threads(steel_utils::threading::worker_threads_for_available(
                config.server.threads.main_runtime,
                workers,
            ))
            .thread_name("steel-main")
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let chunks = Arc::new(
            Builder::new_multi_thread()
                .worker_threads(steel_utils::threading::worker_threads_for_available(
                    config.server.threads.chunk_runtime,
                    workers,
                ))
                .thread_name("steel-chunks")
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?,
        );
        let runtimes = Runtimes { main, chunks };
        let cancel = CancellationToken::new();
        *self.cancellation.lock().map_err(|e| e.to_string())? = Some(cancel.clone());
        let mut steel = runtimes
            .main
            .block_on(SteelServer::new(
                runtimes.chunks.clone(),
                cancel.clone(),
                config,
            ))
            .map_err(|e| e.to_string())?;
        let server = steel.server.clone();
        if !runtimes.main.block_on(server.prepare_spawn_area()) {
            cancel.cancel();
            let save = runtimes.main.block_on(server.save_and_shutdown());
            return Err(match save {
                Ok(()) => "Steel startup was cancelled".into(),
                Err(errors) => format!(
                    "Steel startup was cancelled; shutdown save failed: {}",
                    errors.join("; ")
                ),
            });
        }
        let tracker = TaskTracker::new();
        let task_tracker = tracker.clone();
        let task = runtimes.main.spawn(async move {
            steel.start(task_tracker).await;
        });
        Ok(Running {
            runtimes,
            server,
            cancel,
            tracker,
            task,
            started: Instant::now(),
            settings,
        })
    }

    fn stop(&mut self) -> Result<(), String> {
        let Some(running) = self.running.take() else {
            return Ok(());
        };
        self.state
            .update(|state| state.lifecycle = Lifecycle::Stopping);
        running.cancel.cancel();
        let result = running.runtimes.main.block_on(async {
            let result = running
                .task
                .await
                .map_err(|e| format!("Steel task failed: {e}"));
            running.tracker.close();
            running.tracker.wait().await;
            let save = running
                .server
                .save_and_shutdown()
                .await
                .map_err(|errors| format!("Steel shutdown save failed: {}", errors.join("; ")));
            match (result, save) {
                (Err(task), Err(save)) => Err(format!("{task}; {save}")),
                (Err(error), _) | (_, Err(error)) => Err(error),
                (Ok(()), Ok(())) => Ok(()),
            }
        });
        if let Ok(mut cancellation) = self.cancellation.lock() {
            *cancellation = None;
        }
        self.state.update(|state| {
            state.connected = false;
            state.lifecycle = match &result {
                Ok(()) => Lifecycle::Stopped,
                Err(error) => Lifecycle::Failed(error.clone()),
            };
            for player in &mut state.players {
                player.online = false;
            }
        });
        result
    }

    fn command(&self, command: String) -> Result<(), String> {
        let running = self.running.as_ref().ok_or("Steel is not running")?;
        let command = command.trim().strip_prefix('/').unwrap_or(command.trim());
        if command.is_empty() || command.contains(['\r', '\n', '\0']) {
            return Err("Enter one nonempty command".into());
        }
        if running.cancel.is_cancelled() {
            return Err("Steel is stopping".into());
        }
        running
            .server
            .submit_command(CommandSender::Console, command.to_owned())
            .map_err(|e| format!("Steel command queue: {e:?}"))?;
        // Wait briefly before a close request; the acknowledgement still means queue admission.
        running.runtimes.main.block_on(async {
            tokio::select! {
                () = running.server.wait_until_tick(running.server.current_tick().saturating_add(2)) => {},
                () = running.cancel.cancelled() => {},
                () = tokio::time::sleep(Duration::from_secs(5)) => {
                    logging::append(&self.state, nylium_adapter::LogLevel::Warn,
                        "Command accepted, but Steel has not advanced two ticks yet.".into());
                },
            }
        });
        Ok(())
    }

    fn set(&mut self, section: Section, id: String, value: FieldValue) -> Result<(), String> {
        match section {
            Section::Settings => {
                let fields = self.config.set(&id, value)?;
                self.state.update(|state| {
                    state.restart_required = self
                        .running
                        .as_ref()
                        .is_some_and(|running| running.settings != fields);
                    state.settings = fields;
                });
                Ok(())
            }
            Section::GameRules => {
                let running = self
                    .running
                    .as_ref()
                    .ok_or("Start Steel before changing game rules")?;
                if running.cancel.is_cancelled() {
                    return Err("Steel is stopping".into());
                }
                let world = state::set_rule(&running.server, &id, value)?;
                // Keep the synchronous guard on a blocking thread while Steel persists level data.
                running.runtimes.main.block_on(async move {
                    tokio::task::spawn_blocking(move || {
                        tokio::runtime::Handle::current().block_on(world.level_data.write().save())
                    })
                    .await
                    .map_err(|e| e.to_string())?
                    .map_err(|e| format!("Cannot save game rules: {e}"))
                })
            }
        }
    }

    fn refresh(&mut self) {
        if self.panicked.swap(false, Ordering::AcqRel) {
            let _ = self.stop();
            let error = "A Steel worker panicked; the server has been stopped.".to_string();
            logging::append(&self.state, nylium_adapter::LogLevel::Error, error.clone());
            self.state.update(|state| {
                state.lifecycle = Lifecycle::Failed(error);
                state.connected = false;
            });
            return;
        }

        if self
            .running
            .as_ref()
            .is_some_and(|running| running.task.is_finished())
        {
            let _ = self.stop();
        }
        let fields = match self.config.fields() {
            Ok(fields) => {
                self.last_config_error = None;
                Some(fields)
            }
            Err(error) => {
                if self.last_config_error.as_ref() != Some(&error) {
                    logging::append(&self.state, nylium_adapter::LogLevel::Error, error.clone());
                    self.last_config_error = Some(error);
                }
                None
            }
        };
        let live = self.running.as_ref().map(|running| {
            (
                state::players(&running.server),
                state::game_rules(&running.server),
            )
        });
        let metric = if self.last_metric.elapsed() >= Duration::from_secs(1) {
            self.last_metric = Instant::now();
            self.running.as_ref().and_then(|running| {
                let pid = sysinfo::get_current_pid().ok()?;
                self.system.refresh_processes_specifics(
                    sysinfo::ProcessesToUpdate::Some(&[pid]),
                    false,
                    sysinfo::ProcessRefreshKind::nothing()
                        .with_cpu()
                        .with_memory(),
                );
                self.system.process(pid).map(|process| Metric {
                    elapsed_seconds: running.started.elapsed().as_secs(),
                    cpu_percent: process.cpu_usage()
                        / thread::available_parallelism().map_or(1, usize::from) as f32,
                    memory_bytes: process.memory(),
                })
            })
        } else {
            None
        };
        let current = self.state.snapshot();
        let changed = fields
            .as_ref()
            .is_some_and(|fields| current.settings != *fields)
            || live.as_ref().is_some_and(|(players, rules)| {
                current.players != *players || current.game_rules != *rules
            })
            || metric.is_some();
        if changed {
            self.state.update(|state| {
                if let Some(fields) = fields {
                    state.restart_required =
                        self.running.as_ref().is_some_and(|r| r.settings != fields);
                    state.settings = fields;
                }
                if let Some((players, rules)) = live {
                    state.players = players;
                    state.game_rules = rules;
                }
                if let Some(metric) = metric {
                    state.metrics.push(metric);
                    if state.metrics.len() > 120 {
                        state.metrics.remove(0);
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn real_steel_lifecycle_rules_persistence_and_external_settings() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config/config.toml");
        let config = ConfigFile::open(config_path.clone()).unwrap();
        config
            .set("server.server_port", FieldValue::Number(0))
            .unwrap();
        config
            .set("server.online_mode", FieldValue::Boolean(false))
            .unwrap();
        config
            .set("server.use_favicon", FieldValue::Boolean(false))
            .unwrap();
        for pool in [
            "main_runtime",
            "chunk_runtime",
            "packet_workers",
            "chunk_generation",
            "chunk_encoding",
        ] {
            config
                .set(&format!("server.threads.{pool}"), FieldValue::Number(1))
                .unwrap();
        }
        fs::write(
            dir.path().join("config/worlds.toml"),
            format!(
                r#"
save_path = {:?}
seed = "1"
[domains.minecraft]
default = true
[[domains.minecraft.worlds]]
name = "overworld"
generator = "minecraft:flat"
default = true
"#,
                dir.path().join("worlds").to_str().unwrap()
            ),
        )
        .unwrap();
        let adapter = SteelAdapter::new(config_path).unwrap();
        adapter.install_panic_handler();
        smol::block_on(async {
            adapter.start().await.unwrap();
            adapter.start().await.unwrap();
            assert_eq!(adapter.snapshot().lifecycle, Lifecycle::Running);
            let id = "minecraft:overworld#minecraft:keep_inventory".to_string();
            assert!(
                adapter
                    .snapshot()
                    .game_rules
                    .iter()
                    .any(|field| field.id == id)
            );
            adapter
                .set_field(Section::GameRules, id.clone(), FieldValue::Boolean(true))
                .await
                .unwrap();
            assert_eq!(
                adapter
                    .snapshot()
                    .game_rules
                    .iter()
                    .find(|field| field.id == id)
                    .unwrap()
                    .value,
                FieldValue::Boolean(true)
            );
            // Another process's edit must be observed without an explicit refresh request.
            let updates = adapter.subscribe();
            config
                .set("server.motd", FieldValue::Text("external change".into()))
                .unwrap();
            let observed = smol::future::or(
                async {
                    loop {
                        updates.recv().await.unwrap();
                        if adapter.snapshot().settings.iter().any(|field| {
                            field.id == "server.motd"
                                && field.value == FieldValue::Text("external change".into())
                        }) {
                            return true;
                        }
                    }
                },
                async {
                    smol::Timer::after(Duration::from_secs(5)).await;
                    false
                },
            )
            .await;
            assert!(observed, "external edit was not published");
            assert!(adapter.snapshot().restart_required);
            adapter
                .command("gamerule keep_inventory false".into())
                .await
                .unwrap();
            assert_eq!(
                adapter
                    .snapshot()
                    .game_rules
                    .iter()
                    .find(|field| field.id == id)
                    .unwrap()
                    .value,
                FieldValue::Boolean(false)
            );
            adapter
                .set_field(Section::GameRules, id.clone(), FieldValue::Boolean(true))
                .await
                .unwrap();
            adapter.stop().await.unwrap();
            adapter.stop().await.unwrap();
            assert_eq!(adapter.snapshot().lifecycle, Lifecycle::Stopped);
            adapter.start().await.unwrap();
            assert!(!adapter.snapshot().restart_required);
            assert_eq!(
                adapter
                    .snapshot()
                    .game_rules
                    .iter()
                    .find(|field| field.id == id)
                    .unwrap()
                    .value,
                FieldValue::Boolean(true)
            );
            let updates = adapter.subscribe();
            let worker = thread::Builder::new()
                .name("steel-main-test".into())
                .spawn(|| panic!("injected worker failure"))
                .unwrap();
            assert!(worker.join().is_err());
            let failed = smol::future::or(
                async {
                    loop {
                        updates.recv().await.unwrap();
                        if matches!(adapter.snapshot().lifecycle, Lifecycle::Failed(_)) {
                            return true;
                        }
                    }
                },
                async {
                    smol::Timer::after(Duration::from_secs(5)).await;
                    false
                },
            )
            .await;
            assert!(failed, "worker panic did not stop the server");
            assert!(!adapter.snapshot().connected);
            adapter.stop().await.unwrap();

            adapter.start().await.unwrap();
            let worlds = dir.path().join("worlds");
            fs::rename(&worlds, dir.path().join("worlds-backup")).unwrap();
            fs::write(&worlds, "block world saves").unwrap();
            let error = adapter.stop().await.unwrap_err();
            assert!(error.contains("save"), "{error}");
            assert!(matches!(adapter.snapshot().lifecycle, Lifecycle::Failed(_)));
        });
    }
}
