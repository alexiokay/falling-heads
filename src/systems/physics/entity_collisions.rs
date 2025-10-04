use bevy::prelude::*;
use crate::components::physics::{Velocity, Bouncy};
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::enemy::Enemy;

/// Entity-to-entity collision (Hero, Villagers, and Enemies bounce off each other)
pub fn check_entity_collisions(
    mut entities: Query<(Entity, &mut Transform, &mut Velocity, &Bouncy), Or<(With<Hero>, With<Villager>, With<Enemy>)>>,
) {
    let mut combinations = entities.iter_combinations_mut();

    while let Some([(_entity_a, mut transform_a, mut velocity_a, bouncy_a), (_entity_b, mut transform_b, mut velocity_b, bouncy_b)]) = combinations.fetch_next() {
        let pos_a = transform_a.translation;
        let pos_b = transform_b.translation;

        let size_a = 64.0;
        let size_b = 56.0;

        let half_a = size_a / 2.0;
        let half_b = size_b / 2.0;

        let dx = pos_a.x - pos_b.x;
        let dy = pos_a.y - pos_b.y;

        let overlap_x = half_a + half_b - dx.abs();
        let overlap_y = half_a + half_b - dy.abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            // Collision detected - separate and bounce
            if overlap_x < overlap_y {
                // Horizontal separation
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
                // Vertical separation
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
