use bevy::prelude::*;
use crate::components::enemy::Health;
use crate::components::hero::Hero;
use crate::events::damage_events::{EntityDamaged, EntityDied};

pub fn handle_entity_damage(
    mut commands: Commands,
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

        // Despawn the dead entity
        commands.entity(event.entity).despawn();
        info!("Entity {:?} died and was removed", event.entity);
    }
}
