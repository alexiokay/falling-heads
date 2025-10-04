use bevy::prelude::*;

/// Marker component for health UI that follows an entity
#[derive(Component)]
pub struct HealthBar {
    pub target_entity: Entity,
    pub offset_y: f32,  // Vertical offset from entity
}
