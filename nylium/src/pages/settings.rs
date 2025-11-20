use gpui::*;
use gpui_component::StyledExt;
use gpui_component::form::{field, v_form};
use nylium_adapter::config::NyliumConfig;

pub struct SettingsPage {
    fields: Vec<(SharedString, AnyView)>,
}

impl SettingsPage {
    pub fn new<C>(window: &mut Window, cx: &mut Context<Self>) -> Self
    where
        C: NyliumConfig,
    {
        Self {
            fields: C::generate_form_fields(window, cx),
        }
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().gap_2().overflow_hidden().child(
            div().px_8().py_4().scrollable(Axis::Vertical).child(
                v_form().children(
                    self.fields
                        .iter()
                        .map(|(name, entity)| field().label(name.clone()).child(entity.clone())),
                ),
            ),
        )
    }
}
