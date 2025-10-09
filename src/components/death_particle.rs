use bevy::prelude::*;

/// Component for death particles that fly out when a character dies
#[derive(Component)]
pub struct DeathParticle {
    pub velocity: Vec3,
    pub rotation_velocity: Vec3, // Angular velocity for spinning
    pub lifetime: Timer,
}

impl DeathParticle {
    pub fn new(velocity: Vec3, rotation_velocity: Vec3, lifetime_secs: f32) -> Self {
        Self {
            velocity,
            rotation_velocity,
            lifetime: Timer::from_seconds(lifetime_secs, TimerMode::Once),
        }
    }
}
