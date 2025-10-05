use bevy::prelude::*;
use std::collections::HashMap;

/// Resource to track damage cooldown timer per entity (prevents rapid damage spam)
#[derive(Resource, Default)]
pub struct DamageCooldown {
    pub last_damage_time: HashMap<Entity, f32>,
}

impl DamageCooldown {
    pub fn can_damage(&self, entity: Entity, current_time: f32, cooldown_seconds: f32) -> bool {
        if let Some(&last_time) = self.last_damage_time.get(&entity) {
            current_time - last_time >= cooldown_seconds
        } else {
            true // Never damaged before
        }
    }

    pub fn record_damage(&mut self, entity: Entity, current_time: f32) {
        self.last_damage_time.insert(entity, current_time);
    }
}
