use gpui::SharedString;
use uuid::Uuid;

#[derive(Clone, PartialEq)]
pub struct Player {
    pub id: Uuid,
    pub name: SharedString,
    pub online: bool,
    pub map: PlayerMap,
}

impl Player {
    pub fn new(id: Uuid, name: impl Into<SharedString>, map: PlayerMap, online: bool) -> Self {
        Self {
            id,
            name: name.into(),
            online,
            map,
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum PlayerMap {
    Overworld,
    Nether,
    End,
    Custom(SharedString),
}

impl PlayerMap {
    pub fn get_name(&self) -> SharedString {
        match self {
            PlayerMap::Overworld => "Overworld".into(),
            PlayerMap::Nether => "Nether".into(),
            PlayerMap::End => "End".into(),
            PlayerMap::Custom(name) => name.clone(),
        }
    }
}
