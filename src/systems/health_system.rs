use crate::components::buff::Buff;
use crate::components::damage_visual::{DamageVisual, DeathAnimation, MarkedForDeath};
use crate::components::enemy::Health;
use crate::components::hero::Hero;
use crate::events::damage_events::{EntityDamaged, EntityDied};
use avian3d::prelude::{CollisionLayers, GravityScale, LinearVelocity, RigidBody};
use bevy::prelude::*;

pub fn handle_entity_damage(
    _commands: Commands,
    mut damage_events: MessageReader<EntityDamaged>,
    mut death_events: MessageWriter<EntityDied>,
    mut health_query: Query<&mut Health>,
    hero_query: Query<Entity, With<Hero>>,
    encounter_query: Query<&crate::systems::biome_encounters::EncounterFoe>,
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
            if health.current == 0 {
                continue;
            } // One death/reward event per actor.
            let damage = encounter_query
                .get(event.entity)
                .map_or(event.damage, |foe| foe.received_damage(event.damage));
            health.current = health.current.saturating_sub(damage);

            info!(
                "Entity {:?} took {} damage! Health: {}/{}",
                event.entity, event.damage, health.current, health.max
            );

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
    all_entities: Query<Has<DeathAnimation>>,
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

        // Skip despawned entities and ones already dying (no second death, XP or timer reset)
        if !matches!(all_entities.get(event.entity), Ok(false)) {
            continue;
        }

        // Freeze entity completely by making it Static (non-interactive, no physics)
        // Remove all physics components and buffs to make entity completely frozen
        commands
            .entity(event.entity)
            .insert(RigidBody::Static) // Make static = no forces, no collisions, completely frozen
            .insert(DeathAnimation::new())
            .insert(DamageVisual::new(1.0)) // Show damage visual for full 1 second (red tint)
            .remove::<LinearVelocity>() // Remove velocity
            .remove::<GravityScale>() // Remove gravity
            .insert(CollisionLayers::NONE) // Missing layers restore active defaults; disable explicitly.
            .remove::<crate::components::buff::GrowthPower>()
            .remove::<Buff>(); // Remove all buffs - dead entities shouldn't have active effects

        info!(
            "Entity {:?} died - frozen as static for death animation",
            event.entity
        );
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
            info!(
                "Death animation complete - marked entity {:?} for despawn",
                entity
            );
        }
    }
}

/// Cleanup system - despawns entities marked for death
/// Runs in PostUpdate to ensure all Update systems finish processing first
pub fn cleanup_marked_entities(mut commands: Commands, query: Query<Entity, With<MarkedForDeath>>) {
    for entity in query.iter() {
        // Despawn entity and all its children (health bars, hearts, borders)
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.despawn();
        }
        info!("Cleanup: despawned marked entity {:?}", entity);
    }
}

#[cfg(test)]
mod regression {
    use super::*;
    #[test]
    fn multiple_lethal_hits_emit_only_one_death_reward_event() {
        let mut app = App::new();
        app.add_message::<EntityDamaged>()
            .add_message::<EntityDied>()
            .add_systems(Update, handle_entity_damage);
        let e = app.world_mut().spawn(Health::new(1)).id();
        for _ in 0..3 {
            app.world_mut().write_message(EntityDamaged {
                entity: e,
                damage: 2,
            });
        }
        app.update();
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<EntityDied>>()
                .drain()
                .count(),
            1
        );
    }
}

#[cfg(test)]
mod death_collision_tests {
    use super::*;
    #[test]
    fn dying_heads_disable_collision_instead_of_restoring_default_layers() {
        let mut app = App::new();
        app.add_message::<EntityDied>()
            .add_systems(Update, handle_entity_death);
        let e = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                CollisionLayers::ALL,
                LinearVelocity(Vec3::X),
                GravityScale(1.),
            ))
            .id();
        app.world_mut().write_message(EntityDied { entity: e });
        app.update();
        assert_eq!(
            *app.world().get::<CollisionLayers>(e).unwrap(),
            CollisionLayers::NONE
        );
        assert!(app.world().get::<DeathAnimation>(e).is_some());
        assert!(app.world().get::<LinearVelocity>(e).is_none());
    }
}
