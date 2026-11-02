use bevy::prelude::*;

#[derive(Component)]
pub struct Hero {
    pub move_direction: f32, // -1.0 = left, 1.0 = right
    pub direction_change_timer: Timer,
}

/// The hero while an event (the space prison) plays elsewhere: hidden and
/// frozen in the live world, no collision, not a target.
#[derive(Component)]
pub struct Parked;
