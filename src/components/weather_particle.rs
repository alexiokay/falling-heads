use bevy::prelude::*;

/// Component for weather particles (rain, snow, etc.)
#[derive(Component)]
pub struct WeatherParticle {
    pub velocity: Vec2,
    pub lifetime: Timer,
}

impl WeatherParticle {
    pub fn new(velocity: Vec2, lifetime_secs: f32) -> Self {
        Self {
            velocity,
            lifetime: Timer::from_seconds(lifetime_secs, TimerMode::Once),
        }
    }
}
