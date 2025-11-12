use std::net::SocketAddr;

use corrode_adapter::config::CorrodeConfig;
use gpui::*;
use gpui_component::form::{form_field, v_form};

#[derive(corrode_adapter::CorrodeConfig)]
pub struct Config {
    pub server_address: SocketAddr,
    pub seed: String,
    pub max_players: u32,
    pub online_mode: bool,
}

pub struct SettingsPage {
    fields: Vec<(SharedString, AnyView)>,
}

impl SettingsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.set_global(Config {
            server_address: "127.0.0.1:25565".parse().unwrap(),
            seed: "ExampleSeed".into(),
            max_players: 20,
            online_mode: false,
        });
        cx.observe_global::<Config>(|_this, cx| {
            let config = cx.global::<Config>();
            println!(
                "{} {} {} {}",
                config.server_address,
                config.seed,
                config.max_players,
                config.online_mode,
            )
        })
        .detach();

        Self {
            fields: Config::generate_form_fields(window, cx),
        }
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_form().children(
            self.fields
                .iter()
                .map(|(name, entity)| form_field().label(name.clone()).child(entity.clone())),
        )
    }
}
