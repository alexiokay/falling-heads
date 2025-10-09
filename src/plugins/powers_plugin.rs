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
};
use crate::events::damage_events::EntityDamaged;
use crate::resources::asset_handles::GameTextures;
use crate::states::GameState;
use crate::utils::constants::*;

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
    use crate::components::physics::Gravity;
    use crate::systems::username_label_system::spawn_username_label;

    // Spawn TNT as 3D cube using same UV-mapped mesh system as blocks
    let tnt_entity = commands.spawn((
        Tnt {
            timer: Timer::from_seconds(TNT_FUSE_TIME, TimerMode::Once),
        },
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
        Velocity { x: 0.0, y: 0.0 },
        Gravity { force: -500.0 },
        // NO Bouncy component - TNT doesn't bounce
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
    mut entity_query: Query<(Entity, &Transform, &mut Velocity), Or<(With<Hero>, With<Villager>, With<Enemy>)>>,
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
                for i in 0..particle_count {
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
        let push_blocks = 3.0; // Push entities 3 blocks away
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

// LASER spawning - fires downward from hero
pub fn spawn_laser(
    commands: &mut Commands,
    position: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Laser {
            timer: Timer::from_seconds(LASER_DURATION, TimerMode::Once),
            width: LASER_WIDTH,
        },
        Mesh3d(meshes.add(Rectangle::new(LASER_WIDTH, LASER_HEIGHT))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.0, 1.0, 0.7), // Purple semi-transparent
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_translation(position + Vec3::new(0.0, -LASER_HEIGHT / 2.0, 5.0)), // Z=5 for visibility
    ));

    info!("LASER spawned at {:?}", position);
}

fn laser_update(
    time: Res<Time>,
    mut commands: Commands,
    mut laser_query: Query<(Entity, &mut Laser, &Transform)>,
    block_query: Query<(Entity, &Transform), With<Block>>,
) {
    for (laser_entity, mut laser, laser_transform) in laser_query.iter_mut() {
        laser.timer.tick(time.delta());

        // Destroy blocks that overlap with laser on first frame
        if laser.timer.elapsed_secs() < 0.05 {
            let laser_pos = laser_transform.translation;
            let laser_half_width = laser.width / 2.0;

            for (block_entity, block_transform) in block_query.iter() {
                let block_pos = block_transform.translation;
                let block_half = 32.0; // 3D scale: Block size / 2 (64/2)

                // Check if block is within laser's horizontal range and below hero
                if block_pos.x > laser_pos.x - laser_half_width - block_half
                    && block_pos.x < laser_pos.x + laser_half_width + block_half
                    && block_pos.y < laser_pos.y + 1000.0 // 3D scale: Below the laser origin
                {
                    commands.entity(block_entity).despawn(); // Despawn children too (damage overlays)
                }
            }
        }

        if laser.timer.is_finished() {
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
            let direction = Vec2::new(barrage.current_angle.cos(), barrage.current_angle.sin());
            let velocity = direction * ARROW_SPEED;

            // Spawn arrow slightly offset from hero to avoid instant collision
            let spawn_offset = 80.0;
            let spawn_pos = Vec3::new(
                hero_pos.x + direction.x * spawn_offset,
                hero_pos.y + direction.y * spawn_offset,
                10.0  // Z=10 same as entities
            );

            // Spawn arrow using GLB model
            commands.spawn((
                crate::components::projectile::Arrow::new_friendly(1),  // Friendly arrow - won't hit hero/villagers
                crate::components::bundles::PhysicsBundle::projectile(velocity, ARROW_GRAVITY),
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
) {
    use crate::components::projectile::MagicBarrier;

    // Create a smooth circular ring using Circle mesh
    let barrier_radius = 150.0;

    commands.spawn((
        MagicBarrier {
            lifetime: Timer::from_seconds(10.0, TimerMode::Once),
            radius: barrier_radius,
            owner_entity,
        },
        Mesh3d(meshes.add(Circle::new(barrier_radius))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.5, 0.2, 1.0, 0.4),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })),
        Transform::from_translation(owner_position),
    ));

    info!("🪞 Magic barrier spawned around entity {:?}", owner_entity);
}

/// Update barrier positions to follow the owner entity (hero)
fn barrier_update(
    mut commands: Commands,
    time: Res<Time>,
    mut barrier_query: Query<(Entity, &mut crate::components::projectile::MagicBarrier, &mut Transform)>,
    owner_query: Query<&Transform, (With<Hero>, Without<crate::components::projectile::MagicBarrier>)>,
) {
    for (barrier_entity, mut barrier, mut barrier_transform) in barrier_query.iter_mut() {
        barrier.lifetime.tick(time.delta());

        if barrier.lifetime.is_finished() {
            commands.entity(barrier_entity).despawn();
            info!("🪞 Magic barrier expired");
            continue;
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
    barrier_query: Query<(&Transform, &crate::components::projectile::MagicBarrier)>,
    mut arrow_query: Query<(&Transform, &mut Velocity, &mut Arrow)>,
) {
    use crate::components::projectile::Team;

    for (barrier_transform, barrier) in barrier_query.iter() {
        for (arrow_transform, mut arrow_velocity, mut arrow) in arrow_query.iter_mut() {
            let distance = barrier_transform.translation.distance(arrow_transform.translation);

            if distance < barrier.radius + 20.0 && distance > barrier.radius - 20.0 {
                // Calculate reflection angle (away from barrier center)
                let dx = arrow_transform.translation.x - barrier_transform.translation.x;
                let dy = arrow_transform.translation.y - barrier_transform.translation.y;
                let bounce_angle = dy.atan2(dx);

                // Preserve and amplify the arrow's speed for dramatic effect
                let current_speed = (arrow_velocity.x.powi(2) + arrow_velocity.y.powi(2)).sqrt();
                let bounce_speed = (current_speed * 1.5).max(500.0); // At least 500, or 1.5x current speed

                arrow_velocity.x = bounce_angle.cos() * bounce_speed;
                arrow_velocity.y = bounce_angle.sin() * bounce_speed;

                // Switch team (enemy → friendly, friendly → enemy)
                arrow.team = match arrow.team {
                    Team::Enemy => Team::Friendly,
                    Team::Friendly => Team::Enemy,
                };

                info!("🪞 Arrow reflected at {:.0} speed! New team: {:?}", bounce_speed, arrow.team);
            }
        }
    }
}
