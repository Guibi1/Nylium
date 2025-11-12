use gpui::*;

pub trait CorrodeConfig: Global {
    fn generate_form_fields(window: &mut Window, cx: &mut App) -> Vec<(SharedString, AnyView)>
    where
        Self: Sized;
}
