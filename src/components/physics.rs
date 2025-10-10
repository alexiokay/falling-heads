use bevy::prelude::*;

// Re-export Avian physics components for compatibility
pub use avian2d::prelude::{
    LinearVelocity as AvianVelocity,
    GravityScale,
    Restitution,
    Friction,
    RigidBody,
    Collider,
    LockedAxes,
};

// Legacy components - kept temporarily for migration
// TODO: Remove after full migration to Avian
#[derive(Component)]
#[deprecated(note = "Use avian2d::LinearVelocity instead")]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
#[deprecated(note = "Use avian2d::GravityScale instead")]
pub struct Gravity {
    pub force: f32,
}

#[derive(Component)]
#[deprecated(note = "Use avian2d::Restitution instead")]
pub struct Bouncy {
    pub restitution: f32,
}

// Helper type alias for cleaner code
pub type Velocity2D = avian2d::prelude::LinearVelocity;
