use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{StyledExt, TitleBar};
use nylium_adapter::NyliumServer;
use nylium_assets::Assets;

use crate::logger::NyliumLogger;
use crate::ui::pages::{ConsolePage, GameRulesPage, PlayersPage, SettingsPage};

pub struct NyliumWindow<S, C, G>
where
    C: Copy,
    G: Copy,
    S: NyliumServer<C, G> + 'static,
{
    active_tab: usize,
    console_page: Entity<ConsolePage<S, C, G>>,
    players_page: Entity<PlayersPage<S, C, G>>,
    game_rules_page: Entity<GameRulesPage<S, C, G>>,
    settings_page: Entity<SettingsPage<S, C, G>>,
}

impl<S, C, G> NyliumWindow<S, C, G>
where
    C: Copy + 'static,
    G: Copy + 'static,
    S: NyliumServer<C, G> + 'static,
{
    pub fn new(logger: NyliumLogger, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            active_tab: 0,
            console_page: cx.new(|cx| ConsolePage::new(logger, window, cx)),
            players_page: cx.new(|cx| PlayersPage::new(window, cx)),
            game_rules_page: cx.new(|cx| GameRulesPage::new(window, cx)),
            settings_page: cx.new(|cx| SettingsPage::new(window, cx)),
        }
    }
}

impl<S, C, G> Render for NyliumWindow<S, C, G>
where
    C: Copy + 'static,
    G: Copy + 'static,
    S: NyliumServer<C, G> + 'static,
{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                TitleBar::new().child(
                    div()
                        .h_flex()
                        .w_full()
                        .pr_2()
                        .gap_2()
                        .child(Assets::Cuboid)
                        .child("Nylium"),
                ),
            )
            .child(
                TabBar::new("segmented-tabs")
                    .underline()
                    .selected_index(self.active_tab)
                    .px_4()
                    .on_click(cx.listener(|this, index, _, cx| {
                        this.active_tab = *index;
                        cx.notify();
                    }))
                    .child(Tab::new().label("Console").prefix(Assets::Terminal).px_2())
                    .child(Tab::new().label("Players").prefix(Assets::Users).px_2())
                    .child(
                        Tab::new()
                            .label("Game rules")
                            .prefix(Assets::Settings)
                            .px_2(),
                    )
                    .child(Tab::new().label("Settings").prefix(Assets::Gear).px_2()),
            )
            .when(self.active_tab == 0, |this| {
                this.child(self.console_page.clone())
            })
            .when(self.active_tab == 1, |this| {
                this.child(self.players_page.clone())
            })
            .when(self.active_tab == 2, |this| {
                this.child(self.game_rules_page.clone())
            })
            .when(self.active_tab == 3, |this| {
                this.child(self.settings_page.clone())
            })
    }
}
