use gpui::*;
use gpui_component::Root;

mod pages;
mod window;

use corrode_assets::CorrodeAssetSource;
use window::CorrodeWindow;

fn main() {
    Application::new()
        .with_assets(CorrodeAssetSource)
        .run(move |cx| {
            gpui_component::init(cx);

            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|cx| CorrodeWindow::new(window, cx));
                cx.new(|cx| Root::new(view.into(), window, cx))
            })
            .unwrap();
        });
}
