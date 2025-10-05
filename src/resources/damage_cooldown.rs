use bevy::prelude::*;
use std::collections::HashMap;

/// Resource to track damage cooldown timer per entity (prevents rapid damage spam)
/// Used for: Hero taking damage, Villagers taking damage, AND blocks being damaged
#[derive(Resource, Default)]
pub struct DamageCooldown {
    pub last_damage_time: HashMap<Entity, f32>,
    pub last_attacker_damage_time: HashMap<Entity, f32>, // Track when each attacker last dealt damage
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

    /// Check if an attacker (hero/villager) can deal damage (prevents rapid multi-block damage)
    pub fn can_attacker_damage(&self, attacker: Entity, current_time: f32, cooldown_seconds: f32) -> bool {
        if let Some(&last_time) = self.last_attacker_damage_time.get(&attacker) {
            current_time - last_time >= cooldown_seconds
        } else {
            true // Never attacked before
        }
    }

    /// Record that an attacker dealt damage
    pub fn record_attacker_damage(&mut self, attacker: Entity, current_time: f32) {
        self.last_attacker_damage_time.insert(attacker, current_time);
    }
}

/// Resource to track hero's melee attack cooldown (separate from taking damage)
/// Uses per-enemy cooldown: each enemy has individual attack cooldown timer
#[derive(Resource, Default)]
pub struct MeleeAttackCooldown {
    pub last_hit_time: HashMap<Entity, f32>, // Per-enemy cooldown tracking
}

impl MeleeAttackCooldown {
    /// Check if we can damage this enemy (based on per-enemy cooldown)
    pub fn can_attack(&self, enemy_entity: Entity, current_time: f32, cooldown: f32) -> bool {
        if let Some(&last_time) = self.last_hit_time.get(&enemy_entity) {
            current_time - last_time >= cooldown
        } else {
            true // Never hit this enemy before
        }
    }

    /// Record that we just hit this enemy
    pub fn record_hit(&mut self, enemy_entity: Entity, current_time: f32) {
        self.last_hit_time.insert(enemy_entity, current_time);
    }

    /// Clean up dead enemies from tracking (optional, prevents memory leak)
    pub fn cleanup_dead(&mut self, alive_enemies: &std::collections::HashSet<Entity>) {
        self.last_hit_time.retain(|&enemy, _| alive_enemies.contains(&enemy));
    }
}
