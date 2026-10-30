use super::*;

pub(super) fn handle_enemy_damage_collisions(
    mut commands: Commands,
    mut collision_events: MessageReader<CollisionStart>,
    hero_query: Query<Entity, With<Hero>>,
    villager_query: Query<Entity, With<Villager>>,
    enemy_query: Query<&crate::components::enemy::Enemy>,
    mut damage_events: MessageWriter<EntityDamaged>,
    mut enemy_damage_cooldown: ResMut<DamageCooldown>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs();

    for event in collision_events.read() {
        let entity1 = event.collider1;
        let entity2 = event.collider2;

        // Check if friendly (hero or villager) collides with enemy
        let friendly_entity = if hero_query.contains(entity1) || villager_query.contains(entity1) {
            Some(entity1)
        } else if hero_query.contains(entity2) || villager_query.contains(entity2) {
            Some(entity2)
        } else {
            None
        };

        let enemy_entity = if enemy_query.contains(entity1) {
            Some(entity1)
        } else if enemy_query.contains(entity2) {
            Some(entity2)
        } else {
            None
        };

        // If we have both a friendly and an enemy in this collision
        if let (Some(friendly), Some(enemy)) = (friendly_entity, enemy_entity) {
            if let Ok(enemy_data) = enemy_query.get(enemy) {
                // Skeletons don't melee
                if enemy_data.enemy_type == crate::components::enemy::EnemyType::Skeleton {
                    continue;
                }

                // Check cooldown
                if !enemy_damage_cooldown.can_damage(friendly, current_time, 1.0) {
                    continue;
                }

                // Deal damage
                damage_events.write(EntityDamaged {
                    entity: friendly,
                    damage: 1,
                });

                // Apply burning buff from Pigman
                if enemy_data.enemy_type == crate::components::enemy::EnemyType::Pigman {
                    commands
                        .entity(friendly)
                        .insert(Buff::new(BuffType::Burning, 3.0));
                }

                enemy_damage_cooldown.record_damage(friendly, current_time);
            }
        }
    }
}

/// Consume Avian contact starts; apply per-enemy melee cooldowns.
pub(super) fn handle_avian_collision_melee(
    mut collision_events: MessageReader<CollisionStart>,
    hero_query: Query<(Entity, Option<&crate::components::frozen::FrozenStacks>), With<Hero>>,
    _villager_query: Query<
        (Entity, Option<&crate::components::frozen::FrozenStacks>),
        With<Villager>,
    >,
    enemy_query: Query<Entity, With<crate::components::enemy::Enemy>>,
    mut damage_events: MessageWriter<EntityDamaged>,
    mut melee_cooldown: ResMut<MeleeAttackCooldown>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs();

    for event in collision_events.read() {
        let entity1 = event.collider1;
        let entity2 = event.collider2;

        // Check if hero collides with enemy
        if let Ok((_hero, frozen)) = hero_query.get(entity1) {
            // Skip damage if hero is fully frozen
            if let Some(frozen_stacks) = frozen {
                if frozen_stacks.is_fully_frozen() {
                    continue;
                }
            }

            if let Ok(enemy) = enemy_query.get(entity2) {
                if melee_cooldown.can_attack(enemy, current_time, HERO_MELEE_COOLDOWN) {
                    info!("⚔️ HERO MELEE HIT (collision)! enemy={:?}", enemy);
                    damage_events.write(EntityDamaged {
                        entity: enemy,
                        damage: HERO_MELEE_DAMAGE,
                    });
                    melee_cooldown.record_hit(enemy, current_time);
                }
            }
        }
        // Check reverse (enemy collides with hero)
        if let Ok((_hero, frozen)) = hero_query.get(entity2) {
            // Skip damage if hero is fully frozen
            if let Some(frozen_stacks) = frozen {
                if frozen_stacks.is_fully_frozen() {
                    continue;
                }
            }

            if let Ok(enemy) = enemy_query.get(entity1) {
                if melee_cooldown.can_attack(enemy, current_time, HERO_MELEE_COOLDOWN) {
                    info!("⚔️ HERO MELEE HIT (collision)! enemy={:?}", enemy);
                    damage_events.write(EntityDamaged {
                        entity: enemy,
                        damage: HERO_MELEE_DAMAGE,
                    });
                    melee_cooldown.record_hit(enemy, current_time);
                }
            }
        }
    }
}
