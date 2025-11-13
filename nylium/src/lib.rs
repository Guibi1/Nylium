use gpui::*;
use gpui_component::{Root, TitleBar};
use nylium_adapter::NyliumServer;
use nylium_adapter::config::NyliumConfig;
use nylium_assets::NyliumAssetSource;
use std::{marker::PhantomData, sync::Arc};

mod pages;
mod window;

use crate::window::NyliumWindow;

pub struct Nylium<S, C>
where
    C: NyliumConfig,
    S: NyliumServer<C> + 'static,
{
    server: Arc<S>,
    _phantom: PhantomData<C>,
}

impl<S, C> Nylium<S, C>
where
    C: NyliumConfig,
    S: NyliumServer<C> + 'static,
{
    pub fn new(server: S) -> Self {
        Self {
            server: Arc::new(server),
            _phantom: PhantomData,
        }
    }

    pub fn run(self) {
        let server = self.server.clone();

        Application::new()
            .with_assets(NyliumAssetSource)
            .run(move |cx| {
                gpui_component::init(cx);
                cx.set_global(server.get_config());
                cx.observe_global::<C>({
                    let server = server.clone();
                    move |cx| {
                        server.update_config(cx.global());
                    }
                })
                .detach();

                server.start();

                let window_options = WindowOptions {
                    titlebar: Some(TitleBar::title_bar_options()),
                    window_bounds: Some(WindowBounds::centered(size(px(800.), px(500.)), cx)),
                    ..Default::default()
                };
                cx.open_window(window_options, |window, cx| {
                    let view = cx.new(|cx| NyliumWindow::new::<C>(window, cx));
                    cx.new(|cx| Root::new(view.into(), window, cx))
                })
                .unwrap();
            });
    }
}
