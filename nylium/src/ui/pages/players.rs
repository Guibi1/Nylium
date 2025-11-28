use std::marker::PhantomData;
use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::label::Label;
use gpui_component::menu::{DropdownMenu, PopupMenu};
use gpui_component::scroll::{Scrollbar, ScrollbarState};
use gpui_component::skeleton::Skeleton;
use gpui_component::spinner::Spinner;
use gpui_component::{ActiveTheme, Sizable, VirtualListScrollHandle, v_virtual_list};
use nylium_adapter::{NyliumServer, Player};
use nylium_assets::Assets;

actions!(player, [CopyUuid, Op, Kick, Ban]);

pub struct PlayersPage<S, C, G>
where
    C: Copy,
    G: Copy,
    S: NyliumServer<C, G>,
{
    players: Option<Rc<Vec<Player>>>,
    element_heights: Rc<Vec<Size<Pixels>>>,
    scroll_handle: VirtualListScrollHandle,
    scroll_state: ScrollbarState,
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
            element_heights: Rc::new(Vec::new()),
            scroll_handle: VirtualListScrollHandle::new(),
            scroll_state: ScrollbarState::default(),
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
        let element_heights = self.element_heights.clone();

        div()
            .flex_grow()
            .px_4()
            .pt_4()
            .relative()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .items_center()
                    .child(Label::new("Players").text_xl())
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
            .when_some(self.players.clone(), |this, players| {
                this.child(
                    v_virtual_list(
                        cx.entity().clone(),
                        "players_list",
                        element_heights,
                        move |_, range, _, cx| {
                            range
                                .filter_map(|i| {
                                    players.get(i).map(|player| {
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
                                                    cx.write_to_clipboard(
                                                        ClipboardItem::new_string(id.to_string()),
                                                    );
                                                }
                                            })
                                            .on_action::<Op>({
                                                let name = player.name.clone();
                                                move |_, _, cx| {
                                                    cx.global::<S>()
                                                        .send_command(cx, format!("op {}", name));
                                                }
                                            })
                                            .on_action::<Kick>({
                                                let name = player.name.clone();
                                                move |_, _, cx| {
                                                    cx.global::<S>()
                                                        .send_command(cx, format!("kick {}", name));
                                                }
                                            })
                                            .on_action::<Ban>({
                                                let name = player.name.clone();
                                                move |_, _, cx| {
                                                    cx.global::<S>()
                                                        .send_command(cx, format!("ban {}", name));
                                                }
                                            })
                                            .child(
                                                div()
                                                    .relative()
                                                    .size_6()
                                                    .overflow_hidden()
                                                    .child(
                                                        Skeleton::new()
                                                            .absolute()
                                                            .inset_0()
                                                            .size_full(),
                                                    )
                                                    .child(
                                                        img(format!(
                                                            "https://api.mineatar.io/face/{}",
                                                            player.id
                                                        ))
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
                                                    .when(!player.online, |this| {
                                                        this.child(
                                                            Label::new("Offline")
                                                                .text_xs()
                                                                .text_color(
                                                                    cx.theme().muted_foreground,
                                                                ),
                                                        )
                                                    }),
                                            )
                                            .child(
                                                Button::new(player.id)
                                                    .icon(Assets::Ellipsis)
                                                    .with_variant(ButtonVariant::Link)
                                                    .dropdown_menu({
                                                        let online = player.online;
                                                        move |menu, _, _| {
                                                            create_player_menu(menu, online)
                                                        }
                                                    }),
                                            )
                                    })
                                })
                                .collect()
                        },
                    )
                    .track_scroll(&self.scroll_handle)
                    .px_4(),
                )
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .child(Scrollbar::vertical(&self.scroll_state, &self.scroll_handle)),
                )
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

        let _ = this.update(cx, |this, _cx| {
            if this
                .players
                .as_ref()
                .is_none_or(|p| p.len() != players.len())
            {
                let mut element_heights = vec![Size::new(px(0.), px(38.)); players.len()];
                element_heights.push(Size::new(px(0.), px(8.)));
                this.element_heights = Rc::new(element_heights);
            }
            this.players = Some(Rc::new(players));
        });
    })
    .detach();
}

fn create_player_menu(menu: PopupMenu, online: bool) -> PopupMenu {
    menu.menu_with_icon("Copy Uuid", Assets::Copy, Box::new(CopyUuid))
        .menu_with_icon_and_disabled("OP", Assets::Crown, Box::new(Op), !online)
        .separator()
        .menu_with_icon_and_disabled("Kick", Assets::Logout, Box::new(Kick), !online)
        .menu_with_icon_and_disabled("Ban", Assets::Ban, Box::new(Ban), !online)
}
