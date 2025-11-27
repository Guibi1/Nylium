use gpui::SharedString;

pub enum FieldOptions<K> {
    Boolean(BooleanFieldOptions<K>),
    Number(NumberFieldOptions<K>),
    String(StringFieldOptions<K>),
}

pub struct BooleanFieldOptions<K> {
    pub key: K,
    pub label: SharedString,
    pub id: SharedString,
}

pub struct NumberFieldOptions<K> {
    pub key: K,
    pub label: SharedString,
    pub min: Option<u32>,
    pub max: Option<u32>,
}

pub struct StringFieldOptions<K> {
    pub key: K,
    pub label: SharedString,
}

impl<K> FieldOptions<K>
where
    K: Copy,
{
    pub fn new_bool(key: K, label: impl Into<SharedString>, id: impl Into<SharedString>) -> Self {
        FieldOptions::Boolean(BooleanFieldOptions {
            label: label.into(),
            key,
            id: id.into(),
        })
    }

    pub fn new_number(
        key: K,
        label: impl Into<SharedString>,
        min: Option<u32>,
        max: Option<u32>,
    ) -> Self {
        FieldOptions::Number(NumberFieldOptions {
            label: label.into(),
            key,
            min,
            max,
        })
    }

    pub fn new_string(key: K, label: impl Into<SharedString>) -> Self {
        FieldOptions::String(StringFieldOptions {
            label: label.into(),
            key,
        })
    }

    pub fn key(&self) -> K {
        match self {
            FieldOptions::Boolean(option) => option.key,
            FieldOptions::Number(option) => option.key,
            FieldOptions::String(option) => option.key,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    Boolean(bool),
    Number(u32),
    String(String),
}

impl FieldValue {
    pub fn assert_bool(self) -> bool {
        match self {
            FieldValue::Boolean(value) => value,
            _ => panic!("Config value is a boolean."),
        }
    }

    pub fn assert_number(self) -> u32 {
        match self {
            FieldValue::Number(value) => value,
            _ => panic!("Config value is a number."),
        }
    }

    pub fn assert_string(self) -> String {
        match self {
            FieldValue::String(value) => value,
            _ => panic!("Config value is a string."),
        }
    }
}
