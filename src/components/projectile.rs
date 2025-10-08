use bevy::prelude::*;
use crate::utils::constants::ARROW_LIFETIME;

/// Team affiliation for friendly fire prevention
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Team {
    Friendly,  // Hero + Villagers
    Enemy,     // Skeletons, Zombies, etc.
}

/// Trait for all projectile types - defines common behavior
pub trait ProjectileBehavior {
    /// Get the team that fired this projectile
    fn team(&self) -> Team;

    /// Get damage dealt on hit
    fn damage(&self) -> u32;

    /// Check if projectile should despawn (lifetime expired)
    fn should_despawn(&self) -> bool;

    /// Update projectile state (timers, etc.)
    fn update(&mut self, delta: f32);

    /// Get push force applied on hit
    fn push_force(&self) -> f32 {
        0.0  // Default: no push
    }

    /// Called when projectile hits a target
    fn on_hit(&mut self, _target: Entity) {
        // Default: no special behavior
    }
}

#[derive(Component)]
pub struct Arrow {
    pub damage: u32,
    pub lifetime: Timer,
    pub stuck_to: Option<Entity>, // Entity this arrow is stuck to
    pub team: Team,  // Which team shot this arrow
}

impl Arrow {
    pub fn new(damage: u32) -> Self {
        Self {
            damage,
            lifetime: Timer::from_seconds(ARROW_LIFETIME, TimerMode::Once),
            stuck_to: None,
            team: Team::Enemy,  // Default to enemy (for backwards compatibility with skeletons)
        }
    }

    pub fn new_friendly(damage: u32) -> Self {
        Self {
            damage,
            lifetime: Timer::from_seconds(ARROW_LIFETIME, TimerMode::Once),
            stuck_to: None,
            team: Team::Friendly,  // Friendly arrow (from hero power)
        }
    }

    pub fn new_with_team(damage: u32, team: Team) -> Self {
        Self {
            damage,
            lifetime: Timer::from_seconds(ARROW_LIFETIME, TimerMode::Once),
            stuck_to: None,
            team,
        }
    }
}

impl ProjectileBehavior for Arrow {
    fn team(&self) -> Team {
        self.team
    }

    fn damage(&self) -> u32 {
        self.damage
    }

    fn should_despawn(&self) -> bool {
        self.lifetime.finished()
    }

    fn update(&mut self, delta: f32) {
        self.lifetime.tick(std::time::Duration::from_secs_f32(delta));
    }

    fn push_force(&self) -> f32 {
        150.0  // Arrows have moderate push
    }

    fn on_hit(&mut self, target: Entity) {
        self.stuck_to = Some(target);
    }
}

/// Component to mark arrows that are stuck and should despawn after a delay
#[derive(Component)]
pub struct StuckArrow {
    pub despawn_timer: Timer,
    pub offset: Vec3, // Offset from the entity it's stuck to
    pub stuck_to_entity: Entity, // Which entity this arrow is stuck to
}

#[derive(Component)]
pub struct Laser {
    pub timer: Timer,
    pub width: f32,
}

#[derive(Component)]
pub struct Fireball {
    pub speed: f32,
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
            team: Team::Enemy,  // Yeti throws enemy snowballs
        }
    }
}

impl ProjectileBehavior for Snowball {
    fn team(&self) -> Team {
        self.team
    }

    fn damage(&self) -> u32 {
        0  // Snowballs do no damage
    }

    fn should_despawn(&self) -> bool {
        self.lifetime.finished()
    }

    fn update(&mut self, delta: f32) {
        self.lifetime.tick(std::time::Duration::from_secs_f32(delta));
    }

    fn push_force(&self) -> f32 {
        50.0  // Small push effect
    }
}

#[derive(Component)]
pub struct MagicBarrier {
    pub lifetime: Timer,
    pub radius: f32,
    pub owner_entity: Entity,  // Entity this barrier is protecting (hero)
}
