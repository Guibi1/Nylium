use std::marker::PhantomData;

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::label::Label;
use gpui_component::menu::{ContextMenuExt, DropdownMenu, PopupMenu};
use gpui_component::skeleton::Skeleton;
use gpui_component::spinner::Spinner;
use gpui_component::{ActiveTheme, Sizable, StyledExt};
use nylium_adapter::{NyliumServer, Player};
use nylium_assets::Assets;

actions!(player, [CopyUuid, Op, Kick, Ban]);

pub struct PlayersPage<S, C, G>
where
    C: Copy,
    G: Copy,
    S: NyliumServer<C, G>,
{
    players: Option<Vec<Player>>,
    _phantoms: PhantomData<S>,
    _phantomc: PhantomData<C>,
    _phantomg: PhantomData<G>,
}

impl<S, C, G> PlayersPage<S, C, G>
where
    C: Copy + 'static,
    G: Copy + 'static,
    S: NyliumServer<C, G>,
{
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        load_players(cx);

        Self {
            players: None,
            _phantoms: PhantomData,
            _phantomc: PhantomData,
            _phantomg: PhantomData,
        }
    }
}

impl<S, C, G> Render for PlayersPage<S, C, G>
where
    C: Copy + 'static,
    G: Copy + 'static,
    S: NyliumServer<C, G>,
{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .scrollable(Axis::Vertical)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .items_center()
                    .child(Label::new("Player list").text_xl())
                    .child(
                        Button::new("reload")
                            .icon(Assets::Rotate)
                            .ghost()
                            .on_click(cx.listener(|_, _, _, cx| load_players(cx))),
                    ),
            )
            .when_none(&self.players, |this| {
                this.items_center()
                    .justify_center()
                    .child(Spinner::new().large())
            })
            .when_some(self.players.as_ref(), |this, players| {
                this.children(players.iter().map(|player| {
                    div()
                        .w_full()
                        .flex_grow()
                        .py_0p5()
                        .px_4()
                        .flex()
                        .gap_2()
                        .items_center()
                        .hover(|this| this.bg(cx.theme().muted))
                        .on_action::<CopyUuid>({
                            let id = player.id;
                            move |_, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(id.to_string()));
                            }
                        })
                        .on_action::<Op>({
                            let name = player.name.clone();
                            move |_, _, cx| {
                                cx.global::<S>().send_command(cx, format!("op {}", name));
                            }
                        })
                        .on_action::<Kick>({
                            let name = player.name.clone();
                            move |_, _, cx| {
                                cx.global::<S>().send_command(cx, format!("kick {}", name));
                            }
                        })
                        .on_action::<Ban>({
                            let name = player.name.clone();
                            move |_, _, cx| {
                                cx.global::<S>().send_command(cx, format!("ban {}", name));
                            }
                        })
                        .context_menu(create_player_menu)
                        .child(
                            div()
                                .relative()
                                .size_6()
                                .overflow_hidden()
                                .child(Skeleton::new().absolute().inset_0().size_full())
                                .child(
                                    img(format!("https://api.mineatar.io/face/{}", player.id))
                                        .absolute()
                                        .inset_0()
                                        .size_full(),
                                ),
                        )
                        .child(
                            div()
                                .flex_grow()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .child(Label::new(&player.name))
                                .when(player.online, |this| {
                                    this.child(
                                        Label::new("Offline")
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                }),
                        )
                        .child(
                            Button::new("btn")
                                .icon(Assets::Ellipsis)
                                .with_variant(ButtonVariant::Link)
                                .dropdown_menu(create_player_menu),
                        )
                }))
            })
    }
}

fn load_players<S, C, G>(cx: &mut Context<PlayersPage<S, C, G>>)
where
    C: Copy + 'static,
    G: Copy + 'static,
    S: NyliumServer<C, G>,
{
    cx.spawn(async move |this, cx| {
        let server = cx.read_global::<S, _>(|s, _| s.clone()).unwrap();
        let players = cx
            .background_spawn(async move { server.get_players().await })
            .await;
        let _ = this.update(cx, |this, _cx| this.players = Some(players));
    })
    .detach();
}

fn create_player_menu(
    menu: PopupMenu,
    _window: &mut Window,
    _cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    menu.menu("Copy Uuid", Box::new(CopyUuid))
        .menu("OP", Box::new(Op))
        .separator()
        .menu("Kick", Box::new(Kick))
        .menu("Ban", Box::new(Ban))
}
