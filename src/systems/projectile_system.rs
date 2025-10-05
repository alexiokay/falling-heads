use bevy::prelude::*;
use crate::components::projectile::{Arrow, StuckArrow};
use crate::components::physics::Velocity;
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::block::Block;
use crate::events::damage_events::EntityDamaged;
use crate::states::GameState;
use crate::resources::spatial_hash::SpatialHash;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            update_arrow_lifetime,
            check_arrow_collisions,
            update_stuck_arrows,
        ).run_if(in_state(GameState::Playing)))
        // Run arrow rotation AFTER physics updates velocity
        .add_systems(PostUpdate, update_arrow_rotation.run_if(in_state(GameState::Playing)));

        info!("ProjectilePlugin loaded");
    }
}

/// Update arrow rotation to match velocity direction (realistic arrow physics)
fn update_arrow_rotation(
    mut query: Query<(&Velocity, &mut Transform), With<Arrow>>,
) {
    for (velocity, mut transform) in query.iter_mut() {
        // Calculate angle from velocity direction
        // atan2(y, x) gives the angle where the velocity is pointing
        let angle = velocity.y.atan2(velocity.x);

        // Rotate arrow to point in direction of travel
        // Arrow texture points right (→), so no offset needed
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
            info!("ARROW DESPAWNED BY LIFETIME: entity={:?}", entity);
            commands.entity(entity).despawn();
        }
    }
}

/// Update arrows that are stuck to entities - make them follow the entity
fn update_stuck_arrows(
    mut commands: Commands,
    mut stuck_query: Query<(Entity, &mut StuckArrow, &mut Transform), With<Arrow>>,
    entity_transforms: Query<&Transform, (Without<Arrow>, Or<(With<Hero>, With<Villager>)>)>,
    time: Res<Time>,
) {
    for (arrow_entity, mut stuck, mut arrow_transform) in stuck_query.iter_mut() {
        stuck.despawn_timer.tick(time.delta());

        if stuck.despawn_timer.is_finished() {
            // Timer expired - despawn the stuck arrow
            commands.entity(arrow_entity).despawn();
            continue;
        }

        // Make arrow follow the entity it's stuck to
        if let Ok(entity_transform) = entity_transforms.get(stuck.stuck_to_entity) {
            // Update arrow position to follow the entity, maintaining the offset
            arrow_transform.translation = entity_transform.translation + stuck.offset;
        } else {
            // Entity doesn't exist anymore (died/despawned) - despawn arrow
            commands.entity(arrow_entity).despawn();
        }
    }
}

/// Check arrow collisions - arrows CANNOT go through walls!
fn check_arrow_collisions(
    mut commands: Commands,
    spatial_hash: Res<SpatialHash>,
    arrow_query: Query<(Entity, &Transform, &Arrow), Without<StuckArrow>>, // Skip arrows that are already stuck
    block_query: Query<&Transform, With<Block>>,
    hero_query: Query<(Entity, &Transform), (With<Hero>, Without<Arrow>)>,
    villager_query: Query<(Entity, &Transform), (With<Villager>, Without<Arrow>)>,
    mut damage_events: MessageWriter<EntityDamaged>,
) {
    let arrow_radius = 16.0; // 32x8 arrow, use half width
    let entity_radius = 32.0; // 64x64 entities, use half size
    let block_radius = 32.0; // 64x64 blocks, use half size

    for (arrow_entity, arrow_transform, arrow) in arrow_query.iter() {
        let arrow_pos = arrow_transform.translation;
        let mut arrow_hit_something = false;

        // Check collision with blocks using spatial hash (arrows STOP at walls)
        let nearby_blocks = spatial_hash.query_nearby(arrow_pos.x, arrow_pos.y);
        for block_entity in nearby_blocks {
            if let Ok(block_transform) = block_query.get(block_entity) {
                let block_pos = block_transform.translation;
                // Use 2D distance (ignore Z) for collision detection
                let dx = arrow_pos.x - block_pos.x;
                let dy = arrow_pos.y - block_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < (arrow_radius + block_radius) {
                    // Arrow hit wall - despawn it
                    info!("ARROW HIT BLOCK: entity={:?}, distance={:.1}, arrow_pos=({:.0},{:.0}), block_pos=({:.0},{:.0})",
                        arrow_entity, distance, arrow_pos.x, arrow_pos.y, block_pos.x, block_pos.y);
                    commands.entity(arrow_entity).despawn();
                    arrow_hit_something = true;
                    break; // This arrow is done, move to next arrow
                }
            }
        }

        if arrow_hit_something {
            continue; // Skip to next arrow
        }

        // Check collision with hero - arrows now damage hero!
        if let Ok((hero_entity, hero_transform)) = hero_query.single() {
            let hero_pos = hero_transform.translation;
            // Use 2D distance (ignore Z)
            let dx = arrow_pos.x - hero_pos.x;
            let dy = arrow_pos.y - hero_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < (arrow_radius + entity_radius) {
                // Arrow hit hero - damage and stick to show impact
                info!("Arrow hit hero at distance {:.1}", distance);
                damage_events.write(EntityDamaged {
                    entity: hero_entity,
                    damage: arrow.damage,
                });
                let offset = arrow_pos - hero_pos;
                commands.entity(arrow_entity).insert(StuckArrow {
                    despawn_timer: Timer::from_seconds(2.0, TimerMode::Once),
                    offset,
                    stuck_to_entity: hero_entity,
                });
                // Remove physics components so arrow doesn't move
                commands.entity(arrow_entity).remove::<crate::components::physics::Velocity>();
                commands.entity(arrow_entity).remove::<crate::components::physics::Gravity>();
                continue;
            }
        }

        // Check collision with villagers (villagers have Health and CAN DIE)
        for (villager_entity, villager_transform) in villager_query.iter() {
            let villager_pos = villager_transform.translation;
            // Use 2D distance (ignore Z)
            let dx = arrow_pos.x - villager_pos.x;
            let dy = arrow_pos.y - villager_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < (arrow_radius + entity_radius) {
                // Arrow hit villager - damage and stick to show impact
                info!("Arrow hit villager at distance {:.1}", distance);
                damage_events.write(EntityDamaged {
                    entity: villager_entity,
                    damage: arrow.damage,
                });
                let offset = arrow_pos - villager_pos;
                commands.entity(arrow_entity).insert(StuckArrow {
                    despawn_timer: Timer::from_seconds(2.0, TimerMode::Once),
                    offset,
                    stuck_to_entity: villager_entity,
                });
                // Remove physics components so arrow doesn't move
                commands.entity(arrow_entity).remove::<crate::components::physics::Velocity>();
                commands.entity(arrow_entity).remove::<crate::components::physics::Gravity>();
                continue;
            }
        }
    }
}
