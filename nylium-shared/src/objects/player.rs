use uuid::Uuid;

#[derive(Clone)]
pub struct Player {
    pub id: Uuid,
    pub name: String,
}
