use gpui::*;
use gpui_component::{Root, TitleBar};
use nylium_adapter::NyliumServer;
use nylium_adapter::config::NyliumConfig;
use nylium_assets::NyliumAssetSource;
use std::marker::PhantomData;

mod pages;
mod window;

use crate::window::NyliumWindow;

pub struct Nylium<S, C>
where
    C: NyliumConfig,
    S: NyliumServer<C>,
{
    server: S,
    _phantom: PhantomData<C>,
}

impl<S, C> Nylium<S, C>
where
    C: NyliumConfig,
    S: NyliumServer<C>,
{
    pub fn new(server: S) -> Self {
        Self {
            server,
            _phantom: PhantomData,
        }
    }

    pub fn run(self) {
        Application::new()
            .with_assets(NyliumAssetSource)
            .run(move |cx| {
                gpui_component::init(cx);
                cx.set_global(self.server.get_config());
                cx.set_global(self.server);

                // Update config when changed
                cx.observe_global::<C>({
                    move |cx| {
                        let server = cx.global::<S>();
                        server.update_config(cx.global::<C>());
                    }
                })
                .detach();

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
                    let view = cx.new(|cx| NyliumWindow::<S, C>::new(window, cx));
                    cx.new(|cx| Root::new(AnyView::from(view), window, cx))
                })
                .unwrap();

                // Start server
                cx.background_executor()
                    .spawn({
                        let server = cx.global::<S>().clone();
                        async move { server.start().await }
                    })
                    .detach();
            });
    }
}
