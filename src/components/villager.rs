use bevy::prelude::*;

#[derive(Component)]
pub struct Villager {
    pub ai_timer: Timer,
    pub move_direction: f32,  // -1.0 = left, 1.0 = right, 0.0 = still
    pub direction_change_timer: Timer,
}
