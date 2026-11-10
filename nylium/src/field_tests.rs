use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use gpui::TestAppContext;
use nylium_adapter::{
    Field, FieldKind, FieldValue, Section, ServerAdapter, ServerSnapshot, StatePublisher,
};
use smol::channel::{self, Receiver};

use super::FieldEditor;

struct FakeAdapter {
    state: StatePublisher,
    writes: Mutex<Vec<FieldValue>>,
    gate: Mutex<Option<Receiver<()>>>,
    fail: Mutex<bool>,
}

impl FakeAdapter {
    fn new(field: Field) -> Self {
        Self {
            state: StatePublisher::new(ServerSnapshot {
                settings: vec![field],
                ..Default::default()
            }),
            writes: Mutex::default(),
            gate: Mutex::default(),
            fail: Mutex::new(false),
        }
    }

    fn writes(&self) -> Vec<FieldValue> {
        self.writes.lock().unwrap().clone()
    }
}

#[async_trait]
impl ServerAdapter for FakeAdapter {
    fn subscribe(&self) -> Receiver<()> {
        self.state.subscribe()
    }

    fn snapshot(&self) -> Arc<ServerSnapshot> {
        self.state.snapshot()
    }

    async fn start(&self) -> Result<(), String> {
        Ok(())
    }
    async fn stop(&self) -> Result<(), String> {
        Ok(())
    }
    async fn command(&self, _: String) -> Result<(), String> {
        Ok(())
    }

    async fn set_field(&self, _: Section, _: String, value: FieldValue) -> Result<(), String> {
        self.writes.lock().unwrap().push(value.clone());
        let gate = self.gate.lock().unwrap().take();
        if let Some(gate) = gate {
            let _ = gate.recv().await;
        }
        if *self.fail.lock().unwrap() {
            Err("write failed".into())
        } else {
            self.state.update(|state| state.settings[0].value = value);
            Ok(())
        }
    }
}

fn text_field(value: &str) -> Field {
    Field {
        id: "motd".into(),
        label: "MOTD".into(),
        description: String::new(),
        value: FieldValue::Text(value.into()),
        kind: FieldKind::Text,
    }
}

fn number_field(value: u32) -> Field {
    Field {
        id: "limit".into(),
        label: "Limit".into(),
        description: String::new(),
        value: FieldValue::Number(value),
        kind: FieldKind::Number {
            min: Some(1),
            max: Some(10),
        },
    }
}

fn setup(
    cx: &mut TestAppContext,
    field: Field,
    adapter: Arc<FakeAdapter>,
) -> (gpui::Entity<FieldEditor>, &mut gpui::VisualTestContext) {
    cx.update(|cx| {
        ely::theme::Theme::init(cx);
        ely::forms::bind_keys(cx);
    });
    let (view, cx) = cx.add_window_view(move |window, cx| {
        FieldEditor::new(field, Section::Settings, adapter, window, cx)
    });
    (view, cx)
}

fn advance(cx: &mut gpui::VisualTestContext) {
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(300));
    cx.run_until_parked();
}

#[gpui::test]
fn reconcile_refreshes_clean_draft_without_writing(cx: &mut TestAppContext) {
    let field = text_field("old");
    let adapter = Arc::new(FakeAdapter::new(field.clone()));
    let (view, cx) = setup(cx, field, adapter.clone());
    let replacement = text_field("from backend");
    view.update(cx, |editor, cx| editor.reconcile(replacement, cx));
    view.read_with(cx, |editor, cx| {
        assert_eq!(editor.draft, FieldValue::Text("from backend".into()));
        assert_eq!(editor.input.read(cx).text(), "from backend");
    });
    advance(cx);
    assert!(adapter.writes().is_empty());
}

#[gpui::test]
fn edits_during_a_slow_write_are_serialized_and_keep_the_latest_value(cx: &mut TestAppContext) {
    let adapter = Arc::new(FakeAdapter::new(number_field(5)));
    let (release, gate) = channel::bounded(1);
    *adapter.gate.lock().unwrap() = Some(gate);
    let (view, cx) = setup(cx, number_field(5), adapter.clone());
    view.update(cx, |editor, cx| {
        editor.input.update(cx, |input, cx| input.set_text("6", cx))
    });
    advance(cx);
    assert_eq!(adapter.writes(), [FieldValue::Number(6)]);
    view.update(cx, |editor, cx| {
        editor.input.update(cx, |input, cx| input.set_text("7", cx))
    });
    release.try_send(()).unwrap();
    cx.run_until_parked();
    advance(cx);
    assert_eq!(
        adapter.writes(),
        [FieldValue::Number(6), FieldValue::Number(7)]
    );
    view.read_with(cx, |editor, _| {
        assert_eq!(editor.draft, FieldValue::Number(7))
    });
}

#[gpui::test]
fn invalid_number_is_rejected_and_failed_backend_write_keeps_retryable_draft(
    cx: &mut TestAppContext,
) {
    let adapter = Arc::new(FakeAdapter::new(number_field(5)));
    *adapter.fail.lock().unwrap() = true;
    let (view, cx) = setup(cx, number_field(5), adapter.clone());
    view.update(cx, |editor, cx| {
        editor
            .input
            .update(cx, |input, cx| input.set_text("not a number", cx))
    });
    view.read_with(cx, |editor, _| {
        assert!(editor.invalid);
        assert_eq!(
            editor.error.as_deref(),
            Some("Enter a whole number from 0 to 4294967295")
        );
    });
    assert!(adapter.writes().is_empty());
    view.update(cx, |editor, cx| {
        editor.input.update(cx, |input, cx| input.set_text("8", cx))
    });
    advance(cx);
    view.read_with(cx, |editor, _| {
        assert_eq!(editor.draft, FieldValue::Number(8));
        assert_eq!(editor.error.as_deref(), Some("write failed"));
        assert!(!editor.saving);
    });
    view.update(cx, |editor, cx| editor.retry(cx));
    advance(cx);
    assert_eq!(
        adapter.writes(),
        [FieldValue::Number(8), FieldValue::Number(8)]
    );
}

#[gpui::test]
fn invalid_draft_can_be_discarded_for_the_latest_backend_value(cx: &mut TestAppContext) {
    let adapter = Arc::new(FakeAdapter::new(number_field(5)));
    let (view, cx) = setup(cx, number_field(5), adapter.clone());
    view.update(cx, |editor, cx| {
        editor.input.update(cx, |input, cx| input.set_text("", cx))
    });
    adapter
        .state
        .update(|state| state.settings[0].value = FieldValue::Number(9));
    view.update(cx, |editor, cx| editor.discard(cx));
    cx.run_until_parked();
    view.read_with(cx, |editor, cx| {
        assert_eq!(editor.input.read(cx).text(), "9");
        assert_eq!(editor.draft, FieldValue::Number(9));
        assert!(editor.error.is_none());
    });
    advance(cx);
    assert!(adapter.writes().is_empty());
}

#[gpui::test]
fn removal_retains_dirty_fields_until_explicit_discard(cx: &mut TestAppContext) {
    let adapter = Arc::new(FakeAdapter::new(text_field("old")));
    let (view, cx) = setup(cx, text_field("old"), adapter.clone());
    let clean_adapter = Arc::new(FakeAdapter::new(text_field("old")));
    let (clean_view, cx) = setup(cx, text_field("old"), clean_adapter);
    clean_view.update(cx, |editor, cx| assert!(!editor.mark_removed(cx)));

    adapter.state.update(|state| state.settings.clear());
    view.update(cx, |editor, cx| {
        // A user edit is dirty during the debounce window, before `saving` starts.
        editor.draft = FieldValue::Text("draft".into());
        assert!(editor.mark_removed(cx));
    });
    view.read_with(cx, |editor, _| {
        assert!(editor.removed);
        assert!(editor.error.is_some());
    });
    view.update(cx, |editor, cx| {
        editor.draft = editor.field.value.clone();
        editor.error = None;
        editor.saving = true;
        assert!(editor.mark_removed(cx));
        editor.saving = false;
        editor.error = Some("write failed".into());
        assert!(editor.mark_removed(cx));
    });
    view.update(cx, |editor, cx| editor.discard(cx));
    view.read_with(cx, |editor, _| {
        assert!(editor.removed);
        assert!(editor.discarded);
        assert!(editor.error.is_none());
        assert!(!editor.saving);
    });
    view.update(cx, |editor, cx| assert!(!editor.mark_removed(cx)));
}
