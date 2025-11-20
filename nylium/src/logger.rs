use chrono::Local;
use env_logger::Logger;
use log::{LevelFilter, Log, Metadata, Record};
use smol::channel::{Receiver, Sender};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct NyliumLogger {
    inner: Arc<Logger>,
    logs: Arc<Mutex<String>>,
    tx: Sender<()>,
    rx: Receiver<()>,
}

impl NyliumLogger {
    pub fn init() -> NyliumLogger {
        let (tx, rx) = smol::channel::bounded::<()>(1);
        let logger = Self {
            inner: Arc::new(env_logger::builder().build()),
            logs: Arc::new(Mutex::new(String::new())),
            tx,
            rx,
        };
        log::set_logger(Box::leak(Box::new(logger.clone()))).unwrap();
        log::set_max_level(LevelFilter::Info);
        logger
    }

    pub async fn wait_for_log(&self) -> bool {
        self.rx.recv().await.is_ok()
    }

    pub fn get_logs(&self) -> String {
        self.logs.lock().unwrap().clone()
    }

    pub fn clear_logs(&self) {
        self.logs.lock().unwrap().clear();
    }
}

impl Log for NyliumLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= LevelFilter::Info
    }

    fn log(&self, record: &Record) {
        self.inner.log(record);

        if !self.enabled(record.metadata()) {
            return;
        }

        if ["gpui::", "blade_graphics::", "naga::"]
            .iter()
            .any(|t| record.target().starts_with(t))
        {
            return;
        }

        let timestamp = Local::now().format("%H:%M:%S");
        let thread_name = std::thread::current()
            .name()
            .unwrap_or("unknown")
            .to_string();
        let level = record.level();
        let message = record.args();

        let formatted = format!("[{}] [{}/{}]: {}\n", timestamp, thread_name, level, message);

        if let Ok(mut logs) = self.logs.lock() {
            logs.push_str(&formatted);
            let _ = self.tx.try_send(());
        }
    }

    fn flush(&self) {}
}
