use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct GameSettings {
    pub vsync_enabled: bool,
    pub show_fps: bool,
}

impl GameSettings {
    pub fn new() -> Self {
        Self {
            vsync_enabled: true,  // Default to vsync on
            show_fps: true,       // Default to showing FPS
        }
    }
}
