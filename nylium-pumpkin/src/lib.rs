//! PumpkinMC embedded in Nylium. Pumpkin's Tokio runtimes live on a dedicated OS thread.
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
use pumpkin_core::PumpkinServer;
use pumpkin_core::{command::CommandSender, data::VanillaData, server::Server};
use smol::channel::{self, Receiver, Sender};
use tokio::runtime::{Builder, Runtime};
use tokio_util::sync::CancellationToken;

use config::ConfigFile;
pub use logging::PumpkinLogLayer;

type Reply = Sender<Result<(), String>>;
enum Request {
    Start(Reply),
    Stop(Reply),
    Command(String, Reply),
    Set(Section, String, FieldValue, Reply),
}

/// Owns a PumpkinMC instance and publishes live state for the built-in Nylium pages.
#[derive(Clone)]
pub struct PumpkinAdapter {
    state: Arc<StatePublisher>,
    requests: Sender<Request>,
    cancellation: Arc<Mutex<Option<CancellationToken>>>,
    panicked: Arc<AtomicBool>,
}

impl PumpkinAdapter {
    /// Loads Pumpkin's configuration; relative world paths keep Pumpkin's working-directory semantics.
    pub fn new(config_path: impl Into<PathBuf>) -> Result<Self, String> {
        let config = ConfigFile::open(config_path.into())?;
        let state = Arc::new(StatePublisher::new(ServerSnapshot {
            name: "PumpkinMC".into(),
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
            .name("nylium-pumpkin".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || actor.run(receiver))
            .map_err(|e| format!("Cannot create Pumpkin thread: {e}"))?;
        Ok(Self {
            state,
            requests,
            cancellation,
            panicked,
        })
    }

    /// Add this layer to the host's tracing subscriber to receive Pumpkin's logs and command feedback.
    pub fn logging_layer(&self) -> PumpkinLogLayer {
        PumpkinLogLayer::new(self.state.clone())
    }

    /// Chains the host's panic hook, cancelling Pumpkin on worker panics like its standalone launcher.
    /// Hosts that already cancel their server on panics can keep their own hook instead.
    pub fn install_panic_handler(&self) {
        let cancellation = Arc::downgrade(&self.cancellation);
        let panicked = Arc::downgrade(&self.panicked);
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let current = thread::current();
            let name = current.name().unwrap_or("");
            if !config::validation_in_progress()
                && (name == "nylium-pumpkin"
                    || ["pumpkin-", "world-tick-", "rayon-gen-", "rayon-chunk-"]
                        .iter()
                        .any(|prefix| name.starts_with(prefix)))
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
            .map_err(|_| "Pumpkin worker has stopped".to_string())?;
        result
            .recv()
            .await
            .map_err(|_| "Pumpkin worker stopped before completing the operation".to_string())?
    }
}

#[cfg(feature = "ui")]
impl nylium::UiAdapter for PumpkinAdapter {}

#[async_trait]
impl ServerAdapter for PumpkinAdapter {
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
    /// Execution feedback arrives through the log layer.
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

struct Running {
    runtime: Runtime,
    server: Arc<Server>,
    cancel: CancellationToken,
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
            let error = "Pumpkin worker panicked; inspect the server logs".to_string();
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
        let runtime = Builder::new_multi_thread()
            .thread_name("pumpkin-worker")
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let _ = rustls::crypto::ring::default_provider().install_default();
        let pumpkin = runtime
            .block_on(PumpkinServer::new(
                config.basic,
                config.advanced,
                config.telemetry,
                VanillaData::load(),
                Vec::new(),
                false,
            ))
            .map_err(|e| e.to_string())?;
        runtime.block_on(pumpkin.init_plugins());
        let server = pumpkin.server.clone();
        let cancel = server.stop_token.clone();
        *self.cancellation.lock().map_err(|e| e.to_string())? = Some(cancel.clone());
        let task = runtime.spawn(async move {
            pumpkin.start().await;
        });
        Ok(Running {
            runtime,
            server,
            cancel,
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
        let result = running
            .runtime
            .block_on(running.task)
            .map_err(|e| format!("Pumpkin task failed: {e}"));
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
        let running = self.running.as_ref().ok_or("Pumpkin is not running")?;
        let command = command.trim().strip_prefix('/').unwrap_or(command.trim());
        if command.is_empty() || command.contains(['\r', '\n', '\0']) {
            return Err("Enter one nonempty command".into());
        }
        if running.cancel.is_cancelled() {
            return Err("Pumpkin is stopping".into());
        }
        let _guard = running.runtime.enter();
        running.server.command_dispatcher.load().handle_command(
            &CommandSender::Console.into_source(&running.server),
            command,
        );
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
                    .ok_or("Start Pumpkin before changing game rules")?;
                if running.cancel.is_cancelled() {
                    return Err("Pumpkin is stopping".into());
                }
                state::set_rule(&running.server, &id, value)
            }
        }
    }

    fn refresh(&mut self) {
        if self.panicked.swap(false, Ordering::AcqRel) {
            let _ = self.stop();
            let error = "A Pumpkin worker panicked; the server has been stopped.".to_string();
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
                state::rules(&running.server),
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
mod lifecycle_tests;
