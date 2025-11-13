use std::rc::Rc;
use std::str::FromStr;

use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};

pub struct StringField<C: Global, T: FromStr + ToString + 'static> {
    set: Rc<dyn Fn(T, &mut C)>,
    state: Entity<InputState>,
}

impl<C, T> StringField<C, T>
where
    C: Global,
    T: FromStr + ToString + 'static,
{
    pub fn new<G, S>(get: G, set: S, window: &mut Window, cx: &mut Context<Self>) -> Self
    where
        G: Fn(&C) -> T + 'static,
        S: Fn(T, &mut C) + 'static,
    {
        let state = cx.new(|cx| {
            InputState::new(window, cx).default_value((get)(cx.global::<C>()).to_string())
        });

        cx.subscribe(&state, |this, state, event, cx| match event {
            InputEvent::Change => {
                let text = state.read(cx).value();
                if let Ok(value) = text.parse::<T>() {
                    (this.set)(value, cx.global_mut::<C>());
                }
            }
            _ => {}
        })
        .detach();

        StringField {
            set: Rc::new(set),
            state,
        }
    }
}

impl<C, T> Render for StringField<C, T>
where
    C: Global,
    T: FromStr + ToString + 'static,
{
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.state)
    }
}
