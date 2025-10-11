use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct GameSettings {
    pub vsync_enabled: bool,
    pub show_fps: bool,
    pub dev_mode: bool,  // Enable developer controls
    pub fps_limit: u32,  // FPS limit (0 = unlimited)
}

impl GameSettings {
    pub fn new() -> Self {
        Self {
            vsync_enabled: false,  // Default to vsync off (use FPS limiter instead for less input lag)
            show_fps: true,        // Default to showing FPS
            dev_mode: false,       // Dev mode off by default
            fps_limit: 60,         // Default to 60 FPS (only active when VSync is off)
        }
    }
}
