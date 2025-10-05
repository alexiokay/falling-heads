use bevy::prelude::*;
use crate::components::projectile::{Arrow, StuckArrow};
use crate::components::physics::Velocity;
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::enemy::Enemy;
use crate::components::block::Block;
use crate::events::damage_events::EntityDamaged;
use crate::states::GameState;
use crate::resources::spatial_hash::SpatialHash;
use crate::utils::constants::{ARROW_IMPACT_PUSH, ARROW_IMPACT_LIFT};

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            update_arrow_lifetime,
        ).run_if(in_state(GameState::Playing)))
        // Run arrow collisions in PostUpdate to avoid Velocity query conflicts
        .add_systems(PostUpdate, (
            check_arrow_collisions,
            update_arrow_rotation,
            update_stuck_arrows,  // Run AFTER physics updates to get latest entity positions
        ).chain().run_if(in_state(GameState::Playing)));

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
    entity_transforms: Query<&Transform, (Without<Arrow>, Or<(With<Hero>, With<Villager>, With<Enemy>)>)>,
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
            // Update arrow position to match entity position + original offset
            // The offset is calculated when the arrow first hits, so it stays in the same relative position
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
    arrow_query: Query<(Entity, &Transform, &Arrow, &Velocity), Without<StuckArrow>>, // Skip arrows that are already stuck
    block_query: Query<&Transform, With<Block>>,
    mut entities: ParamSet<(
        Query<(Entity, &Transform, &mut Velocity), (With<Hero>, Without<Arrow>)>,
        Query<(Entity, &Transform, &mut Velocity), (With<Villager>, Without<Arrow>)>,
        Query<(Entity, &Transform, &mut Velocity), (With<Enemy>, Without<Arrow>)>,
    )>,
    mut damage_events: MessageWriter<EntityDamaged>,
) {
    let arrow_radius = 16.0; // 32x8 arrow, use half width
    let entity_radius = 32.0; // 64x64 entities, use half size
    let block_radius = 32.0; // 64x64 blocks, use half size

    // Collect all arrow hits first (can't mutate velocity while iterating arrows)
    let mut hero_hits: Vec<(Entity, Vec3, Vec3, Vec3)> = Vec::new(); // (arrow_entity, arrow_pos, hero_pos, arrow_velocity)
    let mut villager_hits: Vec<(Entity, Entity, Vec3, Vec3, Vec3)> = Vec::new(); // (arrow_entity, villager_entity, arrow_pos, villager_pos, arrow_velocity)
    let mut enemy_hits: Vec<(Entity, Entity, Vec3, Vec3, Vec3)> = Vec::new(); // (arrow_entity, enemy_entity, arrow_pos, enemy_pos, arrow_velocity)

    for (arrow_entity, arrow_transform, arrow, arrow_velocity) in arrow_query.iter() {
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

        // Check collision with hero - skip if friendly fire
        if let Some((hero_entity, hero_transform, _)) = entities.p0().iter().next() {
            // Friendly fire prevention: friendly arrows don't hit hero
            if arrow.team == crate::components::projectile::Team::Friendly {
                // Don't hit hero with friendly arrows - skip to villagers check
            } else {
                let hero_pos = hero_transform.translation;
                let dx = arrow_pos.x - hero_pos.x;
                let dy = arrow_pos.y - hero_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < (arrow_radius + entity_radius) {
                    info!("Enemy arrow hit hero at distance {:.1}", distance);
                    damage_events.write(EntityDamaged {
                        entity: hero_entity,
                        damage: arrow.damage,
                    });

                    // Store hit data to apply push later (including hero position for offset calculation)
                    hero_hits.push((arrow_entity, arrow_pos, hero_pos, Vec3::new(arrow_velocity.x, arrow_velocity.y, 0.0)));
                    continue;
                }
            }
        }

        // Check collision with villagers - skip if friendly fire
        for (villager_entity, villager_transform, _) in entities.p1().iter() {
            // Friendly fire prevention: friendly arrows don't hit villagers
            if arrow.team == crate::components::projectile::Team::Friendly {
                continue; // Don't hit villagers with friendly arrows
            }

            let villager_pos = villager_transform.translation;
            let dx = arrow_pos.x - villager_pos.x;
            let dy = arrow_pos.y - villager_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < (arrow_radius + entity_radius) {
                info!("Enemy arrow hit villager at distance {:.1}", distance);
                damage_events.write(EntityDamaged {
                    entity: villager_entity,
                    damage: arrow.damage,
                });

                // Store hit data to apply push later (including villager position for offset calculation)
                villager_hits.push((arrow_entity, villager_entity, arrow_pos, villager_pos, Vec3::new(arrow_velocity.x, arrow_velocity.y, 0.0)));
                arrow_hit_something = true;
                break;
            }
        }

        if arrow_hit_something {
            continue; // Skip to next arrow
        }

        // Check collision with enemies - only friendly arrows hit enemies
        for (enemy_entity, enemy_transform, _) in entities.p2().iter() {
            // Only friendly arrows hit enemies (enemy arrows don't hit other enemies)
            if arrow.team == crate::components::projectile::Team::Enemy {
                continue; // Don't hit enemies with enemy arrows
            }

            let enemy_pos = enemy_transform.translation;
            let dx = arrow_pos.x - enemy_pos.x;
            let dy = arrow_pos.y - enemy_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < (arrow_radius + entity_radius) {
                info!("Friendly arrow hit enemy at distance {:.1}", distance);
                damage_events.write(EntityDamaged {
                    entity: enemy_entity,
                    damage: arrow.damage,
                });

                // Store hit data to apply push later (including enemy position for offset calculation)
                enemy_hits.push((arrow_entity, enemy_entity, arrow_pos, enemy_pos, Vec3::new(arrow_velocity.x, arrow_velocity.y, 0.0)));
                break;
            }
        }
    }

    // Now apply all hero hits (can mutate velocity now that arrow iteration is done)
    if let Some((hero_entity, _hero_transform, mut hero_velocity)) = entities.p0().iter_mut().next() {
        for (arrow_entity, arrow_pos, hero_pos, arrow_velocity) in hero_hits {
            // Apply impact push force in the direction arrow is traveling
            let arrow_speed = (arrow_velocity.x * arrow_velocity.x + arrow_velocity.y * arrow_velocity.y).sqrt();
            if arrow_speed > 0.0 {
                let push_x = (arrow_velocity.x / arrow_speed) * ARROW_IMPACT_PUSH;
                let push_y = (arrow_velocity.y / arrow_speed) * ARROW_IMPACT_LIFT;
                hero_velocity.x += push_x;
                hero_velocity.y += push_y;
            }

            // Make arrow stick to hero (calculate offset from hero position)
            // Use try_insert to gracefully handle if arrow was despawned
            let offset = arrow_pos - hero_pos;
            if let Ok(mut arrow_cmd) = commands.get_entity(arrow_entity) {
                arrow_cmd.insert(StuckArrow {
                    despawn_timer: Timer::from_seconds(2.0, TimerMode::Once),
                    offset,
                    stuck_to_entity: hero_entity,
                });
                arrow_cmd.remove::<crate::components::physics::Velocity>();
                arrow_cmd.remove::<crate::components::physics::Gravity>();
            }
        }
    }

    // Now apply all villager hits
    for (arrow_entity, villager_entity, arrow_pos, villager_pos, arrow_velocity) in villager_hits {
        if let Ok((_ent, _transform, mut villager_velocity)) = entities.p1().get_mut(villager_entity) {
            // Apply impact push force
            let arrow_speed = (arrow_velocity.x * arrow_velocity.x + arrow_velocity.y * arrow_velocity.y).sqrt();
            if arrow_speed > 0.0 {
                let push_x = (arrow_velocity.x / arrow_speed) * ARROW_IMPACT_PUSH;
                let push_y = (arrow_velocity.y / arrow_speed) * ARROW_IMPACT_LIFT;
                villager_velocity.x += push_x;
                villager_velocity.y += push_y;
            }

            // Make arrow stick to villager (calculate offset from villager position)
            // Use try_insert to gracefully handle if arrow was despawned
            let offset = arrow_pos - villager_pos;
            if let Ok(mut arrow_cmd) = commands.get_entity(arrow_entity) {
                arrow_cmd.insert(StuckArrow {
                    despawn_timer: Timer::from_seconds(2.0, TimerMode::Once),
                    offset,
                    stuck_to_entity: villager_entity,
                });
                arrow_cmd.remove::<crate::components::physics::Velocity>();
                arrow_cmd.remove::<crate::components::physics::Gravity>();
            }
        }
    }

    // Now apply all enemy hits
    for (arrow_entity, enemy_entity, arrow_pos, enemy_pos, arrow_velocity) in enemy_hits {
        if let Ok((_ent, _transform, mut enemy_velocity)) = entities.p2().get_mut(enemy_entity) {
            // Apply impact push force
            let arrow_speed = (arrow_velocity.x * arrow_velocity.x + arrow_velocity.y * arrow_velocity.y).sqrt();
            if arrow_speed > 0.0 {
                let push_x = (arrow_velocity.x / arrow_speed) * ARROW_IMPACT_PUSH;
                let push_y = (arrow_velocity.y / arrow_speed) * ARROW_IMPACT_LIFT;
                enemy_velocity.x += push_x;
                enemy_velocity.y += push_y;
            }

            // Make arrow stick to enemy (calculate offset from enemy position)
            // Use try_insert to gracefully handle if arrow was despawned
            let offset = arrow_pos - enemy_pos;
            if let Ok(mut arrow_cmd) = commands.get_entity(arrow_entity) {
                arrow_cmd.insert(StuckArrow {
                    despawn_timer: Timer::from_seconds(2.0, TimerMode::Once),
                    offset,
                    stuck_to_entity: enemy_entity,
                });
                arrow_cmd.remove::<crate::components::physics::Velocity>();
                arrow_cmd.remove::<crate::components::physics::Gravity>();
            }
        }
    }
}
