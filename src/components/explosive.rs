use bevy::prelude::*;

#[derive(Component)]
pub struct Tnt {
    pub timer: Timer,
}

#[derive(Component)]
pub struct Explosion {
    pub radius: f32,
}

/// Explosion animation - cycles through 16 frames
#[derive(Component)]
pub struct ExplosionAnimation {
    pub timer: Timer,
    pub current_frame: usize,
    pub total_frames: usize,
    pub delay_timer: Timer, // Delay before animation starts
    pub started: bool,
}
