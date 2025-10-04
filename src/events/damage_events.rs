use bevy::prelude::*;

#[derive(Message)]
pub struct EntityDamaged {
    pub entity: Entity,
    pub damage: u32,
}

#[derive(Message)]
pub struct EntityDied {
    pub entity: Entity,
}
