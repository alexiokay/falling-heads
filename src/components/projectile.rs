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
    pub reflected: bool,  // True if already reflected by barrier (prevent multiple reflections)
}

impl Arrow {
    pub fn new(damage: u32) -> Self {
        Self {
            damage,
            lifetime: Timer::from_seconds(ARROW_LIFETIME, TimerMode::Once),
            stuck_to: None,
            team: Team::Enemy,  // Default to enemy (for backwards compatibility with skeletons)
            reflected: false,
        }
    }

    pub fn new_friendly(damage: u32) -> Self {
        Self {
            damage,
            lifetime: Timer::from_seconds(ARROW_LIFETIME, TimerMode::Once),
            stuck_to: None,
            team: Team::Friendly,  // Friendly arrow (from hero power)
            reflected: false,
        }
    }

    pub fn new_with_team(damage: u32, team: Team) -> Self {
        Self {
            damage,
            lifetime: Timer::from_seconds(ARROW_LIFETIME, TimerMode::Once),
            stuck_to: None,
            team,
            reflected: false,
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
        self.lifetime.is_finished()
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
    pub id: Entity,  // Unique ID for this laser instance
}

/// Particle that flows down the laser beam
#[derive(Component)]
pub struct LaserParticle {
    pub velocity: f32,  // Speed moving downward
    pub lifetime: Timer,
}

/// Marks the core beam mesh and links it to a specific laser
#[derive(Component)]
pub struct LaserCore {
    pub laser_id: Entity,  // Which laser this beam belongs to
}

/// Marks the outer glow mesh and links it to a specific laser
#[derive(Component)]
pub struct LaserGlow {
    pub laser_id: Entity,  // Which laser this beam belongs to
}

#[derive(Component)]
pub struct Fireball {
    pub damage: u32,
    pub lifetime: Timer,
    pub team: Team,
    pub reflected: bool,  // True if already reflected by barrier (prevent multiple reflections)
}

impl Fireball {
    pub fn new(damage: u32) -> Self {
        Self {
            damage,
            lifetime: Timer::from_seconds(6.0, TimerMode::Once), // 6 second lifetime
            team: Team::Enemy,  // Ghasts/Dragons shoot enemy fireballs
            reflected: false,
        }
    }
}

impl ProjectileBehavior for Fireball {
    fn team(&self) -> Team {
        self.team
    }

    fn damage(&self) -> u32 {
        self.damage
    }

    fn should_despawn(&self) -> bool {
        self.lifetime.is_finished()
    }

    fn update(&mut self, delta: f32) {
        self.lifetime.tick(std::time::Duration::from_secs_f32(delta));
    }

    fn push_force(&self) -> f32 {
        200.0  // Fireballs have strong push
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
        self.lifetime.is_finished()
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
    pub animation_timer: Timer,
    pub current_frame: usize,
}

/// Stores preloaded texture handles for barrier animation
#[derive(Component)]
pub struct BarrierAnimationFrames {
    pub frames: Vec<Handle<Image>>,
}

/// Visual impact effect when barrier reflects a projectile
#[derive(Component)]
pub struct BarrierImpact {
    pub lifetime: Timer,
    pub initial_alpha: f32,
    pub animation_timer: Timer,
    pub current_frame: usize,
}

/// Stores preloaded texture handles for impact animation
#[derive(Component)]
pub struct ImpactAnimationFrames {
    pub frames: Vec<Handle<Image>>,
}

/// Magnetic field that attracts resources to the hero
#[derive(Component)]
pub struct MagnetField {
    pub lifetime: Timer,
    pub radius: f32,
    pub pull_strength: f32,
    pub owner_entity: Entity,
}
