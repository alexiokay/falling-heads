use bevy::prelude::*;

/// Visual damage effect - red overlay + border that fades out over time
#[derive(Component)]
pub struct DamageVisual {
    pub timer: Timer,
}

impl DamageVisual {
    pub fn new(duration: f32) -> Self {
        Self {
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }

    /// Get current alpha (1.0 at start, 0.0 at end)
    pub fn alpha(&self) -> f32 {
        1.0 - self.timer.fraction()
    }
}

/// Marker for entities that are dead but frozen for death animation
#[derive(Component)]
pub struct DeathAnimation {
    pub timer: Timer,
}

impl DeathAnimation {
    pub fn new() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
        }
    }
}

/// Marker for entities that should be despawned after all systems finish processing
/// This prevents crashes when systems try to access entities that were despawned mid-frame
#[derive(Component)]
pub struct MarkedForDeath;
