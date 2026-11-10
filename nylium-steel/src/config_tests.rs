use super::*;
use std::{
    fs,
    sync::{Arc, Barrier},
    thread,
};
use tempfile::tempdir;

fn config() -> (tempfile::TempDir, ConfigFile) {
    let dir = tempdir().unwrap();
    let config = ConfigFile::open(dir.path().join("config.toml")).unwrap();
    (dir, config)
}

#[test]
fn creates_config_and_saves_valid_update_without_rewriting_other_data() {
    let (_dir, config) = config();
    let path = &config.path;
    let mut original = fs::read_to_string(path).unwrap();
    original = original.replace("max_players = 20", "max_players = 20 # keep this comment");
    fs::write(path, &original).unwrap();

    config
        .set("server.max_players", FieldValue::Number(30))
        .unwrap();
    let saved = fs::read_to_string(path).unwrap();
    assert!(saved.contains("max_players = 30 # keep this comment"));
    assert!(saved.contains("[[server.server_links.links]]"));
    assert!(saved.contains(
        "label = { text = \"Visit the SteelMC Discord\", color = \"blue\", bold = true }"
    ));
    assert_eq!(config.load().unwrap().server.max_players, 30);
}

#[test]
fn invalid_updates_leave_original_file_unchanged() {
    let (_dir, config) = config();
    for (id, value) in [
        ("server.server_port", FieldValue::Number(65_536)),
        ("server.max_players", FieldValue::Text("many".into())),
        ("server.no_such_field", FieldValue::Number(1)),
        ("server.encryption", FieldValue::Boolean(false)),
    ] {
        let before = fs::read(&config.path).unwrap();
        assert!(config.set(id, value).is_err(), "{id} should be rejected");
        assert_eq!(
            fs::read(&config.path).unwrap(),
            before,
            "{id} changed the file"
        );
    }
}

#[test]
fn signed_neighbor_limit_remains_text_for_negative_and_positive_values() {
    let (_dir, config) = config();
    for value in ["-1", "42", "-1"] {
        let fields = config
            .set(
                "server.max_chained_neighbor_updates",
                FieldValue::Text(value.into()),
            )
            .unwrap();
        let field = fields
            .iter()
            .find(|field| field.id == "server.max_chained_neighbor_updates")
            .unwrap();
        assert_eq!(field.value, FieldValue::Text(value.into()));
        assert_eq!(field.kind, FieldKind::Text);
    }
}

#[test]
fn concurrent_updates_to_different_fields_are_not_lost() {
    let (_dir, config) = config();
    let config = Arc::new(config);
    let barrier = Arc::new(Barrier::new(3));
    let updates = [
        ("server.max_players", FieldValue::Number(30)),
        ("server.motd", FieldValue::Text("new motd".into())),
    ];
    let threads: Vec<_> = updates
        .into_iter()
        .map(|(id, value)| {
            let (config, barrier) = (config.clone(), barrier.clone());
            thread::spawn(move || {
                barrier.wait();
                config.set(id, value).unwrap();
            })
        })
        .collect();
    barrier.wait();
    for thread in threads {
        thread.join().unwrap();
    }

    let fields = config.fields().unwrap();
    assert!(
        fields
            .iter()
            .any(|field| field.id == "server.max_players" && field.value == FieldValue::Number(30))
    );
    assert!(fields.iter().any(
        |field| field.id == "server.motd" && field.value == FieldValue::Text("new motd".into())
    ));
}
