use crate::utils::constants::*;
use avian3d::prelude::*;
use bevy::prelude::*;

/// Common 3D sprite rendering components
#[derive(Bundle)]
pub struct SpriteBundle3d {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

impl SpriteBundle3d {
    pub fn new(mesh: Handle<Mesh>, material: Handle<StandardMaterial>, position: Vec3) -> Self {
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

/// Avian Physics Component Sets - helper functions that return tuples
/// Based on Avian examples: components.spawn((RigidBody::Dynamic, Collider::rect(), etc.))
pub mod avian_physics {
    use super::*;
    use crate::components::collision_layers::GameLayer;

    /// Hero/Villager physics (64x64, bouncy) with collision layer
    pub fn character() -> (
        RigidBody,
        LinearVelocity,
        GravityScale,
        Mass,
        Collider,
        Restitution,
        Friction,
        LockedAxes,
        CollisionLayers,
        CollisionEventsEnabled,
    ) {
        (
            RigidBody::Dynamic,
            LinearVelocity(Vec3::ZERO),
            GravityScale(1.5),
            Mass(CHARACTER_MASS), // Light mass for easy bouncing
            Collider::cuboid(HERO_SIZE, HERO_SIZE, HERO_SIZE),
            Restitution::new(HERO_BOUNCE), // 1.5 = extra bouncy
            Friction::new(0.0),            // No friction at all
            LockedAxes::new()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z()
                .lock_translation_z(),
            CollisionLayers::new(GameLayer::Character, GameLayer::Character.filters()),
            CollisionEventsEnabled,
        )
    }

    /// Enemy physics with custom initial velocity and collision layer
    pub fn enemy_with_velocity(
        velocity: Vec3,
    ) -> (
        RigidBody,
        LinearVelocity,
        GravityScale,
        Mass,
        Collider,
        Restitution,
        Friction,
        LockedAxes,
        CollisionLayers,
        CollisionEventsEnabled,
    ) {
        (
            RigidBody::Dynamic,
            LinearVelocity(velocity),
            GravityScale(1.5),
            Mass(CHARACTER_MASS), // Light mass for easy bouncing
            Collider::cuboid(ENEMY_SIZE, ENEMY_SIZE, ENEMY_SIZE),
            Restitution::new(ENEMY_BOUNCE), // 1.5 = extra bouncy
            Friction::new(0.0),             // No friction at all
            LockedAxes::new()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z()
                .lock_translation_z(),
            CollisionLayers::new(GameLayer::Enemy, GameLayer::Enemy.filters()),
            CollisionEventsEnabled,
        )
    }

    /// Flying enemy (no gravity) with collision layer
    pub fn flying_enemy() -> (
        RigidBody,
        LinearVelocity,
        GravityScale,
        Mass,
        Collider,
        Restitution,
        Friction,
        LockedAxes,
        CollisionLayers,
        CollisionEventsEnabled,
    ) {
        (
            RigidBody::Dynamic,
            LinearVelocity(Vec3::ZERO),
            GravityScale(0.0),
            Mass(CHARACTER_MASS), // Light mass for easy bouncing
            Collider::cuboid(ENEMY_SIZE, ENEMY_SIZE, ENEMY_SIZE),
            Restitution::new(0.0),
            Friction::new(0.0), // No friction at all
            LockedAxes::new()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z()
                .lock_translation_z(),
            CollisionLayers::new(GameLayer::Enemy, GameLayer::Enemy.filters()),
            CollisionEventsEnabled,
        )
    }

    /// Arrow physics - Uses Avian Dynamic with gravity for realistic arc
    pub fn arrow(
        velocity: Vec3,
    ) -> (
        RigidBody,
        LinearVelocity,
        GravityScale,
        Mass,
        Collider,
        Sensor,
        LockedAxes,
        CollisionLayers,
        NoTransformEasing,
    ) {
        (
            RigidBody::Dynamic, // Dynamic = responds to gravity for realistic arc
            LinearVelocity(velocity),
            GravityScale(1.0),     // Use global gravity
            Mass(0.1),             // CRITICAL: Light mass prevents NaN in physics
            Collider::sphere(8.0), // Small spherical collider for arrow tip
            Sensor,                // Sensor = detects collisions but doesn't apply forces
            LockedAxes::new()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z()
                .lock_translation_z(), // Prevent physics rotation (manual control)
            CollisionLayers::new(GameLayer::Arrow, GameLayer::Arrow.filters()),
            NoTransformEasing, // Disable interpolation for fast arrows (visual lag)
        )
    }

    /// TNT physics (non-bouncy, settles, light so characters can push it)
    pub fn tnt() -> (
        RigidBody,
        LinearVelocity,
        GravityScale,
        Mass,
        Collider,
        Restitution,
        Friction,
        LockedAxes,
    ) {
        (
            RigidBody::Dynamic,
            LinearVelocity(Vec3::ZERO),
            GravityScale(1.0),
            Mass(TNT_MASS), // Very light so characters can push it
            Collider::cuboid(BLOCK_SIZE, BLOCK_SIZE, BLOCK_SIZE),
            Restitution::new(0.0),
            Friction::new(0.3), // Lower friction so it slides
            LockedAxes::new()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z()
                .lock_translation_z(),
        )
    }

    /// Resource physics (small, settles, fast falling) - NO rotation lock for manual Y-axis spin
    pub fn resource(
        velocity: Vec3,
    ) -> (
        RigidBody,
        LinearVelocity,
        GravityScale,
        Mass,
        Collider,
        Restitution,
        Friction,
        CollisionLayers,
        AngularDamping,
    ) {
        (
            RigidBody::Dynamic,
            LinearVelocity(velocity),
            GravityScale(5.0), // 5x gravity for much faster falling
            Mass(10.0),        // Heavy mass for fast falling
            Collider::cuboid(RESOURCE_SIZE, RESOURCE_SIZE, RESOURCE_SIZE),
            Restitution::new(0.0),
            Friction::new(0.5),
            CollisionLayers::new(GameLayer::Resource, GameLayer::Resource.filters()),
            AngularDamping(10.0), // High damping to prevent physics spin, allow manual rotation
        )
    }
}
