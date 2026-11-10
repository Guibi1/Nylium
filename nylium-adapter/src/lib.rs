use async_trait::async_trait;
use smol::channel::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

#[cfg(feature = "console")]
pub mod console;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Section {
    Settings,
    GameRules,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ServerSnapshot {
    pub name: String,
    pub lifecycle: Lifecycle,
    pub connected: bool,
    pub logs: Vec<LogEntry>,
    pub players: Vec<Player>,
    pub settings: Vec<Field>,
    pub game_rules: Vec<Field>,
    pub metrics: Vec<Metric>,
    pub restart_required: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Lifecycle {
    #[default]
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogEntry {
    pub sequence: u64,
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub id: uuid::Uuid,
    pub name: String,
    pub online: bool,
    pub dimension: String,
    pub operator: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Metric {
    pub elapsed_seconds: u64,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub id: String,
    pub label: String,
    pub description: String,
    pub value: FieldValue,
    pub kind: FieldKind,
}

impl Field {
    pub fn validate(&self, value: &FieldValue) -> Result<(), String> {
        match (&self.kind, value) {
            (FieldKind::Boolean, FieldValue::Boolean(_))
            | (FieldKind::Text, FieldValue::Text(_)) => Ok(()),
            (FieldKind::Number { min, max }, FieldValue::Number(value)) => {
                if min.is_some_and(|min| *value < min) || max.is_some_and(|max| *value > max) {
                    Err(format!("value for field '{}' is out of range", self.id))
                } else {
                    Ok(())
                }
            }
            _ => Err(format!("value type does not match field '{}'", self.id)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Boolean(bool),
    Number(u32),
    Text(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum FieldKind {
    Boolean,
    Number { min: Option<u32>, max: Option<u32> },
    Text,
}

/// A snapshot is authoritative; subscribe before reading it to avoid missing changes.
/// Adapters publish every state change, including log and player updates.
/// `snapshot` must not perform I/O. Operations validate inputs, persist their changes,
/// and publish the resulting state before returning `Ok`. Start and stop are idempotent.
/// Different fields may be written concurrently. Keep log and metric histories bounded.
// async-trait adds a redundant must_use attribute to boxed futures.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait ServerAdapter: Send + Sync + 'static {
    fn subscribe(&self) -> Receiver<()>;
    fn snapshot(&self) -> Arc<ServerSnapshot>;
    async fn start(&self) -> Result<(), String>;
    async fn stop(&self) -> Result<(), String>;
    async fn command(&self, command: String) -> Result<(), String>;
    async fn set_field(
        &self,
        section: Section,
        id: String,
        value: FieldValue,
    ) -> Result<(), String>;
}

struct PublishedState {
    snapshot: Arc<ServerSnapshot>,
    subscribers: Vec<Sender<()>>,
}

/// Holds an authoritative snapshot and broadcasts coalesced, nonblocking invalidations.
pub struct StatePublisher(Mutex<PublishedState>);

impl StatePublisher {
    pub fn new(snapshot: ServerSnapshot) -> Self {
        Self(Mutex::new(PublishedState {
            snapshot: Arc::new(snapshot),
            subscribers: Vec::new(),
        }))
    }

    pub fn snapshot(&self) -> Arc<ServerSnapshot> {
        self.0
            .lock()
            .expect("state mutex poisoned")
            .snapshot
            .clone()
    }

    /// Register before calling `snapshot`; notifications only indicate that a fresh snapshot is available.
    pub fn subscribe(&self) -> Receiver<()> {
        let (sender, receiver) = channel::bounded(1);
        self.0
            .lock()
            .expect("state mutex poisoned")
            .subscribers
            .push(sender);
        receiver
    }

    pub fn update<R>(&self, update: impl FnOnce(&mut ServerSnapshot) -> R) -> R {
        let mut state = self.0.lock().expect("state mutex poisoned");
        let result = update(Arc::make_mut(&mut state.snapshot));
        state
            .subscribers
            .retain(|sender| match sender.try_send(()) {
                Ok(()) | Err(channel::TrySendError::Full(())) => true,
                Err(channel::TrySendError::Closed(())) => false,
            });
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number_field() -> Field {
        Field {
            id: "limit".into(),
            label: "Limit".into(),
            description: String::new(),
            value: FieldValue::Number(5),
            kind: FieldKind::Number {
                min: Some(1),
                max: Some(10),
            },
        }
    }

    #[test]
    fn validates_field_type_and_number_range() {
        let field = number_field();
        assert!(field.validate(&FieldValue::Number(1)).is_ok());
        assert!(field.validate(&FieldValue::Number(10)).is_ok());
        assert!(field.validate(&FieldValue::Number(0)).is_err());
        assert!(field.validate(&FieldValue::Number(11)).is_err());
        assert!(field.validate(&FieldValue::Text("wrong".into())).is_err());
    }

    #[test]
    fn subscribers_are_independent_coalesced_and_read_current_state() {
        let publisher = StatePublisher::new(ServerSnapshot::default());
        let first = publisher.subscribe();
        let second = publisher.subscribe();
        publisher.update(|state| state.name = "one".into());
        publisher.update(|state| state.name = "latest".into());
        assert_eq!(publisher.snapshot().name, "latest");
        assert_eq!(first.try_recv(), Ok(()));
        assert_eq!(first.try_recv(), Err(channel::TryRecvError::Empty));
        assert_eq!(second.try_recv(), Ok(()));
        drop(first);
        publisher.update(|state| state.connected = true);
        assert_eq!(second.try_recv(), Ok(()));
        assert!(publisher.snapshot().connected);
    }
}
