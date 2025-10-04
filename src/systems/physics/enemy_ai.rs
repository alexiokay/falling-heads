use bevy::prelude::*;
use crate::components::physics::Velocity;
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::enemy::Enemy;

/// Simple AI for enemies - move toward nearest entity (hero or villager)
pub fn enemy_ai_movement(
    target_query: Query<&Transform, Or<(With<Hero>, With<Villager>)>>,
    mut enemy_query: Query<(&Transform, &mut Velocity), With<Enemy>>,
) {
    for (enemy_transform, mut velocity) in enemy_query.iter_mut() {
        let enemy_pos = enemy_transform.translation;

        // Find nearest target
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

        // Move toward nearest target
        if let Some(target_pos) = nearest_pos {
            let dx = target_pos.x - enemy_pos.x;

            let speed = 100.0;
            if dx.abs() > 50.0 {
                velocity.x = dx.signum() * speed;
            } else {
                velocity.x *= 0.9;
            }
        }
    }
}
