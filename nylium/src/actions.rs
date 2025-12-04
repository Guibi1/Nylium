use gpui::*;

use uuid::Uuid;

#[derive(Action, Clone, PartialEq)]
#[action(namespace = player, no_json)]
pub struct CopyUuid {
    pub uuid: Uuid,
}

#[derive(Action, Clone, PartialEq)]
#[action(namespace = player, no_json)]
pub struct Op {
    pub name: SharedString,
}

#[derive(Action, Clone, PartialEq)]
#[action(namespace = player, no_json)]
pub struct Kick {
    pub name: SharedString,
}

#[derive(Action, Clone, PartialEq)]
#[action(namespace = player, no_json)]
pub struct Ban {
    pub name: SharedString,
}
