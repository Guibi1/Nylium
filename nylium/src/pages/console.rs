use std::io::BufRead;
use std::marker::PhantomData;
use std::time::Duration;

use gpui::*;
use gpui_component::Sizable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use nylium_adapter::NyliumServer;
use nylium_adapter::config::NyliumConfig;
use nylium_assets::Assets;

pub struct ConsolePage<S, C>
where
    C: NyliumConfig,
    S: NyliumServer<C> + 'static,
{
    logs_state: Entity<InputState>,
    command_state: Entity<InputState>,
    _phantoms: PhantomData<S>,
    _phantomc: PhantomData<C>,
}

impl<S, C> ConsolePage<S, C>
where
    C: NyliumConfig,
    S: NyliumServer<C> + 'static,
{
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        redirect_stdout(cx.entity(), window, cx).detach();

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
                    if let Some(cmd) = cmd {
                        cx.global::<S>().send_command(&cmd);
                    }
                }
            },
        )
        .detach();

        Self {
            command_state,
            logs_state: cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line()
                    .soft_wrap(true)
                    .searchable(true)
            }),
            _phantoms: PhantomData,
            _phantomc: PhantomData,
        }
    }
}

impl<S, C> Render for ConsolePage<S, C>
where
    C: NyliumConfig,
    S: NyliumServer<C> + 'static,
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
                            if let Some(cmd) = cmd {
                                cx.global::<S>().send_command(&cmd);
                            }
                        })),
                ),
            )
    }
}

fn redirect_stdout<S, C>(page: Entity<ConsolePage<S, C>>, window: &mut Window, cx: &App) -> Task<()>
where
    C: NyliumConfig,
    S: NyliumServer<C> + 'static,
{
    let window_handle = window.window_handle();
    let (tx, rx) = smol::channel::bounded::<String>(3);

    cx.spawn(move |cx: &mut AsyncApp| {
        let mut cx = cx.clone();
        async move {
            while let Ok(line) = rx.recv().await {
                window_handle
                    .update(&mut cx, |_, window, cx| {
                        page.update(&mut *cx, |page, cx| {
                            page.logs_state.update(cx, |logs_state, cx| {
                                let logs = logs_state.value();
                                logs_state.set_value(logs.to_string() + &line, window, cx);
                            });
                        });
                    })
                    .unwrap();
            }
        }
    })
    .detach();

    cx.background_spawn(async move {
        let redirect = gag::BufferRedirect::stdout().unwrap();
        let mut reader = std::io::BufReader::new(redirect);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(len) => {
                    if len == 0 {
                        smol::Timer::after(Duration::from_millis(10)).await;
                    } else {
                        let _ = tx.send(line.clone()).await;
                    }
                }
                Err(_) => break,
            }
        }
    })
}
