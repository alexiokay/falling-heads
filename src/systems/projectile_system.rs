use bevy::prelude::*;
use crate::components::projectile::Arrow;
use crate::components::physics::{Velocity, Gravity};
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::block::Block;
use crate::events::damage_events::EntityDamaged;
use crate::states::GameState;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            update_arrow_rotation,
            update_arrow_lifetime,
            check_arrow_collisions,
        ).run_if(in_state(GameState::Playing)));

        info!("ProjectilePlugin loaded");
    }
}

/// Update arrow rotation to match velocity direction (realistic arrow physics)
fn update_arrow_rotation(
    mut query: Query<(&Velocity, &mut Transform), With<Arrow>>,
) {
    for (velocity, mut transform) in query.iter_mut() {
        // Calculate angle from velocity
        let angle = velocity.y.atan2(velocity.x);

        // Rotate arrow to point in direction of travel
        transform.rotation = Quat::from_rotation_z(angle);
    }
}

/// Update arrow lifetime and despawn when expired
fn update_arrow_lifetime(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Arrow)>,
    time: Res<Time>,
) {
    for (entity, mut arrow) in query.iter_mut() {
        arrow.lifetime.tick(time.delta());

        if arrow.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Check arrow collisions with blocks and entities
fn check_arrow_collisions(
    mut commands: Commands,
    arrow_query: Query<(Entity, &Transform, &Arrow), Without<Block>>,
    block_query: Query<&Transform, With<Block>>,
    hero_query: Query<(Entity, &Transform), (With<Hero>, Without<Arrow>)>,
    villager_query: Query<(Entity, &Transform), (With<Villager>, Without<Arrow>)>,
    mut damage_events: MessageWriter<EntityDamaged>,
) {
    let arrow_size = 32.0; // Arrow sprite size
    let block_size = 64.0;
    let entity_size = 64.0;

    for (arrow_entity, arrow_transform, arrow) in arrow_query.iter() {
        let arrow_pos = arrow_transform.translation;

        // Check collision with blocks
        for block_transform in block_query.iter() {
            let block_pos = block_transform.translation;
            let distance = arrow_pos.distance(block_pos);

            if distance < (arrow_size + block_size) / 2.0 {
                // Arrow hit a block - despawn it
                commands.entity(arrow_entity).despawn();
                return;
            }
        }

        // Check collision with hero
        if let Ok((hero_entity, hero_transform)) = hero_query.single() {
            let hero_pos = hero_transform.translation;
            let distance = arrow_pos.distance(hero_pos);

            if distance < (arrow_size + entity_size) / 2.0 {
                // Arrow hit hero - damage and despawn
                damage_events.write(EntityDamaged {
                    entity: hero_entity,
                    damage: arrow.damage,
                });
                commands.entity(arrow_entity).despawn();
                return;
            }
        }

        // Check collision with villagers
        for (villager_entity, villager_transform) in villager_query.iter() {
            let villager_pos = villager_transform.translation;
            let distance = arrow_pos.distance(villager_pos);

            if distance < (arrow_size + entity_size) / 2.0 {
                // Arrow hit villager - damage and despawn
                damage_events.write(EntityDamaged {
                    entity: villager_entity,
                    damage: arrow.damage,
                });
                commands.entity(arrow_entity).despawn();
                return;
            }
        }
    }
}
