use std::{fs, path::PathBuf, sync::Mutex};

use nylium_adapter::{Field, FieldKind, FieldValue};
use tempfile::NamedTempFile;
use toml_edit::{DocumentMut, Item, Value};

pub(crate) struct ConfigFile {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl ConfigFile {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        steel::config::load_or_create(&path)?;
        Ok(Self {
            path,
            write_lock: Mutex::new(()),
        })
    }

    pub fn load(&self) -> Result<steel::config::SteelConfig, String> {
        steel::config::load_or_create(&self.path)
    }

    pub fn fields(&self) -> Result<Vec<Field>, String> {
        let text = fs::read_to_string(&self.path)
            .map_err(|e| format!("failed to read {}: {e}", self.path.display()))?;
        let doc = text
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
        let parts: Vec<_> = id.split('.').collect();
        if parts.first() != Some(&"server") || parts.len() < 2 {
            return Err(format!("unknown config field '{id}'"));
        }
        let mut item = &mut doc["server"];
        for part in &parts[1..parts.len() - 1] {
            item = item
                .get_mut(part)
                .ok_or_else(|| format!("unknown config field '{id}'"))?;
        }
        let target = item
            .get_mut(parts[parts.len() - 1])
            .ok_or_else(|| format!("unknown config field '{id}'"))?;
        let previous = target
            .as_value()
            .ok_or_else(|| format!("unsupported config field '{id}'"))?;
        let mut new_value = match (previous, value) {
            (Value::Boolean(_), FieldValue::Boolean(v)) => Value::from(v),
            (Value::String(_), FieldValue::Text(v)) => Value::from(v),
            (Value::Integer(_), FieldValue::Number(v)) if !signed_field(id) => {
                Value::from(i64::from(v))
            }
            (Value::Integer(_), FieldValue::Text(v)) if signed_field(id) => Value::from(
                v.parse::<i64>()
                    .map_err(|_| format!("invalid integer for '{id}'"))?,
            ),
            _ => return Err(format!("value type does not match config field '{id}'")),
        };
        *new_value.decor_mut() = previous.decor().clone();
        *target.as_value_mut().ok_or("Config field is not scalar")? = new_value;

        let parent = self
            .path
            .parent()
            .ok_or_else(|| format!("config has no parent directory: {}", self.path.display()))?;
        let mut candidate = NamedTempFile::new_in(parent)
            .map_err(|e| format!("failed to create temporary config: {e}"))?;
        use std::io::Write;
        candidate
            .write_all(doc.to_string().as_bytes())
            .map_err(|e| format!("failed to write temporary config: {e}"))?;
        candidate
            .as_file()
            .sync_all()
            .map_err(|e| format!("failed to sync temporary config: {e}"))?;
        steel::config::load_or_create(candidate.path())?;
        if fs::read_to_string(&self.path).map_err(|e| format!("failed to re-read config: {e}"))?
            != original
        {
            return Err("config changed while the update was being validated".into());
        }
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

fn fields_in(doc: &DocumentMut, path: &std::path::Path) -> Vec<Field> {
    fn visit(table: &toml_edit::Table, prefix: &str, path: &std::path::Path, out: &mut Vec<Field>) {
        for (key, item) in table {
            let id = format!("{prefix}.{key}");
            if let Some(table) = item.as_table() {
                visit(table, &id, path, out);
                continue;
            }
            let Some(value) = item.as_value() else {
                continue;
            };
            let (field_value, kind) = match value {
                Value::Boolean(v) => (FieldValue::Boolean(*v.value()), FieldKind::Boolean),
                Value::String(v) => (FieldValue::Text(v.value().clone()), FieldKind::Text),
                Value::Integer(v) if !signed_field(&id) && u32::try_from(*v.value()).is_ok() => (
                    FieldValue::Number(*v.value() as u32),
                    FieldKind::Number {
                        min: Some(0),
                        max: Some(u32::MAX),
                    },
                ),
                Value::Integer(v) if signed_field(&id) => {
                    (FieldValue::Text(v.value().to_string()), FieldKind::Text)
                }
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
    if let Some(Item::Table(server)) = doc.get("server") {
        visit(server, "server", path, &mut out);
    }
    out
}

fn signed_field(id: &str) -> bool {
    matches!(
        id,
        "server.max_chained_neighbor_updates"
            | "server.chat_spam_threshold_seconds"
            | "server.command_spam_threshold_seconds"
            | "server.player_idle_timeout"
    )
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
