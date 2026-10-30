use bevy::prelude::*;

/// Username label component - displays viewer's username above spawned entities
/// Positioned above the entity but below hearts (if entity has health)
#[derive(Component, Clone, Debug)]
pub struct UsernameLabel {
    pub username: String,
    pub owner: Entity, // The entity this label belongs to
    pub offset_y: f32, // Vertical offset from owner's position (positive = above)
}

impl UsernameLabel {
    pub fn with_offset(username: String, owner: Entity, offset_y: f32) -> Self {
        Self {
            username,
            owner,
            offset_y,
        }
    }
}
