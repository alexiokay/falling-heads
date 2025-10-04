use bevy::prelude::*;

/// Resource to track damage cooldown timer (prevents rapid damage spam)
#[derive(Resource, Default)]
pub struct DamageCooldown {
    pub last_damage_time: f32,
}

impl DamageCooldown {
    pub fn can_damage(&self, current_time: f32, cooldown_seconds: f32) -> bool {
        current_time - self.last_damage_time >= cooldown_seconds
    }

    pub fn record_damage(&mut self, current_time: f32) {
        self.last_damage_time = current_time;
    }
}
