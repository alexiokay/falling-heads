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

/// Component marking entities that are currently inside a cobweb
#[derive(Component)]
pub struct CobwebSlow {
    pub slow_multiplier: f32,  // 0.25 = 75% slower
}
