use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{StyledExt, TitleBar};
use nylium_adapter::config::NyliumConfig;
use nylium_assets::Assets;

use crate::pages::{ConsolePage, SettingsPage};

pub struct NyliumWindow {
    active_tab: usize,
    console_page: Entity<ConsolePage>,
    settings_page: Entity<SettingsPage>,
}

impl NyliumWindow {
    pub fn new<C>(window: &mut Window, cx: &mut Context<Self>) -> Self
    where
        C: NyliumConfig,
    {
        Self {
            active_tab: 0,
            console_page: cx.new(|cx| ConsolePage::new::<C>(window, cx)),
            settings_page: cx.new(|cx| SettingsPage::new::<C>(window, cx)),
        }
    }
}

impl Render for NyliumWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
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
                    .child(Tab::new("Console").prefix(Assets::Terminal).px_2())
                    .child(Tab::new("Settings").prefix(Assets::Settings).px_2()),
            )
            .when(self.active_tab == 0, |this| {
                this.child(self.console_page.clone())
            })
            .when(self.active_tab == 1, |this| {
                this.child(self.settings_page.clone())
            })
    }
}
