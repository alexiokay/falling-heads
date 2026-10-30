use bevy::prelude::*;

/// Component marking a health row above an entity
#[derive(Component)]
pub struct HeartUI {
    pub owner: Entity, // The entity this heart belongs to
}
