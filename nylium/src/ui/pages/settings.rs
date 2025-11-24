use std::marker::PhantomData;

use gpui::*;
use gpui_component::StyledExt;
use gpui_component::form::{field, v_form};
use nylium_adapter::NyliumServer;
use nylium_adapter::config::ConfigOptions;

use crate::ui::form::{BooleanField, NumberField, StringField};

pub struct SettingsPage<S, C>
where
    C: Copy,
    S: NyliumServer<C>,
{
    fields: Box<[(AnyView, SharedString)]>,
    _phantomc: PhantomData<C>,
    _phantoms: PhantomData<S>,
}

impl<S, C> SettingsPage<S, C>
where
    C: Copy + 'static,
    S: NyliumServer<C>,
{
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            fields: cx
                .global::<S>()
                .get_config()
                .into_iter()
                .map(|option| match option {
                    ConfigOptions::Boolean(option) => (
                        cx.new(|_| BooleanField::<S, C>::new(&option)).into(),
                        option.label,
                    ),
                    ConfigOptions::Number(option) => (
                        cx.new(|cx| NumberField::<S, C>::new(&option, window, cx))
                            .into(),
                        option.label,
                    ),
                    ConfigOptions::String(option) => (
                        cx.new(|cx| StringField::<S, C>::new(&option, window, cx))
                            .into(),
                        option.label,
                    ),
                })
                .collect(),
            _phantomc: PhantomData,
            _phantoms: PhantomData,
        }
    }
}

impl<S, C> Render for SettingsPage<S, C>
where
    C: Copy + 'static,
    S: NyliumServer<C>,
{
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().gap_2().overflow_hidden().child(
            div().px_8().py_4().scrollable(Axis::Vertical).child(
                v_form().children(
                    self.fields
                        .iter()
                        .map(|(entity, label)| field().label(label.clone()).child(entity.clone())),
                ),
            ),
        )
    }
}
