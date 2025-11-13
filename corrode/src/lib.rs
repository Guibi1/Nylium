use corrode_adapter::CorrodeServer;
use corrode_adapter::config::CorrodeConfig;
use corrode_assets::CorrodeAssetSource;
use gpui::*;
use gpui_component::Root;
use std::{marker::PhantomData, sync::Arc};

mod pages;
mod window;

use crate::window::CorrodeWindow;

pub struct Corrode<S, C>
where
    C: CorrodeConfig,
    S: CorrodeServer<C> + 'static,
{
    server: Arc<S>,
    _phantom: PhantomData<C>,
}

impl<S, C> Corrode<S, C>
where
    C: CorrodeConfig,
    S: CorrodeServer<C> + 'static,
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
            .with_assets(CorrodeAssetSource)
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

                cx.open_window(WindowOptions::default(), |window, cx| {
                    let view = cx.new(|cx| CorrodeWindow::new::<C>(window, cx));
                    cx.new(|cx| Root::new(view.into(), window, cx))
                })
                .unwrap();
            });
    }
}
