use bevy::prelude::*;

#[derive(Component)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

impl Health {
    pub fn new(max: u32) -> Self {
        Self { current: max, max }
    }
}

/// Trait for enemy behavior - allows polymorphic enemy types
pub trait EnemyBehavior {
    /// Get maximum health for this enemy type
    fn max_health(&self) -> u32;

    /// Get movement speed multiplier
    fn movement_speed(&self) -> f32 {
        1.0  // Default speed
    }

    /// Get attack interval (if this enemy shoots projectiles)
    fn shoot_interval(&self) -> Option<f32> {
        None  // Default: doesn't shoot
    }

    /// Get the visual color for this enemy
    fn color(&self) -> Color;

    /// Check if this enemy can shoot projectiles
    fn can_shoot(&self) -> bool {
        self.shoot_interval().is_some()
    }

    /// Get damage dealt by this enemy's attacks
    fn attack_damage(&self) -> u32 {
        1  // Default damage
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyType {
    Zombie,    // Spawns in overworld biomes
    Skeleton,  // Spawns in overworld biomes
    Pigman,    // Spawns in Nether
    Enderman,  // Spawns in End
    Yeti,      // Spawns in Iceland - throws snowballs
    Ghast,     // Spawns in Nether (flying) - shoots fireballs
    Dragon,    // Flying boss enemy - shoots fireballs
}

#[derive(Component)]
pub struct Enemy {
    pub enemy_type: EnemyType,
    pub shoot_timer: Option<Timer>,  // Some enemy types shoot projectiles
    pub move_direction: f32,  // -1.0 = left, 1.0 = right
    pub direction_change_timer: Timer,
}

impl Enemy {
    pub fn new(enemy_type: EnemyType) -> Self {
        use crate::utils::constants::{SKELETON_SHOOT_INTERVAL, ENEMY_DIRECTION_CHANGE_INTERVAL};

        let shoot_timer = match enemy_type {
            EnemyType::Skeleton => Some(Timer::from_seconds(SKELETON_SHOOT_INTERVAL, TimerMode::Repeating)),
            EnemyType::Yeti => Some(Timer::from_seconds(2.0, TimerMode::Repeating)), // Yeti throws snowballs every 2s
            EnemyType::Ghast => Some(Timer::from_seconds(3.5, TimerMode::Repeating)), // Ghast shoots fireballs every 3.5s
            EnemyType::Dragon => Some(Timer::from_seconds(4.0, TimerMode::Repeating)), // Dragon shoots fireballs every 4s
            _ => None,
        };

        Self {
            enemy_type,
            shoot_timer,
            move_direction: 1.0,  // Start moving right
            direction_change_timer: Timer::from_seconds(ENEMY_DIRECTION_CHANGE_INTERVAL, TimerMode::Repeating),
        }
    }
}

impl EnemyBehavior for EnemyType {
    fn max_health(&self) -> u32 {
        match self {
            EnemyType::Zombie => 3,
            EnemyType::Skeleton => 2,
            EnemyType::Pigman => 3,
            EnemyType::Enderman => 3,
            EnemyType::Yeti => 5,
            EnemyType::Ghast => 10, // Flying enemy with decent HP
            EnemyType::Dragon => 20, // Boss enemy with high HP
        }
    }

    fn movement_speed(&self) -> f32 {
        match self {
            EnemyType::Zombie => 0.6,     // Slow
            EnemyType::Skeleton => 0.8,   // Medium
            EnemyType::Pigman => 0.9,     // Fast
            EnemyType::Enderman => 1.2,   // Very fast
            EnemyType::Yeti => 1.0,       // Normal speed
            EnemyType::Ghast => 0.8,      // Slow floating movement
            EnemyType::Dragon => 1.5,     // Fast flying movement
        }
    }

    fn shoot_interval(&self) -> Option<f32> {
        match self {
            EnemyType::Skeleton => Some(3.0),  // Shoots arrows every 3s
            EnemyType::Yeti => Some(2.0),      // Throws snowballs every 2s
            EnemyType::Ghast => Some(3.5),     // Shoots fireballs every 3.5s
            EnemyType::Dragon => Some(4.0),    // Shoots fireballs every 4s
            _ => None,
        }
    }

    fn color(&self) -> Color {
        match self {
            EnemyType::Zombie => Color::srgb(0.2, 0.6, 0.2),
            EnemyType::Skeleton => Color::srgb(0.9, 0.9, 0.9),
            EnemyType::Pigman => Color::srgb(0.9, 0.6, 0.6),
            EnemyType::Enderman => Color::srgb(0.1, 0.0, 0.1),
            EnemyType::Yeti => Color::srgb(0.9, 0.95, 1.0),
            EnemyType::Ghast => Color::srgb(0.95, 0.95, 0.95), // White/pale
            EnemyType::Dragon => Color::srgb(0.6, 0.1, 0.1), // Dark red
        }
    }

    fn attack_damage(&self) -> u32 {
        match self {
            EnemyType::Skeleton => 1,  // Arrow damage
            EnemyType::Yeti => 0,      // Snowballs do no damage (freeze effect instead)
            EnemyType::Ghast => 2,     // Fireball damage
            EnemyType::Dragon => 3,    // Powerful fireball damage
            _ => 1,                    // Default melee damage
        }
    }
}

#[derive(Component)]
pub struct Ghast {
    pub shoot_timer: Timer,
}

/// Component to track ghast shooting animation state
#[derive(Component)]
pub struct GhastShootingAnimation {
    pub timer: Timer,  // How long to show shooting texture (0.5s)
}

#[derive(Component, Clone, Copy, PartialEq)]
pub enum DragonState {
    Emerging,      // Flying out of blocks to free space
    Flying,        // Moving through the air
    Hovering,      // Floating in place
    Landing,       // Descending back to blocks
    Landed,        // Sitting on blocks
}

/// Marker component for flying entities that ignore block collisions
#[derive(Component)]
pub struct Flying;

/// Component to store dragon's animation clip handle
#[derive(Component)]
pub struct DragonAnimationClip(pub Handle<AnimationClip>);

#[derive(Component)]
pub struct Dragon {
    pub state: DragonState,
    pub state_timer: f32,           // Time in current state
    pub hover_offset: f32,          // Y offset for hovering effect
    pub hover_timer: f32,           // Timer for sine wave hovering
    pub target_position: Vec3,      // Where dragon is flying to
    pub spawn_position: Vec3,       // Original spawn position (for landing)
}
