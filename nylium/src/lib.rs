//! Native Minecraft server administration with Ely. Backends publish state; views observe it.
use std::sync::Arc;

use gpui::{prelude::*, *};
use nylium_adapter::{ServerAdapter, ServerSnapshot};

pub mod demo;
mod field;
mod http;
mod window;

pub use ely;
pub use gpui;
pub use nylium_adapter;

/// A backend may register any GPUI view, not just a generated settings page.
pub trait UiAdapter: ServerAdapter {
    fn pages(&self, _window: &mut Window, _cx: &mut App) -> Vec<AdapterPage> {
        Vec::new()
    }
}

type PageUpdate = dyn Fn(Arc<ServerSnapshot>, &mut Window, &mut App);

/// Stable ids must be unique and must not use the reserved `nylium.` prefix.
pub struct AdapterPage {
    pub id: String,
    pub label: String,
    view: AnyView,
    update: Box<PageUpdate>,
}

impl AdapterPage {
    /// `update` runs initially and on every backend invalidation, even while this page is hidden.
    /// A page may also subscribe to its own adapter-specific event streams.
    pub fn new<V: Render + 'static>(
        id: impl Into<String>,
        label: impl Into<String>,
        view: Entity<V>,
        update: impl Fn(&mut V, Arc<ServerSnapshot>, &mut Window, &mut Context<V>) + 'static,
    ) -> Self {
        let target = view.clone();
        Self {
            id: id.into(),
            label: label.into(),
            view: view.into(),
            update: Box::new(move |snapshot, window, cx| {
                target.update(cx, |view, cx| {
                    update(view, snapshot, window, cx);
                    cx.notify();
                });
            }),
        }
    }
}

pub struct Nylium;

impl Nylium {
    /// Starts the backend with the window and stops it before closing.
    pub fn run(adapter: impl UiAdapter) {
        let adapter = Arc::new(adapter);
        gpui_platform::application()
            .with_http_client(Arc::new(http::ImageHttpClient))
            .with_assets(ely::Assets)
            .run(move |cx| {
                ely::init(cx).expect("Ely assets and fonts must load");
                let options = WindowOptions {
                    titlebar: Some(TitlebarOptions {
                        title: Some("Nylium".into()),
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(1100.), px(760.)),
                        cx,
                    ))),
                    window_min_size: Some(size(px(720.), px(480.))),
                    ..Default::default()
                };
                let shutdown = adapter.clone();
                cx.on_app_quit(move |_| {
                    let shutdown = shutdown.clone();
                    async move {
                        if let Err(error) = shutdown.stop().await {
                            eprintln!("Cannot stop server: {error}");
                        }
                    }
                })
                .detach();
                let backend: Arc<dyn ServerAdapter> = adapter.clone();
                let window = cx.open_window(options, move |window, cx| {
                    let pages = adapter.pages(window, cx);
                    cx.new(|cx| window::ServerWindow::new(backend, pages, window, cx))
                });
                if let Err(error) = window {
                    eprintln!("Cannot open Nylium: {error}");
                    cx.quit();
                }
                cx.on_window_closed(|cx, _| {
                    if cx.windows().is_empty() {
                        cx.quit();
                    }
                })
                .detach();
                cx.activate(true);
            });
    }
}
