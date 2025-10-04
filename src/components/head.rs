use bevy::prelude::*;
use crate::components::physics::{Velocity, Gravity, Bouncy};
use crate::components::enemy::Health;
use crate::components::buff::Buffable;

/// Common bundle for all "head" entities (Hero, Villagers, Enemies)
/// This reduces code duplication when spawning entities
#[derive(Bundle)]
pub struct HeadBundle {
    pub health: Health,
    pub buffable: Buffable,
    pub velocity: Velocity,
    pub gravity: Gravity,
    pub bouncy: Bouncy,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

impl HeadBundle {
    /// Create a head bundle with common defaults
    pub fn new(
        health: u32,
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        position: Vec3,
        restitution: f32,
    ) -> Self {
        Self {
            health: Health::new(health),
            buffable: Buffable,
            velocity: Velocity { x: 0.0, y: 0.0 },
            gravity: Gravity { force: -500.0 },
            bouncy: Bouncy { restitution },
            mesh: Mesh3d(mesh),
            material: MeshMaterial3d(material),
            transform: Transform::from_translation(position),
        }
    }
}

/// Marker trait for entities that are "heads" (can receive buffs, have health, physics)
pub trait Head {
    fn max_health() -> u32;
    fn restitution() -> f32;
    fn size() -> f32;
}
