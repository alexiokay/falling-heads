// Legacy physics components - deprecated during Avian3D migration
// Still exported for backward compatibility but marked as deprecated

#[allow(dead_code)]
use bevy::prelude::*;

// Legacy components with deprecation warnings suppressed
#[derive(Component)]
#[allow(dead_code)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct Gravity {
    pub force: f32,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct Bouncy {
    pub restitution: f32,
}
