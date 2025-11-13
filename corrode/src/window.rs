use corrode_adapter::config::CorrodeConfig;
use corrode_assets::Assets;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{StyledExt, TitleBar};

use crate::pages::SettingsPage;

pub struct CorrodeWindow {
    settings: Entity<SettingsPage>,
}

impl CorrodeWindow {
    pub fn new<C>(window: &mut Window, cx: &mut Context<Self>) -> Self
    where
        C: CorrodeConfig,
    {
        let settings = cx.new(|cx| SettingsPage::new::<C>(window, cx));
        Self { settings }
    }
}

impl Render for CorrodeWindow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
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
                    )
                    .child(self.settings.clone()),
            )
    }
}
