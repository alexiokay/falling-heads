use bevy::prelude::*;
use crate::components::physics::{Velocity, Gravity, Bouncy};
use crate::utils::constants::*;

/// Common 3D sprite rendering components
#[derive(Bundle)]
pub struct SpriteBundle3d {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

impl SpriteBundle3d {
    pub fn new(
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        position: Vec3,
    ) -> Self {
        Self {
            mesh: Mesh3d(mesh),
            material: MeshMaterial3d(material),
            transform: Transform::from_translation(position),
        }
    }

    pub fn with_scale(
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        position: Vec3,
        scale: Vec3,
    ) -> Self {
        Self {
            mesh: Mesh3d(mesh),
            material: MeshMaterial3d(material),
            transform: Transform::from_translation(position).with_scale(scale),
        }
    }
}

/// Common physics components for entities with gravity and bouncing
#[derive(Bundle)]
pub struct PhysicsBundle {
    pub velocity: Velocity,
    pub gravity: Gravity,
    pub bouncy: Bouncy,
}

impl PhysicsBundle {
    pub fn new(initial_velocity: Vec2, gravity_force: f32, bounce_factor: f32) -> Self {
        Self {
            velocity: Velocity {
                x: initial_velocity.x,
                y: initial_velocity.y,
            },
            gravity: Gravity { force: gravity_force },
            bouncy: Bouncy { restitution: bounce_factor },
        }
    }

    /// Standard physics for falling entities (no initial velocity, normal gravity, bouncy)
    pub fn falling() -> Self {
        Self {
            velocity: Velocity { x: 0.0, y: 0.0 },
            gravity: Gravity { force: STANDARD_GRAVITY },
            bouncy: Bouncy { restitution: STANDARD_BOUNCE },
        }
    }

    /// Physics for enemies (bouncy like villagers)
    pub fn enemy() -> Self {
        Self {
            velocity: Velocity { x: 0.0, y: 0.0 },
            gravity: Gravity { force: STANDARD_GRAVITY },
            bouncy: Bouncy { restitution: ENEMY_BOUNCE },
        }
    }

    /// Physics for projectiles (custom velocity, gravity, no bounce)
    pub fn projectile(velocity: Vec2, gravity: f32) -> Self {
        Self {
            velocity: Velocity {
                x: velocity.x,
                y: velocity.y,
            },
            gravity: Gravity { force: gravity },
            bouncy: Bouncy { restitution: NO_BOUNCE },
        }
    }

    /// Physics for non-bouncy items like resources (settle on ground)
    pub fn settling(initial_velocity: Vec2) -> Self {
        Self {
            velocity: Velocity {
                x: initial_velocity.x,
                y: initial_velocity.y,
            },
            gravity: Gravity { force: STANDARD_GRAVITY },
            bouncy: Bouncy { restitution: NO_BOUNCE },
        }
    }

    /// Physics for flying dragon (minimal gravity, no bounce)
    pub fn dragon() -> Self {
        Self {
            velocity: Velocity { x: 0.0, y: 0.0 },
            gravity: Gravity { force: 0.1 },  // Very low gravity for hovering effect
            bouncy: Bouncy { restitution: NO_BOUNCE },
        }
    }
}
