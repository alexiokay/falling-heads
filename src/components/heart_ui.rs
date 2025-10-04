use bevy::prelude::*;

/// Component marking a heart UI element above an entity
#[derive(Component)]
pub struct HeartUI {
    pub owner: Entity,  // The entity this heart belongs to
    pub heart_index: usize,  // Which heart position (0, 1, 2...)
}
