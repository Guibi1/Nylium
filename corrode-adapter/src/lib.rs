use std::sync::Arc;

pub mod config;
pub mod gpui;

pub use corrode_config_derive::CorrodeConfig;
pub use corrode_ui::form as form_ui;

pub trait CorrodeServer<C>
where
    C: Sized + Send + Sync + 'static,
{
    fn start(&self);
    fn stop(&self);
    fn send_command(&self, command: &str);

    fn get_config(&self) -> Arc<C>;
}
