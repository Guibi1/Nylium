use gpui::SharedString;

pub enum ConfigOptions<C> {
    Boolean(BooleanConfigOption<C>),
    Number(NumberConfigOption<C>),
    String(StringConfigOption<C>),
}

pub struct BooleanConfigOption<C> {
    pub label: SharedString,
    pub key: C,
    pub id: SharedString,
}

pub struct NumberConfigOption<C> {
    pub label: SharedString,
    pub key: C,
    pub min: Option<u32>,
    pub max: Option<u32>,
}

pub struct StringConfigOption<C> {
    pub label: SharedString,
    pub key: C,
}

impl<C> ConfigOptions<C> {
    pub fn new_bool(key: C, label: impl Into<SharedString>, id: impl Into<SharedString>) -> Self {
        ConfigOptions::Boolean(BooleanConfigOption {
            label: label.into(),
            key,
            id: id.into(),
        })
    }

    pub fn new_number(
        key: C,
        label: impl Into<SharedString>,
        min: Option<u32>,
        max: Option<u32>,
    ) -> Self {
        ConfigOptions::Number(NumberConfigOption {
            label: label.into(),
            key,
            min,
            max,
        })
    }

    pub fn new_string(key: C, label: impl Into<SharedString>) -> Self {
        ConfigOptions::String(StringConfigOption {
            label: label.into(),
            key,
        })
    }
}

#[derive(Debug)]
pub enum ConfigValue {
    Boolean(bool),
    Number(u32),
    String(String),
}

impl ConfigValue {
    pub fn assert_bool(self) -> bool {
        match self {
            ConfigValue::Boolean(value) => value,
            _ => panic!("Config value is a boolean."),
        }
    }

    pub fn assert_number(self) -> u32 {
        match self {
            ConfigValue::Number(value) => value,
            _ => panic!("Config value is a number."),
        }
    }

    pub fn assert_string(self) -> String {
        match self {
            ConfigValue::String(value) => value,
            _ => panic!("Config value is a string."),
        }
    }
}
