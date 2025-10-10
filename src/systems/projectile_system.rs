use bevy::prelude::*;
use avian2d::prelude::*;
use crate::components::projectile::{Arrow, StuckArrow};
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
        app.add_systems(Update,
            update_arrow_lifetime.run_if(in_state(GameState::Playing))
        )
        // Run arrow systems in PostUpdate
        .add_systems(PostUpdate, (
            check_arrow_collisions,  // Manual collision detection for arrow game logic
            update_arrow_rotation,
            update_stuck_arrows,  // Run AFTER physics updates to get latest entity positions
        ).chain().run_if(in_state(GameState::Playing)));

        info!("ProjectilePlugin loaded");
    }
}

// REMOVED: apply_arrow_gravity - Now using Avian's GravityScale instead of manual gravity
// REMOVED: apply_arrow_velocity - Avian handles velocity integration automatically

/// Update arrow rotation to match velocity direction (realistic arrow physics)
fn update_arrow_rotation(
    mut query: Query<(&LinearVelocity, &mut Transform), With<Arrow>>,
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
    mut stuck_query: Query<(Entity, &mut StuckArrow), With<Arrow>>,
    entity_query: Query<Entity, Or<(With<Hero>, With<Villager>, With<Enemy>)>>,
    time: Res<Time>,
) {
    for (arrow_entity, mut stuck) in stuck_query.iter_mut() {
        stuck.despawn_timer.tick(time.delta());

        if stuck.despawn_timer.is_finished() {
            // Timer expired - despawn the stuck arrow
            commands.entity(arrow_entity).despawn();
            continue;
        }

        // Check if the entity still exists
        if entity_query.get(stuck.stuck_to_entity).is_err() {
            // Entity doesn't exist anymore (died/despawned) - despawn arrow
            commands.entity(arrow_entity).despawn();
        }
        // Arrow position is now automatically updated by Bevy's transform hierarchy (it's a child)
    }
}

/// Check arrow collisions - arrows CANNOT go through walls!
fn check_arrow_collisions(
    mut commands: Commands,
    spatial_hash: Res<SpatialHash>,
    mut arrow_query: Query<(Entity, &mut Transform, &Arrow, &LinearVelocity), (Without<StuckArrow>, Without<Block>)>, // Skip arrows that are already stuck
    block_query: Query<&Transform, With<Block>>,
    mut entities: ParamSet<(
        Query<(Entity, &Transform, &mut LinearVelocity), (With<Hero>, Without<Arrow>)>,
        Query<(Entity, &Transform, &mut LinearVelocity), (With<Villager>, Without<Arrow>)>,
        Query<(Entity, &Transform, &mut LinearVelocity), (With<Enemy>, Without<Arrow>, Without<crate::components::damage_visual::DeathAnimation>, Without<crate::components::damage_visual::MarkedForDeath>)>,
    )>,
    mut damage_events: MessageWriter<EntityDamaged>,
) {
    let arrow_tip_radius = 10.0; // Small radius for arrow tip (thin point)
    let entity_radius = 32.0; // 64x64 entities, use half size
    let block_radius = 32.0; // 64x64 blocks, use half size
    let arrow_length = 20.0; // Length offset for arrow tip (arrow center to visual tip)

    // Collect all arrow hits first (can't mutate velocity while iterating arrows)
    let mut hero_hits: Vec<(Entity, Vec3, Vec3, Vec3, Quat)> = Vec::new(); // (arrow_entity, arrow_pos, hero_pos, arrow_velocity, arrow_rotation)
    let mut villager_hits: Vec<(Entity, Entity, Vec3, Vec3, Vec3, Quat)> = Vec::new(); // (arrow_entity, villager_entity, arrow_pos, villager_pos, arrow_velocity, arrow_rotation)
    let mut enemy_hits: Vec<(Entity, Entity, Vec3, Vec3, Vec3, Quat)> = Vec::new(); // (arrow_entity, enemy_entity, arrow_pos, enemy_pos, arrow_velocity, arrow_rotation)

    for (arrow_entity, arrow_transform, arrow, arrow_velocity) in arrow_query.iter_mut() {
        let arrow_pos = arrow_transform.translation;
        let mut arrow_hit_something = false;

        // Calculate arrow tip position (arrow flies point-first along velocity direction)
        let speed = (arrow_velocity.x * arrow_velocity.x + arrow_velocity.y * arrow_velocity.y).sqrt();
        let arrow_direction = if speed > 0.0 {
            Vec2::new(arrow_velocity.x / speed, arrow_velocity.y / speed)
        } else {
            Vec2::ZERO
        };
        let arrow_tip = arrow_pos + Vec3::new(
            arrow_direction.x * arrow_length,
            arrow_direction.y * arrow_length,
            0.0
        );

        // Check collision with blocks using spatial hash (arrows STOP at walls)
        let nearby_blocks = spatial_hash.query_nearby(arrow_tip.x, arrow_tip.y);
        for block_entity in nearby_blocks {
            if let Ok(block_transform) = block_query.get(block_entity) {
                let block_pos = block_transform.translation;
                // Use 2D distance (ignore Z) for collision detection
                let dx = arrow_tip.x - block_pos.x;
                let dy = arrow_tip.y - block_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < (arrow_tip_radius + block_radius) {
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
                let dx = arrow_tip.x - hero_pos.x;
                let dy = arrow_tip.y - hero_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < (arrow_tip_radius + entity_radius) {
                    info!("Enemy arrow hit hero at distance {:.1}", distance);
                    damage_events.write(EntityDamaged {
                        entity: hero_entity,
                        damage: arrow.damage,
                    });

                    // Store hit data to apply push later (including hero position for offset calculation)
                    hero_hits.push((arrow_entity, arrow_pos, hero_pos, Vec3::new(arrow_velocity.x, arrow_velocity.y, 0.0), arrow_transform.rotation));
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
            let dx = arrow_tip.x - villager_pos.x;
            let dy = arrow_tip.y - villager_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < (arrow_tip_radius + entity_radius) {
                info!("Enemy arrow hit villager at distance {:.1}", distance);
                damage_events.write(EntityDamaged {
                    entity: villager_entity,
                    damage: arrow.damage,
                });

                // Store hit data to apply push later (including villager position for offset calculation)
                villager_hits.push((arrow_entity, villager_entity, arrow_pos, villager_pos, Vec3::new(arrow_velocity.x, arrow_velocity.y, 0.0), arrow_transform.rotation));
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
            let dx = arrow_tip.x - enemy_pos.x;
            let dy = arrow_tip.y - enemy_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < (arrow_tip_radius + entity_radius) {
                info!("Friendly arrow hit enemy at distance {:.1}", distance);
                damage_events.write(EntityDamaged {
                    entity: enemy_entity,
                    damage: arrow.damage,
                });

                // Store hit data to apply push later (including enemy position for offset calculation)
                enemy_hits.push((arrow_entity, enemy_entity, arrow_pos, enemy_pos, Vec3::new(arrow_velocity.x, arrow_velocity.y, 0.0), arrow_transform.rotation));
                break;
            }
        }
    }

    // Now apply all hero hits (can mutate velocity now that arrow iteration is done)
    if let Some((hero_entity, _hero_transform, mut hero_velocity)) = entities.p0().iter_mut().next() {
        for (arrow_entity, arrow_pos, hero_pos, arrow_velocity, arrow_rotation) in hero_hits {
            // Apply impact push force in the direction arrow is traveling
            let arrow_speed = (arrow_velocity.x * arrow_velocity.x + arrow_velocity.y * arrow_velocity.y).sqrt();
            if arrow_speed > 0.0 {
                let push_x = (arrow_velocity.x / arrow_speed) * ARROW_IMPACT_PUSH;
                let push_y = (arrow_velocity.y / arrow_speed) * ARROW_IMPACT_LIFT;
                hero_velocity.x += push_x;
                hero_velocity.y += push_y;
            }

            // Make arrow stick to hero (calculate offset from hero position)
            // Adjust offset so arrow tip penetrates entity (GLB model origin is at center/back)
            let arrow_length_offset = 20.0; // Push arrow forward along velocity to account for model origin
            let vel_speed = (arrow_velocity.x * arrow_velocity.x + arrow_velocity.y * arrow_velocity.y).sqrt();
            let direction = if vel_speed > 0.0 {
                Vec2::new(arrow_velocity.x / vel_speed, arrow_velocity.y / vel_speed)
            } else {
                Vec2::ZERO
            };
            let adjusted_arrow_pos = arrow_pos + Vec3::new(direction.x * arrow_length_offset, direction.y * arrow_length_offset, 0.0);
            let offset = adjusted_arrow_pos - hero_pos;
            if let Ok(mut arrow_cmd) = commands.get_entity(arrow_entity) {
                arrow_cmd.insert(StuckArrow {
                    despawn_timer: Timer::from_seconds(2.0, TimerMode::Once),
                    offset,
                    stuck_to_entity: hero_entity,
                });
                // Remove Avian components to stop physics
                arrow_cmd.remove::<LinearVelocity>();
                arrow_cmd.remove::<RigidBody>();
                arrow_cmd.remove::<Collider>();
                // Make arrow a child of hero - transform will update automatically
                arrow_cmd.set_parent_in_place(hero_entity);
                // Update transform to use local position (relative to parent) while preserving rotation
                arrow_cmd.insert(Transform {
                    translation: offset,
                    rotation: arrow_rotation,
                    scale: Vec3::splat(15.0), // Preserve scale
                });
            }
        }
    }

    // Now apply all villager hits
    for (arrow_entity, villager_entity, arrow_pos, villager_pos, arrow_velocity, arrow_rotation) in villager_hits {
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
            // Adjust offset so arrow tip penetrates entity (GLB model origin is at center/back)
            let arrow_length_offset = 20.0; // Push arrow forward along velocity to account for model origin
            let vel_speed = (arrow_velocity.x * arrow_velocity.x + arrow_velocity.y * arrow_velocity.y).sqrt();
            let direction = if vel_speed > 0.0 {
                Vec2::new(arrow_velocity.x / vel_speed, arrow_velocity.y / vel_speed)
            } else {
                Vec2::ZERO
            };
            let adjusted_arrow_pos = arrow_pos + Vec3::new(direction.x * arrow_length_offset, direction.y * arrow_length_offset, 0.0);
            let offset = adjusted_arrow_pos - villager_pos;
            if let Ok(mut arrow_cmd) = commands.get_entity(arrow_entity) {
                arrow_cmd.insert(StuckArrow {
                    despawn_timer: Timer::from_seconds(2.0, TimerMode::Once),
                    offset,
                    stuck_to_entity: villager_entity,
                });
                // Remove Avian components to stop physics
                arrow_cmd.remove::<LinearVelocity>();
                arrow_cmd.remove::<RigidBody>();
                arrow_cmd.remove::<Collider>();
                // Make arrow a child of villager - transform will update automatically
                arrow_cmd.set_parent_in_place(villager_entity);
                // Update transform to use local position (relative to parent) while preserving rotation
                arrow_cmd.insert(Transform {
                    translation: offset,
                    rotation: arrow_rotation,
                    scale: Vec3::splat(15.0), // Preserve scale
                });
            }
        }
    }

    // Now apply all enemy hits
    for (arrow_entity, enemy_entity, arrow_pos, enemy_pos, arrow_velocity, arrow_rotation) in enemy_hits {
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
            // Adjust offset so arrow tip penetrates entity (GLB model origin is at center/back)
            let arrow_length_offset = 20.0; // Push arrow forward along velocity to account for model origin
            let vel_speed = (arrow_velocity.x * arrow_velocity.x + arrow_velocity.y * arrow_velocity.y).sqrt();
            let direction = if vel_speed > 0.0 {
                Vec2::new(arrow_velocity.x / vel_speed, arrow_velocity.y / vel_speed)
            } else {
                Vec2::ZERO
            };
            let adjusted_arrow_pos = arrow_pos + Vec3::new(direction.x * arrow_length_offset, direction.y * arrow_length_offset, 0.0);
            let offset = adjusted_arrow_pos - enemy_pos;
            if let Ok(mut arrow_cmd) = commands.get_entity(arrow_entity) {
                arrow_cmd.insert(StuckArrow {
                    despawn_timer: Timer::from_seconds(2.0, TimerMode::Once),
                    offset,
                    stuck_to_entity: enemy_entity,
                });
                // Remove Avian components to stop physics
                arrow_cmd.remove::<LinearVelocity>();
                arrow_cmd.remove::<RigidBody>();
                arrow_cmd.remove::<Collider>();
                // Make arrow a child of enemy - transform will update automatically
                arrow_cmd.set_parent_in_place(enemy_entity);
                // Update transform to use local position (relative to parent) while preserving rotation
                arrow_cmd.insert(Transform {
                    translation: offset,
                    rotation: arrow_rotation,
                    scale: Vec3::splat(15.0), // Preserve scale
                });
            }
        }
    }
}
