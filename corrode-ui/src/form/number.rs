use std::rc::Rc;
use std::str::FromStr;

use gpui::*;
use gpui_component::input::{InputEvent, InputState, NumberInput, NumberInputEvent, StepAction};
use num_traits::{Num, SaturatingAdd, SaturatingSub};

pub struct NumberField<
    C: Global,
    T: Num + SaturatingAdd + SaturatingSub + FromStr + ToString + Copy + 'static,
> {
    get: Rc<dyn Fn(&C) -> T>,
    set: Rc<dyn Fn(T, &mut C)>,
    state: Entity<InputState>,
}

impl<C, T> NumberField<C, T>
where
    C: Global,
    T: Num + SaturatingAdd + SaturatingSub + FromStr + ToString + Copy + 'static,
{
    pub fn new<G, S>(get: G, set: S, window: &mut Window, cx: &mut Context<Self>) -> Self
    where
        G: Fn(&C) -> T + 'static,
        S: Fn(T, &mut C) + 'static,
    {
        let state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter number")
                .default_value(get(&cx.global::<C>()).to_string())
        });

        cx.subscribe(&state, |view, state, event, cx| match event {
            InputEvent::Change => {
                let text = state.read(cx).value();
                if let Ok(new_value) = text.parse() {
                    (view.set)(new_value, cx.global_mut::<C>());
                }
            }
            _ => {}
        })
        .detach();

        cx.subscribe_in(&state, window, {
            |view, state, event, window, cx| match event {
                NumberInputEvent::Step(step_action) => match step_action {
                    StepAction::Increment => {
                        let new_value = (view.get)(cx.global()).saturating_add(&T::one());
                        (view.set)(new_value, cx.global_mut());
                        state.update(cx, |state, cx| {
                            state.set_value(new_value.to_string(), window, cx);
                        });
                    }
                    StepAction::Decrement => {
                        let new_value = (view.get)(cx.global()).saturating_sub(&T::one());
                        (view.set)(new_value, cx.global_mut());
                        state.update(cx, |state, cx| {
                            state.set_value(new_value.to_string(), window, cx);
                        });
                    }
                },
            }
        })
        .detach();

        NumberField {
            get: Rc::new(get),
            set: Rc::new(set),
            state,
        }
    }
}

impl<C, T> Render for NumberField<C, T>
where
    C: Global,
    T: Num + SaturatingAdd + SaturatingSub + FromStr + ToString + Copy + 'static,
{
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        NumberInput::new(&self.state)
    }
}
