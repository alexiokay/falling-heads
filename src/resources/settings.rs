use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct GameSettings {
    pub vsync_enabled: bool,
    pub show_fps: bool,
    pub dev_mode: bool,  // Enable developer controls
}

impl GameSettings {
    pub fn new() -> Self {
        Self {
            vsync_enabled: true,  // Default to vsync on
            show_fps: true,       // Default to showing FPS
            dev_mode: false,      // Dev mode off by default
        }
    }
}
