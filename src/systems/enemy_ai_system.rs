use bevy::prelude::*;
use crate::components::enemy::{Enemy, EnemyType};
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::projectile::{Arrow, Snowball};
use crate::components::physics::Velocity;
use crate::components::bundles::{SpriteBundle3d, PhysicsBundle};
use crate::resources::asset_handles::ProjectileTextures;
use crate::states::GameState;
use crate::utils::constants::*;

pub struct EnemyAIPlugin;

impl Plugin for EnemyAIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            skeleton_shoot_arrows,
            yeti_throw_snowballs,
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
            let direction = Vec2::new(dx / distance, dy / distance);

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
                PhysicsBundle::projectile(velocity, ARROW_GRAVITY),
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
            let direction = Vec2::new(dx / distance, dy / distance);
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
                Velocity { x: velocity.x, y: velocity.y },
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
