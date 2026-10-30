use crate::components::{
    block::Block,
    bundles::avian_physics,
    collision_layers::GameLayer,
    enemy::Enemy,
    explosive::{Explosion, Tnt},
    hero::Hero,
    projectile::{Arrow, Laser},
    villager::Villager,
};
use crate::events::block_events::BlockDestroyed;
use crate::events::damage_events::EntityDamaged;
use crate::plugins::gameplay_plugin::break_block;
use crate::resources::spatial_hash::SpatialHash;
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;

/// Blocks a power can break: type and biome tile go into `BlockDestroyed`.
type BreakableBlocks<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static crate::components::block::BlockType,
        Option<&'static crate::systems::biome_content::Tile>,
    ),
    With<Block>,
>;
use crate::utils::constants::*;
use avian3d::prelude::*;

/// Component to handle sequential arrow spawning
#[derive(Component)]
pub struct ArrowBarrage {
    pub spawn_timer: Timer,
    pub arrows_remaining: u32,
    pub current_angle: f32,
    pub angle_step: f32,
    pub follow_entity: Entity, // Follow this entity's position (hero)
}

/// Component to handle sequential laser burst spawning
#[derive(Component)]
pub struct LaserBurstBarrage {
    pub spawn_timer: Timer,
    pub beams_remaining: u32,
    pub current_angle: f32,
    pub angle_step: f32,
    pub follow_entity: Entity, // Follow this entity's position (hero)
}

pub struct PowersPlugin;

impl Plugin for PowersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                tnt_countdown,
                tnt_explosion,
                update_explosion_animation,
                laser_update,
                update_arrow_barrage,
                update_laser_burst_barrage,
                animate_laser_burst_growth,
                update_laser_burst_positions,
                barrier_update,
                barrier_reflect_projectiles.after(barrier_update),
                update_barrier_impacts,
                magnet_update,
            )
                .run_if(in_state(crate::states::InGame)),
        );

        info!("PowersPlugin loaded");
    }
}

// TNT spawning will be triggered by API
pub fn spawn_tnt(
    commands: &mut Commands,
    position: Vec3,
    block_atlases: &crate::resources::asset_handles::BlockAtlases,
    mesh_cache: &crate::resources::mesh_cache::MeshCache,
    meshes: &mut Assets<Mesh>, // Still needed for username label
    materials: &mut Assets<StandardMaterial>,
    username: Option<String>,
    asset_server: Option<&AssetServer>,
) {
    use crate::systems::ui::username_label::spawn_username_label;

    // Spawn TNT as 3D cube using same UV-mapped mesh system as blocks
    let tnt_entity = commands
        .spawn((
            Tnt {
                timer: Timer::from_seconds(TNT_FUSE_TIME, TimerMode::Once),
            },
            avian_physics::tnt(), // Avian physics for TNT
            Mesh3d(mesh_cache.tnt_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_atlases.tnt.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                // Use default backface culling - only show front faces
                ..default()
            })),
            Transform::from_translation(position), // No rotation needed - UVs are correct
        ))
        .id();

    // Spawn username label if provided
    if let (Some(username), Some(server)) = (username, asset_server) {
        spawn_username_label(
            commands,
            tnt_entity,
            position,
            username.clone(),
            50.0,
            meshes,
            materials,
            server,
        );
        info!("TNT spawned at {:?} by {}", position, username);
    } else {
        info!("TNT spawned at {:?}", position);
    }
}

fn tnt_countdown(
    time: Res<Time>,
    mut commands: Commands,
    mut tnt_query: Query<(
        Entity,
        &mut Tnt,
        &Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut tnt, transform, material_handle) in tnt_query.iter_mut() {
        tnt.timer.tick(time.delta());

        // Get how much time is left (0.0 to 1.0, where 0.0 = about to explode)
        let time_remaining = tnt.timer.fraction_remaining();

        // Blink faster as it gets closer to exploding
        // At start (time_remaining=1.0): blink every 0.6 seconds
        // Near end (time_remaining=0.0): blink every 0.2 seconds
        let blink_speed = 0.2 + (time_remaining * 0.4); // Range: 0.6 to 0.2
        let blink_phase = (tnt.timer.elapsed_secs() / blink_speed) % 1.0;

        // Flash white when blink_phase > 0.5
        if let Some(mut material) = materials.get_mut(&material_handle.0) {
            if blink_phase > 0.5 {
                // Flash bright white
                material.base_color = Color::srgb(2.0, 2.0, 2.0); // Brighter than white
            } else {
                // Normal color
                material.base_color = Color::WHITE;
            }
        }

        if tnt.timer.is_finished() {
            // Explode!
            commands.spawn((
                Explosion {
                    radius: TNT_EXPLOSION_RADIUS,
                },
                Transform::from_translation(transform.translation),
            ));

            commands.entity(entity).despawn();
            info!("TNT exploded at {:?}", transform.translation);
        }
    }
}

#[allow(clippy::too_many_arguments)] // Bevy system parameters
fn tnt_explosion(
    mut commands: Commands,
    explosion_query: Query<(Entity, &Explosion, &Transform), Added<Explosion>>,
    block_query: BreakableBlocks,
    spatial_hash: Res<SpatialHash>,
    mut destroyed: MessageWriter<BlockDestroyed>,
    mut broken: Local<std::collections::HashSet<Entity>>,
    mut entity_query: Query<
        (Entity, &Transform, &mut LinearVelocity),
        Or<(With<Hero>, With<Villager>, With<Enemy>)>,
    >,
    mut damage_events: MessageWriter<EntityDamaged>,
    explosion_textures: Option<Res<crate::resources::asset_handles::ExplosionTextures>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    broken.clear();
    for (explosion_entity, explosion, explosion_transform) in explosion_query.iter() {
        let explosion_pos = explosion_transform.translation;

        // Spawn multiple explosion animation sprites around the TNT (like Minecraft)
        if let Some(textures) = explosion_textures.as_ref() {
            if !textures.frames.is_empty() {
                use rand::RngExt;
                let mut rng = rand::rng();

                // Spawn 12-16 explosion particles with random positions and delays
                let particle_count = rng.random_range(12..=16);
                for _i in 0..particle_count {
                    // Random offset from explosion center - match explosion radius
                    let max_offset = TNT_EXPLOSION_RADIUS * 0.8; // 80% of explosion radius
                    let offset_x = rng.random_range(-max_offset..max_offset);
                    let offset_y = rng.random_range(-max_offset..max_offset);

                    // Staggered delays - spread explosions over 0-1200ms (much longer)
                    let delay_ms: f32 = rng.random_range(0.0..0.6); // 0-1200ms delay

                    // Smaller random size variation (2x smaller)
                    let size = rng.random_range(85.0..150.0);

                    // Random Z position for depth
                    let z_offset = rng.random_range(15.0..25.0);

                    commands.spawn((
                        crate::components::explosive::ExplosionAnimation {
                            timer: Timer::from_seconds(1.0 / 60.0, TimerMode::Repeating), // One frame per 1/60 s at any FPS
                            current_frame: 0,
                            total_frames: 24,
                            delay_timer: Timer::from_seconds(delay_ms, TimerMode::Once), // Delay before starting
                            started: false,
                        },
                        Mesh3d(meshes.add(Rectangle::new(size, size))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::WHITE,
                            base_color_texture: Some(textures.frames[0].clone()),
                            unlit: true,
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        })),
                        Transform::from_translation(Vec3::new(
                            explosion_pos.x + offset_x,
                            explosion_pos.y + offset_y,
                            z_offset,
                        )),
                        Visibility::Hidden, // Start hidden until delay finishes
                    ));
                }
            }
        }

        // Destroy blocks in radius (spatial hash, not every block in the world)
        let reach = Vec2::splat(explosion.radius);
        for block_entity in
            spatial_hash.rect_iter(explosion_pos.xy() - reach, explosion_pos.xy() + reach)
        {
            let Ok((block_transform, kind, tile)) = block_query.get(block_entity) else {
                continue;
            };
            if explosion_pos.distance(block_transform.translation) < explosion.radius {
                break_block(
                    &mut commands,
                    &mut destroyed,
                    &mut broken,
                    block_entity,
                    *kind,
                    block_transform.translation,
                    tile.copied(),
                );
            }
        }

        // Push all entities away from explosion and deal damage
        let _push_blocks = 3.0; // Push entities 3 blocks away (calculated dynamically below)
        for (entity, entity_transform, mut entity_velocity) in entity_query.iter_mut() {
            let entity_pos = entity_transform.translation;
            let distance = explosion_pos.distance(entity_pos);

            if distance < explosion.radius && distance > 0.0 {
                // Calculate push direction (away from explosion)
                let direction = (entity_pos - explosion_pos).normalize();

                // Push strength decreases with distance (inverse square law)
                let push_strength = 800.0 * (1.0 - distance / explosion.radius);

                // Apply push velocity
                entity_velocity.x += direction.x * push_strength;
                entity_velocity.y += direction.y * push_strength;

                // Deal damage
                damage_events.write(EntityDamaged {
                    entity,
                    damage: TNT_DAMAGE,
                });

                info!(
                    "Explosion pushed entity at distance {} with strength {} and dealt {} damage",
                    distance, push_strength, TNT_DAMAGE
                );
            }
        }

        info!("Explosion destroyed blocks in radius {}", explosion.radius);

        // Clean up explosion entity immediately
        commands.entity(explosion_entity).despawn();
    }
}

/// Update explosion animation frames
fn update_explosion_animation(
    mut commands: Commands,
    time: Res<Time>,
    mut explosion_query: Query<(
        Entity,
        &mut crate::components::explosive::ExplosionAnimation,
        &MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
    )>,
    explosion_textures: Option<Res<crate::resources::asset_handles::ExplosionTextures>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let Some(textures) = explosion_textures.as_ref() else {
        return;
    };

    for (entity, mut animation, material_handle, mut visibility) in explosion_query.iter_mut() {
        // If animation hasn't started yet, tick the delay timer
        if !animation.started {
            animation.delay_timer.tick(time.delta());
            if animation.delay_timer.is_finished() {
                animation.started = true; // Start the animation
                *visibility = Visibility::Visible; // Make visible when animation starts

                // Play random explosion sound when animation starts
                use rand::RngExt;
                let mut rng = rand::rng();
                let sound_index = rng.random_range(1..=4);
                let sound_path = format!("audio/sfx/explode/explode{}.ogg", sound_index);
                commands.spawn((
                    AudioPlayer::new(asset_server.load(sound_path)),
                    PlaybackSettings::DESPAWN
                        .with_volume(bevy::audio::Volume::Linear(EXPLOSION_VOLUME)),
                ));
            } else {
                continue; // Keep waiting (particle stays hidden)
            }
        }

        // Animation is running
        animation.timer.tick(time.delta());

        // Advance by every interval that elapsed, so the animation lasts the same at any FPS
        let steps = animation.timer.times_finished_this_tick() as usize;
        if steps > 0 {
            animation.current_frame += steps;

            if animation.current_frame >= animation.total_frames {
                // Animation finished - despawn explosion sprite
                commands.entity(entity).despawn();
            } else {
                // Update to next frame (clamp to available frames)
                let frame_index = animation.current_frame.min(15); // Max 15 (0-15 frames)
                if let Some(mut material) = materials.get_mut(&material_handle.0) {
                    material.base_color_texture = Some(textures.frames[frame_index].clone());
                }
            }
        }
    }
}

// LASER spawning - fires downward from top of screen to hero position
pub fn spawn_laser(
    commands: &mut Commands,
    position: Vec3,
    _meshes: &mut Assets<Mesh>,
    _materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    use crate::systems::energy_vfx::{EnergyKind, EnergyVisual};

    // Calculate laser position - extends from way above to way below, covering entire screen
    // With LASER_HEIGHT = 3000, center it higher up so it extends from top of screen down
    // Laser center at Y + 900 means: top at Y+2400, bottom at Y-600
    let laser_center_y = position.y + 300.0; // Center well above hero so top edge is at screen top
    let laser_position = Vec3::new(position.x, laser_center_y, 10.0); // Z=10 to be in front

    // Main laser entity (invisible, just holds the timer and collision logic)
    // Spawn it first to get the ID, then we'll insert the ID into the component
    let laser_entity = commands
        .spawn((
            Transform::from_translation(laser_position),
            Visibility::Inherited,
        ))
        .id();

    commands.entity(laser_entity).insert((Laser {
        timer: Timer::from_seconds(LASER_DURATION, TimerMode::Once),
        width: LASER_WIDTH,
        id: laser_entity, // Use its own entity ID
    },));

    commands.entity(laser_entity).with_child((
        EnergyVisual(EnergyKind::Laser),
        Transform::IDENTITY,
        Visibility::Inherited,
    ));

    // Play laser beam sound
    commands.spawn((
        AudioPlayer::new(asset_server.load("audio/sfx/laser/laser_beam.ogg")),
        PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.5)),
    ));

    info!(
        "LASER spawned from top of screen at x={} down to hero",
        position.x
    );
}

/// Spawn laser burst barrage - creates a controller that spawns beams one by one
pub fn spawn_laser_burst(
    commands: &mut Commands,
    hero_entity: Entity,
    _meshes: &mut Assets<Mesh>,
    _materials: &mut Assets<StandardMaterial>,
    _asset_server: &AssetServer,
) {
    use std::f32::consts::PI;

    const BURST_COUNT: u32 = 10;
    let angle_step = -(2.0 * PI) / BURST_COUNT as f32; // Negative for counter-clockwise rotation

    // Spawn a barrage controller entity that follows the hero
    commands.spawn(LaserBurstBarrage {
        spawn_timer: Timer::from_seconds(0.1, TimerMode::Repeating), // Spawn every 0.1 seconds (slower)
        beams_remaining: BURST_COUNT,
        current_angle: 0.0, // Start at 0 degrees (right)
        angle_step,
        follow_entity: hero_entity, // Track hero entity to follow their position
    });

    info!(
        "LASER BURST started - will spawn {} beams around hero",
        BURST_COUNT
    );
}

fn laser_update(
    time: Res<Time>,
    mut commands: Commands,
    mut laser_query: Query<(
        Entity,
        &mut Laser,
        &Transform,
        Option<&crate::components::projectile::LaserBurstBeam>,
    )>,
    block_query: BreakableBlocks,
    spatial_hash: Res<SpatialHash>,
    mut destroyed: MessageWriter<BlockDestroyed>,
    mut broken: Local<std::collections::HashSet<Entity>>,
) {
    broken.clear();
    for (laser_entity, mut laser, laser_transform, burst_beam) in laser_query.iter_mut() {
        laser.timer.tick(time.delta());

        let laser_pos = laser_transform.translation;

        if let Some(burst_beam) = burst_beam {
            // BURST BEAM: Continuously check collision as the beam grows
            let rotation = laser_transform.rotation;
            // Get the forward direction of the beam (rotation around Z axis)
            let direction = rotation.mul_vec3(Vec3::X); // Beam extends along X axis

            let laser_half_width = laser.width / 2.0;
            let laser_half_length = burst_beam.length / 2.0;
            let block_half = 32.0; // Block size / 2 (64/2)

            // Bounding box of the rotated beam, for the spatial-hash lookup
            let start = laser_pos.xy() - direction.xy() * block_half;
            let end = laser_pos.xy() + direction.xy() * (laser_half_length * 2.0 + block_half);
            let pad = Vec2::splat(laser_half_width + block_half);
            for block_entity in spatial_hash.rect_iter(start.min(end) - pad, start.max(end) + pad) {
                let Ok((block_transform, kind, tile)) = block_query.get(block_entity) else {
                    continue;
                };
                let block_pos = block_transform.translation;

                // Vector from laser center to block
                let to_block = block_pos - laser_pos;

                // Project onto beam direction to get distance along beam
                let along_beam = to_block.dot(direction);

                // Get perpendicular direction for width check
                let perpendicular = Vec3::new(-direction.y, direction.x, 0.0);
                let across_beam = to_block.dot(perpendicular).abs();

                // Check if block is within beam's rotated bounding box
                if along_beam > -block_half
                    && along_beam < laser_half_length * 2.0 + block_half
                    && across_beam < laser_half_width + block_half
                {
                    break_block(
                        &mut commands,
                        &mut destroyed,
                        &mut broken,
                        block_entity,
                        *kind,
                        block_pos,
                        tile.copied(),
                    );
                }
            }
        } else if laser.timer.elapsed_secs() < 0.05 {
            // REGULAR LASER: Only check on first frame (instant beam from top of screen)
            let laser_half_width = laser.width / 2.0;
            let laser_half_height = LASER_HEIGHT / 2.0;
            let block_half = 32.0;
            let reach = Vec2::new(
                laser_half_width + block_half,
                laser_half_height + block_half,
            );

            for block_entity in
                spatial_hash.rect_iter(laser_pos.xy() - reach, laser_pos.xy() + reach)
            {
                let Ok((block_transform, kind, tile)) = block_query.get(block_entity) else {
                    continue;
                };
                let block_pos = block_transform.translation;

                // Check if block is within laser's horizontal range and vertical range
                if block_pos.x > laser_pos.x - laser_half_width - block_half
                    && block_pos.x < laser_pos.x + laser_half_width + block_half
                    && block_pos.y > laser_pos.y - laser_half_height - block_half
                    && block_pos.y < laser_pos.y + laser_half_height + block_half
                {
                    break_block(
                        &mut commands,
                        &mut destroyed,
                        &mut broken,
                        block_entity,
                        *kind,
                        block_pos,
                        tile.copied(),
                    );
                }
            }
        }

        if laser.timer.is_finished() {
            // Despawn laser entity itself
            commands.entity(laser_entity).despawn();
        }
    }
}

/// System to spawn laser beams one by one from burst barrage controllers
fn update_laser_burst_barrage(
    mut commands: Commands,
    mut barrage_query: Query<(Entity, &mut LaserBurstBarrage)>,
    hero_query: Query<&Transform, With<Hero>>,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
) {
    use crate::systems::energy_vfx::{EnergyKind, EnergyVisual};

    const BURST_LENGTH: f32 = 500.0; // ~8 blocks - longer beams
    const BURST_WIDTH: f32 = 15.0; // Thin beams

    for (barrage_entity, mut barrage) in barrage_query.iter_mut() {
        barrage.spawn_timer.tick(time.delta());

        if barrage.spawn_timer.just_finished() && barrage.beams_remaining > 0 {
            // Get current hero position (follow hero as they move)
            let Ok(hero_transform) = hero_query.get(barrage.follow_entity) else {
                // Hero doesn't exist anymore - despawn barrage
                commands.entity(barrage_entity).despawn();
                continue;
            };

            let hero_pos = hero_transform.translation;
            let angle = barrage.current_angle;
            let direction = Vec2::new(angle.cos(), angle.sin());

            // Beam starts at hero position and extends outward
            // Rectangle mesh: width=X, height=Y. For a thin horizontal beam, width should be length, height should be thin
            // We offset by half length in the beam direction so beam starts at hero position
            let beam_center = hero_pos + (direction * BURST_LENGTH / 2.0).extend(0.0);
            let beam_position = Vec3::new(beam_center.x, beam_center.y, -2.0); // Slightly behind hero

            // Rotation to point the beam in the right direction
            // By default, Rectangle extends along X axis (width), so we just rotate by the angle
            let rotation = Quat::from_rotation_z(angle);

            // Main laser entity (this is what gets checked for block destruction)
            let laser_entity = commands
                .spawn((
                    Transform::from_translation(beam_position).with_rotation(rotation),
                    Visibility::Inherited,
                ))
                .id();

            commands.entity(laser_entity).insert((
                Laser {
                    timer: Timer::from_seconds(0.25, TimerMode::Once), // Short duration - max 2-3 beams visible
                    width: BURST_WIDTH * 0.1,                          // Start very thin
                    id: laser_entity,
                },
                crate::components::projectile::LaserBurstBeam {
                    length: BURST_LENGTH * 0.1,           // Start very short
                    follow_entity: barrage.follow_entity, // Follow the hero
                    angle,                                // Store the angle offset
                    growth_timer: Timer::from_seconds(0.2, TimerMode::Once), // Grow over 0.2 seconds (slower)
                    target_length: BURST_LENGTH,
                    target_width: BURST_WIDTH,
                },
            ));

            commands.entity(laser_entity).with_child((
                EnergyVisual(EnergyKind::Burst),
                Transform::IDENTITY,
                Visibility::Inherited,
            ));

            // Play burst sound for each beam spawn
            commands.spawn((
                AudioPlayer::new(asset_server.load("audio/sfx/burst/burst.ogg")),
                PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.25)),
            ));

            // Update for next beam
            barrage.current_angle += barrage.angle_step;
            barrage.beams_remaining -= 1;

            // Despawn controller when all beams spawned
            if barrage.beams_remaining == 0 {
                commands.entity(barrage_entity).despawn();
                info!("LASER BURST complete");
            }
        }
    }
}

/// System to animate laser burst beam growth
fn animate_laser_burst_growth(
    time: Res<Time>,
    mut lasers: Query<(
        &mut Laser,
        &mut crate::components::projectile::LaserBurstBeam,
    )>,
) {
    for (mut laser, mut beam) in &mut lasers {
        beam.growth_timer.tick(time.delta());
        let progress = 1.0 - (1.0 - beam.growth_timer.fraction()).powi(3);
        beam.length = beam.target_length * 0.1 + beam.target_length * 0.9 * progress;
        laser.width = beam.target_width * 0.1 + beam.target_width * 0.9 * progress;
    }
}

/// System to update laser burst beam positions to follow the hero
fn update_laser_burst_positions(
    mut lasers: Query<
        (
            &mut Transform,
            &crate::components::projectile::LaserBurstBeam,
        ),
        With<Laser>,
    >,
    heroes: Query<&Transform, (With<Hero>, Without<Laser>)>,
) {
    for (mut transform, beam) in &mut lasers {
        let Ok(hero) = heroes.get(beam.follow_entity) else {
            continue;
        };
        let direction = Vec2::new(beam.angle.cos(), beam.angle.sin());
        let center = hero.translation + (direction * beam.length / 2.0).extend(0.0);
        transform.translation = Vec3::new(center.x, center.y, -2.0);
        transform.rotation = Quat::from_rotation_z(beam.angle);
    }
}

// ARROW spawning - creates a barrage controller that spawns arrows one by one
pub fn spawn_arrows(
    commands: &mut Commands,
    hero_entity: Entity,
    _meshes: &mut Assets<Mesh>,
    _materials: &mut Assets<StandardMaterial>,
    _projectile_textures: &crate::resources::asset_handles::ProjectileTextures,
) {
    let num_arrows = 48; // 48 arrows total = 2 full rotations (every 7.5 degrees)
    let angle_step = -std::f32::consts::TAU / 24.0; // Step size for 24 arrows per rotation

    // Spawn a barrage controller entity that follows the hero
    commands.spawn(ArrowBarrage {
        spawn_timer: Timer::from_seconds(0.05, TimerMode::Repeating), // Spawn every 0.05 seconds
        arrows_remaining: num_arrows,
        current_angle: 0.0, // Start at 0 degrees (right)
        angle_step,
        follow_entity: hero_entity, // Track hero entity to follow their position
    });

    info!(
        "ARROW barrage started - will spawn {} arrows (2 rotations) counter-clockwise following hero",
        num_arrows
    );
}

/// System to spawn arrows one by one from barrage controllers
fn update_arrow_barrage(
    mut commands: Commands,
    mut barrage_query: Query<(Entity, &mut ArrowBarrage)>,
    hero_query: Query<&Transform, With<Hero>>,
    time: Res<Time>,
    projectile_models: Option<Res<crate::resources::asset_handles::ProjectileModels>>,
) {
    let Some(projectile_models) = projectile_models else {
        return;
    };

    for (barrage_entity, mut barrage) in barrage_query.iter_mut() {
        barrage.spawn_timer.tick(time.delta());

        if barrage.spawn_timer.just_finished() && barrage.arrows_remaining > 0 {
            // Get current hero position (follow hero as they move)
            let Ok(hero_transform) = hero_query.get(barrage.follow_entity) else {
                // Hero doesn't exist anymore - despawn barrage
                commands.entity(barrage_entity).despawn();
                continue;
            };

            let hero_pos = hero_transform.translation;

            // Calculate direction vector
            let direction = Vec3::new(
                barrage.current_angle.cos(),
                barrage.current_angle.sin(),
                0.0,
            );
            let velocity = direction * ARROW_SPEED;

            // Spawn arrow slightly offset from hero to avoid instant collision
            let spawn_offset = 80.0;
            let spawn_pos = Vec3::new(
                hero_pos.x + direction.x * spawn_offset,
                hero_pos.y + direction.y * spawn_offset,
                10.0, // Z=10 same as entities
            );

            // Spawn arrow using GLB model with Avian physics
            commands.spawn((
                crate::components::projectile::Arrow::new_friendly(1), // Friendly arrow - won't hit hero/villagers
                RigidBody::Dynamic,
                LinearVelocity(velocity),
                GravityScale(1.0),     // Use global gravity
                Mass(0.1),             // CRITICAL: Must have mass to prevent NaN
                Collider::sphere(8.0), // Small spherical collider for arrow tip (follows center of mass)
                Sensor,                // Don't physically collide, just detect overlaps
                CollisionLayers::new(GameLayer::Arrow, GameLayer::Arrow.filters()),
                LockedAxes::ROTATION_LOCKED, // Prevent physics from rotating arrow (we control rotation manually)
                NoTransformEasing, // Disable interpolation for fast-moving arrows (causes visual lag)
                WorldAssetRoot(projectile_models.arrow.clone()),
                Transform::from_translation(spawn_pos).with_scale(Vec3::new(15.0, 15.0, 15.0)),
                Visibility::Visible,
            ));

            // Update for next arrow
            barrage.current_angle += barrage.angle_step;
            barrage.arrows_remaining -= 1;

            // Despawn barrage controller when done
            if barrage.arrows_remaining == 0 {
                commands.entity(barrage_entity).despawn();
                info!("ARROW barrage completed!");
            }
        }
    }
}

/// Spawn a magic barrier around an entity that reflects projectiles
pub fn spawn_magic_barrier(
    commands: &mut Commands,
    owner_entity: Entity,
    owner_position: Vec3,
    asset_server: &Res<AssetServer>,
) {
    use crate::{components::projectile::MagicBarrier, systems::mirror_vfx::MirrorVisual};
    let radius = 150.0;
    commands
        .spawn((
            MagicBarrier {
                lifetime: Timer::from_seconds(10.0, TimerMode::Once),
                radius,
                owner_entity,
            },
            Transform::from_translation(owner_position),
            Visibility::Inherited,
            AudioPlayer::new(asset_server.load("audio/sfx/shield-mirror/Steady_75_Hz_sub-bas.ogg")),
            PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::Linear(0.3)),
        ))
        .with_child((
            MirrorVisual,
            Transform::from_scale(Vec3::splat(radius)),
            Visibility::Inherited,
        ));
}

/// Update barrier positions to follow the owner entity (hero)
fn barrier_update(
    mut commands: Commands,
    time: Res<Time>,
    mut barriers: Query<(
        Entity,
        &mut crate::components::projectile::MagicBarrier,
        &mut Transform,
    )>,
    owners: Query<
        &Transform,
        (
            With<Hero>,
            Without<crate::components::projectile::MagicBarrier>,
        ),
    >,
) {
    for (entity, mut barrier, mut transform) in &mut barriers {
        barrier.lifetime.tick(time.delta());
        if barrier.lifetime.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }
        if let Ok(owner) = owners.get(barrier.owner_entity) {
            transform.translation = owner.translation + Vec3::Z * 5.;
        } else {
            commands.entity(entity).despawn();
        }
    }
}

/// Reflect projectiles that hit the magic barrier
fn barrier_reflect_projectiles(
    mut commands: Commands,
    barrier_query: Query<(
        Entity,
        &Transform,
        &crate::components::projectile::MagicBarrier,
    )>,
    mut arrow_query: Query<(&Transform, &mut LinearVelocity, &mut Arrow)>,
    mut fireball_query: Query<
        (
            &Transform,
            &mut LinearVelocity,
            &mut crate::components::projectile::Fireball,
        ),
        Without<Arrow>,
    >,
    mut snowball_query: Query<
        (
            &Transform,
            &mut LinearVelocity,
            &mut crate::components::projectile::Snowball,
        ),
        (
            Without<Arrow>,
            Without<crate::components::projectile::Fireball>,
        ),
    >,
) {
    use crate::components::projectile::Team;

    for (barrier_entity, barrier_transform, barrier) in barrier_query.iter() {
        let barrier_center = barrier_transform.translation;

        // Reflect arrows
        for (arrow_transform, mut arrow_velocity, mut arrow) in arrow_query.iter_mut() {
            // Skip if already reflected
            if arrow.reflected {
                continue;
            }

            let distance = barrier_transform
                .translation
                .distance(arrow_transform.translation);

            if distance < barrier.radius + 20.0 && distance > barrier.radius - 20.0 {
                // Calculate projectile direction (where it's coming from)

                // Calculate reflection angle (away from barrier center)
                let dx = arrow_transform.translation.x - barrier_center.x;
                let dy = arrow_transform.translation.y - barrier_center.y;
                let bounce_angle = dy.atan2(dx);

                // Preserve arrow's speed (just bounce, no acceleration)
                let current_speed = (arrow_velocity.x.powi(2) + arrow_velocity.y.powi(2)).sqrt();
                let bounce_speed = current_speed; // Keep same speed, just reflect direction

                arrow_velocity.x = bounce_angle.cos() * bounce_speed;
                arrow_velocity.y = bounce_angle.sin() * bounce_speed;

                // Switch team (enemy → friendly, friendly → enemy)
                arrow.team = match arrow.team {
                    Team::Enemy => Team::Friendly,
                    Team::Friendly => Team::Enemy,
                };

                // Mark as reflected to prevent multiple reflections
                arrow.reflected = true;

                // Spawn impact effect at collision point with projectile direction
                spawn_barrier_impact(
                    &mut commands,
                    barrier_entity,
                    arrow_transform.translation,
                    barrier_center,
                );

                info!(
                    "�� Arrow reflected at {:.0} speed! New team: {:?}",
                    bounce_speed, arrow.team
                );
            }
        }

        // Reflect fireballs - use inside-radius detection for fast projectiles
        for (fireball_transform, mut fireball_velocity, mut fireball) in fireball_query.iter_mut() {
            // Skip if already reflected
            if fireball.reflected {
                continue;
            }

            let fireball_pos = fireball_transform.translation;
            let dx = fireball_pos.x - barrier_center.x;
            let dy = fireball_pos.y - barrier_center.y;
            let distance = (dx * dx + dy * dy).sqrt();

            // Detect if fireball is inside or very close to barrier edge
            // Fireballs are 32 units, so detect within radius + 64 to catch fast movement
            if distance < barrier.radius + 64.0 {
                // Check if moving towards center (only reflect once)
                let to_center_x = barrier_center.x - fireball_pos.x;
                let to_center_y = barrier_center.y - fireball_pos.y;
                let dot_product =
                    fireball_velocity.x * to_center_x + fireball_velocity.y * to_center_y;

                // Only reflect if moving towards center (positive dot product)
                if dot_product > 0.0 {
                    // Calculate projectile direction (where it's coming from)

                    // Calculate reflection angle (away from barrier center)
                    let bounce_angle = dy.atan2(dx);

                    // Preserve fireball's speed
                    let current_speed =
                        (fireball_velocity.x.powi(2) + fireball_velocity.y.powi(2)).sqrt();

                    fireball_velocity.x = bounce_angle.cos() * current_speed;
                    fireball_velocity.y = bounce_angle.sin() * current_speed;

                    // Switch team (enemy → friendly, friendly → enemy)
                    fireball.team = match fireball.team {
                        Team::Enemy => Team::Friendly,
                        Team::Friendly => Team::Enemy,
                    };

                    // Mark as reflected to prevent multiple reflections
                    fireball.reflected = true;

                    // Spawn impact effect at collision point with projectile direction
                    spawn_barrier_impact(
                        &mut commands,
                        barrier_entity,
                        fireball_pos,
                        barrier_center,
                    );

                    info!(
                        "🪞 Fireball reflected at {:.0} speed! Distance={:.1}, New team: {:?}",
                        current_speed, distance, fireball.team
                    );
                }
            }
        }

        // Reflect snowballs - use inside-radius detection
        for (snowball_transform, mut snowball_velocity, mut snowball) in snowball_query.iter_mut() {
            let snowball_pos = snowball_transform.translation;
            let dx = snowball_pos.x - barrier_center.x;
            let dy = snowball_pos.y - barrier_center.y;
            let distance = (dx * dx + dy * dy).sqrt();

            // Detect if snowball is inside or very close to barrier edge
            // Snowballs are ~32 units, so detect within radius + 64 to catch fast movement
            if distance < barrier.radius + 64.0 {
                // Check if moving towards center (only reflect once)
                let to_center_x = barrier_center.x - snowball_pos.x;
                let to_center_y = barrier_center.y - snowball_pos.y;
                let dot_product =
                    snowball_velocity.x * to_center_x + snowball_velocity.y * to_center_y;

                // Only reflect if moving towards center (positive dot product)
                if dot_product > 0.0 {
                    // Calculate projectile direction (where it's coming from)

                    // Calculate reflection angle (away from barrier center)
                    let bounce_angle = dy.atan2(dx);

                    // Preserve snowball's speed
                    let current_speed =
                        (snowball_velocity.x.powi(2) + snowball_velocity.y.powi(2)).sqrt();
                    snowball_velocity.x = bounce_angle.cos() * current_speed;
                    snowball_velocity.y = bounce_angle.sin() * current_speed;

                    // Switch team
                    snowball.team = match snowball.team {
                        Team::Enemy => Team::Friendly,
                        Team::Friendly => Team::Enemy,
                    };

                    // Spawn impact effect at collision point with projectile direction
                    spawn_barrier_impact(
                        &mut commands,
                        barrier_entity,
                        snowball_pos,
                        barrier_center,
                    );

                    info!(
                        "🪞 Snowball reflected at {:.0} speed! Distance={:.1}, New team: {:?}",
                        current_speed, distance, snowball.team
                    );
                }
            }
        }
    }
}

/// Spawn a cyan glow impact effect at the barrier collision point
/// Creates a single curved arc oriented radially along the barrier curve
/// The arc is parented to the barrier so it moves with it
pub(crate) fn spawn_barrier_impact(
    commands: &mut Commands,
    barrier_entity: Entity,
    impact_position: Vec3,
    barrier_center: Vec3,
) {
    use crate::{
        components::projectile::BarrierImpact,
        systems::energy_vfx::{EnergyKind, EnergyVisual},
    };
    let offset = impact_position - barrier_center;
    let angle = offset.y.atan2(offset.x) + std::f32::consts::FRAC_PI_2;
    commands.entity(barrier_entity).with_child((
        BarrierImpact {
            lifetime: Timer::from_seconds(0.4, TimerMode::Once),
        },
        EnergyVisual(EnergyKind::Impact),
        Visibility::Inherited,
        Transform::from_translation(Vec3::new(offset.x, offset.y, 8.))
            .with_rotation(Quat::from_rotation_z(angle))
            .with_scale(Vec3::new(120., 80., 1.)),
    ));
}

/// Animate and fade out barrier impact effects
fn update_barrier_impacts(
    mut commands: Commands,
    time: Res<Time>,
    mut impacts: Query<(Entity, &mut crate::components::projectile::BarrierImpact)>,
) {
    for (entity, mut impact) in &mut impacts {
        impact.lifetime.tick(time.delta());
        if impact.lifetime.is_finished() {
            commands.entity(entity).try_despawn();
        }
    }
}

/// Spawn a magnetic field around the hero that attracts resources
pub fn spawn_magnet_field(
    commands: &mut Commands,
    owner_entity: Entity,
    owner_position: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &Res<AssetServer>,
) {
    use crate::components::projectile::MagnetField;

    let magnet_radius = 600.0; // Radius in which resources are attracted (doubled)
    let pull_strength = 2000.0; // Force applied to pull resources (4x stronger)

    // Load magnet visual effect (golden/orange glowing ring)
    let magnet_texture = asset_server.load("textures/effects/magnet_field.png");

    // Spawn magnet field visual
    commands.spawn((
        MagnetField {
            lifetime: Timer::from_seconds(8.0, TimerMode::Once),
            radius: magnet_radius,
            pull_strength,
            owner_entity,
        },
        Mesh3d(meshes.add(Circle::new(magnet_radius))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.8, 0.0, 0.4), // Golden semi-transparent
            base_color_texture: Some(magnet_texture),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })),
        Transform::from_translation(owner_position).with_scale(Vec3::splat(1.2)),
    ));

    info!("🧲 Magnet field spawned around entity {:?}", owner_entity);
}

/// Update magnet field positions to follow the owner (hero) and apply magnetic pull to resources
fn magnet_update(
    mut commands: Commands,
    time: Res<Time>,
    mut magnet_query: Query<(
        Entity,
        &mut crate::components::projectile::MagnetField,
        &mut Transform,
    )>,
    hero_query: Query<
        &Transform,
        (
            With<crate::components::hero::Hero>,
            Without<crate::components::projectile::MagnetField>,
        ),
    >,
    mut resource_query: Query<
        (&Transform, &mut LinearVelocity),
        (
            With<crate::components::resource::Resource>,
            Without<crate::components::projectile::MagnetField>,
        ),
    >,
) {
    for (magnet_entity, mut magnet, mut magnet_transform) in magnet_query.iter_mut() {
        magnet.lifetime.tick(time.delta());

        // Despawn when lifetime expires
        if magnet.lifetime.is_finished() {
            commands.entity(magnet_entity).despawn();
            info!("🧲 Magnet field expired");
            continue;
        }

        // Follow the hero
        if let Ok(hero_transform) = hero_query.get(magnet.owner_entity) {
            let hero_pos = hero_transform.translation;
            magnet_transform.translation = hero_pos;

            // Apply magnetic pull to all resources within radius
            for (resource_transform, mut resource_velocity) in resource_query.iter_mut() {
                let resource_pos = resource_transform.translation;
                let dx = hero_pos.x - resource_pos.x;
                let dy = hero_pos.y - resource_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                // Only pull if within magnet radius
                if distance < magnet.radius && distance > 10.0 {
                    // Normalize direction and apply pull force
                    let pull_x = (dx / distance) * magnet.pull_strength;
                    let mut pull_y = (dy / distance) * magnet.pull_strength;

                    // BOOST upward pull to overcome gravity when hero is above resource
                    if dy > 0.0 {
                        // Hero is above the resource
                        pull_y *= 3.0; // Triple the upward force to overcome gravity
                    }

                    // Add pull force to current velocity
                    resource_velocity.x += pull_x * time.delta_secs();
                    resource_velocity.y += pull_y * time.delta_secs();
                }
            }
        } else {
            // Owner doesn't exist, despawn magnet
            commands.entity(magnet_entity).despawn();
        }
    }
}

#[cfg(test)]
mod beam_cleanup_tests {
    use super::*;
    #[test]
    fn overlapping_beams_remove_a_block_and_its_embedded_arrow_once() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(SpatialHash::new(256.))
            .add_message::<BlockDestroyed>()
            .add_systems(Update, laser_update);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.01));
        let block = app
            .world_mut()
            .spawn((
                Block {
                    health: 1,
                    max_health: 1,
                },
                crate::components::block::BlockType::Stone,
                Transform::IDENTITY,
            ))
            .id();
        app.world_mut()
            .resource_mut::<SpatialHash>()
            .insert(block, 0., 0.);
        let child = app
            .world_mut()
            .spawn((ChildOf(block), Transform::IDENTITY))
            .id();
        for _ in 0..2 {
            let e = app.world_mut().spawn(Transform::IDENTITY).id();
            app.world_mut().entity_mut(e).insert(Laser {
                timer: Timer::from_seconds(1., TimerMode::Once),
                width: 100.,
                id: e,
            });
        }
        app.update();
        assert!(app.world().get_entity(block).is_err());
        assert!(app.world().get_entity(child).is_err());
        let destroyed = app.world().resource::<Messages<BlockDestroyed>>();
        assert_eq!(
            destroyed.iter_current_update_messages().count(),
            1,
            "one block, one destroy event"
        );
    }
}
