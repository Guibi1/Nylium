use std::marker::PhantomData;

use gpui::*;
use gpui_component::switch::Switch;
use nylium_adapter::{
    NyliumServer,
    config::{BooleanConfigOption, ConfigValue},
};

pub struct BooleanField<S, C>
where
    C: Copy,
    S: NyliumServer<C>,
{
    key: C,
    id: SharedString,
    _phantom: PhantomData<S>,
}

impl<S, C> BooleanField<S, C>
where
    C: Copy,
    S: NyliumServer<C>,
{
    pub fn new(option: &BooleanConfigOption<C>) -> Self {
        Self {
            key: option.key,
            id: option.id.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S, C> Render for BooleanField<S, C>
where
    C: Copy + 'static,
    S: NyliumServer<C>,
{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Switch::new(self.id.clone())
            .checked(cx.global::<S>().get_config_value(self.key).assert_bool())
            .on_click(cx.listener(move |this, checked, _window, cx| {
                cx.global_mut::<S>()
                    .set_config_value(this.key, ConfigValue::Boolean(*checked));
            }))
    }
}
