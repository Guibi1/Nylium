use nylium_adapter::config::NyliumConfig;
use nylium_assets::Assets;
use gpui::*;
use gpui_component::Sizable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};

pub struct ConsolePage {
    logs_state: Entity<InputState>,
    console_state: Entity<InputState>,
}

impl ConsolePage {
    pub fn new<C>(window: &mut Window, cx: &mut Context<Self>) -> Self
    where
        C: NyliumConfig,
    {
        Self {
            logs_state: cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line()
                    .code_editor("logs")
                    .searchable(true)
                    .default_value(
                        "05:59:33 [INFO] Starting Steel Server\n\
05:59:33 [INFO] Vanilla registry loaded in 7.179925ms\n\
05:59:33 [INFO] Started Steel Server",
                    )
            }),
            console_state: cx
                .new(|cx| InputState::new(window, cx).placeholder("Enter a command...")),
        }
    }
}

impl Render for ConsolePage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .px_8()
            .py_4()
            .gap_2()
            .child(Input::new(&self.logs_state).disabled(true).flex_grow())
            .child(
                Input::new(&self.console_state)
                    .suffix(Button::new("info").ghost().xsmall().icon(Assets::Enter)),
            )
    }
}
