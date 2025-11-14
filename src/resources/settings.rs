use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct GameSettings {
    pub vsync_enabled: bool,
    pub show_fps: bool,
    pub dev_mode: bool,  // Enable developer controls
    pub fps_limit: u32,  // FPS limit (0 = unlimited)
    pub effects_volume: f32,  // Sound effects volume (0.0 to 1.0)
    pub music_volume: f32,    // Background music volume (0.0 to 1.0)
}

impl GameSettings {
    pub fn new() -> Self {
        Self {
            vsync_enabled: false,  // Default to vsync off (use FPS limiter instead for less input lag)
            show_fps: true,        // Default to showing FPS
            dev_mode: false,       // Dev mode off by default
            fps_limit: 60,         // Default to 60 FPS (only active when VSync is off)
            effects_volume: 0.7,   // 70% effects volume by default
            music_volume: 0.5,     // 50% music volume by default
        }
    }

    /// Get effects volume (clamped 0.0-1.0)
    pub fn get_effects_volume(&self) -> f32 {
        self.effects_volume.clamp(0.0, 1.0)
    }

    /// Get music volume (clamped 0.0-1.0)
    pub fn get_music_volume(&self) -> f32 {
        self.music_volume.clamp(0.0, 1.0)
    }

    /// Set effects volume (clamped 0.0-1.0)
    pub fn set_effects_volume(&mut self, volume: f32) {
        self.effects_volume = volume.clamp(0.0, 1.0);
    }

    /// Set music volume (clamped 0.0-1.0)
    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume.clamp(0.0, 1.0);
    }
}
