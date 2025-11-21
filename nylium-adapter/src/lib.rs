use async_trait::async_trait;
use gpui::{App, AppContext, SharedString};

pub mod config;
pub mod gpui;

pub use gpui::Global;
pub use nylium_config_derive::NyliumConfig;
pub use nylium_shared::form as form_ui;
pub use nylium_shared::objects::Player;

use crate::config::NyliumConfig;

#[async_trait]
pub trait NyliumServer<C>: Clone + Send + Global
where
    C: NyliumConfig,
{
    async fn start(&self);
    async fn stop(&self);

    fn get_config(&self) -> C;
    fn update_config(&self, config: &C) -> bool;

    async fn run_command(&self, command: &str);
    async fn get_players(&self) -> Vec<Player>;

    fn send_command(&self, cx: &App, command: impl Into<SharedString>) {
        let server = self.clone();
        let command = command.into();
        cx.background_spawn(async move { server.run_command(&command).await })
            .detach();
    }
}
