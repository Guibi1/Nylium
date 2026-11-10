use std::{sync::Arc, time::Duration};

use ely::{
    buttons::{Button, ButtonVariant},
    feedback::{SaveState, SavingIndicator},
    forms::{Input, InputEvent, Switch, TextInput},
    theme::ActiveTheme,
};
use gpui::{prelude::*, *};
use nylium_adapter::{Field, FieldKind, FieldValue, Section, ServerAdapter};

/// A field keeps its draft while a write is pending. Writes to one field are serialized.
pub(crate) struct FieldEditor {
    pub field: Field,
    section: Section,
    adapter: Arc<dyn ServerAdapter>,
    input: Entity<TextInput>,
    draft: FieldValue,
    pending: Option<FieldValue>,
    pub saving: bool,
    pub error: Option<String>,
    pub removed: bool,
    pub discarded: bool,
    invalid: bool,
    _subscription: Subscription,
    _save: Option<Task<()>>,
}

impl FieldEditor {
    pub fn new(
        field: Field,
        section: Section,
        adapter: Arc<dyn ServerAdapter>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| {
            let mut input = TextInput::new(window, cx).label(field.label.clone());
            input.set_text(display(&field.value), cx);
            input
        });
        let subscription = cx.subscribe(&input, |this, input, event, cx| {
            if *event != InputEvent::Changed {
                return;
            }
            let text = input.read(cx).text();
            // Programmatic reconciliation emits the same event as typing.
            if text == display(&this.field.value) && !this.saving && this.pending.is_none() {
                this.invalid = false;
                this.error = None;
                cx.notify();
                return;
            }
            let value = match this.field.kind {
                FieldKind::Number { .. } => text
                    .parse::<u32>()
                    .map(FieldValue::Number)
                    .map_err(|_| "Enter a whole number from 0 to 4294967295".to_string()),
                _ => Ok(FieldValue::Text(text.to_string())),
            };
            match value.and_then(|value| this.field.validate(&value).map(|_| value)) {
                Ok(value) => this.queue(value, cx),
                Err(error) => {
                    this.pending = None;
                    this.invalid = true;
                    this.error = Some(error);
                    cx.notify();
                }
            }
        });
        Self {
            draft: field.value.clone(),
            field,
            section,
            adapter,
            input,
            pending: None,
            saving: false,
            error: None,
            removed: false,
            discarded: false,
            invalid: false,
            _subscription: subscription,
            _save: None,
        }
    }

    pub fn reconcile(&mut self, field: Field, cx: &mut Context<Self>) {
        self.removed = false;
        self.discarded = false;
        self.field = field;
        if !self.saving && !self.invalid && self.error.is_none() && self.pending.is_none() {
            self.draft = self.field.value.clone();
            let text = display(&self.field.value);
            if self.input.read(cx).text() != text {
                self.input.update(cx, |input, cx| input.set_text(text, cx));
            }
        }
        cx.notify();
    }

    pub fn mark_removed(&mut self, cx: &mut Context<Self>) -> bool {
        self.removed = true;
        let unresolved = !self.discarded
            && (self.draft != self.field.value
                || self.pending.is_some()
                || self.saving
                || self.invalid
                || self.error.is_some());
        if unresolved && self.error.is_none() {
            self.error = Some("This field was removed while it had unsaved changes.".into());
        }
        cx.notify();
        unresolved
    }

    pub fn retry(&mut self, cx: &mut Context<Self>) {
        if self.error.is_some() && !self.invalid && !self.saving {
            self.queue(self.draft.clone(), cx);
        }
    }

    fn discard(&mut self, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        self.error = None;
        self.invalid = false;
        let state = self.adapter.snapshot();
        let fields = match self.section {
            Section::Settings => &state.settings,
            Section::GameRules => &state.game_rules,
        };
        if let Some(field) = fields.iter().find(|f| f.id == self.field.id) {
            self.reconcile(field.clone(), cx);
        } else {
            self.error = None;
            self.invalid = false;
            self.discarded = true;
            cx.notify();
        }
    }

    fn queue(&mut self, value: FieldValue, cx: &mut Context<Self>) {
        self.draft = value.clone();
        self.pending = Some(value);
        self.error = None;
        self.invalid = false;
        if self.saving {
            cx.notify();
            return;
        }
        self.saving = true;
        let adapter = self.adapter.clone();
        let section = self.section;
        let id = self.field.id.clone();
        self._save = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(300))
                    .await;
                let Ok(value) = this.update(cx, |this, _| this.pending.take()) else {
                    break;
                };
                let Some(value) = value else {
                    let _ = this.update(cx, |this, cx| {
                        this.saving = false;
                        cx.notify();
                    });
                    break;
                };
                let task = cx.background_spawn({
                    let adapter = adapter.clone();
                    let id = id.clone();
                    async move { adapter.set_field(section, id, value).await }
                });
                let result = task.await;
                let Ok(again) = this.update(cx, |this, cx| {
                    if this.pending.is_none() {
                        if !this.invalid {
                            this.error = result.err();
                        }
                        this.saving = false;
                        if this.error.is_none() {
                            let snapshot = adapter.snapshot();
                            let fields = match section {
                                Section::Settings => &snapshot.settings,
                                Section::GameRules => &snapshot.game_rules,
                            };
                            if let Some(field) = fields.iter().find(|field| field.id == id) {
                                this.reconcile(field.clone(), cx);
                            }
                        }
                    }
                    cx.notify();
                    this.pending.is_some()
                }) else {
                    break;
                };
                if !again {
                    break;
                }
            }
        }));
        cx.notify();
    }
}

pub(crate) fn display(value: &FieldValue) -> String {
    match value {
        FieldValue::Boolean(value) => value.to_string(),
        FieldValue::Number(value) => value.to_string(),
        FieldValue::Text(value) => value.clone(),
    }
}

impl Render for FieldEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().colors.fg_muted;
        let danger = cx.theme().colors.danger;
        let weak = cx.entity().downgrade();
        let control = match self.draft {
            FieldValue::Boolean(on) => Switch::new(SharedString::from(self.field.id.clone()), on)
                .name(self.field.label.clone())
                .on_change(move |on, _, cx| {
                    let _ = weak.update(cx, |this, cx| this.queue(FieldValue::Boolean(on), cx));
                })
                .into_any_element(),
            _ => Input::new(&self.input)
                .invalid(self.invalid)
                .into_any_element(),
        };
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .py_3()
            .border_b_1()
            .border_color(cx.theme().colors.border)
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_4()
                    .child(self.field.label.clone())
                    .when(self.saving, |row| {
                        row.child(SavingIndicator::new("field-saving", SaveState::Saving))
                    }),
            )
            .when(!self.field.description.is_empty(), |el| {
                el.child(
                    div()
                        .text_sm()
                        .text_color(muted)
                        .child(self.field.description.clone()),
                )
            })
            .when(self.removed, |el| {
                el.child("This field was removed by the server.")
            })
            .when(!self.removed, |el| el.child(control))
            .when(self.removed, |el| {
                el.child(
                    Button::new("discard-removed-field", "Discard draft")
                        .variant(ButtonVariant::Ghost)
                        .disabled(self.saving)
                        .on_click(cx.listener(|this, _, _, cx| this.discard(cx))),
                )
            })
            .when_some(self.error.clone(), |el, error| {
                el.child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .text_color(danger)
                        .child(error)
                        .when(!self.invalid && !self.removed, |row| {
                            row.child(
                                Button::new("retry-field", "Retry")
                                    .variant(ButtonVariant::Link)
                                    .on_click(cx.listener(|this, _, _, cx| this.retry(cx))),
                            )
                        })
                        .child(
                            Button::new("discard-field", "Discard draft")
                                .variant(ButtonVariant::Ghost)
                                .disabled(self.saving)
                                .on_click(cx.listener(|this, _, _, cx| this.discard(cx))),
                        ),
                )
            })
    }
}

#[cfg(test)]
#[path = "field_tests.rs"]
mod tests;
