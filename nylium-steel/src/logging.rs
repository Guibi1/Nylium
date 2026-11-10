use nylium_adapter::{LogEntry, LogLevel, StatePublisher};
use std::{fmt, sync::Arc};
use tracing::{
    Event, Subscriber,
    field::{Field, Visit},
};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{Layer, layer::Context};

/// Captures Steel's tracing/log events without installing or replacing a global subscriber.
#[derive(Clone)]
pub struct SteelLogLayer {
    state: Arc<StatePublisher>,
}
impl SteelLogLayer {
    pub(crate) fn new(state: Arc<StatePublisher>) -> Self {
        Self { state }
    }
}

struct Message(String);
impl Visit for Message {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name().starts_with("log.") {
            return;
        }
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        if field.name() != "message" {
            self.0.push_str(field.name());
            self.0.push('=');
        }
        self.0.push_str(&format!("{value:?}"));
    }
}
impl<S: Subscriber> Layer<S> for SteelLogLayer {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let normalized = event.normalized_metadata();
        let metadata = normalized.as_ref().unwrap_or_else(|| event.metadata());
        if !metadata.target().starts_with("steel") {
            return;
        }
        let mut message = Message(String::new());
        event.record(&mut message);
        let level = match *metadata.level() {
            tracing::Level::ERROR => LogLevel::Error,
            tracing::Level::WARN => LogLevel::Warn,
            _ => LogLevel::Info,
        };
        append(&self.state, level, message.0);
    }
}

pub(crate) fn append(state: &StatePublisher, level: LogLevel, message: String) {
    state.update(|state| {
        let sequence = state
            .logs
            .last()
            .map_or(1, |entry| entry.sequence.saturating_add(1));
        state.logs.push(LogEntry {
            sequence,
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            level,
            message: strip_ansi_escapes::strip_str(message),
        });
        if state.logs.len() > 1000 {
            state.logs.remove(0);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::prelude::*;
    #[test]
    fn captures_steel_events_and_bounds_history() {
        tracing_log::LogTracer::init().unwrap();
        let state = Arc::new(StatePublisher::new(Default::default()));
        let subscriber = tracing_subscriber::registry().with(SteelLogLayer::new(state.clone()));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "steel_core::server", "Hello Steel");
            tracing::info!(target: "gpui", "Ignore this");
            log::warn!(target: "steel_core::command", "Legacy feedback");
        });
        assert_eq!(state.snapshot().logs.len(), 2);
        assert!(state.snapshot().logs[1].message.contains("Legacy feedback"));
        assert_eq!(state.snapshot().logs[1].level, LogLevel::Warn);
        assert!(state.snapshot().logs[0].message.contains("Hello Steel"));
        for i in 0..1001 {
            append(&state, LogLevel::Info, i.to_string());
        }
        let logs = state.snapshot();
        assert_eq!(logs.logs.len(), 1000);
        assert_eq!(logs.logs.last().unwrap().sequence, 1003);
    }
}
