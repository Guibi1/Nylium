use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use ely::{
    buttons::{Button, ButtonVariant},
    data_display::Avatar,
    feedback::{ConnectionStatus, SaveState, SavingIndicator, SyncState, SyncStatus},
    forms::{Choice, Input, InputEvent, TextInput},
    navigation::Tabs,
    primitives::FocusScope,
    shell::{Connectivity, TitleBar},
    theme::{ActiveTheme, Mode, Theme},
};
use ely_charts::{AreaChart, Series};
use gpui::{prelude::*, *};
use nylium_adapter::{Lifecycle, LogLevel, Section, ServerAdapter, ServerSnapshot};

use crate::{AdapterPage, field::FieldEditor};

const BUILTINS: [(&str, &str); 5] = [
    ("nylium.console", "Console"),
    ("nylium.statistics", "Statistics"),
    ("nylium.players", "Players"),
    ("nylium.rules", "Game rules"),
    ("nylium.settings", "Settings"),
];

pub(crate) struct ServerWindow {
    adapter: Arc<dyn ServerAdapter>,
    snapshot: Arc<ServerSnapshot>,
    pages: Vec<AdapterPage>,
    active: String,
    fields: HashMap<(Section, String), Entity<FieldEditor>>,
    command: Entity<TextInput>,
    search: Entity<TextInput>,
    focus: FocusHandle,
    error: Option<String>,
    busy: bool,
    closing: bool,
    commands: usize,
    stream_closed: bool,
    follow_logs: bool,
    logs_scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
    _stream: Task<()>,
}

impl ServerWindow {
    pub fn new(
        adapter: Arc<dyn ServerAdapter>,
        pages: Vec<AdapterPage>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut ids = HashSet::new();
        let mut error = None;
        let pages = pages
            .into_iter()
            .filter(|page| {
                let valid = !page.id.is_empty()
                    && !page.id.starts_with("nylium.")
                    && ids.insert(page.id.clone());
                if !valid {
                    error = Some(format!("Invalid or duplicate adapter page id: {}", page.id));
                }
                valid
            })
            .collect();
        let events = adapter.subscribe();
        let snapshot = adapter.snapshot();
        let command = cx.new(|cx| {
            TextInput::new(window, cx)
                .label("Server command")
                .placeholder("Enter a command…")
        });
        let search = cx.new(|cx| {
            TextInput::new(window, cx)
                .label("Filter logs")
                .placeholder("Filter logs…")
        });
        let subscriptions = vec![
            cx.subscribe(&command, |this, _, event, cx| {
                if *event == InputEvent::Submit {
                    this.send_command(cx);
                }
            }),
            cx.subscribe(&search, |_, _, _, cx| cx.notify()),
        ];
        let handle = window.window_handle();
        let stream = cx.spawn(async move |this, cx| {
            while events.recv().await.is_ok() {
                if handle
                    .update(cx, |_, window, cx| {
                        this.update(cx, |this, cx| this.refresh(window, cx))
                    })
                    .is_err()
                {
                    return;
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.stream_closed = true;
                this.error = Some("The adapter stopped publishing updates.".into());
                cx.notify();
            });
        });
        let mut this = Self {
            adapter,
            snapshot,
            pages,
            active: BUILTINS[0].0.into(),
            fields: HashMap::new(),
            command,
            search,
            focus: cx.focus_handle(),
            error,
            busy: false,
            closing: false,
            commands: 0,
            stream_closed: false,
            follow_logs: true,
            logs_scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
            _stream: stream,
        };
        this.refresh(window, cx);
        this.lifecycle(true, cx);
        this
    }

    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let snapshot = self.adapter.snapshot();
        for section in [Section::Settings, Section::GameRules] {
            let fields = match section {
                Section::Settings => &snapshot.settings,
                Section::GameRules => &snapshot.game_rules,
            };
            self.fields.retain(|(s, id), editor| {
                *s != section || fields.iter().any(|field| &field.id == id) || {
                    let mut keep = false;
                    editor.update(cx, |editor, cx| {
                        keep = editor.mark_removed(cx);
                    });
                    keep
                }
            });
            for field in fields {
                let key = (section, field.id.clone());
                if let Some(editor) = self.fields.get(&key) {
                    editor.update(cx, |editor, cx| editor.reconcile(field.clone(), cx));
                } else {
                    let editor = cx.new(|cx| {
                        FieldEditor::new(field.clone(), section, self.adapter.clone(), window, cx)
                    });
                    let subscription = cx.observe(&editor, |_, _, cx| cx.notify());
                    // Dropping a removed field drops its observer too.
                    subscription.detach();
                    self.fields.insert(key, editor);
                }
            }
        }
        for page in &self.pages {
            (page.update)(snapshot.clone(), window, cx);
        }
        if self.follow_logs && self.snapshot.logs.last() != snapshot.logs.last() {
            self.logs_scroll.scroll_to_bottom();
        }
        self.snapshot = snapshot;
        cx.notify();
    }

    fn lifecycle(&mut self, start: bool, cx: &mut Context<Self>) {
        if self.busy || self.closing {
            return;
        }
        if !start && (self.commands > 0 || self.fields.values().any(|field| field.read(cx).saving))
        {
            self.error = Some("Wait for commands and pending saves before stopping.".into());
            cx.notify();
            return;
        }
        self.busy = true;
        self.error = None;
        let adapter = self.adapter.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    if start {
                        adapter.start().await
                    } else {
                        adapter.stop().await
                    }
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                this.error = result.err();
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn send_command(&mut self, cx: &mut Context<Self>) {
        let command = self.command.read(cx).text().trim().to_string();
        if command.is_empty()
            || self.busy
            || self.closing
            || self.commands > 0
            || self.snapshot.lifecycle != Lifecycle::Running
        {
            return;
        }
        self.execute(command, true, cx);
    }

    fn execute(&mut self, command: String, from_input: bool, cx: &mut Context<Self>) {
        let adapter = self.adapter.clone();
        if self.closing || self.busy || self.commands > 0 {
            return;
        }
        let submitted = command.clone();
        self.commands += 1;
        self.error = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { adapter.command(command).await })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.commands -= 1;
                if result.is_ok() && from_input && this.command.read(cx).text().trim() == submitted
                {
                    this.command.update(cx, |input, cx| input.set_text("", cx));
                }
                this.error = result.err();
                cx.notify();
            });
        })
        .detach();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.closing {
            return;
        }
        if self.busy || self.commands > 0 || self.fields.values().any(|field| field.read(cx).saving)
        {
            self.error =
                Some("Wait for the current operation and pending saves before closing.".into());
            cx.notify();
            return;
        }
        if self.fields.values().any(|field| {
            let field = field.read(cx);
            field.error.is_some() || (field.removed && !field.discarded)
        }) {
            self.error = Some(
                "Some settings could not be saved. Correct them or retry before closing.".into(),
            );
            cx.notify();
            return;
        }
        self.closing = true;
        let handle = window.window_handle();
        let adapter = self.adapter.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { adapter.stop().await })
                .await;
            let _ = handle.update(cx, |_, window, cx| {
                let _ = this.update(cx, |this, cx| match result {
                    Ok(()) => window.remove_window(),
                    Err(error) => {
                        this.closing = false;
                        this.error = Some(error);
                        cx.notify();
                    }
                });
            });
        })
        .detach();
        cx.notify();
    }

    fn console(&self, cx: &mut Context<Self>) -> AnyElement {
        let filter = self.search.read(cx).text().to_lowercase();
        let colors = &cx.theme().colors;
        let mut logs = div()
            .id("logs")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.logs_scroll)
            .font_family("JetBrains Mono")
            .text_sm()
            .p_4()
            .bg(colors.sunken)
            .rounded_lg();
        let entries: Vec<_> = self
            .snapshot
            .logs
            .iter()
            .filter(|entry| filter.is_empty() || entry.message.to_lowercase().contains(&filter))
            .collect();
        if entries.is_empty() {
            logs = logs.child("No matching logs.");
        }
        for entry in entries {
            logs = logs.child(
                div()
                    .id(ElementId::Name(SharedString::from(format!(
                        "log-{}",
                        entry.sequence
                    ))))
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .text_color(colors.fg_muted)
                            .child(entry.timestamp.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(match entry.level {
                                LogLevel::Info => colors.fg,
                                LogLevel::Warn => colors.warning,
                                LogLevel::Error => colors.danger,
                            })
                            .child(entry.message.clone()),
                    ),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_3()
            .size_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().flex_1().child(Input::new(&self.search).clearable()))
                    .child(
                        Button::new(
                            "follow",
                            if self.follow_logs {
                                "Following logs"
                            } else {
                                "Follow logs"
                            },
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.follow_logs = !this.follow_logs;
                            if this.follow_logs {
                                this.logs_scroll.scroll_to_bottom();
                            }
                            cx.notify();
                        })),
                    ),
            )
            .child(logs)
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.command)))
                    .child(
                        Button::new("send", "Send")
                            .primary()
                            .disabled(
                                self.snapshot.lifecycle != Lifecycle::Running
                                    || self.busy
                                    || self.commands > 0
                                    || self.closing,
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.send_command(cx))),
                    ),
            )
            .into_any_element()
    }

    fn statistics(&self, cx: &App) -> AnyElement {
        let metrics: Vec<_> = self
            .snapshot
            .metrics
            .iter()
            .rev()
            .take(120)
            .rev()
            .filter(|metric| (0.0..=100.0).contains(&metric.cpu_percent))
            .collect();
        let labels: Vec<_> = metrics
            .iter()
            .map(|metric| format!("{}s", metric.elapsed_seconds))
            .collect();
        let mut page = div()
            .id("stats")
            .overflow_y_scroll()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .child("Process statistics");
        if let Some(metric) = metrics.last() {
            page = page.child(
                div()
                    .flex()
                    .gap_6()
                    .text_xl()
                    .child(format!("CPU {:.1}%", metric.cpu_percent))
                    .child(format!(
                        "Memory {:.1} MB",
                        metric.memory_bytes as f64 / 1_000_000.
                    )),
            );
        } else {
            page = page.child("Waiting for the first measurement…");
        }
        if !metrics.is_empty() {
            page = page
                .child(
                    AreaChart::new("cpu", labels.clone())
                        .series(Series::new(
                            "CPU (%)",
                            metrics.iter().map(|m| m.cpu_percent as f64),
                        ))
                        .h(px(210.)),
                )
                .child(
                    AreaChart::new("memory", labels)
                        .series(Series::new(
                            "Memory (MB)",
                            metrics.iter().map(|m| m.memory_bytes as f64 / 1_000_000.),
                        ))
                        .h(px(210.)),
                );
        }
        page.child(
            div()
                .text_sm()
                .text_color(cx.theme().colors.fg_muted)
                .child("Metrics are supplied by the server adapter."),
        )
        .into_any_element()
    }

    fn players(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = &cx.theme().colors;
        let mut players: Vec<_> = self.snapshot.players.iter().collect();
        players.sort_by_key(|p| (!p.online, p.name.to_lowercase()));
        let mut page = div()
            .id("players")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2()
            .child(format!(
                "{} connected · {} known players",
                players.iter().filter(|p| p.online).count(),
                players.len()
            ));
        if players.is_empty() {
            page = page.child("No players yet.");
        }
        for player in players {
            let id = player.id;
            let mut actions = div().flex().gap_2().child(
                Button::new(SharedString::from(format!("copy-{id}")), "Copy UUID")
                    .variant(ButtonVariant::Ghost)
                    .on_click(move |_, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(id.to_string()))
                    }),
            );
            for (verb, label) in [
                (
                    if player.operator { "deop" } else { "op" },
                    if player.operator { "De-op" } else { "OP" },
                ),
                ("kick", "Kick"),
                ("ban", "Ban"),
            ] {
                let command = format!("{verb} {id}");
                actions = actions.child(
                    Button::new(SharedString::from(format!("{verb}-{id}")), label)
                        .variant(if verb == "ban" {
                            ButtonVariant::Danger
                        } else {
                            ButtonVariant::Secondary
                        })
                        .disabled(
                            (verb == "kick" && !player.online)
                                || self.commands > 0
                                || self.busy
                                || self.snapshot.lifecycle != Lifecycle::Running
                                || self.closing,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.execute(command.clone(), false, cx)
                        })),
                );
            }
            page = page.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .items_center()
                    .p_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(colors.border)
                    .child(
                        Avatar::new(
                            SharedString::from(format!("avatar-{id}")),
                            player.name.clone(),
                        )
                        .image(format!("https://api.mineatar.io/face/{id}"))
                        .square(),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(120.))
                            .child(format!(
                                "{}{}",
                                player.name,
                                if player.operator { " · OP" } else { "" }
                            ))
                            .child(div().text_sm().text_color(colors.fg_muted).child(
                                if player.online {
                                    player.dimension.clone()
                                } else {
                                    "Offline".into()
                                },
                            )),
                    )
                    .child(actions),
            );
        }
        page.into_any_element()
    }

    fn settings(&self, section: Section, cx: &mut Context<Self>) -> AnyElement {
        let fields = match section {
            Section::Settings => &self.snapshot.settings,
            Section::GameRules => &self.snapshot.game_rules,
        };
        let editors: Vec<_> = self
            .fields
            .iter()
            .filter(|((s, _), editor)| *s == section && !editor.read(cx).discarded)
            .map(|(_, editor)| editor.clone())
            .collect();
        let saving = editors.iter().filter(|f| f.read(cx).saving).count();
        let failed = editors.iter().any(|f| {
            let field = f.read(cx);
            field.error.is_some() || field.removed
        });
        let retry_fields: Vec<_> = editors
            .iter()
            .filter(|f| !f.read(cx).removed)
            .cloned()
            .collect();
        div().size_full().flex().flex_col().gap_3()
            .child(div().flex().flex_wrap().justify_between().gap_3().items_center()
                .child(div().text_xl().child(if section == Section::Settings { "Server settings" } else { "Game rules" }))
                .child(div().flex().flex_wrap().gap_4().items_center()
                    .child(SavingIndicator::new("saving", if failed { SaveState::Failed } else if saving > 0 { SaveState::Saving } else { SaveState::Saved }).on_retry(move |_, cx| { for field in &retry_fields { field.update(cx, |field, cx| field.retry(cx)); } }))
                    .child(SyncStatus::new("sync", if self.stream_closed || failed { SyncState::Failed } else if saving > 0 { SyncState::Syncing(saving) } else { SyncState::Synced }))
                    .child(ConnectionStatus::new("connection", if self.snapshot.connected && !self.stream_closed { Connectivity::Online } else { Connectivity::Offline }))))
            .when(section == Section::Settings, |page| page.child(div().text_sm().text_color(cx.theme().colors.fg_muted)
                .child(if self.snapshot.restart_required { "Changes saved. Restart the server to apply them." } else { "Changes save automatically. Server settings take effect after a restart." })))
            .child(div().id("settings-fields").overflow_y_scroll().flex_1().min_h_0()
                .when(fields.is_empty(), |page| page.child("The adapter has no fields in this section."))
                .children(fields.iter().filter_map(|f| self.fields.get(&(section, f.id.clone())).cloned()))
                .children(self.fields.iter().filter(|((s, id), editor)| *s == section && !fields.iter().any(|f| &f.id == id) && !editor.read(cx).discarded).map(|(_, editor)| editor.clone())))
            .into_any_element()
    }
}

impl Render for ServerWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors.clone();
        let weak = cx.entity().downgrade();
        let tabs = BUILTINS
            .iter()
            .copied()
            .chain(
                self.pages
                    .iter()
                    .map(|page| (page.id.as_str(), page.label.as_str())),
            )
            .map(|(id, label)| Choice::new(id.to_string(), label.to_string()));
        let nav = Tabs::new("pages", tabs, self.active.clone()).on_change(cx.listener(
            |this, id: &SharedString, _, cx| {
                this.active = id.to_string();
                cx.notify();
            },
        ));
        let page = match self.active.as_str() {
            "nylium.console" => self.console(cx),
            "nylium.statistics" => self.statistics(cx),
            "nylium.players" => self.players(cx),
            "nylium.rules" => self.settings(Section::GameRules, cx),
            "nylium.settings" => self.settings(Section::Settings, cx),
            id => self
                .pages
                .iter()
                .find(|page| page.id == id)
                .map(|page| page.view.clone().into_any_element())
                .unwrap_or_else(|| div().child("Page unavailable").into_any_element()),
        };
        let lifecycle = match &self.snapshot.lifecycle {
            Lifecycle::Stopped => "Stopped".into(),
            Lifecycle::Starting => "Starting…".into(),
            Lifecycle::Running => "Running".into(),
            Lifecycle::Stopping => "Stopping…".into(),
            Lifecycle::Failed(error) => format!("Failed: {error}"),
        };
        FocusScope::new(&self.focus).root().size_full().child(
            div()
                .size_full()
                .flex()
                .flex_col()
                .bg(colors.bg)
                .text_color(colors.fg)
                .font_family("IBM Plex Sans")
                .child(
                    TitleBar::new("titlebar")
                        .title(format!("Nylium · {}", self.snapshot.name))
                        .on_close(move |window, cx| {
                            let _ = weak.update(cx, |this, cx| this.close(window, cx));
                        })
                        .action(
                            Button::new("theme", "Theme")
                                .variant(ButtonVariant::Ghost)
                                .on_click(|_, _, cx| {
                                    let mode = if cx.theme().mode() == Mode::Dark {
                                        Mode::Light
                                    } else {
                                        Mode::Dark
                                    };
                                    Theme::set_mode(mode, cx);
                                }),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_3()
                        .px_4()
                        .py_2()
                        .child(div().flex_1().child(lifecycle))
                        .child(
                            Button::new("start", "Start")
                                .disabled(
                                    self.busy
                                        || self.closing
                                        || !matches!(
                                            self.snapshot.lifecycle,
                                            Lifecycle::Stopped | Lifecycle::Failed(_)
                                        ),
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.lifecycle(true, cx))),
                        )
                        .child(
                            Button::new("stop", "Stop")
                                .disabled(
                                    self.busy
                                        || self.closing
                                        || self.snapshot.lifecycle != Lifecycle::Running,
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.lifecycle(false, cx))),
                        ),
                )
                .child(nav)
                .when_some(self.error.clone(), |root, error| {
                    root.child(
                        div()
                            .flex()
                            .gap_3()
                            .items_center()
                            .px_4()
                            .py_2()
                            .text_color(colors.danger)
                            .child(div().flex_1().child(error))
                            .child(
                                Button::new("dismiss", "Dismiss")
                                    .variant(ButtonVariant::Ghost)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.error = None;
                                        cx.notify();
                                    })),
                            ),
                    )
                })
                .child(div().flex_1().min_h_0().p_4().child(page)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{AdapterPage, ServerWindow};
    use crate::demo::DemoServer;
    use gpui::{
        AppContext, Context, IntoElement, ParentElement, Render, TestAppContext, Window, div,
    };
    use nylium_adapter::{FieldValue, Lifecycle, Section, ServerAdapter};
    use std::sync::Arc;

    struct Probe {
        calls: usize,
        players_online: usize,
        logs: usize,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().child("Adapter page")
        }
    }

    #[gpui::test]
    fn external_changes_update_builtin_and_hidden_adapter_pages(cx: &mut TestAppContext) {
        cx.update(|cx| {
            ely::theme::Theme::init(cx);
            ely::forms::bind_keys(cx);
        });
        let backend = Arc::new(DemoServer::new());
        let source = backend.clone();
        let (view, cx) = cx.add_window_view(move |window, cx| {
            let probe = cx.new(|_| Probe {
                calls: 0,
                players_online: 0,
                logs: 0,
            });
            let page = AdapterPage::new("demo.probe", "Probe", probe, |probe, state, _, _| {
                probe.calls += 1;
                probe.players_online = state.players.iter().filter(|p| p.online).count();
                probe.logs = state.logs.len();
            });
            ServerWindow::new(source, vec![page], window, cx)
        });
        cx.run_until_parked();
        assert_eq!(
            view.read_with(cx, |view, _| view.snapshot.lifecycle.clone()),
            Lifecycle::Running
        );
        smol::block_on(async {
            backend.command("kick Steve".into()).await.unwrap();
            backend.command("say external change".into()).await.unwrap();
            backend
                .set_field(
                    Section::Settings,
                    "max_players".into(),
                    FieldValue::Number(42),
                )
                .await
                .unwrap();
            backend
                .set_field(
                    Section::GameRules,
                    "keep_inventory".into(),
                    FieldValue::Boolean(true),
                )
                .await
                .unwrap();
        });
        cx.run_until_parked();
        view.read_with(cx, |view, cx| {
            assert_eq!(view.active, "nylium.console");
            assert!(
                !view
                    .snapshot
                    .players
                    .iter()
                    .find(|p| p.name == "Steve")
                    .unwrap()
                    .online
            );
            assert!(
                view.snapshot
                    .logs
                    .iter()
                    .any(|entry| entry.message.contains("external change"))
            );
            assert_eq!(
                view.fields[&(Section::Settings, "max_players".into())]
                    .read(cx)
                    .field
                    .value,
                FieldValue::Number(42)
            );
            assert_eq!(
                view.fields[&(Section::GameRules, "keep_inventory".into())]
                    .read(cx)
                    .field
                    .value,
                FieldValue::Boolean(true)
            );
            let probe = view.pages[0].view.clone().downcast::<Probe>().unwrap();
            assert!(probe.read(cx).calls >= 2);
            assert_eq!(probe.read(cx).players_online, 1);
            assert_eq!(probe.read(cx).logs, view.snapshot.logs.len());
        });
        smol::block_on(backend.stop()).unwrap();
    }
    #[gpui::test]
    fn command_failure_keeps_text_and_success_clears_it(cx: &mut TestAppContext) {
        cx.update(|cx| {
            ely::theme::Theme::init(cx);
            ely::forms::bind_keys(cx);
        });
        let backend = Arc::new(DemoServer::new());
        let source = backend.clone();
        let (view, cx) =
            cx.add_window_view(move |window, cx| ServerWindow::new(source, Vec::new(), window, cx));
        cx.run_until_parked();
        view.update(cx, |view, cx| {
            view.command
                .update(cx, |input, cx| input.set_text("unknown", cx));
            view.send_command(cx);
        });
        cx.run_until_parked();
        view.read_with(cx, |view, cx| {
            assert!(view.error.is_some());
            assert_eq!(view.command.read(cx).text(), "unknown");
            assert_eq!(view.commands, 0);
        });
        view.update(cx, |view, cx| {
            view.command
                .update(cx, |input, cx| input.set_text("say hello", cx));
            view.send_command(cx);
        });
        cx.run_until_parked();
        view.read_with(cx, |view, cx| {
            assert!(view.error.is_none());
            assert_eq!(view.command.read(cx).text(), "");
        });
        smol::block_on(backend.stop()).unwrap();
    }
}
