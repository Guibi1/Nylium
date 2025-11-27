use gpui::*;
use gpui_component::input::{InputEvent, InputState, NumberInput, NumberInputEvent, StepAction};

use crate::ui::form::ChangeEvent;

pub struct NumberField {
    label: SharedString,
    min: Option<u32>,
    max: Option<u32>,
    state: Entity<InputState>,
}

impl NumberField {
    pub fn new(
        label: SharedString,
        initial: u32,
        min: Option<u32>,
        max: Option<u32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter number")
                .default_value(initial.to_string())
        });

        cx.subscribe(&state, |this, state, event, cx| match event {
            InputEvent::Change => {
                let text = state.read(cx).value();
                if let Ok(new_value) = text.parse() {
                    if this.min.map(|min| new_value >= min).unwrap_or(true)
                        && this.max.map(|max| new_value <= max).unwrap_or(true)
                    {
                        cx.emit(ChangeEvent::new_number(new_value));
                    }
                }
            }
            _ => {}
        })
        .detach();

        cx.subscribe_in(&state, window, {
            |this, state, event, window, cx| match event {
                NumberInputEvent::Step(step_action) => {
                    let text = state.read(cx).value();
                    if let Ok(value) = text.parse::<u32>() {
                        if this.min.map(|min| value > min).unwrap_or(true)
                            && this.max.map(|max| value < max).unwrap_or(true)
                        {
                            let new_value = match step_action {
                                StepAction::Increment => value.saturating_add(1),
                                StepAction::Decrement => value.saturating_sub(1),
                            };

                            state.update(cx, |state, cx| {
                                state.set_value(new_value.to_string(), window, cx);
                            });
                        }
                    }
                }
            }
        })
        .detach();

        Self {
            label,
            min,
            max,
            state,
        }
    }
}

impl EventEmitter<ChangeEvent> for NumberField {}

impl Render for NumberField {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(self.label.clone())
            .child(NumberInput::new(&self.state))
    }
}
