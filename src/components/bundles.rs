use bevy::prelude::*;
use crate::components::physics::{Velocity, Gravity, Bouncy};
use crate::utils::constants::*;
use avian2d::prelude::*;

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

    /// Physics for characters/villagers (bouncy)
    pub fn character() -> Self {
        Self {
            velocity: Velocity { x: 0.0, y: 0.0 },
            gravity: Gravity { force: STANDARD_GRAVITY },
            bouncy: Bouncy { restitution: HERO_BOUNCE },
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

/// Avian Physics Component Sets - helper functions that return tuples
/// Based on Avian examples: components.spawn((RigidBody::Dynamic, Collider::rect(), etc.))
pub mod avian_physics {
    use super::*;
    use crate::components::collision_layers::GameLayer;

    /// Static block physics with collision layer
    pub fn static_block() -> (RigidBody, Collider, Friction, LockedAxes, CollisionLayers) {
        (
            RigidBody::Static,
            Collider::rectangle(BLOCK_SIZE, BLOCK_SIZE),
            Friction::new(1.0),
            LockedAxes::ROTATION_LOCKED,
            CollisionLayers::new(GameLayer::Block, GameLayer::Block.filters()),
        )
    }

    /// Hero/Villager physics (64x64, bouncy) with collision layer
    pub fn character() -> (RigidBody, LinearVelocity, GravityScale, Mass, Collider, Restitution, Friction, LockedAxes, CollisionLayers, CollisionEventsEnabled) {
        (
            RigidBody::Dynamic,
            LinearVelocity(Vec2::ZERO),
            GravityScale(1.5),
            Mass(CHARACTER_MASS),  // Light mass for easy bouncing
            Collider::rectangle(HERO_SIZE, HERO_SIZE),
            Restitution::new(HERO_BOUNCE),  // 1.5 = extra bouncy
            Friction::new(0.0),  // No friction at all
            LockedAxes::ROTATION_LOCKED,
            CollisionLayers::new(GameLayer::Character, GameLayer::Character.filters()),
            CollisionEventsEnabled,
        )
    }

    /// Enemy physics (64x64, bouncy) with collision layer
    pub fn enemy() -> (RigidBody, LinearVelocity, GravityScale, Mass, Collider, Restitution, Friction, LockedAxes, CollisionLayers, CollisionEventsEnabled) {
        (
            RigidBody::Dynamic,
            LinearVelocity(Vec2::ZERO),
            GravityScale(1.5),
            Mass(CHARACTER_MASS),  // Light mass for easy bouncing
            Collider::rectangle(ENEMY_SIZE, ENEMY_SIZE),
            Restitution::new(ENEMY_BOUNCE),  // 1.5 = extra bouncy
            Friction::new(0.0),  // No friction at all
            LockedAxes::ROTATION_LOCKED,
            CollisionLayers::new(GameLayer::Enemy, GameLayer::Enemy.filters()),
            CollisionEventsEnabled,
        )
    }

    /// Enemy physics with custom initial velocity and collision layer
    pub fn enemy_with_velocity(velocity: Vec2) -> (RigidBody, LinearVelocity, GravityScale, Mass, Collider, Restitution, Friction, LockedAxes, CollisionLayers, CollisionEventsEnabled) {
        (
            RigidBody::Dynamic,
            LinearVelocity(velocity),
            GravityScale(1.5),
            Mass(CHARACTER_MASS),  // Light mass for easy bouncing
            Collider::rectangle(ENEMY_SIZE, ENEMY_SIZE),
            Restitution::new(ENEMY_BOUNCE),  // 1.5 = extra bouncy
            Friction::new(0.0),  // No friction at all
            LockedAxes::ROTATION_LOCKED,
            CollisionLayers::new(GameLayer::Enemy, GameLayer::Enemy.filters()),
            CollisionEventsEnabled,
        )
    }

    /// Flying enemy (no gravity) with collision layer
    pub fn flying_enemy() -> (RigidBody, LinearVelocity, GravityScale, Mass, Collider, Restitution, Friction, LockedAxes, CollisionLayers, CollisionEventsEnabled) {
        (
            RigidBody::Dynamic,
            LinearVelocity(Vec2::ZERO),
            GravityScale(0.0),
            Mass(CHARACTER_MASS),  // Light mass for easy bouncing
            Collider::rectangle(ENEMY_SIZE, ENEMY_SIZE),
            Restitution::new(0.0),
            Friction::new(0.0),  // No friction at all
            LockedAxes::ROTATION_LOCKED,
            CollisionLayers::new(GameLayer::Enemy, GameLayer::Enemy.filters()),
            CollisionEventsEnabled,
        )
    }

    /// Arrow physics - Uses Avian Dynamic with gravity for realistic arc
    pub fn arrow(velocity: Vec2) -> (RigidBody, LinearVelocity, GravityScale, Collider, Sensor, LockedAxes, CollisionLayers, NoTransformEasing) {
        (
            RigidBody::Dynamic,  // Dynamic = responds to gravity for realistic arc
            LinearVelocity(velocity),
            GravityScale(1.0),  // Use global gravity
            Collider::circle(8.0),  // Small circular collider for arrow tip
            Sensor,  // Sensor = detects collisions but doesn't apply forces
            LockedAxes::ROTATION_LOCKED,  // Prevent physics rotation (manual control)
            CollisionLayers::new(GameLayer::Arrow, GameLayer::Arrow.filters()),
            NoTransformEasing,  // Disable interpolation for fast arrows (visual lag)
        )
    }

    /// TNT physics (non-bouncy, settles, light so characters can push it)
    pub fn tnt() -> (RigidBody, LinearVelocity, GravityScale, Mass, Collider, Restitution, Friction, LockedAxes) {
        (
            RigidBody::Dynamic,
            LinearVelocity(Vec2::ZERO),
            GravityScale(1.0),
            Mass(TNT_MASS),  // Very light so characters can push it
            Collider::rectangle(BLOCK_SIZE, BLOCK_SIZE),
            Restitution::new(0.0),
            Friction::new(0.3),  // Lower friction so it slides
            LockedAxes::ROTATION_LOCKED,
        )
    }

    /// Resource physics (small, settles, fast falling) - NO rotation lock for manual Y-axis spin
    pub fn resource(velocity: Vec2) -> (RigidBody, LinearVelocity, GravityScale, Mass, Collider, Restitution, Friction, CollisionLayers, AngularDamping) {
        (
            RigidBody::Dynamic,
            LinearVelocity(velocity),
            GravityScale(5.0),  // 5x gravity for much faster falling
            Mass(10.0),  // Heavy mass for fast falling
            Collider::rectangle(RESOURCE_SIZE, RESOURCE_SIZE),
            Restitution::new(0.0),
            Friction::new(0.5),
            CollisionLayers::new(GameLayer::Resource, GameLayer::Resource.filters()),
            AngularDamping(10.0),  // High damping to prevent physics spin, allow manual rotation
        )
    }

    /// Dragon boss (minimal gravity, large)
    pub fn dragon() -> (RigidBody, LinearVelocity, GravityScale, Collider, Restitution, Friction, LockedAxes) {
        (
            RigidBody::Dynamic,
            LinearVelocity(Vec2::ZERO),
            GravityScale(0.0002), // Very low gravity for hovering
            Collider::rectangle(128.0, 128.0),
            Restitution::new(0.0),
            Friction::new(0.3),
            LockedAxes::ROTATION_LOCKED,
        )
    }
}
