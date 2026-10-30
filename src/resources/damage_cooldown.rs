use bevy::prelude::*;
use std::collections::HashMap;

/// Resource to track damage cooldown timer per entity (prevents rapid damage spam)
/// Used for: Hero taking damage, Villagers taking damage, AND blocks being damaged
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
}

/// Drops cooldown entries for despawned entities (mined blocks, dead enemies),
/// which otherwise pile up for the whole session.
pub fn prune_cooldowns(
    mut damage: ResMut<DamageCooldown>,
    mut melee: ResMut<MeleeAttackCooldown>,
    alive: Query<()>,
) {
    damage
        .last_damage_time
        .retain(|&entity, _| alive.contains(entity));
    melee
        .last_hit_time
        .retain(|&entity, _| alive.contains(entity));
}
