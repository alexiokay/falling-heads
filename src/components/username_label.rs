use bevy::prelude::*;

/// Username label component - displays viewer's username above spawned entities
/// Positioned above the entity but below hearts (if entity has health)
#[derive(Component, Clone, Debug)]
pub struct UsernameLabel {
    pub username: String,
    pub owner: Entity,  // The entity this label belongs to
    pub offset_y: f32,  // Vertical offset from owner's position (positive = above)
}

impl UsernameLabel {
    pub fn new(username: String, owner: Entity) -> Self {
        Self {
            username,
            owner,
            offset_y: 50.0,  // Default: 50 units above entity (below hearts which are at ~80+)
        }
    }

    pub fn with_offset(username: String, owner: Entity, offset_y: f32) -> Self {
        Self {
            username,
            owner,
            offset_y,
        }
    }

    /// Offset for entities with hearts (villagers, enemies)
    pub fn for_entity_with_hearts(username: String, owner: Entity) -> Self {
        Self {
            username,
            owner,
            offset_y: 40.0,  // Below hearts (hearts start ~60-80 units above)
        }
    }

    /// Offset for simple entities (TNT, items)
    pub fn for_simple_entity(username: String, owner: Entity) -> Self {
        Self {
            username,
            owner,
            offset_y: 50.0,  // Just above entity
        }
    }
}
