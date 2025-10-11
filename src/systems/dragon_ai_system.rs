use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;
use crate::components::enemy::{Dragon, DragonState};
use crate::components::block::Block;
use crate::components::hero::Hero;
use rand::Rng;

const DRAGON_FLIGHT_SPEED: f32 = 200.0;  // Much faster!
const HOVER_AMPLITUDE: f32 = 30.0;
const HOVER_FREQUENCY: f32 = 2.0;
const FLYING_DURATION: f32 = 4.0;  // Fly across screen for 4 seconds
const HOVERING_DURATION: f32 = 2.0;  // Hover briefly
const LANDING_DURATION: f32 = 1.5;
const LANDED_DURATION: f32 = 3.0;  // Sit on blocks for 3 seconds
const FIREBALL_SHOOT_INTERVAL: f32 = 1.5;  // Shoot every 1.5s while flying

/// Dragon AI system - dynamic flying, landing, and fireball shooting
pub fn dragon_ai_system(
    time: Res<Time>,
    mut dragon_query: Query<(&mut Dragon, &mut Transform, &mut LinearVelocity, Entity)>,
    block_query: Query<&Transform, (With<Block>, Without<Dragon>)>,
    hero_query: Query<&Transform, (With<Hero>, Without<Dragon>)>,
) {
    let delta = time.delta_secs();
    let mut rng = rand::thread_rng();

    for (mut dragon, mut transform, mut velocity, _entity) in dragon_query.iter_mut() {
        dragon.state_timer += delta;
        dragon.hover_timer += delta;

        // Calculate hover offset (sine wave)
        dragon.hover_offset = (dragon.hover_timer * HOVER_FREQUENCY * std::f32::consts::TAU).sin() * HOVER_AMPLITUDE;

        match dragon.state {
            DragonState::Emerging => {
                // Fly across screen from right to left
                let direction = (dragon.target_position - transform.translation).normalize_or_zero();
                let distance = transform.translation.distance(dragon.target_position);

                if distance > 50.0 && dragon.state_timer < FLYING_DURATION {
                    // Move toward target (fly left)
                    velocity.x = direction.x * DRAGON_FLIGHT_SPEED;
                    velocity.y = direction.y * DRAGON_FLIGHT_SPEED * 0.3;  // Slight vertical movement

                    // Add hover bobbing while flying
                    transform.translation.y += dragon.hover_offset * delta * 2.0;

                    // Rotate to face movement direction
                    rotate_toward_velocity(&mut transform, direction);

                    // TODO: Shoot fireballs while flying
                    // if dragon.state_timer % FIREBALL_SHOOT_INTERVAL < delta { ... }
                } else {
                    // Reached target or timeout - find landing spot
                    info!("🐉 Dragon looking for landing spot");
                    dragon.state = DragonState::Landing;
                    dragon.state_timer = 0.0;

                    // Find nearest block below
                    if let Some(landing_pos) = find_landing_spot(&transform.translation, &block_query) {
                        dragon.target_position = landing_pos;
                    } else {
                        // No blocks found, hover in place
                        dragon.target_position = transform.translation;
                        dragon.state = DragonState::Hovering;
                    }
                }
            }

            DragonState::Hovering => {
                // Float in place with sine wave
                velocity.x = 0.0;
                velocity.y = 0.0;

                // Apply hover offset
                transform.translation.y += dragon.hover_offset * delta * 0.5;

                // After hovering, either land or fly away
                if dragon.state_timer > HOVERING_DURATION {
                    if rng.gen_bool(0.7) {
                        // 70% chance to land
                        dragon.state = DragonState::Landing;
                        dragon.state_timer = 0.0;

                        if let Some(landing_pos) = find_landing_spot(&transform.translation, &block_query) {
                            dragon.target_position = landing_pos;
                        }
                    } else {
                        // 30% chance to fly away
                        dragon.state = DragonState::Flying;
                        dragon.state_timer = 0.0;

                        // Pick random direction to fly
                        let hero_pos = hero_query.iter().next().map(|t| t.translation).unwrap_or(Vec3::ZERO);
                        dragon.target_position = Vec3::new(
                            hero_pos.x + rng.gen_range(-800.0..800.0),
                            hero_pos.y + rng.gen_range(200.0..500.0),
                            200.0,
                        );
                    }
                }
            }

            DragonState::Flying => {
                // Fly to new position
                let direction = (dragon.target_position - transform.translation).normalize_or_zero();
                let distance = transform.translation.distance(dragon.target_position);

                if distance > 50.0 && dragon.state_timer < FLYING_DURATION {
                    velocity.x = direction.x * DRAGON_FLIGHT_SPEED;
                    velocity.y = direction.y * DRAGON_FLIGHT_SPEED * 0.5;

                    // Add hover bobbing
                    transform.translation.y += dragon.hover_offset * delta * 2.0;

                    rotate_toward_velocity(&mut transform, direction);
                } else {
                    // Done flying, hover
                    dragon.state = DragonState::Hovering;
                    dragon.state_timer = 0.0;
                    velocity.x = 0.0;
                    velocity.y = 0.0;
                }
            }

            DragonState::Landing => {
                // Descend to landing position
                let direction = (dragon.target_position - transform.translation).normalize_or_zero();
                let distance = transform.translation.distance(dragon.target_position);

                if distance > 30.0 {
                    // Move toward landing spot
                    velocity.x = direction.x * DRAGON_FLIGHT_SPEED * 0.5;
                    velocity.y = direction.y * DRAGON_FLIGHT_SPEED * 0.8;

                    rotate_toward_velocity(&mut transform, direction);
                } else {
                    // Landed
                    info!("🐉 Dragon landed!");
                    dragon.state = DragonState::Landed;
                    dragon.state_timer = 0.0;
                    velocity.x = 0.0;
                    velocity.y = 0.0;

                    // Snap to landing position
                    transform.translation = dragon.target_position;
                }
            }

            DragonState::Landed => {
                // Sitting on blocks
                velocity.x = 0.0;
                velocity.y = 0.0;

                // After some time, take off and fly
                if dragon.state_timer > LANDED_DURATION {
                    info!("🐉 Dragon taking off!");
                    dragon.state = DragonState::Flying;
                    dragon.state_timer = 0.0;

                    // Pick direction to fly (across screen or out of bounds)
                    let hero_pos = hero_query.iter().next().map(|t| t.translation).unwrap_or(Vec3::ZERO);
                    let go_offscreen = rng.gen_bool(0.4);  // 40% chance to fly offscreen

                    dragon.target_position = if go_offscreen {
                        // Fly far away offscreen
                        Vec3::new(
                            hero_pos.x + rng.gen_range(-1500.0..1500.0),
                            hero_pos.y + rng.gen_range(400.0..800.0),
                            200.0,
                        )
                    } else {
                        // Fly across screen
                        Vec3::new(
                            hero_pos.x + rng.gen_range(-600.0..600.0),
                            hero_pos.y + rng.gen_range(200.0..400.0),
                            200.0,
                        )
                    };
                }
            }
        }
    }
}

/// Find a safe landing spot - must be an exposed surface block with clearance
fn find_landing_spot(dragon_pos: &Vec3, block_query: &Query<&Transform, (With<Block>, Without<Dragon>)>) -> Option<Vec3> {
    const BLOCK_SIZE: f32 = 64.0;
    const MIN_HEIGHT_CLEARANCE: f32 = BLOCK_SIZE * 6.0;  // 6 blocks up must be clear
    const MIN_SIDE_CLEARANCE: f32 = BLOCK_SIZE * 3.0;    // 3 blocks radius around must be clear

    // Collect all block positions
    let block_positions: Vec<Vec3> = block_query.iter().map(|t| t.translation).collect();

    // Find potential landing blocks (below dragon)
    let mut valid_landing_spots = Vec::new();

    for block_pos in &block_positions {
        // Only consider blocks below dragon
        if block_pos.y > dragon_pos.y - 200.0 {
            continue;
        }

        let horizontal_dist = ((dragon_pos.x - block_pos.x).powi(2) + (dragon_pos.z - block_pos.z).powi(2)).sqrt();

        // Must be within reasonable range
        if horizontal_dist > 800.0 {
            continue;
        }

        // Check if this is an EXPOSED surface block (no block directly above)
        let has_block_above = block_positions.iter().any(|other_pos| {
            let dx = (other_pos.x - block_pos.x).abs();
            let dz = (other_pos.z - block_pos.z).abs();
            let dy = other_pos.y - block_pos.y;

            // There's a block directly above this one
            dx < BLOCK_SIZE * 0.5 && dz < BLOCK_SIZE * 0.5 && dy > 0.0 && dy < BLOCK_SIZE * 1.5
        });

        // Skip if there's a block above (not exposed surface)
        if has_block_above {
            continue;
        }

        // Landing position on top of this block
        let landing_pos = Vec3::new(block_pos.x, block_pos.y + BLOCK_SIZE + 80.0, 200.0);

        // Check FULL height clearance (6 blocks up must be empty)
        let has_height_clearance = !block_positions.iter().any(|other_pos| {
            let dx = (other_pos.x - landing_pos.x).abs();
            let dz = (other_pos.z - landing_pos.z).abs();
            let dy = other_pos.y - landing_pos.y;

            // Any block within 6 blocks height above landing spot
            dx < BLOCK_SIZE * 1.5 && dz < BLOCK_SIZE * 1.5 && dy > 0.0 && dy < MIN_HEIGHT_CLEARANCE
        });

        if !has_height_clearance {
            continue;
        }

        // Check FULL side clearance (3 blocks radius around must be empty at dragon height)
        let has_side_clearance = !block_positions.iter().any(|other_pos| {
            let dx = (other_pos.x - landing_pos.x).abs();
            let dz = (other_pos.z - landing_pos.z).abs();
            let dy = (other_pos.y - landing_pos.y).abs();

            // Blocks at dragon's landing height within 3-block radius
            let horizontal_dist = (dx * dx + dz * dz).sqrt();
            dy < BLOCK_SIZE * 2.0 && horizontal_dist < MIN_SIDE_CLEARANCE && horizontal_dist > BLOCK_SIZE * 0.5
        });

        if !has_side_clearance {
            continue;
        }

        // Valid exposed surface block with full clearance!
        let distance = dragon_pos.distance(landing_pos);
        valid_landing_spots.push((landing_pos, distance));
    }

    // Return closest valid spot
    if valid_landing_spots.is_empty() {
        info!("🐉 No valid landing spots found - dragon will keep flying");
        None
    } else {
        valid_landing_spots.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let spot = valid_landing_spots.first().map(|(pos, _)| *pos);
        info!("🐉 Found {} valid landing spots, choosing closest", valid_landing_spots.len());
        spot
    }
}

/// Rotate dragon to face movement direction (turn left/right, not just tilt)
fn rotate_toward_velocity(transform: &mut Transform, direction: Vec3) {
    if direction.length_squared() > 0.01 {
        // Calculate yaw angle to actually turn the dragon's body toward flight direction
        // Flying left (negative X) should make dragon face left
        // Flying right (positive X) should make dragon face right
        let yaw = std::f32::consts::PI - (direction.x * 0.8);  // Turn body left/right

        // Calculate pitch (nose up/down)
        let pitch = -direction.y * 0.2;

        // Calculate roll (banking in turns)
        let roll = -direction.x * 0.25;

        // Create target rotation: yaw rotates the body, pitch tilts nose, roll banks
        let target_rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

        // Smooth interpolation
        transform.rotation = transform.rotation.slerp(target_rotation, 0.12);
    } else {
        // Face forward when not moving
        let base_rotation = Quat::from_rotation_y(std::f32::consts::PI);
        transform.rotation = transform.rotation.slerp(base_rotation, 0.05);
    }
}
