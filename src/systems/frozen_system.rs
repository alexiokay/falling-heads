use bevy::prelude::*;
use crate::components::frozen::{FrozenStacks, FreezeOverlay};
use crate::components::projectile::Snowball;
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::physics::Velocity;
use crate::components::block::Block;
use crate::resources::spatial_hash::SpatialHash;
use crate::states::GameState;

pub struct FrozenPlugin;

impl Plugin for FrozenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            snowball_collision_system,
            frozen_stacks_update_system,
            apply_freeze_movement_penalty,
        ).run_if(in_state(GameState::Playing)));

        info!("FrozenPlugin loaded");
    }
}

/// Detect snowball collisions with hero/villagers and apply freeze stacks
fn snowball_collision_system(
    mut commands: Commands,
    spatial_hash: Res<SpatialHash>,
    snowball_query: Query<(Entity, &Transform), With<Snowball>>,
    block_query: Query<&Transform, With<Block>>,
    mut target_query: Query<(Entity, &Transform, Option<&mut FrozenStacks>, &mut Velocity), Or<(With<Hero>, With<Villager>)>>,
) {
    const SNOWBALL_HIT_RADIUS: f32 = 40.0;  // Collision radius for entities
    const SNOWBALL_RADIUS: f32 = 12.0;      // Snowball size (24x24, use half)
    const BLOCK_RADIUS: f32 = 32.0;         // Block size (64x64, use half)
    const PUSH_FORCE: f32 = 50.0;           // Small push on hit

    for (snowball_entity, snowball_transform) in snowball_query.iter() {
        let snowball_pos = snowball_transform.translation;
        let mut snowball_hit_something = false;

        // Check collision with blocks using spatial hash (snowballs STOP at walls like arrows)
        let nearby_blocks = spatial_hash.query_nearby(snowball_pos.x, snowball_pos.y);
        for block_entity in nearby_blocks {
            if let Ok(block_transform) = block_query.get(block_entity) {
                let block_pos = block_transform.translation;
                // Use 2D distance (ignore Z) for collision detection
                let dx = snowball_pos.x - block_pos.x;
                let dy = snowball_pos.y - block_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < (SNOWBALL_RADIUS + BLOCK_RADIUS) {
                    // Snowball hit wall - despawn it (just like arrows)
                    info!("SNOWBALL HIT BLOCK: entity={:?}, distance={:.1}, snowball_pos=({:.0},{:.0}), block_pos=({:.0},{:.0})",
                        snowball_entity, distance, snowball_pos.x, snowball_pos.y, block_pos.x, block_pos.y);
                    commands.entity(snowball_entity).despawn();
                    snowball_hit_something = true;
                    break; // This snowball is done, move to next snowball
                }
            }
        }

        if snowball_hit_something {
            continue; // Skip to next snowball
        }

        // Check collision with hero/villagers
        for (target_entity, target_transform, frozen_stacks, mut velocity) in target_query.iter_mut() {
            let target_pos = target_transform.translation;
            let distance = snowball_pos.distance(target_pos);

            if distance < SNOWBALL_HIT_RADIUS {
                // Hit! Despawn snowball
                commands.entity(snowball_entity).despawn();

                // Add freeze stack
                if let Some(mut stacks) = frozen_stacks {
                    stacks.add_stack();
                    info!("Snowball hit! Freeze stacks: {}/6", stacks.stack_count());
                } else {
                    // First hit - add FrozenStacks component
                    let mut new_stacks = FrozenStacks::new();
                    new_stacks.add_stack();
                    commands.entity(target_entity).insert(new_stacks);
                    info!("First snowball hit! Freeze stack: 1/6");
                }

                // Small push back effect
                let push_direction = (target_pos - snowball_pos).normalize();
                velocity.x += push_direction.x * PUSH_FORCE;
                velocity.y += push_direction.y * PUSH_FORCE;

                break;  // Snowball can only hit one target
            }
        }
    }
}

/// Update frozen stack timers and remove expired stacks
fn frozen_stacks_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut frozen_query: Query<(Entity, &mut FrozenStacks)>,
) {
    for (entity, mut frozen) in frozen_query.iter_mut() {
        // Tick all stack timers
        frozen.stacks.retain_mut(|stack| {
            stack.timer.tick(time.delta());
            !stack.timer.finished()  // Keep stacks that haven't expired
        });

        // If all stacks expired, remove FrozenStacks component
        if frozen.stacks.is_empty() {
            commands.entity(entity).remove::<FrozenStacks>();
            info!("All freeze stacks expired");
        }
    }
}

/// Apply movement speed penalty based on freeze level
fn apply_freeze_movement_penalty(
    mut frozen_query: Query<(&FrozenStacks, &mut Velocity), Or<(With<Hero>, With<Villager>)>>,
) {
    for (frozen, mut velocity) in frozen_query.iter_mut() {
        let speed_mult = frozen.speed_multiplier();

        // Apply freeze slowdown
        velocity.x *= speed_mult;
        velocity.y *= speed_mult;

        // If fully frozen (6 stacks), stop completely
        if frozen.is_fully_frozen() {
            velocity.x = 0.0;
            velocity.y = 0.0;
        }
    }
}

/// Update snowball lifetime and despawn expired ones
pub fn update_snowballs(
    mut commands: Commands,
    time: Res<Time>,
    mut snowball_query: Query<(Entity, &mut Snowball)>,
) {
    for (entity, mut snowball) in snowball_query.iter_mut() {
        snowball.lifetime.tick(time.delta());

        if snowball.lifetime.finished() {
            commands.entity(entity).despawn();
        }
    }
}
