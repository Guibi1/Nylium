use gpui::*;

pub trait NyliumConfig: Global {
    fn generate_form_fields(window: &mut Window, cx: &mut App) -> Vec<(SharedString, AnyView)>
    where
        Self: Sized;
}
