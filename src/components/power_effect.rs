use bevy::prelude::*;

#[derive(Component)]
pub struct SpeedModifier {
    pub multiplier: f32,
    pub duration: Timer,
}

#[derive(Component)]
pub struct MagnetEffect {
    pub radius: f32,
    pub strength: f32,
    pub duration: Timer,
}
