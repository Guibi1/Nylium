use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};

use crate::ui::form::ChangeEvent;

pub struct StringField {
    label: SharedString,
    state: Entity<InputState>,
}

impl StringField {
    pub fn new(
        label: SharedString,
        initial: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let state = cx.new(|cx| InputState::new(window, cx).default_value(initial));

        cx.subscribe(&state, |_, state, event, cx| match event {
            InputEvent::Change => {
                let new_value = state.read(cx).value();
                cx.emit(ChangeEvent::new_string(new_value.into()));
            }
            _ => {}
        })
        .detach();

        Self { label, state }
    }
}

impl EventEmitter<ChangeEvent> for StringField {}

impl Render for StringField {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(self.label.clone())
            .child(Input::new(&self.state))
    }
}
