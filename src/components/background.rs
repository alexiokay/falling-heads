use bevy::prelude::*;
use crate::components::block::BiomeType;

/// Component marking a background entity
#[derive(Component)]
pub struct Background {
    pub biome: BiomeType,
    pub grid_width: i32,
    pub grid_height: i32,
}
