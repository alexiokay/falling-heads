use bevy::prelude::*;

/// Resource to track if a lootbox is currently active
#[derive(Resource, Default)]
pub struct LootboxState {
    pub is_active: bool,
}
