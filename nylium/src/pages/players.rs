use std::marker::PhantomData;

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::menu::{ContextMenuExt, DropdownMenu, PopupMenu};
use gpui_component::{ActiveTheme, StyledExt};
use nylium_adapter::NyliumServer;
use nylium_adapter::config::NyliumConfig;
use nylium_assets::Assets;
use nylium_shared::objects::Player;

actions!(
    player_options,
    [CopyPlayer, OpPlayer, KickPlayer, BanPlayer]
);

pub struct PlayersPage<S, C>
where
    C: NyliumConfig,
    S: NyliumServer<C> + 'static,
{
    players: Option<Vec<Player>>,
    _phantoms: PhantomData<S>,
    _phantomc: PhantomData<C>,
}

impl<S, C> PlayersPage<S, C>
where
    C: NyliumConfig,
    S: NyliumServer<C> + 'static,
{
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let server = cx.global::<S>().clone();
        cx.spawn(async move |this, cx| {
            let players = cx
                .background_spawn(async move { server.get_players().await })
                .await;
            let _ = this.update(cx, |this, _cx| this.players = Some(players));
        })
        .detach();

        Self {
            players: None,
            _phantoms: PhantomData,
            _phantomc: PhantomData,
        }
    }
}

impl<S, C> Render for PlayersPage<S, C>
where
    C: NyliumConfig,
    S: NyliumServer<C> + 'static,
{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .px_4()
            .py_4()
            .gap_2()
            .scrollable(Axis::Vertical)
            .when_none(&self.players, |this| this.child(div().child("Loading")))
            .when_some(self.players.as_ref(), |this, players| {
                this.children(players.iter().map(|player| {
                    div()
                        .w_full()
                        .flex()
                        .gap_2()
                        .py_2()
                        .px_4()
                        .items_center()
                        .context_menu(create_player_menu)
                        .child(
                            img(format!("https://api.mineatar.io/face/{}", player.id))
                                .size_6()
                                .bg(cx.theme().muted),
                        )
                        .child(div().flex_grow().child(SharedString::from(&player.name)))
                        .child(
                            Button::new("btn")
                                .icon(Assets::Ellipsis)
                                .with_variant(ButtonVariant::Ghost)
                                .dropdown_menu(create_player_menu),
                        )
                }))
            })
    }
}

fn create_player_menu(
    menu: PopupMenu,
    _window: &mut Window,
    _cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    menu.menu("Copy name", Box::new(CopyPlayer))
        .menu("Kick", Box::new(KickPlayer))
        .separator()
        .menu("OP", Box::new(OpPlayer))
        .menu("Ban", Box::new(BanPlayer))
}
