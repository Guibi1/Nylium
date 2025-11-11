use gpui::*;
use gpui_component::Root;

mod window;

use corrode_assets::CorrodeAssetSource;
use window::CorrodeWindow;

fn main() {
    Application::new()
        .with_assets(CorrodeAssetSource)
        .run(move |cx| {
            gpui_component::init(cx);

            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| CorrodeWindow);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view.into(), window, cx))
            })
            .unwrap();
        });
}
