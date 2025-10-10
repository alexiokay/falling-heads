use bevy::prelude::*;

/// Component for weather particles (rain, snow, etc.)
#[derive(Component)]
pub struct WeatherParticle {
    pub velocity: Vec2,
    pub lifetime: Timer,
    pub follows_camera: bool,  // If true, particle respawns at top when off-screen
}

impl WeatherParticle {
    pub fn new(velocity: Vec2, lifetime_secs: f32) -> Self {
        Self {
            velocity,
            lifetime: Timer::from_seconds(lifetime_secs, TimerMode::Once),
            follows_camera: false,
        }
    }

    pub fn new_camera_following(velocity: Vec2, lifetime_secs: f32) -> Self {
        Self {
            velocity,
            lifetime: Timer::from_seconds(lifetime_secs, TimerMode::Repeating),  // Repeating for infinite particles
            follows_camera: true,
        }
    }
}
