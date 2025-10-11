use bevy::prelude::*;
use crate::components::enemy::{Enemy, EnemyType, Flying};
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::projectile::{Arrow, Snowball, Fireball};
use crate::components::bundles::{SpriteBundle3d, avian_physics};
use crate::resources::asset_handles::ProjectileTextures;
use crate::states::GameState;
use crate::utils::constants::*;
use avian3d::prelude::*;

pub struct EnemyAIPlugin;

impl Plugin for EnemyAIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            skeleton_shoot_arrows,
            yeti_throw_snowballs,
            ghast_shoot_fireballs,
            ghast_floating_movement,
            update_ghast_shooting_animation,
        ).run_if(in_state(GameState::Playing)));

        info!("EnemyAIPlugin loaded - ARROW_SPEED constant = {}", ARROW_SPEED);
    }
}

/// Skeletons shoot arrows at the nearest entity (hero or villager)
fn skeleton_shoot_arrows(
    mut commands: Commands,
    mut enemy_query: Query<(&Transform, &mut Enemy)>,
    target_query: Query<&Transform, Or<(With<Hero>, With<Villager>)>>,
    time: Res<Time>,
    projectile_models: Option<Res<crate::resources::asset_handles::ProjectileModels>>,
) {
    // Wait for GLB models to load
    let Some(projectile_models) = projectile_models else {
        return;
    };

    for (enemy_transform, mut enemy) in enemy_query.iter_mut() {
        // Only skeletons shoot arrows
        if enemy.enemy_type != EnemyType::Skeleton {
            continue;
        }

        let Some(ref mut shoot_timer) = enemy.shoot_timer else {
            continue;
        };

        shoot_timer.tick(time.delta());

        if shoot_timer.just_finished() {
            let enemy_pos = enemy_transform.translation;

            // Find nearest target to shoot at
            let mut nearest_pos: Option<Vec3> = None;
            let mut nearest_distance = f32::MAX;

            for target_transform in target_query.iter() {
                let target_pos = target_transform.translation;
                let distance = enemy_pos.distance(target_pos);

                if distance < nearest_distance && distance <= SKELETON_SHOOT_RANGE {
                    nearest_distance = distance;
                    nearest_pos = Some(target_pos);
                }
            }

            // Only shoot if we have a target within range
            let Some(target_pos) = nearest_pos else {
                info!("Skeleton ready to shoot but no target in range (range={})", SKELETON_SHOOT_RANGE);
                continue;
            };

            // Simple velocity - point at target
            let dx = target_pos.x - enemy_pos.x;
            let dy = target_pos.y - enemy_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();

            // Calculate normalized direction (unit vector)
            let direction = Vec3::new(dx / distance, dy / distance, 0.0);

            // Apply ARROW_SPEED directly to the direction
            let velocity = direction * ARROW_SPEED;

            // Spawn arrow OFFSET from skeleton to avoid instant block collision
            let spawn_offset = 80.0; // Spawn arrow 80 units away from skeleton
            let spawn_pos = Vec3::new(
                enemy_pos.x + direction.x * spawn_offset,
                enemy_pos.y + direction.y * spawn_offset,
                0.0 // Z=0 same as gameplay plane for proper collision detection
            );

            info!("Arrow shot: speed={}, distance={:.1}, vx={:.1}, vy={:.1}, actual_speed={:.1}",
                ARROW_SPEED, distance, velocity.x, velocity.y, velocity.length());

            info!("🎯 DEBUG: Arrow GLB handle: {:?}", projectile_models.arrow);

            // Spawn arrow using GLB model (scale: bigger and longer)
            let arrow_entity = commands.spawn((
                Arrow::new(1),
                avian_physics::arrow(velocity),  // Avian physics for arrows
                SceneRoot(projectile_models.arrow.clone()),
                Transform::from_translation(spawn_pos).with_scale(Vec3::new(15.0, 15.0, 15.0)),
                Visibility::Visible,
            )).id();

            info!("ARROW SPAWNED (GLB): entity={:?}, pos=({:.0},{:.0},{:.0}), vel=({:.1},{:.1}), speed={}, scale=15x",
                arrow_entity, spawn_pos.x, spawn_pos.y, spawn_pos.z, velocity.x, velocity.y, ARROW_SPEED);
        }
    }
}

/// Yeti throws snowballs at the nearest entity (hero or villager)
/// Snowballs apply freeze stacks and push back slightly
fn yeti_throw_snowballs(
    mut commands: Commands,
    mut enemy_query: Query<(&Transform, &mut Enemy)>,
    target_query: Query<&Transform, Or<(With<Hero>, With<Villager>)>>,
    time: Res<Time>,
    projectile_textures: Option<Res<ProjectileTextures>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Wait for textures to load
    let Some(projectile_textures) = projectile_textures else {
        return;
    };

    const SNOWBALL_SPEED: f32 = 300.0;  // Slower than arrows
    const SNOWBALL_RANGE: f32 = 800.0;   // Medium range
    const SNOWBALL_SIZE: f32 = 24.0;     // Smaller than arrows

    for (enemy_transform, mut enemy) in enemy_query.iter_mut() {
        // Only Yetis throw snowballs
        if enemy.enemy_type != EnemyType::Yeti {
            continue;
        }

        let Some(ref mut shoot_timer) = enemy.shoot_timer else {
            continue;
        };

        shoot_timer.tick(time.delta());

        if shoot_timer.just_finished() {
            let enemy_pos = enemy_transform.translation;

            // Find nearest target
            let mut nearest_pos: Option<Vec3> = None;
            let mut nearest_distance = f32::MAX;

            for target_transform in target_query.iter() {
                let target_pos = target_transform.translation;
                let distance = enemy_pos.distance(target_pos);

                if distance < nearest_distance && distance <= SNOWBALL_RANGE {
                    nearest_distance = distance;
                    nearest_pos = Some(target_pos);
                }
            }

            let Some(target_pos) = nearest_pos else {
                continue;
            };

            // Calculate velocity toward target
            let dx = target_pos.x - enemy_pos.x;
            let dy = target_pos.y - enemy_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();
            let direction = Vec3::new(dx / distance, dy / distance, 0.0);
            let velocity = direction * SNOWBALL_SPEED;

            // Spawn snowball offset from Yeti
            let spawn_offset = 80.0;
            let spawn_pos = Vec3::new(
                enemy_pos.x + direction.x * spawn_offset,
                enemy_pos.y + direction.y * spawn_offset,
                0.0
            );

            let material = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(projectile_textures.snowball.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                ..default()
            });

            commands.spawn((
                Snowball::new(),
                LinearVelocity(velocity),  // Avian LinearVelocity for snowballs
                RigidBody::Dynamic,
                Mass(0.1),  // CRITICAL: Must have mass to prevent NaN
                Collider::sphere(SNOWBALL_SIZE / 2.0),
                GravityScale(0.6),  // Light projectile with some gravity
                Restitution::new(0.0),
                Friction::new(0.1),
                LockedAxes::ROTATION_LOCKED,
                SpriteBundle3d::new(
                    meshes.add(Circle::new(SNOWBALL_SIZE / 2.0)),
                    material,
                    spawn_pos,
                ),
            ));

            info!("SNOWBALL THROWN by Yeti at ({:.0},{:.0})", spawn_pos.x, spawn_pos.y);
        }
    }
}

/// Ghasts shoot fireballs at the nearest entity (hero or villager)
fn ghast_shoot_fireballs(
    mut commands: Commands,
    mut enemy_query: Query<(Entity, &Transform, &mut Enemy)>,
    target_query: Query<&Transform, Or<(With<Hero>, With<Villager>)>>,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const FIREBALL_SPEED: f32 = 500.0;  // Much faster - was too slow
    const FIREBALL_SIZE: f32 = 32.0;
    const GHAST_SHOOT_RANGE: f32 = 350.0;  // Reduced range - ghast must get closer

    for (ghast_entity, enemy_transform, mut enemy) in enemy_query.iter_mut() {
        // Only Ghasts shoot fireballs
        if enemy.enemy_type != EnemyType::Ghast {
            continue;
        }

        let Some(ref mut shoot_timer) = enemy.shoot_timer else {
            continue;
        };

        shoot_timer.tick(time.delta());

        if shoot_timer.just_finished() {
            let enemy_pos = enemy_transform.translation;

            // Find nearest target to shoot at
            let mut nearest_pos: Option<Vec3> = None;
            let mut nearest_distance = f32::MAX;

            for target_transform in target_query.iter() {
                let target_pos = target_transform.translation;
                let distance = enemy_pos.distance(target_pos);

                if distance < nearest_distance && distance <= GHAST_SHOOT_RANGE {
                    nearest_distance = distance;
                    nearest_pos = Some(target_pos);
                }
            }

            let Some(target_pos) = nearest_pos else {
                continue;
            };

            // Calculate velocity toward target
            let dx = target_pos.x - enemy_pos.x;
            let dy = target_pos.y - enemy_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();
            let direction = Vec3::new(dx / distance, dy / distance, 0.0);
            let velocity = direction * FIREBALL_SPEED;

            // Spawn fireball offset from Ghast (must be outside ghast's 160x160 collider = 80 radius)
            let spawn_offset = 100.0;  // Outside ghast's collision box to prevent immediate explosion
            let spawn_pos = Vec3::new(
                enemy_pos.x + direction.x * spawn_offset,
                enemy_pos.y + direction.y * spawn_offset,
                0.0
            );

            let fireball_texture = asset_server.load("textures/projectiles/fireball.png");
            let material = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(fireball_texture),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                ..default()
            });

            commands.spawn((
                Fireball::new(2),  // 2 damage fireballs
                LinearVelocity(velocity),
                RigidBody::Dynamic,
                Mass(1.0),  // CRITICAL: Must have mass to prevent NaN in physics calculations
                Collider::sphere(FIREBALL_SIZE / 2.0),
                GravityScale(0.0),  // No gravity - flies straight
                Restitution::new(0.0),
                Friction::new(0.0),
                LockedAxes::ROTATION_LOCKED,
                Sensor,  // Fireball is a sensor - triggers collision without physics push
                CollisionEventsEnabled,
                CollisionLayers::new(
                    crate::components::collision_layers::GameLayer::Snowball,  // Use Snowball layer (same projectile category)
                    crate::components::collision_layers::GameLayer::Snowball.filters()  // Collide with blocks, characters, enemies
                ),
                SpriteBundle3d::new(
                    meshes.add(Circle::new(FIREBALL_SIZE / 2.0)),
                    material,
                    spawn_pos,
                ),
            ));

            // Add shooting animation component to ghast
            commands.entity(ghast_entity).insert(crate::components::enemy::GhastShootingAnimation {
                timer: Timer::from_seconds(0.5, TimerMode::Once),  // Show shooting texture for 0.5s
            });

            // Play ghast shooting sound
            commands.spawn((
                AudioPlayer::new(asset_server.load("audio/sfx/mobs/ghast/charge.ogg")),
                PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.6)),
            ));

            info!("🔥 FIREBALL shot by Ghast at ({:.0},{:.0})", spawn_pos.x, spawn_pos.y);
        }
    }
}

/// Ghasts chase characters while bobbing up and down
fn ghast_floating_movement(
    mut query: Query<(Entity, &Enemy, &Transform, &mut LinearVelocity), With<Flying>>,
    target_query: Query<&Transform, Or<(With<Hero>, With<Villager>)>>,
    time: Res<Time>,
) {
    const CHASE_SPEED: f32 = 100.0;        // Speed toward target
    const BOB_AMPLITUDE: f32 = 40.0;       // Vertical bobbing amount
    const BOB_FREQUENCY: f32 = 2.0;        // Bobbing speed
    const OPTIMAL_DISTANCE: f32 = 250.0;   // Preferred distance from target

    for (entity, enemy, ghast_transform, mut velocity) in query.iter_mut() {
        if enemy.enemy_type == EnemyType::Ghast {
            let ghast_pos = ghast_transform.translation;

            // Find nearest target
            let mut nearest_target: Option<Vec3> = None;
            let mut nearest_distance = f32::MAX;

            for target_transform in target_query.iter() {
                let distance = ghast_pos.distance(target_transform.translation);
                if distance < nearest_distance {
                    nearest_distance = distance;
                    nearest_target = Some(target_transform.translation);
                }
            }

            if let Some(target_pos) = nearest_target {
                // Chase or retreat to maintain optimal distance
                let to_target = target_pos - ghast_pos;
                let distance = to_target.length();

                if distance > 0.0 {
                    let direction = to_target.normalize();

                    // Move toward target if too far, away if too close
                    let chase_strength = if distance > OPTIMAL_DISTANCE {
                        1.0  // Chase
                    } else if distance < OPTIMAL_DISTANCE * 0.7 {
                        -0.5  // Retreat
                    } else {
                        0.0  // Stay
                    };

                    // Apply gentle acceleration (works with LinearDamping for smooth movement)
                    let acceleration_x = direction.x * CHASE_SPEED * chase_strength * 5.0;  // 5.0 = force multiplier
                    let acceleration_y = direction.y * CHASE_SPEED * chase_strength * 5.0;

                    // Apply acceleration per frame (integrates with Avian's collision response)
                    velocity.x += acceleration_x * time.delta_secs();
                    velocity.y += acceleration_y * time.delta_secs();

                    // Add subtle bobbing when hovering (not actively chasing)
                    if chase_strength.abs() < 0.1 {
                        let phase = entity.index() as f32 * 0.5;
                        let t = time.elapsed_secs();
                        velocity.y += BOB_AMPLITUDE * ((t * BOB_FREQUENCY) + phase).sin() * time.delta_secs();
                    }
                }
            }
        }
    }
}

/// Update ghast texture when shooting
fn update_ghast_shooting_animation(
    mut commands: Commands,
    mut query: Query<(Entity, &mut crate::components::enemy::GhastShootingAnimation, &MeshMaterial3d<StandardMaterial>)>,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut anim, material_handle) in query.iter_mut() {
        if !anim.timer.tick(time.delta()).is_finished() {
            // Still shooting - update texture to shooting version if not already
            if let Some(material) = materials.get_mut(&material_handle.0) {
                let shooting_texture = asset_server.load("textures/entities/enemy/ghast/ghast_shooting.png");
                if material.base_color_texture.as_ref() != Some(&shooting_texture) {
                    material.base_color_texture = Some(shooting_texture);
                }
            }
        } else {
            // Timer finished - switch back to normal texture and remove component
            if let Some(material) = materials.get_mut(&material_handle.0) {
                let normal_texture = asset_server.load("textures/entities/enemy/ghast/ghast.png");
                material.base_color_texture = Some(normal_texture);
            }
            commands.entity(entity).remove::<crate::components::enemy::GhastShootingAnimation>();
        }
    }
}
