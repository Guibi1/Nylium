use std::{
    cell::Cell,
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    sync::Mutex,
};

use nylium_adapter::{Field, FieldKind, FieldValue};
use pumpkin_config::{LoadConfiguration, PumpkinConfig};
use tempfile::NamedTempFile;
use toml_edit::{DocumentMut, Item, Value};

pub(crate) struct ConfigFile {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl ConfigFile {
    pub fn open(config_dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&config_dir).map_err(|e| {
            format!(
                "failed to create config directory {}: {e}",
                config_dir.display()
            )
        })?;
        let path = config_dir.join(PumpkinConfig::get_path());
        if !path.exists() {
            let content = toml_edit::ser::to_string(&PumpkinConfig::default())
                .map_err(|e| format!("failed to serialize default config: {e}"))?;
            let parent = path.parent().ok_or("config path has no parent directory")?;
            let mut candidate = NamedTempFile::new_in(parent)
                .map_err(|e| format!("failed to create temporary config: {e}"))?;
            use std::io::Write;
            candidate
                .write_all(content.as_bytes())
                .map_err(|e| format!("failed to write temporary config: {e}"))?;
            candidate
                .as_file()
                .sync_all()
                .map_err(|e| format!("failed to sync temporary config: {e}"))?;
            candidate
                .persist(&path)
                .map_err(|e| format!("failed to create config: {}", e.error))?;
        }
        let config = Self {
            path,
            write_lock: Mutex::new(()),
        };
        config.load()?;
        Ok(config)
    }

    pub fn load(&self) -> Result<PumpkinConfig, String> {
        let source = fs::read_to_string(&self.path)
            .map_err(|e| format!("failed to read {}: {e}", self.path.display()))?;
        parse_and_validate(&source)
    }

    pub fn fields(&self) -> Result<Vec<Field>, String> {
        let source = fs::read_to_string(&self.path)
            .map_err(|e| format!("failed to read {}: {e}", self.path.display()))?;
        let doc = source
            .parse::<DocumentMut>()
            .map_err(|e| format!("failed to parse config: {e}"))?;
        Ok(fields_in(&doc, &self.path))
    }

    pub fn set(&self, id: &str, value: FieldValue) -> Result<Vec<Field>, String> {
        let _guard = self.write_lock.lock().map_err(|e| e.to_string())?;
        let original = fs::read_to_string(&self.path)
            .map_err(|e| format!("failed to read {}: {e}", self.path.display()))?;
        let mut doc = original
            .parse::<DocumentMut>()
            .map_err(|e| format!("failed to parse config: {e}"))?;
        if !fields_in(&doc, &self.path)
            .iter()
            .any(|field| field.id == id)
        {
            return Err(format!("unknown config field '{id}'"));
        }
        let parts: Vec<_> = id.split('.').collect();
        if parts.len() < 2 || !matches!(parts[0], "basic" | "advanced") {
            return Err(format!("unknown config field '{id}'"));
        }
        let key_parts = &parts[1..];
        let mut target: &mut Item = doc
            .get_mut(key_parts[0])
            .ok_or_else(|| format!("unknown config field '{id}'"))?;
        for part in &key_parts[1..] {
            target = target
                .get_mut(part)
                .ok_or_else(|| format!("unknown config field '{id}'"))?;
        }
        let previous = target
            .as_value()
            .ok_or_else(|| format!("unsupported config field '{id}'"))?;
        let mut new_value = match (previous, value) {
            (Value::Boolean(_), FieldValue::Boolean(v)) => Value::from(v),
            (Value::String(_), FieldValue::Text(v)) => Value::from(v),
            (Value::Integer(_), FieldValue::Number(v)) => Value::from(i64::from(v)),
            (Value::Integer(_), FieldValue::Text(v)) => Value::from(
                v.parse::<i64>()
                    .map_err(|_| format!("invalid integer for '{id}'"))?,
            ),
            (Value::Float(_), FieldValue::Text(v)) => Value::from(
                v.parse::<f64>()
                    .map_err(|_| format!("invalid number for '{id}'"))?,
            ),
            _ => return Err(format!("value type does not match config field '{id}'")),
        };
        *new_value.decor_mut() = previous.decor().clone();
        *target.as_value_mut().ok_or("config field is not scalar")? = new_value;

        let updated = doc.to_string();
        parse_and_validate(&updated)?;
        if fs::read_to_string(&self.path).map_err(|e| format!("failed to re-read config: {e}"))?
            != original
        {
            return Err("config changed while the update was being validated".into());
        }
        let parent = self
            .path
            .parent()
            .ok_or_else(|| format!("config has no parent directory: {}", self.path.display()))?;
        let mut candidate = NamedTempFile::new_in(parent)
            .map_err(|e| format!("failed to create temporary config: {e}"))?;
        use std::io::Write;
        candidate
            .write_all(updated.as_bytes())
            .map_err(|e| format!("failed to write temporary config: {e}"))?;
        candidate
            .as_file()
            .sync_all()
            .map_err(|e| format!("failed to sync temporary config: {e}"))?;
        let permissions = fs::metadata(&self.path)
            .map_err(|e| format!("failed to read config permissions: {e}"))?
            .permissions();
        candidate
            .as_file()
            .set_permissions(permissions)
            .map_err(|e| format!("failed to preserve config permissions: {e}"))?;
        candidate
            .persist(&self.path)
            .map_err(|e| format!("failed to replace config: {}", e.error))?;
        Ok(fields_in(&doc, &self.path))
    }
}

thread_local! {
    static VALIDATING: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn validation_in_progress() -> bool {
    VALIDATING.try_with(Cell::get).unwrap_or(false)
}

fn parse_and_validate(source: &str) -> Result<PumpkinConfig, String> {
    let config: PumpkinConfig = toml_edit::de::from_str(source)
        .map_err(|e| format!("failed to deserialize Pumpkin config: {e}"))?;
    // Pumpkin validates with assertions; those caught panics must not stop a running server.
    let previous = VALIDATING.with(|flag| flag.replace(true));
    let validation = catch_unwind(AssertUnwindSafe(|| config.validate()));
    VALIDATING.with(|flag| flag.set(previous));
    validation.map_err(|_| "Pumpkin config validation failed".to_string())?;
    Ok(config)
}

fn fields_in(doc: &DocumentMut, path: &Path) -> Vec<Field> {
    const BASIC: &[&str] = &[
        "seed",
        "default_difficulty",
        "op_permission_level",
        "allow_nether",
        "allow_end",
        "hardcore",
        "tps",
        "default_gamemode",
        "force_gamemode",
        "scrub_ips",
        "use_favicon",
        "favicon_path",
        "default_level_name",
        "allow_chat_reports",
        "white_list",
        "enforce_whitelist",
        "accepts_transfers",
        "spawn_protection",
    ];
    const ADVANCED: &[&str] = &[
        "logging",
        "resource_pack",
        "world",
        "networking",
        "commands",
        "chat",
        "pvp",
        "server_links",
        "player_data",
        "fun",
        "recipe",
        "plugins",
        "advancement",
        "update_checker",
    ];
    fn scalar_paths(table: &dyn toml_edit::TableLike, prefix: &str, out: &mut Vec<String>) {
        for (key, item) in table.iter() {
            let path = if prefix.is_empty() {
                key.to_string()
            } else {
                format!("{prefix}.{key}")
            };
            if let Some(table) = item.as_table_like() {
                scalar_paths(table, &path, out);
            } else if item.as_value().is_some() {
                out.push(path);
            }
        }
    }
    let known_paths = toml_edit::ser::to_string(&PumpkinConfig::default())
        .ok()
        .and_then(|source| source.parse::<DocumentMut>().ok())
        .map(|schema| {
            let mut paths = Vec::new();
            scalar_paths(schema.as_table(), "", &mut paths);
            paths
        })
        .unwrap_or_default();
    fn visit(
        table: &dyn toml_edit::TableLike,
        prefix: &str,
        path: &Path,
        known_paths: &[String],
        out: &mut Vec<Field>,
    ) {
        for (key, item) in table.iter() {
            let id = format!("{prefix}.{key}");
            if let Some(table) = item.as_table_like() {
                visit(table, &id, path, known_paths, out);
                continue;
            }
            let Some(value) = item.as_value() else {
                continue;
            };
            if !known_paths.iter().any(|known| {
                id.strip_prefix("basic.")
                    .or_else(|| id.strip_prefix("advanced."))
                    == Some(known.as_str())
            }) {
                continue;
            }
            let (field_value, kind) = match value {
                Value::Boolean(v) => (FieldValue::Boolean(*v.value()), FieldKind::Boolean),
                Value::String(v) => (FieldValue::Text(v.value().clone()), FieldKind::Text),
                Value::Integer(v) if u32::try_from(*v.value()).is_ok() => (
                    FieldValue::Number(*v.value() as u32),
                    FieldKind::Number {
                        min: Some(0),
                        max: Some(u32::MAX),
                    },
                ),
                Value::Integer(v) => (FieldValue::Text(v.value().to_string()), FieldKind::Text),
                Value::Float(v) => (FieldValue::Text(v.value().to_string()), FieldKind::Text),
                _ => continue,
            };
            out.push(Field {
                id: id.clone(),
                label: id
                    .split('.')
                    .skip(1)
                    .map(|s| s.replace('_', " "))
                    .collect::<Vec<_>>()
                    .join(" / "),
                description: format!("Saved in {}; applies on restart", path.display()),
                value: field_value,
                kind,
            });
        }
    }
    let mut out = Vec::new();
    for (key, item) in doc.iter() {
        let prefix = if BASIC.contains(&key) {
            "basic"
        } else if ADVANCED.contains(&key) {
            "advanced"
        } else {
            continue;
        };
        if let Some(table) = item.as_table_like() {
            visit(
                table,
                &format!("{prefix}.{key}"),
                path,
                &known_paths,
                &mut out,
            );
        } else if let Some(value) = item.as_value() {
            if !known_paths.iter().any(|known| known == key) {
                continue;
            }
            let (field_value, kind) = match value {
                Value::Boolean(v) => (FieldValue::Boolean(*v.value()), FieldKind::Boolean),
                Value::String(v) => (FieldValue::Text(v.value().clone()), FieldKind::Text),
                Value::Integer(v) if u32::try_from(*v.value()).is_ok() => (
                    FieldValue::Number(*v.value() as u32),
                    FieldKind::Number {
                        min: Some(0),
                        max: Some(u32::MAX),
                    },
                ),
                Value::Integer(v) => (FieldValue::Text(v.value().to_string()), FieldKind::Text),
                Value::Float(v) => (FieldValue::Text(v.value().to_string()), FieldKind::Text),
                _ => continue,
            };
            out.push(Field {
                id: format!("{prefix}.{key}"),
                label: key.replace('_', " "),
                description: format!("Saved in {}; applies on restart", path.display()),
                value: field_value,
                kind,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn creates_defaults_and_preserves_comments_on_valid_update() {
        let dir = tempdir().unwrap();
        let config = ConfigFile::open(dir.path().to_path_buf()).unwrap();
        let original = fs::read_to_string(&config.path).unwrap();
        fs::write(
            &config.path,
            original.replace("tps = 20.0", "tps = 20.0 # keep"),
        )
        .unwrap();
        config
            .set("basic.tps", FieldValue::Text("19.5".into()))
            .unwrap();
        let saved = fs::read_to_string(&config.path).unwrap();
        assert!(saved.contains("tps = 19.5 # keep"));
        assert_eq!(config.load().unwrap().basic.tps, 19.5);
        assert!(
            config
                .fields()
                .unwrap()
                .iter()
                .any(|field| field.id == "advanced.networking.java.view_distance")
        );
    }

    #[test]
    fn rejects_invalid_or_unknown_updates_without_changing_file() {
        let dir = tempdir().unwrap();
        let config = ConfigFile::open(dir.path().to_path_buf()).unwrap();
        let before = fs::read(&config.path).unwrap();
        for (id, value) in [
            (
                "advanced.networking.java.view_distance",
                FieldValue::Number(1),
            ),
            (
                "advanced.networking.java.encryption",
                FieldValue::Boolean(false),
            ),
            ("unknown.field", FieldValue::Number(1)),
        ] {
            assert!(config.set(id, value).is_err(), "{id}");
            assert_eq!(fs::read(&config.path).unwrap(), before);
        }
    }
}
