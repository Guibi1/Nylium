pub mod config;
pub mod gpui;

pub use corrode_config_derive::CorrodeConfig;
pub use corrode_ui::form as form_ui;

use crate::config::CorrodeConfig;

pub trait CorrodeServer<C>
where
    C: CorrodeConfig,
{
    fn start(&self);
    fn stop(&self);
    fn send_command(&self, command: &str);

    fn get_config(&self) -> C;
    fn update_config(&self, config: &C) -> bool;
}
