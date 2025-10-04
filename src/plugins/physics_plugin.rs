use bevy::prelude::*;
use crate::components::physics::{Velocity, Gravity, Bouncy};
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::block::{Block, BlockType, BiomeType};
use crate::components::explosive::Tnt;
use crate::components::enemy::Enemy;
use crate::components::resource::Resource;
use crate::components::buff::{Buff, BuffType};
use crate::components::chunk::ChunkMember;
use crate::events::block_events::BlockDamaged;
use crate::events::damage_events::EntityDamaged;
use crate::resources::damage_cooldown::DamageCooldown;
use crate::states::GameState;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<DamageCooldown>()
            .add_systems(Update, (
                apply_gravity,
                apply_velocity,
                enemy_ai_movement,
                check_collisions,
                check_villager_collisions,
                check_tnt_collisions,
                check_resource_collisions,
                check_hero_tnt_push,
                check_enemy_collisions,
                check_hero_enemy_collision,
                check_entity_collisions,
            ).run_if(in_state(GameState::Playing)));

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
    mut commands: Commands,
    mut hero_query: Query<(Entity, &mut Transform, &mut Velocity, &Bouncy), (With<Hero>, Without<Block>)>,
    block_query: Query<(Entity, &Transform, &BlockType, &ChunkMember), (With<Block>, Without<Hero>)>,
    chunk_query: Query<&crate::components::chunk::Chunk>,
    mut damage_events: MessageWriter<BlockDamaged>,
) {
    let Ok((hero_entity, mut hero_transform, mut hero_velocity, bouncy)) = hero_query.single_mut() else {
        return;
    };

    let hero_pos = hero_transform.translation;
    let hero_size = 64.0;  // 2D sprite size (64x64)
    let block_size = 64.0;  // 3D blocks are 1.0 unit cubes scaled by 64.0, so total size is 64.0

    for (block_entity, block_transform, block_type, chunk_member) in block_query.iter() {
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

            // Apply buff based on block type and biome
            if let Some(chunk) = chunk_query.iter().find(|c| c.chunk_x == chunk_member.chunk_x && c.chunk_y == chunk_member.chunk_y) {
                apply_block_buff(&mut commands, hero_entity, *block_type, chunk.biome);
            }

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
                        // Cap bounce velocity to prevent crazy high bounces from long falls
                        let max_bounce_velocity = 400.0; // Maximum upward velocity after bounce
                        hero_velocity.y = -hero_velocity.y * bouncy.restitution;
                        hero_velocity.y = hero_velocity.y.min(max_bounce_velocity);

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
                            // Cap bounce velocity to prevent crazy high bounces from long falls
                            let max_bounce_velocity = 350.0; // Villagers bounce slightly less than hero
                            villager_velocity.y = -villager_velocity.y * bouncy.restitution;
                            villager_velocity.y = villager_velocity.y.min(max_bounce_velocity);

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
        let tnt_size = 64.0;  // 3D cube scaled to 64x64x64
        let block_size = 64.0;  // 3D blocks also 64x64x64

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
        let resource_size = 32.0;  // 2D sprite 32x32
        let block_size = 64.0;  // 3D blocks 64x64x64

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
        let entity_size = 64.0;

        for (mut tnt_transform, mut tnt_velocity, mut gravity) in tnt_query.iter_mut() {
            let tnt_pos = tnt_transform.translation;
            let tnt_size = 64.0;

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

                let push_strength = 160.0; // Increased - strong collision force

                if overlap_x < overlap_y {
                    // Horizontal collision
                    let push_direction = dx.signum();

                    // TNT gets small push (heavy)
                    tnt_velocity.x += push_direction * push_strength * tnt_push_ratio * 0.3;

                    // Entity gets big push (lighter)
                    entity_velocity.x -= push_direction * push_strength * entity_push_ratio * 1.2;
                } else {
                    // Vertical collision
                    if dy > 0.0 {
                        // TNT above entity - heroes CAN hold/stop TNT with strong upward push
                        tnt_velocity.y += push_strength * tnt_push_ratio * 0.6; // STRONG upward push - heroes can hold TNT
                        entity_velocity.y -= push_strength * entity_push_ratio * 0.8;
                    }
                    // Also horizontal component
                    let push_direction_x = dx.signum();
                    tnt_velocity.x += push_direction_x * push_strength * tnt_push_ratio * 0.25;
                    entity_velocity.x -= push_direction_x * push_strength * entity_push_ratio * 1.1;
                }
            }
        }
    }
}

// Entity-to-entity collision (Hero, Villagers, and Enemies bounce off each other)
fn check_entity_collisions(
    mut entities: Query<(Entity, &mut Transform, &mut Velocity, &Bouncy), Or<(With<Hero>, With<Villager>, With<Enemy>)>>,
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

/// Simple AI for enemies - move toward nearest entity (hero or villager)
fn enemy_ai_movement(
    target_query: Query<&Transform, Or<(With<Hero>, With<Villager>)>>,
    mut enemy_query: Query<(&Transform, &mut Velocity), With<Enemy>>,
) {
    for (enemy_transform, mut velocity) in enemy_query.iter_mut() {
        let enemy_pos = enemy_transform.translation;

        // Find nearest target (hero or villager)
        let mut nearest_pos: Option<Vec3> = None;
        let mut nearest_distance = f32::MAX;

        for target_transform in target_query.iter() {
            let target_pos = target_transform.translation;
            let distance = enemy_pos.distance(target_pos);

            if distance < nearest_distance {
                nearest_distance = distance;
                nearest_pos = Some(target_pos);
            }
        }

        // Move toward nearest target horizontally
        if let Some(target_pos) = nearest_pos {
            let dx = target_pos.x - enemy_pos.x;

            let speed = 100.0;
            if dx.abs() > 50.0 {  // Only move if not too close
                velocity.x = dx.signum() * speed;
            } else {
                velocity.x *= 0.9;  // Slow down when close
            }
        }
    }
}

/// Enemy vs Blocks collision
fn check_enemy_collisions(
    mut enemy_query: Query<(&mut Transform, &mut Velocity, &Bouncy), (With<Enemy>, Without<Block>)>,
    block_query: Query<&Transform, (With<Block>, Without<Enemy>)>,
) {
    for (mut enemy_transform, mut enemy_velocity, bouncy) in enemy_query.iter_mut() {
        let enemy_pos = enemy_transform.translation;
        let enemy_size = 64.0;  // Enemy sprite size (64x64 mesh)
        let block_size = 64.0;  // Block size

        for block_transform in block_query.iter() {
            let block_pos = block_transform.translation;

            let enemy_half = enemy_size / 2.0;
            let block_half = block_size / 2.0;

            let dx = enemy_pos.x - block_pos.x;
            let dy = enemy_pos.y - block_pos.y;

            let overlap_x = enemy_half + block_half - dx.abs();
            let overlap_y = enemy_half + block_half - dy.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                // Collision detected!
                if overlap_x < overlap_y {
                    // Resolve horizontally
                    if dx > 0.0 {
                        enemy_transform.translation.x += overlap_x;
                    } else {
                        enemy_transform.translation.x -= overlap_x;
                    }
                    enemy_velocity.x = -enemy_velocity.x * 0.5;
                } else {
                    // Resolve vertically
                    if dy > 0.0 {
                        enemy_transform.translation.y += overlap_y;
                        if enemy_velocity.y < 0.0 {
                            enemy_velocity.y = -enemy_velocity.y * bouncy.restitution;
                        }
                    } else {
                        enemy_transform.translation.y -= overlap_y;
                        if enemy_velocity.y > 0.0 {
                            enemy_velocity.y = 0.0;
                        }
                    }
                }
            }
        }
    }
}


/// Hero vs Enemy collision - damage hero and apply burning buff
fn check_hero_enemy_collision(
    mut commands: Commands,
    mut entity_query: Query<(Entity, &Transform), Or<(With<Hero>, With<Villager>)>>,
    enemy_query: Query<(&Transform, &crate::components::enemy::Enemy)>,
    mut damage_events: MessageWriter<EntityDamaged>,
    time: Res<Time>,
    mut cooldown: ResMut<DamageCooldown>,
) {
    // Check cooldown (half heart damage per second max)
    if !cooldown.can_damage(time.elapsed_secs(), 1.0) {
        return;
    }

    let entity_size = 64.0 * 64.0;

    for (entity, entity_transform) in entity_query.iter() {
        let entity_pos = entity_transform.translation;

        for (enemy_transform, enemy) in enemy_query.iter() {
            let enemy_pos = enemy_transform.translation;
            let enemy_size = 64.0;  // Enemy sprite size

            let distance = entity_pos.distance(enemy_pos);
            let collision_distance = (entity_size + enemy_size) / 2.0;

            if distance < collision_distance {
                // Damage entity with half heart (0.5 damage = 1 health since 2 health = 1 heart)
                // But damage is u32 so we use 1 damage which represents half heart
                damage_events.write(EntityDamaged {
                    entity,
                    damage: 1,  // 1 damage = half heart
                });

                // Apply burning buff from Pigman
                if enemy.enemy_type == crate::components::enemy::EnemyType::Pigman {
                    commands.entity(entity).insert(Buff::new(BuffType::Burning, 3.0));
                }

                cooldown.record_damage(time.elapsed_secs());
                break;
            }
        }
    }
}

/// Apply buffs based on block type and biome
fn apply_block_buff(
    commands: &mut Commands,
    entity: Entity,
    block_type: BlockType,
    biome: BiomeType,
) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // 20% chance to apply buff on collision
    if !rng.gen_bool(0.2) {
        return;
    }

    let buff = match biome {
        BiomeType::Nether => {
            // Nether: burning from lava blocks
            match block_type {
                BlockType::Netherrack | BlockType::Glowstone => Some((BuffType::Burning, 4.0)),
                BlockType::SoulSand => Some((BuffType::Slowness, 3.0)),
                _ => None,
            }
        },
        BiomeType::End => {
            // End: slowness from chorus
            match block_type {
                BlockType::ChorusFruit => Some((BuffType::Speed, 5.0)),
                _ => None,
            }
        },
        BiomeType::Mountains => {
            // Mountains: freezing from snow
            match block_type {
                BlockType::Snow => Some((BuffType::Freezing, 3.0)),
                _ => None,
            }
        },
        BiomeType::Underground => {
            // Underground: poisoned from mushrooms
            match block_type {
                BlockType::Mushroom => Some((BuffType::Poisoned, 4.0)),
                _ => None,
            }
        },
        _ => None,
    };

    if let Some((buff_type, duration)) = buff {
        commands.entity(entity).insert(Buff::new(buff_type, duration));
    }
}
