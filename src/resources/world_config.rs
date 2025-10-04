use bevy::prelude::*;

#[derive(Resource)]
pub struct WorldConfig {
    pub block_size: f32,
    pub spawn_rate: f32,
    pub difficulty_scale: f32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            block_size: 32.0,
            spawn_rate: 1.0,
            difficulty_scale: 1.0,
        }
    }
}
