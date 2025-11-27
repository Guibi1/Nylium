use nylium_adapter::config::FieldValue;

mod boolean;
mod number;
mod string;

pub use boolean::BooleanField;
pub use number::NumberField;
pub use string::StringField;

#[derive(Clone)]
pub struct ChangeEvent {
    pub value: FieldValue,
}

impl ChangeEvent {
    pub fn new_bool(value: bool) -> Self {
        Self {
            value: FieldValue::Boolean(value),
        }
    }

    pub fn new_number(value: u32) -> Self {
        Self {
            value: FieldValue::Number(value),
        }
    }

    pub fn new_string(value: String) -> Self {
        Self {
            value: FieldValue::String(value),
        }
    }
}
