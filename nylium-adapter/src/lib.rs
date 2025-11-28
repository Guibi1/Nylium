use async_trait::async_trait;
use gpui::{App, AppContext, SharedString};

pub mod config;
mod player;

pub use crate::player::*;
pub use gpui::Global;

use crate::config::{FieldOptions, FieldValue};

#[async_trait]
pub trait NyliumServer<C: Copy, G: Copy>: Clone + Send + Global {
    async fn start(&self);
    async fn stop(&self);

    fn get_config(&self) -> Box<[FieldOptions<C>]>;
    fn get_config_value(&self, key: C) -> FieldValue;
    fn set_config_value(&self, key: C, value: FieldValue);

    fn get_gamerules(&self) -> Box<[FieldOptions<G>]>;
    fn get_gamerule_value(&self, key: G) -> FieldValue;
    fn set_gamerule_value(&self, key: G, value: FieldValue);

    async fn run_command(&self, command: &str);
    async fn get_players(&self) -> Vec<Player>;

    fn send_command(&self, cx: &App, command: impl Into<SharedString>) {
        let server = self.clone();
        let command = command.into();
        cx.background_spawn(async move { server.run_command(&command).await })
            .detach();
    }
}
