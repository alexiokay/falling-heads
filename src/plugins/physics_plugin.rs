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
use crate::systems::buff_system::apply_block_buff;
use crate::states::GameState;
use crate::utils::constants::*;

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
            .add_systems(Update,
                apply_gravity.in_set(PhysicsSet::Forces)
            )
            .add_systems(Update, (
                hero_ai_movement,
                villager_ai_movement,
                enemy_ai_movement,
            ).in_set(PhysicsSet::AI))
            .add_systems(Update,
                apply_velocity.in_set(PhysicsSet::Movement)
            )
            .add_systems(Update,
                rebuild_spatial_hash.in_set(PhysicsSet::Collision)
            )
            .add_systems(Update, (
                check_all_entity_block_collisions,  // Hero + Villagers + Enemies (unified)
                check_tnt_collisions,
                check_resource_collisions,
                check_hero_tnt_push,
                check_hero_enemy_collision,
                check_hero_melee_damage,  // Hero damages enemies on collision
                check_entity_collisions,
            ).in_set(PhysicsSet::Collision).after(rebuild_spatial_hash));

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
    mut query: Query<(&Velocity, &mut Transform, Option<&crate::components::projectile::Arrow>), Without<crate::components::damage_visual::DeathAnimation>>,
) {
    for (velocity, mut transform, arrow) in query.iter_mut() {
        let delta = time.delta_secs();
        let dx = velocity.x * delta;
        let dy = velocity.y * delta;

        transform.translation.x += dx;
        transform.translation.y += dy;
    }
}

/// Rebuild spatial hash every frame for fast collision queries
fn rebuild_spatial_hash(
    mut spatial_hash: ResMut<SpatialHash>,
    block_query: Query<(Entity, &Transform), With<Block>>,
    time: Res<Time>,
) {
    spatial_hash.clear();

    let mut block_count = 0;
    for (entity, transform) in block_query.iter() {
        let pos = transform.translation;
        spatial_hash.insert(entity, pos.x, pos.y);
        block_count += 1;
    }

    // Log every 5 seconds to monitor performance
    if time.elapsed_secs() % 5.0 < time.delta_secs() {
        info!("Spatial hash rebuilt with {} blocks", block_count);
    }
}

/// ALL entities (Hero + Villagers + Enemies) collision - unified physics
fn check_all_entity_block_collisions(
    mut commands: Commands,
    spatial_hash: Res<SpatialHash>,
    mut entity_query: Query<(Entity, &mut Transform, &mut Velocity, &Bouncy, Option<&Enemy>), (Or<(With<Hero>, With<Villager>, With<Enemy>)>, Without<Block>)>,
    block_query: Query<(Entity, &Transform, &BlockType, &ChunkMember), With<Block>>,
    chunk_query: Query<&crate::components::chunk::Chunk>,
    mut damage_events: MessageWriter<BlockDamaged>,
    mut block_cooldown: ResMut<DamageCooldown>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs();
    for (entity, mut transform, mut velocity, bouncy, is_enemy) in entity_query.iter_mut() {
        let pos = transform.translation;
        let entity_size = HERO_SIZE;
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
            let entity_half = entity_size / 2.0;
            let block_half = block_size / 2.0;

            let dx = pos.x - block_pos.x;
            let dy = pos.y - block_pos.y;

            let overlap_x = entity_half + block_half - dx.abs();
            let overlap_y = entity_half + block_half - dy.abs();

            // Only friendlies damage blocks and get buffs
            if is_enemy.is_none() {
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
                    apply_block_buff(&mut commands, entity, block_type, chunk.biome);
                }
            }

            // Collision resolution (SAME for all entities)
            if overlap_x < overlap_y {
                // Horizontal - strong bounce
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

/// Enemy collision - NO block damage, NO buffs, uses Bouncy component like friendlies
fn check_enemy_block_collisions(
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
    mut tnt_query: Query<(&mut Transform, &mut Velocity, &mut Gravity), (With<Tnt>, Without<Block>)>,
    block_query: Query<&Transform, (With<Block>, Without<Tnt>)>,
) {
    for (mut tnt_transform, mut tnt_velocity, mut gravity) in tnt_query.iter_mut() {
        // If TNT is settled (no velocity and gravity disabled), skip collision resolution entirely
        let is_settled = gravity.force == 0.0 && tnt_velocity.x.abs() < 1.0 && tnt_velocity.y.abs() < 1.0;

        if is_settled {
            continue; // Skip completely - TNT is locked in place
        }

        let tnt_pos = tnt_transform.translation;
        let tnt_size = BLOCK_SIZE;  // TNT is same size as blocks
        let block_size = BLOCK_SIZE;

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

        // Lock TNT in place when truly settled - feels HEAVY
        if is_grounded && tnt_velocity.y.abs() < 1.0 && tnt_velocity.x.abs() < 1.0 {
            gravity.force = 0.0; // Disable gravity - TNT is locked
            tnt_velocity.x = 0.0;
            tnt_velocity.y = 0.0;
        } else if !is_grounded {
            gravity.force = -500.0; // Re-enable gravity only when airborne
        }
    }
}

// Resource vs Blocks collision
fn check_resource_collisions(
    mut resource_query: Query<(&mut Transform, &mut Velocity), (With<Resource>, Without<Block>)>,
    block_query: Query<&Transform, (With<Block>, Without<Resource>)>,
) {
    for (mut resource_transform, mut resource_velocity) in resource_query.iter_mut() {
        let resource_pos = resource_transform.translation;
        let resource_size = RESOURCE_SIZE;
        let block_size = BLOCK_SIZE;

        for block_transform in block_query.iter() {
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

        for (mut tnt_transform, mut tnt_velocity, mut gravity) in tnt_query.iter_mut() {
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
            // SUPER AGGRESSIVE separation - the deeper the overlap, the stronger the explosion
            // Scale separation force based on overlap depth (deeper = stronger push)
            if overlap_x < overlap_y {
                // Separate horizontally - EXPLOSIVE force proportional to overlap
                let separation = overlap_x * 0.8 + 5.0;  // MUCH stronger: 80% of overlap + bigger buffer
                if dx > 0.0 {
                    transform_a.translation.x += separation;
                    transform_b.translation.x -= separation;
                } else {
                    transform_a.translation.x -= separation;
                    transform_b.translation.x += separation;
                }

                // MASSIVE horizontal push force to blow clusters apart
                let avg_vx = (velocity_a.x + velocity_b.x) * 0.5;
                let push_strength = 500.0 * (overlap_x / (half_a + half_b)); // MUCH stronger push
                velocity_a.x = (velocity_a.x - avg_vx) * -0.8 + avg_vx + (if dx > 0.0 { push_strength } else { -push_strength });
                velocity_b.x = (velocity_b.x - avg_vx) * -0.8 + avg_vx + (if dx > 0.0 { -push_strength } else { push_strength });
            } else {
                // Separate vertically - EXPLOSIVE force proportional to overlap
                let separation = overlap_y * 0.8 + 5.0;  // MUCH stronger: 80% of overlap + bigger buffer
                if dy > 0.0 {
                    transform_a.translation.y += separation;
                    transform_b.translation.y -= separation;
                } else {
                    transform_a.translation.y -= separation;
                    transform_b.translation.y += separation;
                }

                // MASSIVE vertical bounce to blast entities apart when stacked
                let avg_vy = (velocity_a.y + velocity_b.y) * 0.5;
                velocity_a.y = (velocity_a.y - avg_vy) * -bouncy_a.restitution + avg_vy;
                velocity_b.y = (velocity_b.y - avg_vy) * -bouncy_b.restitution + avg_vy;

                // SUPER STRONG upward explosion force when stuck
                if dy > 0.0 {
                    velocity_a.y = velocity_a.y.max(300.0);  // EXPLOSIVE upward force (doubled)
                } else {
                    velocity_b.y = velocity_b.y.max(300.0);  // EXPLOSIVE upward force (doubled)
                }

                // Also add horizontal spreading force when vertically stacked
                let horizontal_spread = 300.0 * (overlap_y / (half_a + half_b));
                velocity_a.x += if dx > 0.0 { horizontal_spread } else { -horizontal_spread };
                velocity_b.x += if dx > 0.0 { -horizontal_spread } else { horizontal_spread };
            }
        }
        }
    }
}

/// Hero AI - automatic movement (same as villagers), can be overridden by player input
fn hero_ai_movement(
    time: Res<Time>,
    mut hero_query: Query<(&mut Hero, &mut Velocity)>,
) {
    for (mut hero, mut velocity) in hero_query.iter_mut() {
        // Tick direction change timer
        hero.direction_change_timer.tick(time.delta());

        // Change direction when timer finishes
        if hero.direction_change_timer.just_finished() {
            hero.move_direction = -hero.move_direction;  // Flip direction
        }

        // Apply automatic horizontal movement (same speed as villagers)
        velocity.x += hero.move_direction * VILLAGER_MOVE_SPEED * time.delta_secs();
        velocity.x = velocity.x.clamp(-VILLAGER_MAX_SPEED, VILLAGER_MAX_SPEED);
    }
}

/// Villager AI - walk back and forth with periodic direction changes
fn villager_ai_movement(
    time: Res<Time>,
    mut villager_query: Query<(&mut Villager, &mut Velocity)>,
) {
    for (mut villager, mut velocity) in villager_query.iter_mut() {
        // Tick direction change timer
        villager.direction_change_timer.tick(time.delta());

        // Change direction when timer finishes
        if villager.direction_change_timer.just_finished() {
            villager.move_direction = -villager.move_direction;  // Flip direction
        }

        // Apply horizontal movement
        velocity.x += villager.move_direction * VILLAGER_MOVE_SPEED * time.delta_secs();
        velocity.x = velocity.x.clamp(-VILLAGER_MAX_SPEED, VILLAGER_MAX_SPEED);
    }
}

/// Enemy AI - chase nearest hero or villager (melee enemies) OR keep distance (ranged skeletons)
fn enemy_ai_movement(
    time: Res<Time>,
    mut enemy_query: Query<(&Transform, &mut Velocity, &Enemy)>,
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
                // MELEE ENEMIES (Pigman, etc.): Chase normally
                let direction = if target_x > enemy_pos.x { 1.0 } else { -1.0 };
                velocity.x += direction * VILLAGER_MOVE_SPEED * time.delta_secs();
                velocity.x = velocity.x.clamp(-400.0, 400.0);
            }
        }
    }
}

/// Hero damages enemies on collision (melee attack)
/// Uses per-enemy cooldown: each enemy has separate attack cooldown (industry standard)
fn check_hero_melee_damage(
    hero_query: Query<(Entity, &Transform), With<Hero>>,
    enemy_query: Query<(Entity, &Transform), With<crate::components::enemy::Enemy>>,
    mut damage_events: MessageWriter<EntityDamaged>,
    mut melee_cooldown: ResMut<MeleeAttackCooldown>,
    time: Res<Time>,
) {
    let hero_size = HERO_SIZE;
    let current_time = time.elapsed_secs();

    if let Some((_hero_entity, hero_transform)) = hero_query.iter().next() {
        let hero_pos = hero_transform.translation;
        let hero_half = hero_size / 2.0;

        // Check all enemies for collision - USE SAME COLLISION AS BOUNCE SYSTEM (AABB)
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
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

/// Enemy vs Hero/Villager collision - damage hero/villager and apply buffs
fn check_hero_enemy_collision(
    mut commands: Commands,
    mut entity_query: Query<(Entity, &Transform), Or<(With<Hero>, With<Villager>)>>,
    enemy_query: Query<(&Transform, &crate::components::enemy::Enemy)>,
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

        for (enemy_transform, enemy) in enemy_query.iter() {
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
