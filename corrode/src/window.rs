use corrode_adapter::config::CorrodeConfig;
use corrode_assets::Assets;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{StyledExt, TitleBar};

use crate::pages::SettingsPage;

pub struct CorrodeWindow {
    active_tab: usize,
    settings_page: Entity<SettingsPage>,
}

impl CorrodeWindow {
    pub fn new<C>(window: &mut Window, cx: &mut Context<Self>) -> Self
    where
        C: CorrodeConfig,
    {
        let settings_page = cx.new(|cx| SettingsPage::new::<C>(window, cx));

        Self {
            active_tab: 0,
            settings_page,
        }
    }
}

impl Render for CorrodeWindow {
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
                        .child("Corrode"),
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
                this.child(
                    div()
                        .p_5()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .child("Hello, World!")
                        .child(
                            Button::new("ok")
                                .primary()
                                .label("Let's Go!")
                                .on_click(|_, _, _| println!("Clicked!")),
                        ),
                )
            })
            .when(self.active_tab == 1, |this| {
                this.child(self.settings_page.clone())
            })
    }
}
