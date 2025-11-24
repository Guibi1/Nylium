use std::marker::PhantomData;

use gpui::*;
use gpui_component::Sizable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use nylium_adapter::NyliumServer;
use nylium_assets::Assets;

use crate::logger::NyliumLogger;

pub struct ConsolePage<S, C>
where
    C: Copy,
    S: NyliumServer<C>,
{
    logs_state: Entity<InputState>,
    command_state: Entity<InputState>,
    _phantoms: PhantomData<S>,
    _phantomc: PhantomData<C>,
}

impl<S, C> ConsolePage<S, C>
where
    C: Copy + 'static,
    S: NyliumServer<C>,
{
    pub fn new(logger: NyliumLogger, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let command_state = cx.new(|cx| {
            InputState::new(window, cx)
                .clean_on_escape()
                .placeholder("Enter a command...")
        });

        cx.subscribe_in(
            &command_state,
            window,
            |_this, command_state, event, window, cx| {
                if let InputEvent::PressEnter { .. } = *event {
                    let cmd = command_state.update(cx, |console_state, cx| {
                        let cmd = console_state.value();
                        if cmd.is_empty() {
                            return None;
                        }
                        console_state.set_value("", window, cx);
                        Some(cmd)
                    });
                    if let Some(command) = cmd {
                        cx.global::<S>().send_command(cx, command);
                    }
                }
            },
        )
        .detach();

        let logs_state = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line()
                .soft_wrap(true)
                .searchable(true)
        });

        let window_handle = window.window_handle();
        cx.spawn(
            move |this: WeakEntity<ConsolePage<S, C>>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    while logger.wait_for_log().await {
                        let _ = window_handle.update(&mut cx, |_, window, cx| {
                            let _ = this.update(&mut *cx, |this, cx| {
                                this.logs_state.update(cx, |logs_state, cx| {
                                    logs_state.set_value(logger.get_logs(), window, cx);
                                });
                            });
                        });
                    }
                }
            },
        )
        .detach();

        Self {
            command_state,
            logs_state,
            _phantoms: PhantomData,
            _phantomc: PhantomData,
        }
    }
}

impl<S, C> Render for ConsolePage<S, C>
where
    C: Copy + 'static,
    S: NyliumServer<C>,
{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .px_8()
            .py_4()
            .gap_2()
            .child(Input::new(&self.logs_state).disabled(true).flex_grow())
            .child(
                Input::new(&self.command_state).suffix(
                    Button::new("cmd")
                        .ghost()
                        .xsmall()
                        .icon(Assets::Enter)
                        .on_click(cx.listener(|this, _event, window, cx| {
                            let cmd = this.command_state.update(cx, |console_state, cx| {
                                let cmd = console_state.value();
                                if cmd.is_empty() {
                                    return None;
                                }
                                console_state.set_value("", window, cx);
                                Some(cmd)
                            });
                            if let Some(command) = cmd {
                                cx.global::<S>().send_command(cx, command);
                            }
                        })),
                ),
            )
    }
}
