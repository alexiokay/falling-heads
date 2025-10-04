use bevy::prelude::*;
use crate::components::physics::{Velocity, Gravity};

/// Apply gravity force to all entities with Velocity and Gravity components
pub fn apply_gravity(
    time: Res<Time>,
    mut query: Query<(&Gravity, &mut Velocity)>,
) {
    for (gravity, mut velocity) in query.iter_mut() {
        velocity.y += gravity.force * time.delta_secs();
    }
}
