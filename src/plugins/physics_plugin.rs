use bevy::prelude::*;
use crate::components::physics::{Velocity, Gravity, Bouncy};
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::block::Block;
use crate::components::explosive::Tnt;
use crate::events::block_events::BlockDamaged;
use crate::states::GameState;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                apply_gravity,
                apply_velocity,
                check_collisions,
                check_villager_collisions,
                check_tnt_collisions,
                check_entity_collisions,
            ).chain().run_if(in_state(GameState::Playing)));

        info!("PhysicsPlugin loaded");
    }
}

fn apply_gravity(
    time: Res<Time>,
    mut query: Query<(&Gravity, &mut Velocity)>,
) {
    for (gravity, mut velocity) in query.iter_mut() {
        velocity.y += gravity.force * time.delta_secs();
    }
}

fn apply_velocity(
    time: Res<Time>,
    mut query: Query<(&Velocity, &mut Transform)>,
) {
    for (velocity, mut transform) in query.iter_mut() {
        transform.translation.x += velocity.x * time.delta_secs();
        transform.translation.y += velocity.y * time.delta_secs();
    }
}

fn check_collisions(
    mut hero_query: Query<(&mut Transform, &mut Velocity, &Bouncy), (With<Hero>, Without<Block>)>,
    block_query: Query<(Entity, &Transform), (With<Block>, Without<Hero>)>,
    mut damage_events: MessageWriter<BlockDamaged>,
) {
    let Ok((mut hero_transform, mut hero_velocity, bouncy)) = hero_query.single_mut() else {
        return;
    };

    let hero_pos = hero_transform.translation;
    let hero_size = 64.0;  // 2D sprite size (64x64)
    let block_size = 64.0;  // 3D blocks are 1.0 unit cubes scaled by 64.0, so total size is 64.0

    for (block_entity, block_transform) in block_query.iter() {
        let block_pos = block_transform.translation;

        let hero_half = hero_size / 2.0;
        let block_half = block_size / 2.0;

        let dx = hero_pos.x - block_pos.x;
        let dy = hero_pos.y - block_pos.y;

        let overlap_x = hero_half + block_half - dx.abs();
        let overlap_y = hero_half + block_half - dy.abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            // Collision detected! Damage block on ANY collision
            damage_events.write(BlockDamaged {
                entity: block_entity,
                damage: 1,
                remaining_health: 0, // Will be updated in damage system
            });

            // Resolve based on smallest overlap - push entity out completely
            if overlap_x < overlap_y {
                // Resolve horizontally - add buffer to prevent re-collision
                let separation = overlap_x + 1.0;
                if dx > 0.0 {
                    hero_transform.translation.x += separation;
                } else {
                    hero_transform.translation.x -= separation;
                }
                hero_velocity.x = -hero_velocity.x * 0.5;
            } else {
                // Resolve vertically - larger buffer to prevent penetration
                let separation = overlap_y + 1.0;
                if dy > 0.0 {
                    // Hero is above block - push upward
                    hero_transform.translation.y += separation;
                    if hero_velocity.y < 0.0 {
                        hero_velocity.y = -hero_velocity.y * bouncy.restitution;

                        // Add horizontal randomness on bounce
                        use rand::Rng;
                        let mut rng = rand::thread_rng();
                        let horizontal_bounce = rng.gen_range(-50.0..50.0);
                        hero_velocity.x += horizontal_bounce;
                    }
                } else {
                    // Hero is below block - push downward
                    hero_transform.translation.y -= separation;
                    if hero_velocity.y > 0.0 {
                        hero_velocity.y = 0.0;
                    }
                }
            }
        }
    }
}

// Villagers vs Blocks collision
fn check_villager_collisions(
    mut villager_query: Query<(&mut Transform, &mut Velocity, &Bouncy), (With<Villager>, Without<Block>)>,
    block_query: Query<(Entity, &Transform), (With<Block>, Without<Villager>)>,
    mut damage_events: MessageWriter<BlockDamaged>,
) {
    for (mut villager_transform, mut villager_velocity, bouncy) in villager_query.iter_mut() {
        let villager_pos = villager_transform.translation;
        let villager_size = 56.0;  // 2D scale: matches sprite size (56x56)
        let block_size = 64.0;  // 2D scale

        for (block_entity, block_transform) in block_query.iter() {
            let block_pos = block_transform.translation;

            let villager_half = villager_size / 2.0;
            let block_half = block_size / 2.0;

            let dx = villager_pos.x - block_pos.x;
            let dy = villager_pos.y - block_pos.y;

            let overlap_x = villager_half + block_half - dx.abs();
            let overlap_y = villager_half + block_half - dy.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                // Collision detected! Damage block on ANY collision
                damage_events.write(BlockDamaged {
                    entity: block_entity,
                    damage: 1,
                    remaining_health: 0,
                });

                // Resolve based on smallest overlap - push entity out completely
                if overlap_x < overlap_y {
                    // Resolve horizontally - add buffer to prevent re-collision
                    let separation = overlap_x + 1.0;
                    if dx > 0.0 {
                        villager_transform.translation.x += separation;
                    } else {
                        villager_transform.translation.x -= separation;
                    }
                    villager_velocity.x = -villager_velocity.x * 0.5;
                } else {
                    // Resolve vertically - larger buffer to prevent penetration
                    let separation = overlap_y + 1.0;
                    if dy > 0.0 {
                        // Villager is above block - push upward
                        villager_transform.translation.y += separation;
                        if villager_velocity.y < 0.0 {
                            villager_velocity.y = -villager_velocity.y * bouncy.restitution;

                            // Add horizontal randomness on bounce
                            use rand::Rng;
                            let mut rng = rand::thread_rng();
                            let horizontal_bounce = rng.gen_range(-50.0..50.0);
                            villager_velocity.x += horizontal_bounce;
                        }
                    } else {
                        // Villager is below block - push downward
                        villager_transform.translation.y -= separation;
                        if villager_velocity.y > 0.0 {
                            villager_velocity.y = 0.0;
                        }
                    }
                }
            }
        }
    }
}

// TNT vs Blocks collision
fn check_tnt_collisions(
    mut tnt_query: Query<(&mut Transform, &mut Velocity, &Bouncy), (With<Tnt>, Without<Block>)>,
    block_query: Query<&Transform, (With<Block>, Without<Tnt>)>,
) {
    for (mut tnt_transform, mut tnt_velocity, bouncy) in tnt_query.iter_mut() {
        let tnt_pos = tnt_transform.translation;
        let tnt_size = 32.0;  // 2D scale
        let block_size = 64.0;  // 2D scale

        for block_transform in block_query.iter() {
            let block_pos = block_transform.translation;

            let tnt_half = tnt_size / 2.0;
            let block_half = block_size / 2.0;

            let dx = tnt_pos.x - block_pos.x;
            let dy = tnt_pos.y - block_pos.y;

            let overlap_x = tnt_half + block_half - dx.abs();
            let overlap_y = tnt_half + block_half - dy.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                // Collision detected! Resolve based on smallest overlap
                if overlap_x < overlap_y {
                    // Resolve horizontally
                    if dx > 0.0 {
                        tnt_transform.translation.x += overlap_x;
                    } else {
                        tnt_transform.translation.x -= overlap_x;
                    }
                    tnt_velocity.x = -tnt_velocity.x * 0.5;
                } else {
                    // Resolve vertically
                    if dy > 0.0 {
                        // TNT is above block
                        tnt_transform.translation.y += overlap_y;
                        if tnt_velocity.y < 0.0 {
                            tnt_velocity.y = -tnt_velocity.y * bouncy.restitution;

                            // Add horizontal randomness on bounce
                            use rand::Rng;
                            let mut rng = rand::thread_rng();
                            let horizontal_bounce = rng.gen_range(-50.0..50.0);
                            tnt_velocity.x += horizontal_bounce;
                        }
                    } else {
                        // TNT is below block
                        tnt_transform.translation.y -= overlap_y;
                        if tnt_velocity.y > 0.0 {
                            tnt_velocity.y = 0.0;
                        }
                    }
                }
            }
        }
    }
}

// Entity-to-entity collision (Hero and Villagers bounce off each other)
fn check_entity_collisions(
    mut entities: Query<(Entity, &mut Transform, &mut Velocity, &Bouncy), Or<(With<Hero>, With<Villager>)>>,
) {
    let mut combinations = entities.iter_combinations_mut();

    while let Some([(_entity_a, mut transform_a, mut velocity_a, bouncy_a), (_entity_b, mut transform_b, mut velocity_b, bouncy_b)]) = combinations.fetch_next() {
        let pos_a = transform_a.translation;
        let pos_b = transform_b.translation;

        // Use actual sprite sizes for tighter collision
        let size_a = 64.0; // Hero sprite size (64x64)
        let size_b = 56.0; // Villager sprite size (56x56)

        let half_a = size_a / 2.0;
        let half_b = size_b / 2.0;

        let dx = pos_a.x - pos_b.x;
        let dy = pos_a.y - pos_b.y;

        let overlap_x = half_a + half_b - dx.abs();
        let overlap_y = half_a + half_b - dy.abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            // Collision detected! Fully separate entities with buffer
            if overlap_x < overlap_y {
                // Separate horizontally - split the overlap evenly
                let separation = (overlap_x / 2.0) + 0.5;
                if dx > 0.0 {
                    transform_a.translation.x += separation;
                    transform_b.translation.x -= separation;
                } else {
                    transform_a.translation.x -= separation;
                    transform_b.translation.x += separation;
                }

                // Bounce velocities with damping
                let avg_vx = (velocity_a.x + velocity_b.x) * 0.5;
                velocity_a.x = (velocity_a.x - avg_vx) * -0.8 + avg_vx;
                velocity_b.x = (velocity_b.x - avg_vx) * -0.8 + avg_vx;
            } else {
                // Separate vertically - split the overlap evenly
                let separation = (overlap_y / 2.0) + 0.5;
                if dy > 0.0 {
                    transform_a.translation.y += separation;
                    transform_b.translation.y -= separation;
                } else {
                    transform_a.translation.y -= separation;
                    transform_b.translation.y += separation;
                }

                // Bounce velocities with restitution
                let avg_vy = (velocity_a.y + velocity_b.y) * 0.5;
                velocity_a.y = (velocity_a.y - avg_vy) * -bouncy_a.restitution + avg_vy;
                velocity_b.y = (velocity_b.y - avg_vy) * -bouncy_b.restitution + avg_vy;
            }
        }
    }
}
