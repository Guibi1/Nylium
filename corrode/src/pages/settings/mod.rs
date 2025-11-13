use corrode_adapter::config::CorrodeConfig;
use gpui::*;
use gpui_component::form::{form_field, v_form};

pub struct SettingsPage {
    fields: Vec<(SharedString, AnyView)>,
}

impl SettingsPage {
    pub fn new<C>(window: &mut Window, cx: &mut Context<Self>) -> Self
    where
        C: CorrodeConfig,
    {
        Self {
            fields: C::generate_form_fields(window, cx),
        }
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().px_8().py_4().child(
            v_form().children(
                self.fields
                    .iter()
                    .map(|(name, entity)| form_field().label(name.clone()).child(entity.clone())),
            ),
        )
    }
}
