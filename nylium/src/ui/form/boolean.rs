use gpui::*;
use gpui_component::switch::Switch;

use crate::ui::form::ChangeEvent;

pub struct BooleanField {
    label: SharedString,
    id: SharedString,
    value: bool,
}

impl BooleanField {
    pub fn new(label: SharedString, initial: bool, id: SharedString) -> Self {
        Self {
            label,
            id,
            value: initial,
        }
    }
}

impl EventEmitter<ChangeEvent> for BooleanField {}

impl Render for BooleanField {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(self.label.clone())
            .child(
                Switch::new(self.id.clone())
                    .checked(self.value)
                    .on_click(cx.listener(move |this, checked, _window, cx| {
                        cx.emit(ChangeEvent::new_bool(*checked));
                        this.value = *checked;
                    })),
            )
    }
}
