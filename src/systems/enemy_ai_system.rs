use bevy::prelude::*;
use crate::components::enemy::{Enemy, EnemyType};
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::projectile::Arrow;
use crate::components::physics::{Velocity, Gravity};
use crate::resources::asset_handles::ProjectileTextures;
use crate::states::GameState;

pub struct EnemyAIPlugin;

impl Plugin for EnemyAIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            skeleton_shoot_arrows,
        ).run_if(in_state(GameState::Playing)));

        info!("EnemyAIPlugin loaded");
    }
}

/// Skeletons shoot arrows at the nearest entity (hero or villager)
fn skeleton_shoot_arrows(
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

                if distance < nearest_distance && distance <= 800.0 {
                    nearest_distance = distance;
                    nearest_pos = Some(target_pos);
                }
            }

            // Only shoot if we have a target within range
            let Some(target_pos) = nearest_pos else {
                continue;
            };

            // Calculate direction to target
            let direction = (target_pos - enemy_pos).normalize_or_zero();

            // Arrow speed and initial velocity
            let arrow_speed = 400.0;
            let velocity = direction * arrow_speed;

            // Spawn arrow
            commands.spawn((
                Arrow::new(1), // 1 damage
                Velocity { x: velocity.x, y: velocity.y },
                Gravity { force: -300.0 }, // Arrows have gravity
                Mesh3d(meshes.add(Rectangle::new(32.0, 8.0))), // Arrow is 32x8
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(projectile_textures.arrow.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform::from_xyz(enemy_pos.x, enemy_pos.y, 15.0), // Z=15 to render above entities
            ));

            info!("Skeleton shot arrow at target from distance {}", nearest_distance);
        }
    }
}
