use bevy::prelude::*;
use crate::components::enemy::Health;
use crate::components::hero::Hero;
use crate::components::damage_visual::{DamageVisual, DeathAnimation, MarkedForDeath};
use crate::components::physics::Velocity;
use crate::events::damage_events::{EntityDamaged, EntityDied};

pub fn handle_entity_damage(
    _commands: Commands,
    mut damage_events: MessageReader<EntityDamaged>,
    mut death_events: MessageWriter<EntityDied>,
    mut health_query: Query<&mut Health>,
    hero_query: Query<Entity, With<Hero>>,
) {
    let hero_entity = hero_query.single().ok();

    for event in damage_events.read() {
        // HERO IS INVINCIBLE - skip damage for hero
        if let Some(hero) = hero_entity {
            if event.entity == hero {
                continue; // Hero cannot take damage
            }
        }

        if let Ok(mut health) = health_query.get_mut(event.entity) {
            health.current = health.current.saturating_sub(event.damage);

            info!("Entity {:?} took {} damage! Health: {}/{}",
                event.entity, event.damage, health.current, health.max);

            if health.current == 0 {
                // Entity died!
                death_events.write(EntityDied {
                    entity: event.entity,
                });
            }
        }
    }
}

pub fn handle_entity_death(
    mut commands: Commands,
    mut death_events: MessageReader<EntityDied>,
    hero_query: Query<Entity, With<Hero>>,
    all_entities: Query<Entity>,
) {
    let hero_entity = hero_query.single().ok();

    for event in death_events.read() {
        // HERO IS INVINCIBLE - never despawn hero
        if let Some(hero) = hero_entity {
            if event.entity == hero {
                warn!("Attempted to kill hero - hero is invincible!");
                continue;
            }
        }

        // Check if entity still exists (might have been despawned by another system)
        if all_entities.get(event.entity).is_err() {
            continue; // Entity already despawned, skip
        }

        // Freeze entity and add death animation (1 second)
        commands.entity(event.entity)
            .remove::<Velocity>()  // Stop movement
            .insert(DeathAnimation::new())
            .insert(DamageVisual::new(1.0));  // Show damage visual for full 1 second

        info!("Entity {:?} died - frozen for death animation", event.entity);
    }
}

/// Update death animations and mark for despawn when finished
pub fn update_death_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DeathAnimation)>,
    time: Res<Time>,
) {
    for (entity, mut death_anim) in query.iter_mut() {
        death_anim.timer.tick(time.delta());

        if death_anim.timer.is_finished() {
            // Mark for death instead of despawning immediately
            // This allows other systems to finish processing this entity
            commands.entity(entity).insert(MarkedForDeath);
            info!("Death animation complete - marked entity {:?} for despawn", entity);
        }
    }
}

/// Cleanup system - despawns entities marked for death
/// Runs in PostUpdate to ensure all Update systems finish processing first
pub fn cleanup_marked_entities(
    mut commands: Commands,
    query: Query<Entity, With<MarkedForDeath>>,
) {
    for entity in query.iter() {
        // Despawn entity and all its children (health bars, hearts, borders)
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.despawn();
        }
        info!("Cleanup: despawned marked entity {:?}", entity);
    }
}
