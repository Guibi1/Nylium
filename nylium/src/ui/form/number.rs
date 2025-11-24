use std::marker::PhantomData;

use gpui::*;
use gpui_component::input::{InputEvent, InputState, NumberInput, NumberInputEvent, StepAction};
use nylium_adapter::NyliumServer;
use nylium_adapter::config::{ConfigValue, NumberConfigOption};

pub struct NumberField<S, C>
where
    C: Copy,
    S: NyliumServer<C>,
{
    key: C,
    min: Option<u32>,
    max: Option<u32>,
    state: Entity<InputState>,
    _phantom: PhantomData<S>,
}

impl<S, C> NumberField<S, C>
where
    C: Copy + 'static,
    S: NyliumServer<C>,
{
    pub fn new(
        option: &NumberConfigOption<C>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter number")
                .default_value(
                    cx.global::<S>()
                        .get_config_value(option.key)
                        .assert_number()
                        .to_string(),
                )
        });

        cx.subscribe(&state, |this, state, event, cx| match event {
            InputEvent::Change => {
                let text = state.read(cx).value();
                if let Ok(new_value) = text.parse() {
                    if this.min.map(|min| new_value >= min).unwrap_or(true)
                        && this.max.map(|max| new_value <= max).unwrap_or(true)
                    {
                        cx.global_mut::<S>()
                            .set_config_value(this.key, ConfigValue::Number(new_value));
                    }
                }
            }
            _ => {}
        })
        .detach();

        cx.subscribe_in(&state, window, {
            |this, state, event, window, cx| match event {
                NumberInputEvent::Step(step_action) => {
                    let new_value = match step_action {
                        StepAction::Increment => {
                            let value = cx.global::<S>().get_config_value(this.key).assert_number();
                            if this.max.map(|max| value < max).unwrap_or(true) {
                                value.saturating_add(1)
                            } else {
                                value
                            }
                        }
                        StepAction::Decrement => {
                            let value = cx.global::<S>().get_config_value(this.key).assert_number();
                            if this.min.map(|min| value > min).unwrap_or(true) {
                                value.saturating_sub(1)
                            } else {
                                value
                            }
                        }
                    };

                    cx.global_mut::<S>()
                        .set_config_value(this.key, ConfigValue::Number(new_value));
                    state.update(cx, |state, cx| {
                        state.set_value(new_value.to_string(), window, cx);
                    });
                }
            }
        })
        .detach();

        Self {
            key: option.key,
            min: option.min,
            max: option.max,
            state,
            _phantom: PhantomData,
        }
    }
}

impl<S, C> Render for NumberField<S, C>
where
    C: Copy + 'static,
    S: NyliumServer<C>,
{
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        NumberInput::new(&self.state)
    }
}
