use async_trait::async_trait;
use gpui::{App, AppContext, SharedString};

pub mod config;
mod player;

pub use crate::player::Player;
pub use gpui::Global;

use crate::config::{ConfigOptions, ConfigValue};

#[async_trait]
pub trait NyliumServer<C: Copy>: Clone + Send + Global {
    async fn start(&self);
    async fn stop(&self);

    fn get_config(&self) -> Box<[ConfigOptions<C>]>;
    fn get_config_value(&self, key: C) -> ConfigValue;
    fn set_config_value(&self, key: C, value: ConfigValue);

    async fn run_command(&self, command: &str);
    async fn get_players(&self) -> Vec<Player>;

    fn send_command(&self, cx: &App, command: impl Into<SharedString>) {
        let server = self.clone();
        let command = command.into();
        cx.background_spawn(async move { server.run_command(&command).await })
            .detach();
    }
}
