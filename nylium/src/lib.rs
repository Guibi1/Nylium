use std::marker::PhantomData;
use std::sync::Arc;

use gpui::*;
use gpui_component::{Root, TitleBar};
use nylium_adapter::NyliumServer;
use nylium_assets::NyliumAssetSource;

mod http_client;
mod logger;
mod ui;

use crate::http_client::HttpClient;
use crate::ui::NyliumWindow;

pub use crate::logger::NyliumLogger;

pub struct Nylium<S, C, G>
where
    C: Copy,
    G: Copy,
    S: NyliumServer<C, G>,
{
    server: S,
    logger: NyliumLogger,
    _phantomc: PhantomData<C>,
    _phantomg: PhantomData<G>,
}

impl<S, C, G> Nylium<S, C, G>
where
    C: Copy + 'static,
    G: Copy + 'static,
    S: NyliumServer<C, G>,
{
    pub fn new(server: S, logger: NyliumLogger) -> Self {
        Self {
            server,
            logger,
            _phantomc: PhantomData,
            _phantomg: PhantomData,
        }
    }

    pub fn run(self) {
        Application::new()
            .with_assets(NyliumAssetSource)
            .with_http_client(Arc::new(HttpClient::new()))
            .run(move |cx| {
                gpui_component::init(cx);
                cx.set_global(self.server);

                // Stop server when closing Nylium
                cx.on_app_quit(|cx| {
                    let server = cx.global::<S>().clone();
                    async move { server.stop().await }
                })
                .detach();

                // Open Nylium window
                let window_options = WindowOptions {
                    titlebar: Some(TitleBar::title_bar_options()),
                    window_bounds: Some(WindowBounds::centered(size(px(800.), px(500.)), cx)),
                    ..Default::default()
                };
                cx.open_window(window_options, |window, cx| {
                    let view = cx.new(|cx| NyliumWindow::<S, C, G>::new(self.logger, window, cx));
                    cx.new(|cx| Root::new(AnyView::from(view), window, cx))
                })
                .unwrap();

                // Start server
                let server = cx.global::<S>().clone();
                cx.background_executor()
                    .spawn(async move { server.start().await })
                    .detach();
            });
    }
}
