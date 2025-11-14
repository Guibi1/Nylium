use gpui::Global;

pub mod config;
pub mod gpui;

pub use nylium_config_derive::NyliumConfig;
pub use nylium_ui::form as form_ui;

use crate::config::NyliumConfig;

pub trait NyliumServer<C>: Global
where
    C: NyliumConfig,
{
    fn start(&self);
    fn stop(&self);
    fn send_command(&self, command: &str);

    fn get_config(&self) -> C;
    fn update_config(&self, config: &C) -> bool;
}
