use bevy::prelude::*;
use crate::components::physics::{Velocity, Gravity, Bouncy};
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::block::{Block, BlockType};
use crate::components::explosive::Tnt;
use crate::components::enemy::Enemy;
use crate::components::resource::Resource;
use crate::components::buff::{Buff, BuffType};
use crate::components::chunk::ChunkMember;
use crate::events::block_events::BlockDamaged;
use crate::events::damage_events::EntityDamaged;
use crate::resources::damage_cooldown::{DamageCooldown, MeleeAttackCooldown};
use crate::resources::spatial_hash::SpatialHash;
use crate::systems::buffs::buff_core::apply_block_buff;
use crate::states::GameState;
use crate::utils::constants::*;
use avian3d::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhysicsSet {
    Forces,      // Apply gravity and forces
    AI,          // Enemy AI movement
    Movement,    // Update positions
    Collision,   // Detect and resolve collisions
}

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<DamageCooldown>()
            .init_resource::<MeleeAttackCooldown>()
            .insert_resource(SpatialHash::new(256.0)) // 256 units per spatial chunk (4 blocks)
            .configure_sets(Update, (
                PhysicsSet::Forces,
                PhysicsSet::AI,
                PhysicsSet::Movement,
                PhysicsSet::Collision,
            ).chain().run_if(in_state(GameState::Playing)))
            // DISABLED: apply_gravity - Avian handles gravity via GravityScale
            // DISABLED: apply_velocity - Avian handles velocity integration
            .add_systems(Update, (
                hero_ai_movement,
                villager_ai_movement,
                enemy_ai_movement,
            ).in_set(PhysicsSet::AI))
            // Keep spatial hash - blocks use custom spatial hash (faster than Avian for 15k+ static blocks)
            .add_systems(Update,
                rebuild_spatial_hash.in_set(PhysicsSet::Collision)
            )
            .add_systems(Update, (
                check_all_entity_block_collisions,  // Blocks use custom spatial hash, not Avian
                check_tnt_block_collisions,  // TNT needs block collision
                check_resource_collisions,  // Resources need block collision
                handle_avian_collision_melee,  // Hero melee damage via Avian collision events
                handle_enemy_damage_collisions,  // Enemy damage to hero/villagers via Avian collision events
                clamp_entity_velocities,  // Prevent extreme velocities from cumulative collision forces
            ).in_set(PhysicsSet::Collision).after(rebuild_spatial_hash));

        info!("PhysicsPlugin loaded");
    }
}

// REMOVED: apply_gravity - Avian handles this via GravityScale
// REMOVED: apply_velocity - Avian handles this via LinearVelocity integration

/// TNT vs Blocks collision - simpler version that just stops TNT when it hits blocks
fn check_tnt_block_collisions(
    mut tnt_query: Query<(&mut Transform, &mut LinearVelocity), With<Tnt>>,
    block_query: Query<&Transform, (With<Block>, Without<Tnt>)>,
) {
    for (mut tnt_transform, mut tnt_velocity) in tnt_query.iter_mut() {
        let tnt_pos = tnt_transform.translation;
        let tnt_size = BLOCK_SIZE;
        let block_size = BLOCK_SIZE;

        for block_transform in block_query.iter() {
            let block_pos = block_transform.translation;

            let tnt_half = tnt_size / 2.0;
            let block_half = block_size / 2.0;

            let dx = tnt_pos.x - block_pos.x;
            let dy = tnt_pos.y - block_pos.y;

            let overlap_x = tnt_half + block_half - dx.abs();
            let overlap_y = tnt_half + block_half - dy.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                // Collision resolution
                if overlap_x < overlap_y {
                    // Horizontal
                    let separation = overlap_x + 0.5;
                    if dx > 0.0 {
                        tnt_transform.translation.x += separation;
                    } else {
                        tnt_transform.translation.x -= separation;
                    }
                    tnt_velocity.x = 0.0;
                } else {
                    // Vertical
                    let separation = overlap_y + 0.5;
                    if dy > 0.0 {
                        tnt_transform.translation.y += separation;
                        if tnt_velocity.y < 0.0 {
                            tnt_velocity.y = 0.0;
                            tnt_velocity.x *= 0.9; // Friction
                        }
                    } else {
                        tnt_transform.translation.y -= separation;
                        if tnt_velocity.y > 0.0 {
                            tnt_velocity.y = 0.0;
                        }
                    }
                }
            }
        }
    }
}

/// Rebuild spatial hash - OPTIMIZED to only rebuild every 10 frames instead of every frame
/// Blocks are static so we don't need to rebuild constantly
/// Entities use Avian's collision detection, blocks use spatial hash since they don't have Avian colliders
fn rebuild_spatial_hash(
    mut spatial_hash: ResMut<SpatialHash>,
    block_query: Query<(Entity, &Transform), With<Block>>,
    time: Res<Time>,
    mut frame_counter: Local<u32>,
) {
    *frame_counter += 1;

    // Only rebuild every 10 frames (blocks are static, no need to rebuild constantly)
    if *frame_counter % 10 != 0 {
        return;
    }

    spatial_hash.clear();

    let mut block_count = 0;

    // Add blocks to spatial hash (blocks don't use Avian physics)
    for (entity, transform) in block_query.iter() {
        let pos = transform.translation;
        spatial_hash.insert(entity, pos.x, pos.y);
        block_count += 1;
    }

    // Log every 5 seconds to monitor performance
    if time.elapsed_secs() % 5.0 < time.delta_secs() {
        info!("🗺️ Spatial hash rebuilt: {} blocks (every 10 frames for performance)", block_count);
    }
}

/// ALL entities (Hero + Villagers + Enemies) collision - unified physics
fn check_all_entity_block_collisions(
    mut commands: Commands,
    spatial_hash: Res<SpatialHash>,
    mut entity_query: Query<(Entity, &mut Transform, &mut LinearVelocity, &Restitution, Option<&Enemy>, Option<&crate::components::lootbox::RandomBox>), (Or<(With<Hero>, With<Villager>, With<Enemy>, With<crate::components::lootbox::RandomBox>)>, Without<Block>, Without<crate::components::enemy::Flying>)>,
    block_query: Query<(Entity, &Transform, &BlockType, &ChunkMember), With<Block>>,
    chunk_query: Query<&crate::components::chunk::Chunk>,
    mut buff_query: Query<&mut crate::components::buff::Buff>,
    mut damage_events: MessageWriter<BlockDamaged>,
    mut block_cooldown: ResMut<DamageCooldown>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs();
    for (entity, mut transform, mut velocity, restitution, is_enemy, is_randombox) in entity_query.iter_mut() {
        let pos = transform.translation;
        // Use actual scaled size (supports Grow buff - scale changes via Transform)
        let base_size = if is_randombox.is_some() {
            48.0 // RANDOMBOX_SIZE
        } else {
            HERO_SIZE
        };
        let entity_size = base_size * transform.scale.x;  // Apply scale from Transform (e.g., 64 * 1.2 = 76.8 for Grow buff)
        let block_size = BLOCK_SIZE;

        let nearby_blocks = spatial_hash.query_nearby(pos.x, pos.y);

        // Find the CLOSEST colliding block to prevent multi-block damage bug
        let mut closest_collision: Option<(Entity, f32, Vec3, BlockType, ChunkMember)> = None;

        for block_entity in nearby_blocks {
            let Ok((_, block_transform, block_type, chunk_member)) = block_query.get(block_entity) else {
                continue;
            };
            let block_pos = block_transform.translation;

            let entity_half = entity_size / 2.0;
            let block_half = block_size / 2.0;

            let dx = pos.x - block_pos.x;
            let dy = pos.y - block_pos.y;

            let overlap_x = entity_half + block_half - dx.abs();
            let overlap_y = entity_half + block_half - dy.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                // Calculate distance to find closest block
                let distance = (dx * dx + dy * dy).sqrt();

                if closest_collision.is_none() || distance < closest_collision.as_ref().unwrap().1 {
                    closest_collision = Some((block_entity, distance, block_pos, *block_type, *chunk_member));
                }
            }
        }

        // Only process collision with the CLOSEST block
        if let Some((block_entity, _, block_pos, block_type, chunk_member)) = closest_collision {
            // Skip collision with cobwebs - they're non-solid (slow effect handled separately)
            if block_type == BlockType::Cobweb {
                continue;
            }

            let entity_half = entity_size / 2.0;
            let block_half = block_size / 2.0;

            let dx = pos.x - block_pos.x;
            let dy = pos.y - block_pos.y;

            let overlap_x = entity_half + block_half - dx.abs();
            let overlap_y = entity_half + block_half - dy.abs();

            // Only friendlies (not enemies, not randombox) damage blocks and get buffs
            if is_enemy.is_none() && is_randombox.is_none() {
                // Check BOTH per-block AND per-attacker cooldown (prevents rapid multi-block damage)
                if block_cooldown.can_damage(block_entity, current_time, BLOCK_DAMAGE_COOLDOWN)
                    && block_cooldown.can_attacker_damage(entity, current_time, BLOCK_DAMAGE_COOLDOWN) {
                    damage_events.write(BlockDamaged {
                        entity: block_entity,
                        damage: 1,
                        remaining_health: 0,
                    });
                    block_cooldown.record_damage(block_entity, current_time);
                    block_cooldown.record_attacker_damage(entity, current_time); // Track attacker cooldown
                }

                if let Some(chunk) = chunk_query.iter().find(|c| c.chunk_x == chunk_member.chunk_x && c.chunk_y == chunk_member.chunk_y) {
                    // Query for existing buff to enable stacking
                    if let Ok(mut existing_buff) = buff_query.get_mut(entity) {
                        apply_block_buff(&mut commands, entity, block_type, chunk.biome, Some(&mut existing_buff));
                    } else {
                        apply_block_buff(&mut commands, entity, block_type, chunk.biome, None);
                    }
                }
            }

            // Collision resolution (SAME for all entities)
            if overlap_x < overlap_y {
                // Horizontal - bounce with same physics as vertical
                let separation = overlap_x + 1.0;
                if dx > 0.0 {
                    transform.translation.x += separation;
                } else {
                    transform.translation.x -= separation;
                }
                // Apply restitution to horizontal bounces (same as vertical)
                velocity.x = -velocity.x * restitution.coefficient;
                // Clamp horizontal bounce to prevent infinite acceleration
                velocity.x = velocity.x.clamp(-MAX_BOUNCE_VELOCITY, MAX_BOUNCE_VELOCITY);
            } else {
                // Vertical
                let separation = overlap_y + 1.0;
                if dy > 0.0 {
                    transform.translation.y += separation;
                    if velocity.y < 0.0 {
                        velocity.y = -velocity.y * restitution.coefficient;
                        velocity.y = velocity.y.clamp(MIN_BOUNCE_VELOCITY, MAX_BOUNCE_VELOCITY);
                    }
                } else {
                    transform.translation.y -= separation;
                    if velocity.y > 0.0 {
                        velocity.y = 0.0;
                    }
                }
            }
        }
    }
}

// REMOVED: check_enemy_block_collisions - now unified with check_all_entity_block_collisions

// REMOVED: check_tnt_collisions - Avian handles TNT physics
// REMOVED: check_resource_collisions - Avian handles resource physics
// REMOVED: check_hero_tnt_push - Avian handles entity-entity collisions
// REMOVED: check_entity_collisions - Avian handles entity-entity collisions
// REMOVED: check_hero_melee_damage - Will use Avian collision events
// REMOVED: check_hero_enemy_collision - Will use Avian collision events

// TODO: Re-implement gameplay collision handlers using Avian's CollisionStarted/CollisionEnded events

fn _disabled_check_enemy_block_collisions(
    spatial_hash: Res<SpatialHash>,
    mut enemy_query: Query<(&mut Transform, &mut Velocity, &Bouncy), (With<Enemy>, Without<Block>)>,
    block_query: Query<&Transform, With<Block>>,
) {
    for (mut transform, mut velocity, bouncy) in enemy_query.iter_mut() {
        let pos = transform.translation;
        let entity_size = ENEMY_SIZE;
        let block_size = BLOCK_SIZE;

        let nearby_blocks = spatial_hash.query_nearby(pos.x, pos.y);

        for block_entity in nearby_blocks {
            let Ok(block_transform) = block_query.get(block_entity) else {
                continue;
            };
            let block_pos = block_transform.translation;

            let entity_half = entity_size / 2.0;
            let block_half = block_size / 2.0;

            let dx = pos.x - block_pos.x;
            let dy = pos.y - block_pos.y;

            let overlap_x = entity_half + block_half - dx.abs();
            let overlap_y = entity_half + block_half - dy.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                // Collision resolution - uses Bouncy component (same as friendlies)
                if overlap_x < overlap_y {
                    // Horizontal
                    let separation = overlap_x + 1.0;
                    if dx > 0.0 {
                        transform.translation.x += separation;
                    } else {
                        transform.translation.x -= separation;
                    }
                    velocity.x = -velocity.x * 0.8;
                } else {
                    // Vertical
                    let separation = overlap_y + 1.0;
                    if dy > 0.0 {
                        transform.translation.y += separation;
                        if velocity.y < 0.0 {
                            velocity.y = -velocity.y * bouncy.restitution;
                            velocity.y = velocity.y.clamp(MIN_BOUNCE_VELOCITY, MAX_BOUNCE_VELOCITY);
                        }
                    } else {
                        transform.translation.y -= separation;
                        if velocity.y > 0.0 {
                            velocity.y = 0.0;
                        }
                    }
                }
            }
        }
    }
}

// TNT vs Blocks collision
fn check_tnt_collisions(
    mut tnt_query: Query<(&mut Transform, &mut Velocity, &mut Gravity), With<Tnt>>,
    block_query: Query<&Transform, (With<Block>, Without<Tnt>)>,
    mut entity_query: Query<(&Transform, &mut Velocity), (Or<(With<Hero>, With<Villager>, With<Enemy>)>, Without<Tnt>, Without<Block>)>,
) {
    for (mut tnt_transform, mut tnt_velocity, mut gravity) in tnt_query.iter_mut() {
        let tnt_pos = tnt_transform.translation;
        let tnt_size = BLOCK_SIZE;  // TNT is same size as blocks
        let block_size = BLOCK_SIZE;

        // If TNT is settled, check if there's still a block underneath
        // If not, wake it up (block was destroyed)
        if gravity.force == 0.0 {
            let mut still_grounded = false;

            // Check if there's a block directly below
            for block_transform in block_query.iter() {
                let block_pos = block_transform.translation;
                let dx = (tnt_pos.x - block_pos.x).abs();
                let dy = tnt_pos.y - block_pos.y;

                // Block is below TNT and close enough horizontally
                if dy > 0.0 && dy < 70.0 && dx < 35.0 {
                    still_grounded = true;
                    break;
                }
            }

            if !still_grounded {
                // Wake up TNT - block was destroyed!
                gravity.force = -500.0;
                // Don't skip - let it fall
            } else {
                continue; // TNT is still locked in place
            }
        }

        let mut is_grounded = false;

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
                    // Resolve horizontally - TNT NEVER bounces
                    let separation = overlap_x + 0.5; // Smaller buffer to reduce micro-adjustments
                    if dx > 0.0 {
                        tnt_transform.translation.x += separation;
                    } else {
                        tnt_transform.translation.x -= separation;
                    }
                    tnt_velocity.x = 0.0; // Complete stop - NO bouncing
                } else {
                    // Resolve vertically
                    let separation = overlap_y + 0.5; // Smaller buffer
                    if dy > 0.0 {
                        // TNT is above block - land on top
                        tnt_transform.translation.y += separation;
                        if tnt_velocity.y < 0.0 {
                            // TNT doesn't bounce - just stop
                            tnt_velocity.y = 0.0;
                            tnt_velocity.x *= 0.9; // Heavy friction when landing
                            is_grounded = true; // TNT is resting on a block
                        }
                    } else {
                        // TNT is below block (hitting ceiling)
                        tnt_transform.translation.y -= separation;
                        if tnt_velocity.y > 0.0 {
                            tnt_velocity.y = 0.0;
                        }
                    }
                }
            }
        }

        // Check collision with characters (Hero, Villagers, Enemies)
        // TNT is HEAVY - it pushes characters based on its velocity
        let entity_size = 64.0; // Characters are 64x64
        let mut touched_character = false; // Track if TNT touched any character
        for (entity_transform, mut entity_velocity) in entity_query.iter_mut() {
            let entity_pos = entity_transform.translation;

            let tnt_half = tnt_size / 2.0;
            let entity_half = entity_size / 2.0;

            let dx = tnt_pos.x - entity_pos.x;
            let dy = tnt_pos.y - entity_pos.y;

            let overlap_x = tnt_half + entity_half - dx.abs();
            let overlap_y = tnt_half + entity_half - dy.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                touched_character = true; // TNT is touching a character - can't be grounded

                // Collision with character - TNT bounces and pushes based on velocity
                if overlap_x < overlap_y {
                    // Horizontal collision
                    let separation = overlap_x + 0.5;
                    if dx > 0.0 {
                        tnt_transform.translation.x += separation;
                    } else {
                        tnt_transform.translation.x -= separation;
                    }

                    // Push character horizontally based on TNT velocity
                    let push_force = tnt_velocity.x.abs() * 0.4;
                    if dx > 0.0 {
                        entity_velocity.x += push_force;
                    } else {
                        entity_velocity.x -= push_force;
                    }

                    // TNT bounces back horizontally
                    tnt_velocity.x *= -0.5; // Bounce in opposite direction
                } else {
                    // Vertical collision
                    let separation = overlap_y + 0.5;
                    if dy > 0.0 {
                        // TNT is above character - lands on character's head
                        tnt_transform.translation.y += separation;

                        // Push character DOWN based on TNT's falling velocity
                        let push_force = tnt_velocity.y.abs() * 0.3; // Light push - keep player mobile
                        entity_velocity.y -= push_force;

                        // TNT bounces UP off character's head based on impact velocity
                        let bounce_strength = tnt_velocity.y.abs() * 0.4; // 40% bounce
                        tnt_velocity.y = bounce_strength; // Bounce upward
                    } else {
                        // Character is above TNT (character landing on TNT from above)
                        // DON'T push TNT down - it might be resting on a block
                        // Instead, push character back up and make them bounce
                        let bounce_force = entity_velocity.y.abs() * 0.3; // Character bounces off TNT
                        entity_velocity.y = bounce_force; // Bounce character upward

                        // TNT is HEAVY - character can't push it down much
                        // Only allow slight compression if TNT is not grounded
                        if gravity.force != 0.0 {
                            tnt_velocity.y -= 50.0; // Slight downward push
                        }
                    }
                }
            }
        }

        // If TNT touched a character, it can't be grounded (prevents floating)
        if touched_character {
            is_grounded = false;
        }

        // Lock TNT in place when truly settled - feels HEAVY
        // STRICT: Only lock if grounded THIS frame AND nearly stopped
        if is_grounded && tnt_velocity.y.abs() < 1.0 && tnt_velocity.x.abs() < 1.0 {
            gravity.force = 0.0; // Disable gravity - TNT is locked
            tnt_velocity.x = 0.0;
            tnt_velocity.y = 0.0;
        } else {
            // Always re-enable gravity if not grounded or moving
            gravity.force = -500.0;
        }
    }
}

// Resource vs Blocks collision
fn check_resource_collisions(
    mut resource_query: Query<(&mut Transform, &mut LinearVelocity), (With<Resource>, Without<Block>)>,
    block_query: Query<(&Transform, &BlockType), (With<Block>, Without<Resource>)>,
) {
    for (mut resource_transform, mut resource_velocity) in resource_query.iter_mut() {
        let resource_pos = resource_transform.translation;
        let resource_size = RESOURCE_SIZE;
        let block_size = BLOCK_SIZE;

        for (block_transform, block_type) in block_query.iter() {
            // Skip cobwebs - resources pass through them
            if *block_type == BlockType::Cobweb {
                continue;
            }

            let block_pos = block_transform.translation;

            let resource_half = resource_size / 2.0;
            let block_half = block_size / 2.0;

            let dx = resource_pos.x - block_pos.x;
            let dy = resource_pos.y - block_pos.y;

            let overlap_x = resource_half + block_half - dx.abs();
            let overlap_y = resource_half + block_half - dy.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                // Collision detected! Resolve based on smallest overlap
                if overlap_x < overlap_y {
                    // Resolve horizontally
                    let separation = overlap_x + 1.0; // Add buffer
                    if dx > 0.0 {
                        resource_transform.translation.x += separation;
                    } else {
                        resource_transform.translation.x -= separation;
                    }
                    resource_velocity.x = -resource_velocity.x * 0.3; // Dampen horizontal bounce
                } else {
                    // Resolve vertically
                    let separation = overlap_y + 1.0; // Add buffer
                    if dy > 0.0 {
                        // Resource is above block - land on top
                        resource_transform.translation.y += separation;
                        if resource_velocity.y < 0.0 {
                            // Resources don't bounce - just stop
                            resource_velocity.y = 0.0;
                            resource_velocity.x *= 0.8; // Friction when landing
                        }
                    } else {
                        // Resource is below block (hitting ceiling)
                        resource_transform.translation.y -= separation;
                        if resource_velocity.y > 0.0 {
                            resource_velocity.y = 0.0;
                        }
                    }
                }
            }
        }
    }
}

// Mass-based collision between entities and TNT - TNT is heavier so entities get pushed more
fn check_hero_tnt_push(
    mut entity_query: Query<(&Transform, &mut Velocity), (Or<(With<Hero>, With<Villager>, With<Enemy>)>, Without<Tnt>)>,
    mut tnt_query: Query<(&mut Transform, &mut Velocity, &mut Gravity), With<Tnt>>,
) {
    for (entity_transform, mut entity_velocity) in entity_query.iter_mut() {
        let entity_pos = entity_transform.translation;
        let entity_size = HERO_SIZE;  // Hero/villager size

        for (tnt_transform, mut tnt_velocity, mut gravity) in tnt_query.iter_mut() {
            let tnt_pos = tnt_transform.translation;
            let tnt_size = BLOCK_SIZE;

            let entity_half = entity_size / 2.0;
            let tnt_half = tnt_size / 2.0;

            let dx = tnt_pos.x - entity_pos.x;
            let dy = tnt_pos.y - entity_pos.y;

            let overlap_x = entity_half + tnt_half - dx.abs();
            let overlap_y = entity_half + tnt_half - dy.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                // Wake up settled TNT when pushed
                if gravity.force == 0.0 {
                    gravity.force = -500.0; // Re-enable gravity when pushed
                }

                // REALISTIC MASS-BASED PHYSICS:
                // TNT mass = 6.0 (heavy but heroes can hold it)
                // Entity mass = 1.0 (lighter)
                let tnt_mass = 5.0;
                let entity_mass = 3.0;
                let total_mass = tnt_mass + entity_mass;

                // Calculate push ratios based on mass (heavier object moves less)
                let tnt_push_ratio = entity_mass / total_mass;  // 1/7 = 0.14 (TNT moves less)
                let entity_push_ratio = tnt_mass / total_mass;   // 6/7 = 0.86 (entity moves more)

                let push_strength = 80.0; // Reduced - gentler collision force

                if overlap_x < overlap_y {
                    // Horizontal collision
                    let push_direction = dx.signum();

                    // TNT gets small push (heavy)
                    tnt_velocity.x += push_direction * push_strength * tnt_push_ratio * 0.3;

                    // Entity gets moderate push (lighter)
                    entity_velocity.x -= push_direction * push_strength * entity_push_ratio * 0.5;
                } else {
                    // Vertical collision
                    if dy > 0.0 {
                        // TNT above entity - heroes CAN hold/stop TNT with strong upward push
                        tnt_velocity.y += push_strength * tnt_push_ratio * 0.6; // STRONG upward push - heroes can hold TNT
                        entity_velocity.y -= push_strength * entity_push_ratio * 0.4;
                    }
                    // Also horizontal component
                    let push_direction_x = dx.signum();
                    tnt_velocity.x += push_direction_x * push_strength * tnt_push_ratio * 0.25;
                    entity_velocity.x -= push_direction_x * push_strength * entity_push_ratio * 0.5;
                }
            }
        }
    }
}

// Entity-to-entity collision (Hero, Villagers, and Enemies bounce off each other)
fn check_entity_collisions(
    mut entities: Query<(Entity, &mut Transform, &mut Velocity, &Bouncy, Option<&Hero>, Option<&Villager>, Option<&Enemy>), Or<(With<Hero>, With<Villager>, With<Enemy>)>>,
) {
    // Run multiple passes to handle complex multi-entity clusters
    for _pass in 0..5 {  // Increased from 3 to 5 passes
        let mut combinations = entities.iter_combinations_mut();

        while let Some([
            (_entity_a, mut transform_a, mut velocity_a, bouncy_a, hero_a, villager_a, enemy_a),
            (_entity_b, mut transform_b, mut velocity_b, bouncy_b, hero_b, villager_b, enemy_b)
        ]) = combinations.fetch_next() {
        let pos_a = transform_a.translation;
        let pos_b = transform_b.translation;

        // Determine actual size based on entity type
        let size_a = if hero_a.is_some() {
            HERO_SIZE
        } else if villager_a.is_some() {
            VILLAGER_SIZE
        } else if enemy_a.is_some() {
            ENEMY_SIZE
        } else {
            HERO_SIZE // fallback
        };

        let size_b = if hero_b.is_some() {
            HERO_SIZE
        } else if villager_b.is_some() {
            VILLAGER_SIZE
        } else if enemy_b.is_some() {
            ENEMY_SIZE
        } else {
            HERO_SIZE // fallback
        };

        let half_a = size_a / 2.0;
        let half_b = size_b / 2.0;

        let dx = pos_a.x - pos_b.x;
        let dy = pos_a.y - pos_b.y;

        let overlap_x = half_a + half_b - dx.abs();
        let overlap_y = half_a + half_b - dy.abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            // Ball-like physics: simple elastic collisions
            if overlap_x < overlap_y {
                // Horizontal collision - separate and bounce
                let separation = (overlap_x / 2.0 + 1.0).min(100.0);
                if dx > 0.0 {
                    transform_a.translation.x += separation;
                    transform_b.translation.x -= separation;
                } else {
                    transform_a.translation.x -= separation;
                    transform_b.translation.x += separation;
                }

                // Simple velocity swap with restitution (billiard ball physics)
                let temp_vx = velocity_a.x;
                velocity_a.x = velocity_b.x * bouncy_a.restitution;
                velocity_b.x = temp_vx * bouncy_b.restitution;

                // Clamp to prevent infinite acceleration
                velocity_a.x = velocity_a.x.clamp(-MAX_BOUNCE_VELOCITY, MAX_BOUNCE_VELOCITY);
                velocity_b.x = velocity_b.x.clamp(-MAX_BOUNCE_VELOCITY, MAX_BOUNCE_VELOCITY);
            } else {
                // Vertical collision - separate and bounce
                let separation = (overlap_y / 2.0 + 1.0).min(100.0);
                if dy > 0.0 {
                    transform_a.translation.y += separation;
                    transform_b.translation.y -= separation;
                } else {
                    transform_a.translation.y -= separation;
                    transform_b.translation.y += separation;
                }

                // Simple velocity swap with restitution (billiard ball physics)
                let temp_vy = velocity_a.y;
                velocity_a.y = velocity_b.y * bouncy_a.restitution;
                velocity_b.y = temp_vy * bouncy_b.restitution;

                // Clamp vertical velocity
                velocity_a.y = velocity_a.y.clamp(-MAX_BOUNCE_VELOCITY * 2.0, MAX_BOUNCE_VELOCITY * 2.0);
                velocity_b.y = velocity_b.y.clamp(-MAX_BOUNCE_VELOCITY * 2.0, MAX_BOUNCE_VELOCITY * 2.0);
            }
        }
        }
    }
}

/// Hero AI - automatic movement (same as villagers), can be overridden by player input
fn hero_ai_movement(
    time: Res<Time>,
    mut hero_query: Query<(&mut Hero, &mut LinearVelocity, Option<&crate::components::power_effect::SpeedModifier>, Option<&crate::components::frozen::FrozenStacks>)>,
) {
    for (mut hero, mut velocity, speed_mod, frozen) in hero_query.iter_mut() {
        // Skip AI movement if fully frozen
        if let Some(frozen_stacks) = frozen {
            if frozen_stacks.is_fully_frozen() {
                continue;
            }
        }

        // Tick direction change timer
        hero.direction_change_timer.tick(time.delta());

        // Check if we have turbo/speed boost active
        let has_turbo = speed_mod.is_some();

        // Only change direction when velocity is VERY LOW (nearly stationary)
        // Don't change direction when character has momentum from physics/player
        if hero.direction_change_timer.just_finished() && velocity.x.abs() < 50.0 {
            hero.move_direction = -hero.move_direction;  // Flip direction
        }

        // Only apply AI movement when:
        // 1. Nearly stationary (velocity < 50), OR
        // 2. Has turbo active (speed boost allows AI to help even when moving)
        if velocity.x.abs() < 50.0 || has_turbo {
            velocity.x += hero.move_direction * VILLAGER_MOVE_SPEED * time.delta_secs();

            // Clamp to AI max speed
            velocity.x = velocity.x.clamp(-VILLAGER_MAX_SPEED, VILLAGER_MAX_SPEED);
        }
        // When velocity >= 50 and no turbo, AI doesn't touch it - full physics/player control
    }
}

/// Villager AI - walk back and forth with periodic direction changes
fn villager_ai_movement(
    time: Res<Time>,
    mut villager_query: Query<(&mut Villager, &mut LinearVelocity, Option<&crate::components::frozen::FrozenStacks>)>,
) {
    for (mut villager, mut velocity, frozen) in villager_query.iter_mut() {
        // Skip AI movement if fully frozen
        if let Some(frozen_stacks) = frozen {
            if frozen_stacks.is_fully_frozen() {
                continue;
            }
        }

        // Tick direction change timer
        villager.direction_change_timer.tick(time.delta());

        // Only change direction when velocity is LOW (not bouncing from walls/collisions)
        if villager.direction_change_timer.just_finished() && velocity.x.abs() < 250.0 {
            villager.move_direction = -villager.move_direction;  // Flip direction
        }

        // Only apply AI movement when velocity is low - don't fight against bounces
        if velocity.x.abs() < 300.0 {
            velocity.x += villager.move_direction * VILLAGER_MOVE_SPEED * time.delta_secs();

            // Clamp to AI max speed
            velocity.x = velocity.x.clamp(-VILLAGER_MAX_SPEED, VILLAGER_MAX_SPEED);
        }
        // When velocity > 300 (bouncing), AI doesn't touch it - full physics control
    }
}

/// Enemy AI - chase nearest hero or villager (melee enemies) OR keep distance (ranged skeletons)
fn enemy_ai_movement(
    time: Res<Time>,
    mut enemy_query: Query<(&Transform, &mut LinearVelocity, &Enemy)>,
    target_query: Query<&Transform, Or<(With<Hero>, With<Villager>)>>,
) {
    for (enemy_transform, mut velocity, enemy) in enemy_query.iter_mut() {
        let enemy_pos = enemy_transform.translation;

        // Find nearest target (hero or villager)
        let mut nearest_distance = f32::MAX;
        let mut nearest_target_x = None;

        for target_transform in target_query.iter() {
            let target_pos = target_transform.translation;
            let distance = enemy_pos.distance(target_pos);

            if distance < nearest_distance {
                nearest_distance = distance;
                nearest_target_x = Some(target_pos.x);
            }
        }

        if let Some(target_x) = nearest_target_x {
            // SKELETON AI: Keep distance, never melee
            if enemy.enemy_type == crate::components::enemy::EnemyType::Skeleton {
                // Skeletons override physics when maintaining range (tactical positioning)
                if nearest_distance < SKELETON_MIN_RANGE {
                    // Too close - back away!
                    let direction = if target_x > enemy_pos.x { -1.0 } else { 1.0 }; // Move AWAY
                    velocity.x += direction * VILLAGER_MOVE_SPEED * 1.5 * time.delta_secs(); // Faster retreat
                    velocity.x = velocity.x.clamp(-400.0, 400.0);
                } else if nearest_distance > SKELETON_MAX_RANGE {
                    // Too far - move closer (but not too much)
                    let direction = if target_x > enemy_pos.x { 1.0 } else { -1.0 }; // Move TOWARDS
                    velocity.x += direction * VILLAGER_MOVE_SPEED * 0.5 * time.delta_secs(); // Slow approach
                    velocity.x = velocity.x.clamp(-400.0, 400.0);
                }
                // If in ideal range (300-600), don't move horizontally - just shoot
            } else {
                // MELEE ENEMIES (Pigman, etc.): Chase normally - blend with collision physics
                let direction = if target_x > enemy_pos.x { 1.0 } else { -1.0 };
                velocity.x += direction * VILLAGER_MOVE_SPEED * time.delta_secs();

                // Only clamp if moving in chase direction (don't kill collision bounces)
                if (direction > 0.0 && velocity.x > 0.0) || (direction < 0.0 && velocity.x < 0.0) {
                    velocity.x = velocity.x.clamp(-400.0, 400.0);
                }
            }
        }
    }
}

/// Handle enemy damage to hero/villagers using Avian collision events
/// Triggers when enemy physically collides with hero/villager
fn handle_enemy_damage_collisions(
    mut commands: Commands,
    mut collision_events: MessageReader<CollisionStart>,
    hero_query: Query<Entity, With<Hero>>,
    villager_query: Query<Entity, With<Villager>>,
    enemy_query: Query<&crate::components::enemy::Enemy>,
    mut damage_events: MessageWriter<EntityDamaged>,
    mut enemy_damage_cooldown: ResMut<DamageCooldown>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs();

    for event in collision_events.read() {
        let entity1 = event.collider1;
        let entity2 = event.collider2;

        // Check if friendly (hero or villager) collides with enemy
        let friendly_entity = if hero_query.contains(entity1) || villager_query.contains(entity1) {
            Some(entity1)
        } else if hero_query.contains(entity2) || villager_query.contains(entity2) {
            Some(entity2)
        } else {
            None
        };

        let enemy_entity = if enemy_query.contains(entity1) {
            Some(entity1)
        } else if enemy_query.contains(entity2) {
            Some(entity2)
        } else {
            None
        };

        // If we have both a friendly and an enemy in this collision
        if let (Some(friendly), Some(enemy)) = (friendly_entity, enemy_entity) {
            if let Ok(enemy_data) = enemy_query.get(enemy) {
                // Skeletons don't melee
                if enemy_data.enemy_type == crate::components::enemy::EnemyType::Skeleton {
                    continue;
                }

                // Check cooldown
                if !enemy_damage_cooldown.can_damage(friendly, current_time, 1.0) {
                    continue;
                }

                // Deal damage
                damage_events.write(EntityDamaged {
                    entity: friendly,
                    damage: 1,
                });

                // Apply burning buff from Pigman
                if enemy_data.enemy_type == crate::components::enemy::EnemyType::Pigman {
                    commands.entity(friendly).insert(Buff::new(BuffType::Burning, 3.0));
                }

                enemy_damage_cooldown.record_damage(friendly, current_time);
            }
        }
    }
}

/// DEPRECATED: Replaced by handle_entity_collisions using Avian events
/// Hero damages enemies on collision (melee attack)
/// Uses per-enemy cooldown: each enemy has separate attack cooldown (industry standard)
/// OPTIMIZED: Uses spatial hash to only check nearby enemies
fn _deprecated_check_hero_melee_damage(
    spatial_hash: Res<SpatialHash>,
    hero_query: Query<(Entity, &Transform), With<Hero>>,
    enemy_query: Query<(Entity, &Transform), (With<crate::components::enemy::Enemy>, Without<crate::components::damage_visual::DeathAnimation>, Without<crate::components::damage_visual::MarkedForDeath>)>,
    mut damage_events: MessageWriter<EntityDamaged>,
    mut melee_cooldown: ResMut<MeleeAttackCooldown>,
    time: Res<Time>,
) {
    let hero_size = HERO_SIZE;
    let current_time = time.elapsed_secs();

    if let Some((_hero_entity, hero_transform)) = hero_query.iter().next() {
        let hero_pos = hero_transform.translation;
        let hero_half = hero_size / 2.0;

        // OPTIMIZED: Only check enemies in nearby chunks (not all enemies!)
        let nearby_entities = spatial_hash.query_nearby(hero_pos.x, hero_pos.y);

        for nearby_entity in nearby_entities {
            // Check if this nearby entity is actually an enemy
            let Ok((enemy_entity, enemy_transform)) = enemy_query.get(nearby_entity) else {
                continue; // Not an enemy, skip
            };

            let enemy_pos = enemy_transform.translation;
            let enemy_size = ENEMY_SIZE;
            let enemy_half = enemy_size / 2.0;

            // AABB collision (same as bounce system) - more generous than circle collision
            let dx = (hero_pos.x - enemy_pos.x).abs();
            let dy = (hero_pos.y - enemy_pos.y).abs();

            let overlap_x = hero_half + enemy_half - dx;
            let overlap_y = hero_half + enemy_half - dy;

            if overlap_x > 0.0 && overlap_y > 0.0 {
                // Colliding with this enemy (AABB) - check per-enemy cooldown
                if melee_cooldown.can_attack(enemy_entity, current_time, HERO_MELEE_COOLDOWN) {
                    info!("✅ HERO MELEE! Damaged enemy {:?}, overlap: ({:.1}, {:.1})", enemy_entity, overlap_x, overlap_y);

                    damage_events.write(EntityDamaged {
                        entity: enemy_entity,
                        damage: HERO_MELEE_DAMAGE,
                    });

                    melee_cooldown.record_hit(enemy_entity, current_time);
                } else {
                    // On cooldown - log for debugging
                    let time_left = HERO_MELEE_COOLDOWN - (current_time - melee_cooldown.last_hit_time.get(&enemy_entity).unwrap_or(&0.0));
                    info!("❌ Enemy {:?} on cooldown, {:.2}s remaining", enemy_entity, time_left.max(0.0));
                }
            }
        }
    }
}

/// DEPRECATED: Replaced by handle_entity_collisions using Avian events
/// Enemy vs Hero/Villager collision - damage hero/villager and apply buffs
/// OPTIMIZED: Uses spatial hash to only check nearby enemies
fn _deprecated_check_hero_enemy_collision(
    mut commands: Commands,
    spatial_hash: Res<SpatialHash>,
    entity_query: Query<(Entity, &Transform), Or<(With<Hero>, With<Villager>)>>,
    enemy_query: Query<(Entity, &Transform, &crate::components::enemy::Enemy), (Without<crate::components::damage_visual::DeathAnimation>, Without<crate::components::damage_visual::MarkedForDeath>)>,
    mut damage_events: MessageWriter<EntityDamaged>,
    time: Res<Time>,
    mut cooldown: ResMut<DamageCooldown>,
) {
    let current_time = time.elapsed_secs();
    let entity_size = HERO_SIZE;

    for (entity, entity_transform) in entity_query.iter() {
        // Check cooldown PER ENTITY (not global)
        if !cooldown.can_damage(entity, current_time, 1.0) {
            continue; // This entity is on cooldown, skip to next
        }

        let entity_pos = entity_transform.translation;

        // OPTIMIZED: Only check enemies in nearby chunks (not all enemies!)
        let nearby_entities = spatial_hash.query_nearby(entity_pos.x, entity_pos.y);

        for nearby_entity in nearby_entities {
            // Check if this nearby entity is actually an enemy
            let Ok((_enemy_entity, enemy_transform, enemy)) = enemy_query.get(nearby_entity) else {
                continue; // Not an enemy, skip
            };

            let enemy_pos = enemy_transform.translation;
            let enemy_size = ENEMY_SIZE;

            let distance = entity_pos.distance(enemy_pos);
            let collision_distance = (entity_size + enemy_size) / 2.0;

            if distance < collision_distance {
                // SKELETONS DON'T MELEE - only damage from arrows!
                if enemy.enemy_type == crate::components::enemy::EnemyType::Skeleton {
                    continue; // Skip skeleton melee damage - they're ranged only
                }

                // Damage entity with half heart (melee enemies only)
                info!("ENEMY COLLISION! Entity {:?} hit by {:?} enemy at distance {}", entity, enemy.enemy_type, distance);

                damage_events.write(EntityDamaged {
                    entity,
                    damage: 1,  // 1 damage = half heart
                });

                // Apply burning buff from Pigman
                if enemy.enemy_type == crate::components::enemy::EnemyType::Pigman {
                    commands.entity(entity).insert(Buff::new(BuffType::Burning, 3.0));
                }

                cooldown.record_damage(entity, current_time);
                break; // Only damage once per entity per frame
            }
        }
    }
}

/// Handle melee damage using Avian collision events
/// Triggers whenever hero/villager physically collides with enemy
fn handle_avian_collision_melee(
    mut collision_events: MessageReader<CollisionStart>,
    hero_query: Query<(Entity, Option<&crate::components::frozen::FrozenStacks>), With<Hero>>,
    _villager_query: Query<(Entity, Option<&crate::components::frozen::FrozenStacks>), With<Villager>>,
    enemy_query: Query<Entity, With<crate::components::enemy::Enemy>>,
    mut damage_events: MessageWriter<EntityDamaged>,
    mut melee_cooldown: ResMut<MeleeAttackCooldown>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs();

    for event in collision_events.read() {
        let entity1 = event.collider1;
        let entity2 = event.collider2;

        // Check if hero collides with enemy
        if let Ok((_hero, frozen)) = hero_query.get(entity1) {
            // Skip damage if hero is fully frozen
            if let Some(frozen_stacks) = frozen {
                if frozen_stacks.is_fully_frozen() {
                    continue;
                }
            }

            if let Ok(enemy) = enemy_query.get(entity2) {
                if melee_cooldown.can_attack(enemy, current_time, HERO_MELEE_COOLDOWN) {
                    info!("⚔️ HERO MELEE HIT (collision)! enemy={:?}", enemy);
                    damage_events.write(EntityDamaged {
                        entity: enemy,
                        damage: HERO_MELEE_DAMAGE,
                    });
                    melee_cooldown.record_hit(enemy, current_time);
                }
            }
        }
        // Check reverse (enemy collides with hero)
        if let Ok((_hero, frozen)) = hero_query.get(entity2) {
            // Skip damage if hero is fully frozen
            if let Some(frozen_stacks) = frozen {
                if frozen_stacks.is_fully_frozen() {
                    continue;
                }
            }

            if let Ok(enemy) = enemy_query.get(entity1) {
                if melee_cooldown.can_attack(enemy, current_time, HERO_MELEE_COOLDOWN) {
                    info!("⚔️ HERO MELEE HIT (collision)! enemy={:?}", enemy);
                    damage_events.write(EntityDamaged {
                        entity: enemy,
                        damage: HERO_MELEE_DAMAGE,
                    });
                    melee_cooldown.record_hit(enemy, current_time);
                }
            }
        }
    }
}

/// Clamp entity velocities to prevent extreme speeds from cumulative collision forces
/// When many entities overlap, Avian applies push forces from each one, causing velocity to accumulate
fn clamp_entity_velocities(
    mut query: Query<&mut LinearVelocity, Or<(With<Hero>, With<Villager>, With<Enemy>)>>,
) {
    for mut velocity in query.iter_mut() {
        // Clamp horizontal velocity
        velocity.x = velocity.x.clamp(-MAX_ENTITY_HORIZONTAL_SPEED, MAX_ENTITY_HORIZONTAL_SPEED);

        // Clamp vertical velocity
        velocity.y = velocity.y.clamp(-MAX_ENTITY_VERTICAL_SPEED, MAX_ENTITY_VERTICAL_SPEED);

        // Z should always be near zero (2D game)
        velocity.z = velocity.z.clamp(-10.0, 10.0);
    }
}
