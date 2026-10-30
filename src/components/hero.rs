use bevy::prelude::*;

#[derive(Component)]
pub struct Hero {
    pub move_direction: f32, // -1.0 = left, 1.0 = right
    pub direction_change_timer: Timer,
}
