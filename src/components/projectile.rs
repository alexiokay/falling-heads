use crate::utils::constants::ARROW_LIFETIME;
use bevy::prelude::*;

/// Team affiliation for friendly fire prevention
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Team {
    Friendly, // Hero + Villagers
    Enemy,    // Skeletons, Zombies, etc.
}

#[derive(Component)]
pub struct Arrow {
    pub damage: u32,
    pub lifetime: Timer,
    pub stuck_to: Option<Entity>, // Entity this arrow is stuck to
    pub team: Team,               // Which team shot this arrow
    pub reflected: bool, // True if already reflected by barrier (prevent multiple reflections)
}

impl Arrow {
    pub fn new(damage: u32) -> Self {
        Self {
            damage,
            lifetime: Timer::from_seconds(ARROW_LIFETIME, TimerMode::Once),
            stuck_to: None,
            team: Team::Enemy, // Default to enemy (for backwards compatibility with skeletons)
            reflected: false,
        }
    }

    pub fn new_friendly(damage: u32) -> Self {
        Self {
            damage,
            lifetime: Timer::from_seconds(ARROW_LIFETIME, TimerMode::Once),
            stuck_to: None,
            team: Team::Friendly, // Friendly arrow (from hero power)
            reflected: false,
        }
    }
}

/// Component to mark arrows that are stuck and should despawn after a delay
#[derive(Component)]
pub struct StuckArrow {
    pub despawn_timer: Timer,
    pub stuck_to_entity: Entity, // Which entity this arrow is stuck to
}

#[derive(Component)]
pub struct Laser {
    pub timer: Timer,
    pub width: f32,
    pub id: Entity, // Unique ID for this laser instance
}

/// Marker for laser burst beams (rotated beams that need directional collision)
#[derive(Component)]
pub struct LaserBurstBeam {
    pub length: f32,           // Current length of the beam for collision detection
    pub follow_entity: Entity, // Entity to follow (hero)
    pub angle: f32,            // Angle offset from the entity
    pub growth_timer: Timer,   // Timer for growth animation (0 to 1)
    pub target_length: f32,    // Final length to reach
    pub target_width: f32,     // Final width to reach
}

#[derive(Component)]
pub struct Fireball {
    pub damage: u32,
    pub lifetime: Timer,
    pub team: Team,
    pub reflected: bool, // True if already reflected by barrier (prevent multiple reflections)
}

impl Fireball {
    pub fn new(damage: u32) -> Self {
        Self {
            damage,
            lifetime: Timer::from_seconds(6.0, TimerMode::Once), // 6 second lifetime
            team: Team::Enemy, // Ghasts/Dragons shoot enemy fireballs
            reflected: false,
        }
    }
}

#[derive(Component)]
pub struct Snowball {
    pub lifetime: Timer,
    pub team: Team,
}

impl Snowball {
    pub fn new() -> Self {
        Self {
            lifetime: Timer::from_seconds(5.0, TimerMode::Once),
            team: Team::Enemy, // Yeti throws enemy snowballs
        }
    }
}

#[derive(Component)]
pub struct MagicBarrier {
    pub lifetime: Timer,
    pub radius: f32,
    pub owner_entity: Entity,
}
/// Visual impact lifetime; animation lives in the energy material.
#[derive(Component)]
pub struct BarrierImpact {
    pub lifetime: Timer,
}

/// Magnetic field that attracts resources to the hero
#[derive(Component)]
pub struct MagnetField {
    pub lifetime: Timer,
    pub radius: f32,
    pub pull_strength: f32,
    pub owner_entity: Entity,
}
