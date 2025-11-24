use gpui::SharedString;
use uuid::Uuid;

#[derive(Clone, PartialEq)]
pub struct Player {
    pub id: Uuid,
    pub name: SharedString,
    pub online: bool,
}

impl Player {
    pub fn new(id: Uuid, name: impl Into<SharedString>, online: bool) -> Self {
        Self {
            id,
            name: name.into(),
            online,
        }
    }
}
