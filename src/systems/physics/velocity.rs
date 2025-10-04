use bevy::prelude::*;
use crate::components::physics::Velocity;

/// Apply velocity to transform (move entities)
pub fn apply_velocity(
    time: Res<Time>,
    mut query: Query<(&Velocity, &mut Transform)>,
) {
    for (velocity, mut transform) in query.iter_mut() {
        transform.translation.x += velocity.x * time.delta_secs();
        transform.translation.y += velocity.y * time.delta_secs();
    }
}
