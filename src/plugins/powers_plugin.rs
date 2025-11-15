use bevy::prelude::*;
use bevy::scene::SceneRoot;
use crate::components::{
    block::Block,
    explosive::{Tnt, Explosion},
    physics::Velocity,
    projectile::{Laser, Arrow},
    hero::Hero,
    villager::Villager,
    enemy::Enemy,
    bundles::avian_physics,
    collision_layers::GameLayer,
};
use crate::events::damage_events::EntityDamaged;
use crate::states::GameState;
use crate::utils::constants::*;
use avian3d::prelude::*;

/// Component to handle sequential arrow spawning
#[derive(Component)]
pub struct ArrowBarrage {
    pub spawn_timer: Timer,
    pub arrows_remaining: u32,
    pub current_angle: f32,
    pub angle_step: f32,
    pub follow_entity: Entity,  // Follow this entity's position (hero)
}

pub struct PowersPlugin;

impl Plugin for PowersPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                tnt_countdown,
                tnt_explosion,
                update_explosion_animation,
                laser_update,
                update_arrow_barrage,
                barrier_update,
                barrier_reflect_projectiles,
                update_barrier_impacts,
                magnet_update,
            ).run_if(in_state(GameState::Playing)));
            // arrow_update REMOVED - handled by projectile_system with spatial hash instead

        info!("PowersPlugin loaded");
    }
}

// TNT spawning will be triggered by API
pub fn spawn_tnt(
    commands: &mut Commands,
    position: Vec3,
    block_atlases: &crate::resources::asset_handles::BlockAtlases,
    mesh_cache: &crate::resources::mesh_cache::MeshCache,
    meshes: &mut Assets<Mesh>,  // Still needed for username label
    materials: &mut Assets<StandardMaterial>,
    username: Option<String>,
    asset_server: Option<&AssetServer>,
) {
    
    use crate::systems::ui::username_label::spawn_username_label;

    // Spawn TNT as 3D cube using same UV-mapped mesh system as blocks
    let tnt_entity = commands.spawn((
        Tnt {
            timer: Timer::from_seconds(TNT_FUSE_TIME, TimerMode::Once),
        },
        avian_physics::tnt(),  // Avian physics for TNT
        Mesh3d(mesh_cache.tnt_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(block_atlases.tnt.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            // Use default backface culling - only show front faces
            ..default()
        })),
        Transform::from_translation(position),  // No rotation needed - UVs are correct
    )).id();

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
    mut tnt_query: Query<(Entity, &mut Tnt, &Transform, &MeshMaterial3d<StandardMaterial>)>,
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
        if let Some(material) = materials.get_mut(&material_handle.0) {
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
                Explosion { radius: TNT_EXPLOSION_RADIUS },
                Transform::from_translation(transform.translation),
            ));

            commands.entity(entity).despawn();
            info!("TNT exploded at {:?}", transform.translation);
        }
    }
}

fn tnt_explosion(
    mut commands: Commands,
    explosion_query: Query<(Entity, &Explosion, &Transform), Added<Explosion>>,
    block_query: Query<(Entity, &Transform), With<Block>>,
    mut entity_query: Query<(Entity, &Transform, &mut LinearVelocity), Or<(With<Hero>, With<Villager>, With<Enemy>)>>,
    mut damage_events: MessageWriter<EntityDamaged>,
    explosion_textures: Option<Res<crate::resources::asset_handles::ExplosionTextures>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (explosion_entity, explosion, explosion_transform) in explosion_query.iter() {
        let explosion_pos = explosion_transform.translation;

        // Spawn multiple explosion animation sprites around the TNT (like Minecraft)
        if let Some(textures) = explosion_textures.as_ref() {
            if !textures.frames.is_empty() {
                use rand::Rng;
                let mut rng = rand::thread_rng();

                // Spawn 12-16 explosion particles with random positions and delays
                let particle_count = rng.gen_range(12..=16);
                for _i in 0..particle_count {
                    // Random offset from explosion center - match explosion radius
                    let max_offset = TNT_EXPLOSION_RADIUS * 0.8; // 80% of explosion radius
                    let offset_x = rng.gen_range(-max_offset..max_offset);
                    let offset_y = rng.gen_range(-max_offset..max_offset);

                    // Staggered delays - spread explosions over 0-1200ms (much longer)
                    let delay_ms: f32 = rng.gen_range(0.0..0.6); // 0-1200ms delay

                    // Smaller random size variation (2x smaller)
                    let size = rng.gen_range(85.0..150.0);

                    // Random Z position for depth
                    let z_offset = rng.gen_range(15.0..25.0);

                    commands.spawn((
                        crate::components::explosive::ExplosionAnimation {
                            timer: Timer::from_seconds(0.01, TimerMode::Repeating), // 20ms per frame (much faster)
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

        // Destroy all blocks in radius
        for (block_entity, block_transform) in block_query.iter() {
            let distance = explosion_pos.distance(block_transform.translation);

            if distance < explosion.radius {
                commands.entity(block_entity).despawn(); // Despawn children too (damage overlays)
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

                info!("Explosion pushed entity at distance {} with strength {} and dealt {} damage", distance, push_strength, TNT_DAMAGE);
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
    mut explosion_query: Query<(Entity, &mut crate::components::explosive::ExplosionAnimation, &MeshMaterial3d<StandardMaterial>, &mut Visibility)>,
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
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let sound_index = rng.gen_range(1..=4);
                let sound_path = format!("audio/sfx/explode/explode{}.ogg", sound_index);
                commands.spawn((
                    AudioPlayer::new(asset_server.load(sound_path)),
                    PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(EXPLOSION_VOLUME)),
                ));
            } else {
                continue; // Keep waiting (particle stays hidden)
            }
        }

        // Animation is running
        animation.timer.tick(time.delta());

        if animation.timer.just_finished() {
            animation.current_frame += 1;

            if animation.current_frame >= animation.total_frames {
                // Animation finished - despawn explosion sprite
                commands.entity(entity).despawn();
            } else {
                // Update to next frame (clamp to available frames)
                let frame_index = animation.current_frame.min(15); // Max 15 (0-15 frames)
                if let Some(material) = materials.get_mut(&material_handle.0) {
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
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    use crate::components::projectile::{LaserCore, LaserGlow};

    // Calculate laser position - extends from way above to way below, covering entire screen
    // With LASER_HEIGHT = 3000, center it higher up so it extends from top of screen down
    // Laser center at Y + 900 means: top at Y+2400, bottom at Y-600
    let laser_center_y = position.y + 300.0; // Center well above hero so top edge is at screen top
    let laser_position = Vec3::new(position.x, laser_center_y, 10.0); // Z=10 to be in front

    // Main laser entity (invisible, just holds the timer and collision logic)
    // Spawn it first to get the ID, then we'll insert the ID into the component
    let laser_entity = commands.spawn(Transform::from_translation(laser_position)).id();

    commands.entity(laser_entity).insert((
        Laser {
            timer: Timer::from_seconds(LASER_DURATION, TimerMode::Once),
            width: LASER_WIDTH,
            id: laser_entity,  // Use its own entity ID
        },
    ));

    // Layer 1: Outer glow (wide, soft purple, very transparent)
    let glow_width = LASER_WIDTH * 1.5;
    commands.spawn((
        LaserGlow { laser_id: laser_entity },
        Mesh3d(meshes.add(Rectangle::new(glow_width, LASER_HEIGHT))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.8, 0.2, 1.0, 0.5), // More visible - increased alpha
            emissive: LinearRgba::new(1.5, 0.5, 1.8, 1.0), // Brighter glow
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_translation(laser_position + Vec3::new(0.0, 0.0, -1.0)),
        Visibility::Visible,
    ));

    // Layer 2: Mid beam (medium width, brighter purple)
    let mid_width = LASER_WIDTH * 0.7;
    commands.spawn((
        LaserCore { laser_id: laser_entity },
        Mesh3d(meshes.add(Rectangle::new(mid_width, LASER_HEIGHT))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.3, 1.0, 0.7), // Bright magenta, very visible
            emissive: LinearRgba::new(2.5, 0.8, 2.5, 1.0), // Brighter magenta glow
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_translation(laser_position + Vec3::new(0.0, 0.0, 0.0)),
        Visibility::Visible,
    ));

    // Layer 3: Core beam (narrow, very bright white/cyan center)
    let core_width = LASER_WIDTH * 0.4;
    commands.spawn((
        LaserCore { laser_id: laser_entity },
        Mesh3d(meshes.add(Rectangle::new(core_width, LASER_HEIGHT))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 1.0), // Bright white, fully visible
            emissive: LinearRgba::new(4.0, 3.5, 4.0, 1.0), // VERY bright white-cyan core
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_translation(laser_position + Vec3::new(0.0, 0.0, 1.0)),
        Visibility::Visible,
    ));


    // Play laser beam sound
    commands.spawn((
        AudioPlayer::new(asset_server.load("audio/sfx/laser/laser_beam.ogg")),
        PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.5)),
    ));

    info!("LASER spawned from top of screen at x={} down to hero", position.x);
}

fn laser_update(
    time: Res<Time>,
    mut commands: Commands,
    mut laser_query: Query<(Entity, &mut Laser, &Transform)>,
    core_query: Query<(Entity, &crate::components::projectile::LaserCore)>,
    glow_query: Query<(Entity, &crate::components::projectile::LaserGlow)>,
    block_query: Query<(Entity, &Transform), With<Block>>,
) {
    for (laser_entity, mut laser, laser_transform) in laser_query.iter_mut() {
        laser.timer.tick(time.delta());

        // Destroy blocks that overlap with laser on first frame
        if laser.timer.elapsed_secs() < 0.05 {
            let laser_pos = laser_transform.translation;
            let laser_half_width = laser.width / 2.0;
            let laser_half_height = LASER_HEIGHT / 2.0;

            for (block_entity, block_transform) in block_query.iter() {
                let block_pos = block_transform.translation;
                let block_half = 32.0; // 3D scale: Block size / 2 (64/2)

                // Check if block is within laser's horizontal range and vertical range
                if block_pos.x > laser_pos.x - laser_half_width - block_half
                    && block_pos.x < laser_pos.x + laser_half_width + block_half
                    && block_pos.y > laser_pos.y - laser_half_height - block_half // Above bottom of laser
                    && block_pos.y < laser_pos.y + laser_half_height + block_half // Below top of laser
                {
                    commands.entity(block_entity).despawn(); // Despawn children too (damage overlays)
                }
            }
        }

        if laser.timer.is_finished() {
            // Despawn only the beam layers that belong to THIS laser
            for (beam_entity, core) in core_query.iter() {
                if core.laser_id == laser_entity {
                    commands.entity(beam_entity).despawn();
                }
            }
            for (beam_entity, glow) in glow_query.iter() {
                if glow.laser_id == laser_entity {
                    commands.entity(beam_entity).despawn();
                }
            }

            // Despawn laser entity itself
            commands.entity(laser_entity).despawn();
            info!("LASER expired");
        }
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
    let num_arrows = 48;  // 48 arrows total = 2 full rotations (every 7.5 degrees)
    let angle_step = -std::f32::consts::TAU / 24.0;  // Step size for 24 arrows per rotation

    // Spawn a barrage controller entity that follows the hero
    commands.spawn(ArrowBarrage {
        spawn_timer: Timer::from_seconds(0.05, TimerMode::Repeating),  // Spawn every 0.05 seconds
        arrows_remaining: num_arrows,
        current_angle: 0.0,  // Start at 0 degrees (right)
        angle_step,
        follow_entity: hero_entity,  // Track hero entity to follow their position
    });

    info!("ARROW barrage started - will spawn {} arrows (2 rotations) counter-clockwise following hero", num_arrows);
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
            let direction = Vec3::new(barrage.current_angle.cos(), barrage.current_angle.sin(), 0.0);
            let velocity = direction * ARROW_SPEED;

            // Spawn arrow slightly offset from hero to avoid instant collision
            let spawn_offset = 80.0;
            let spawn_pos = Vec3::new(
                hero_pos.x + direction.x * spawn_offset,
                hero_pos.y + direction.y * spawn_offset,
                10.0  // Z=10 same as entities
            );

            // Spawn arrow using GLB model with Avian physics
            commands.spawn((
                crate::components::projectile::Arrow::new_friendly(1),  // Friendly arrow - won't hit hero/villagers
                RigidBody::Dynamic,
                LinearVelocity(velocity),
                GravityScale(1.0),  // Use global gravity
                Mass(0.1),  // CRITICAL: Must have mass to prevent NaN
                Collider::sphere(8.0),  // Small spherical collider for arrow tip (follows center of mass)
                Sensor,  // Don't physically collide, just detect overlaps
                CollisionLayers::new(GameLayer::Arrow, GameLayer::Arrow.filters()),
                LockedAxes::ROTATION_LOCKED,  // Prevent physics from rotating arrow (we control rotation manually)
                NoTransformEasing,  // Disable interpolation for fast-moving arrows (causes visual lag)
                SceneRoot(projectile_models.arrow.clone()),
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

fn arrow_update(
    mut commands: Commands,
    arrow_query: Query<(Entity, &Transform, &Velocity), With<Arrow>>,
    block_query: Query<(Entity, &Transform), With<Block>>,
) {
    for (arrow_entity, arrow_transform, _velocity) in arrow_query.iter() {
        let arrow_pos = arrow_transform.translation;

        // Check collision with blocks
        for (block_entity, block_transform) in block_query.iter() {
            let distance = arrow_pos.distance(block_transform.translation);

            if distance < 64.0 {  // 3D scale: block size
                commands.entity(block_entity).despawn(); // Despawn children too (damage overlays)
                commands.entity(arrow_entity).despawn();
                break;
            }
        }

        // Despawn arrows that go too far (3D scale)
        if arrow_pos.y < -2000.0 || arrow_pos.y > 2000.0
            || arrow_pos.x < -2000.0 || arrow_pos.x > 2000.0 {
            commands.entity(arrow_entity).despawn();
        }
    }
}

/// Spawn a magic barrier around an entity that reflects projectiles
pub fn spawn_magic_barrier(
    commands: &mut Commands,
    owner_entity: Entity,
    owner_position: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &Res<AssetServer>,
) {
    use crate::components::projectile::MagicBarrier;

    // Create a smooth circular ring using Circle mesh
    let barrier_radius = 150.0;

    // Preload last 4 barrier animation frames from self folder
    let frame1 = asset_server.load("textures/effects/barrier/self/001.png");
    let frame2 = asset_server.load("textures/effects/barrier/self/002.png");
    let frame3 = asset_server.load("textures/effects/barrier/self/003.png");
    let frame4 = asset_server.load("textures/effects/barrier/self/004.png");

    let frames = vec![frame1.clone(), frame2, frame3, frame4];

    // Load the barrier sound effect
    let barrier_sound = asset_server.load("audio/sfx/shield-mirror/Steady_75_Hz_sub-bas.ogg");

    commands.spawn((
        MagicBarrier {
            lifetime: Timer::from_seconds(10.0, TimerMode::Once),
            radius: barrier_radius,
            owner_entity,
            animation_timer: Timer::from_seconds(0.25, TimerMode::Repeating), // 250ms per frame = 4 frames in 1 second
            current_frame: 0,
        },
        crate::components::projectile::BarrierAnimationFrames { frames },
        Mesh3d(meshes.add(Circle::new(barrier_radius))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.8, 1.0, 1.0, 0.6), // Cyan-white tint
            base_color_texture: Some(frame1),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })),
        AudioPlayer::new(barrier_sound),
        PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::Linear(0.3)), // Loop at 30% volume
        Transform::from_translation(owner_position)
            .with_scale(Vec3::splat(1.1)), // Scale up 10%
    ));

    info!("🪞 Magic barrier spawned around entity {:?}", owner_entity);
}

/// Update barrier positions to follow the owner entity (hero)
fn barrier_update(
    mut commands: Commands,
    time: Res<Time>,
    mut barrier_query: Query<(
        Entity,
        &mut crate::components::projectile::MagicBarrier,
        &crate::components::projectile::BarrierAnimationFrames,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>
    )>,
    owner_query: Query<&Transform, (With<Hero>, Without<crate::components::projectile::MagicBarrier>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (barrier_entity, mut barrier, animation_frames, mut barrier_transform, material_handle) in barrier_query.iter_mut() {
        barrier.lifetime.tick(time.delta());

        if barrier.lifetime.is_finished() {
            commands.entity(barrier_entity).despawn();
            info!("🪞 Magic barrier expired");
            continue;
        }

        // Tick animation timer
        barrier.animation_timer.tick(time.delta());

        // Change frame when timer finishes
        if barrier.animation_timer.just_finished() {
            barrier.current_frame = (barrier.current_frame + 1) % 4; // Loop through 0-3

            // Use preloaded texture handle
            let frame_texture = animation_frames.frames[barrier.current_frame].clone();

            if let Some(material) = materials.get_mut(&material_handle.0) {
                material.base_color_texture = Some(frame_texture);
            }
        }

        // Follow owner - center on owner position
        if let Ok(owner_transform) = owner_query.get(barrier.owner_entity) {
            barrier_transform.translation.x = owner_transform.translation.x;
            barrier_transform.translation.y = owner_transform.translation.y;
            barrier_transform.translation.z = owner_transform.translation.z + 5.0;
        } else {
            commands.entity(barrier_entity).despawn();
        }
    }
}

/// Reflect projectiles that hit the magic barrier
fn barrier_reflect_projectiles(
    mut commands: Commands,
    barrier_query: Query<(Entity, &Transform, &crate::components::projectile::MagicBarrier)>,
    mut arrow_query: Query<(&Transform, &mut LinearVelocity, &mut Arrow)>,
    mut fireball_query: Query<(&Transform, &mut LinearVelocity, &mut crate::components::projectile::Fireball), Without<Arrow>>,
    mut snowball_query: Query<(&Transform, &mut LinearVelocity, &mut crate::components::projectile::Snowball), (Without<Arrow>, Without<crate::components::projectile::Fireball>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
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

            let distance = barrier_transform.translation.distance(arrow_transform.translation);

            if distance < barrier.radius + 20.0 && distance > barrier.radius - 20.0 {
                // Calculate projectile direction (where it's coming from)
                let projectile_angle = arrow_velocity.y.atan2(arrow_velocity.x);

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
                spawn_barrier_impact(&mut commands, barrier_entity, arrow_transform.translation, barrier_center, projectile_angle, &mut meshes, &mut materials, &asset_server);

                info!("�� Arrow reflected at {:.0} speed! New team: {:?}", bounce_speed, arrow.team);
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
                let dot_product = fireball_velocity.x * to_center_x + fireball_velocity.y * to_center_y;

                // Only reflect if moving towards center (positive dot product)
                if dot_product > 0.0 {
                    // Calculate projectile direction (where it's coming from)
                    let projectile_angle = fireball_velocity.y.atan2(fireball_velocity.x);

                    // Calculate reflection angle (away from barrier center)
                    let bounce_angle = dy.atan2(dx);

                    // Preserve fireball's speed
                    let current_speed = (fireball_velocity.x.powi(2) + fireball_velocity.y.powi(2)).sqrt();

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
                    spawn_barrier_impact(&mut commands, barrier_entity, fireball_pos, barrier_center, projectile_angle, &mut meshes, &mut materials, &asset_server);

                    info!("🪞 Fireball reflected at {:.0} speed! Distance={:.1}, New team: {:?}", current_speed, distance, fireball.team);
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
                let dot_product = snowball_velocity.x * to_center_x + snowball_velocity.y * to_center_y;

                // Only reflect if moving towards center (positive dot product)
                if dot_product > 0.0 {
                    // Calculate projectile direction (where it's coming from)
                    let projectile_angle = snowball_velocity.y.atan2(snowball_velocity.x);

                    // Calculate reflection angle (away from barrier center)
                    let bounce_angle = dy.atan2(dx);

                    // Preserve snowball's speed
                    let current_speed = (snowball_velocity.x.powi(2) + snowball_velocity.y.powi(2)).sqrt();
                    snowball_velocity.x = bounce_angle.cos() * current_speed;
                    snowball_velocity.y = bounce_angle.sin() * current_speed;

                    // Switch team
                    snowball.team = match snowball.team {
                        Team::Enemy => Team::Friendly,
                        Team::Friendly => Team::Enemy,
                    };

                    // Spawn impact effect at collision point with projectile direction
                    spawn_barrier_impact(&mut commands, barrier_entity, snowball_pos, barrier_center, projectile_angle, &mut meshes, &mut materials, &asset_server);

                    info!("🪞 Snowball reflected at {:.0} speed! Distance={:.1}, New team: {:?}", current_speed, distance, snowball.team);
                }
            }
        }
    }
}

/// Spawn a cyan glow impact effect at the barrier collision point
/// Creates a single curved arc oriented radially along the barrier curve
/// The arc is parented to the barrier so it moves with it
fn spawn_barrier_impact(
    commands: &mut Commands,
    barrier_entity: Entity,
    impact_position: Vec3,
    barrier_center: Vec3,
    _projectile_angle: f32,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
) {
    // Calculate angle from barrier center to impact point (radial direction)
    let dx = impact_position.x - barrier_center.x;
    let dy = impact_position.y - barrier_center.y;
    let radial_angle = dy.atan2(dx);

    // Rotate 90 degrees + 180 degrees (flip) so texture runs along the barrier curve correctly
    // The +PI flips it 180 degrees to face the right direction
    let arc_angle = radial_angle + std::f32::consts::FRAC_PI_2 + std::f32::consts::PI;

    // Size for the barrier impact texture - make it bigger and wider
    let impact_width = 120.0;  // Width along the barrier curve
    let impact_height = 80.0;  // Height pointing outward from barrier

    // Use actual impact position (where projectile hit)
    // Compensate for parent barrier's 1.1 scale by dividing by it
    let barrier_scale = 1.1;
    let local_x = dx / barrier_scale;
    let local_y = dy / barrier_scale;
    let local_z = 8.0; // Z offset to render in front of barrier

    // Preload all 4 impact animation frames to prevent flickering
    let frame1 = asset_server.load("textures/effects/barrier/impact_hit/001.png");
    let frame2 = asset_server.load("textures/effects/barrier/impact_hit/002.png");
    let frame3 = asset_server.load("textures/effects/barrier/impact_hit/003.png");
    let frame4 = asset_server.load("textures/effects/barrier/impact_hit/004.png");
    let frames = vec![frame1.clone(), frame2, frame3, frame4];

    // Spawn as child of barrier so it moves with the barrier
    let impact_entity = commands.spawn((
        crate::components::projectile::BarrierImpact {
            lifetime: Timer::from_seconds(0.4, TimerMode::Once),
            initial_alpha: 1.0,
            animation_timer: Timer::from_seconds(0.1, TimerMode::Repeating), // 100ms per frame = 4 frames in 400ms
            current_frame: 0,
        },
        crate::components::projectile::ImpactAnimationFrames {
            frames: frames.clone(),
        },
        Mesh3d(meshes.add(Rectangle::new(impact_width, impact_height))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.0, 1.0, 1.0, 1.0), // Cyan tint
            base_color_texture: Some(frame1), // Start with first frame
            emissive: LinearRgba::new(0.0, 5.0, 5.0, 1.0), // Very bright cyan glow
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })),
        // Use local position relative to barrier
        // Rotate to follow the circular barrier curve (tangent to the circle)
        // Scale down to compensate for parent's 1.1 scale (1/1.1 ≈ 0.909)
        Transform::from_translation(Vec3::new(local_x, local_y, local_z))
            .with_rotation(Quat::from_rotation_z(arc_angle))
            .with_scale(Vec3::splat(1.0 / barrier_scale)),
    )).id();

    // Make impact a child of the barrier
    commands.entity(barrier_entity).add_child(impact_entity);
}

/// Animate and fade out barrier impact effects
fn update_barrier_impacts(
    mut commands: Commands,
    time: Res<Time>,
    mut impact_query: Query<(
        Entity,
        &mut crate::components::projectile::BarrierImpact,
        &crate::components::projectile::ImpactAnimationFrames,
        &MeshMaterial3d<StandardMaterial>
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut impact, animation_frames, material_handle) in impact_query.iter_mut() {
        impact.lifetime.tick(time.delta());

        if impact.lifetime.is_finished() {
            commands.entity(entity).despawn();
        } else {
            // Tick animation timer
            impact.animation_timer.tick(time.delta());

            // Change frame when timer finishes
            if impact.animation_timer.just_finished() && impact.current_frame < 3 {
                impact.current_frame += 1;

                // Use preloaded texture handle
                let frame_texture = animation_frames.frames[impact.current_frame].clone();

                if let Some(material) = materials.get_mut(&material_handle.0) {
                    material.base_color_texture = Some(frame_texture);
                }
            }

            // Animate based on progress
            let progress = impact.lifetime.fraction();

            // Smooth fade with easing - quick flash then fade
            let fade_curve = 1.0 - (progress * progress); // Quadratic ease-out
            let alpha = impact.initial_alpha * fade_curve;

            // NO SIZE CHANGE - just fade in place like a real impact flash

            // Update material alpha and glow
            if let Some(material) = materials.get_mut(&material_handle.0) {
                material.base_color.set_alpha(alpha);

                // Emissive glow fades faster than alpha for dramatic flash effect
                let glow_fade = 1.0 - (progress * progress * progress); // Cubic fade
                let emissive_strength = 5.0 * glow_fade;
                material.emissive = LinearRgba::new(0.0, emissive_strength, emissive_strength, 1.0);
            }
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
        Transform::from_translation(owner_position)
            .with_scale(Vec3::splat(1.2)),
    ));

    info!("🧲 Magnet field spawned around entity {:?}", owner_entity);
}

/// Update magnet field positions to follow the owner (hero) and apply magnetic pull to resources
fn magnet_update(
    mut commands: Commands,
    time: Res<Time>,
    mut magnet_query: Query<(Entity, &mut crate::components::projectile::MagnetField, &mut Transform)>,
    hero_query: Query<&Transform, (With<crate::components::hero::Hero>, Without<crate::components::projectile::MagnetField>)>,
    mut resource_query: Query<(&Transform, &mut LinearVelocity), (With<crate::components::resource::Resource>, Without<crate::components::projectile::MagnetField>)>,
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
                    if dy > 0.0 {  // Hero is above the resource
                        pull_y *= 3.0;  // Triple the upward force to overcome gravity
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

