use bevy::prelude::*;

#[derive(Resource)]
pub struct GameStateResource {
    pub depth: f32,
    pub blocks_destroyed: u32,
    pub game_speed: f32,
}

impl Default for GameStateResource {
    fn default() -> Self {
        Self {
            depth: 0.0,
            blocks_destroyed: 0,
            game_speed: 1.0,
        }
    }
}
