use std::marker::PhantomData;

use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use nylium_adapter::NyliumServer;
use nylium_adapter::config::{ConfigValue, StringConfigOption};

pub struct StringField<S, C>
where
    C: Copy,
    S: NyliumServer<C>,
{
    key: C,
    state: Entity<InputState>,
    _phantom: PhantomData<S>,
}

impl<S, C> StringField<S, C>
where
    C: Copy + 'static,
    S: NyliumServer<C>,
{
    pub fn new(
        option: &StringConfigOption<C>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let state = cx.new(|cx| {
            InputState::new(window, cx).default_value(
                cx.global::<S>()
                    .get_config_value(option.key)
                    .assert_string(),
            )
        });

        cx.subscribe(&state, |this, state, event, cx| match event {
            InputEvent::Change => {
                let new_value = state.read(cx).value();
                cx.global_mut::<S>()
                    .set_config_value(this.key, ConfigValue::String(new_value.into()));
            }
            _ => {}
        })
        .detach();

        Self {
            key: option.key,
            state,
            _phantom: PhantomData,
        }
    }
}

impl<S, C> Render for StringField<S, C>
where
    C: Copy + 'static,
    S: NyliumServer<C>,
{
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.state)
    }
}
