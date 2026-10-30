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
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyType {
    Slime,
    Creeper,
    Bogged,
    Husk,
    Breeze,
    Silverfish,
    Blaze,
    MagmaCube,
    Stray,
    Shulker,
    Witch,
    Miner,
    Sentinel,
    Spider,
    Bat,
    Zombie,   // Spawns in overworld biomes
    Skeleton, // Spawns in overworld biomes
    Pigman,   // Spawns in Nether
    Enderman, // Spawns in End
    Yeti,     // Spawns in Iceland - throws snowballs
    Ghast,    // Spawns in Nether (flying) - shoots fireballs
    Dragon,   // Flying boss enemy - shoots fireballs
}

#[derive(Component)]
pub struct Enemy {
    pub enemy_type: EnemyType,
    pub shoot_timer: Option<Timer>, // Some enemy types shoot projectiles
    pub move_direction: f32,        // -1.0 = left, 1.0 = right
    pub direction_change_timer: Timer,
}

impl Enemy {
    pub fn new(enemy_type: EnemyType) -> Self {
        use crate::utils::constants::{ENEMY_DIRECTION_CHANGE_INTERVAL, SKELETON_SHOOT_INTERVAL};

        let shoot_timer = match enemy_type {
            EnemyType::Skeleton => Some(Timer::from_seconds(
                SKELETON_SHOOT_INTERVAL,
                TimerMode::Repeating,
            )),
            EnemyType::Yeti => Some(Timer::from_seconds(2.0, TimerMode::Repeating)), // Yeti throws snowballs every 2s
            EnemyType::Ghast => Some(Timer::from_seconds(3.5, TimerMode::Repeating)), // Ghast shoots fireballs every 3.5s
            EnemyType::Dragon => Some(Timer::from_seconds(4.0, TimerMode::Repeating)), // Dragon shoots fireballs every 4s
            _ => None,
        };

        Self {
            enemy_type,
            shoot_timer,
            move_direction: 1.0, // Start moving right
            direction_change_timer: Timer::from_seconds(
                ENEMY_DIRECTION_CHANGE_INTERVAL,
                TimerMode::Repeating,
            ),
        }
    }
}

impl EnemyBehavior for EnemyType {
    fn max_health(&self) -> u32 {
        match self {
            EnemyType::Bat => 2,
            EnemyType::Spider => 4,
            EnemyType::Witch => 4,
            EnemyType::Miner => 6,
            EnemyType::Sentinel => 4,
            EnemyType::Zombie => 3,
            EnemyType::Skeleton => 2,
            EnemyType::Pigman => 3,
            EnemyType::Enderman => 3,
            EnemyType::Yeti => 5,
            EnemyType::Ghast => 10, // Flying enemy with decent HP
            EnemyType::Slime | EnemyType::Silverfish => 2,
            EnemyType::Creeper | EnemyType::Bogged | EnemyType::Stray => 3,
            EnemyType::Husk | EnemyType::Breeze | EnemyType::MagmaCube => 4,
            EnemyType::Blaze | EnemyType::Shulker => 6,
            EnemyType::Dragon => 20, // Boss enemy with high HP
        }
    }
}

/// Component to track ghast shooting animation state
#[derive(Component)]
pub struct GhastShootingAnimation {
    pub timer: Timer, // How long to show shooting texture (0.5s)
}

/// Flying actors ignore block response.
#[derive(Component)]
pub struct Flying;

#[derive(Component)]
pub(crate) struct TerrainFootprint(pub Vec2);
