use bevy::prelude::*;
use crate::components::enemy::Health;
use crate::components::hero::Hero;
use crate::components::damage_visual::{DamageVisual, DeathAnimation};
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

/// Update death animations and despawn when finished
pub fn update_death_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DeathAnimation)>,
    time: Res<Time>,
) {
    for (entity, mut death_anim) in query.iter_mut() {
        death_anim.timer.tick(time.delta());

        if death_anim.timer.is_finished() {
            commands.entity(entity).despawn(); // Despawn children too (health bars, hearts)
            info!("Death animation complete - despawned entity {:?}", entity);
        }
    }
}
