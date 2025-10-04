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

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyType {
    Zombie,    // Spawns in overworld biomes
    Skeleton,  // Spawns in overworld biomes
    Pigman,    // Spawns in Nether
    Enderman,  // Spawns in End
}

#[derive(Component)]
pub struct Enemy {
    pub enemy_type: EnemyType,
    pub shoot_timer: Option<Timer>,  // Some enemy types shoot projectiles
}

impl Enemy {
    pub fn new(enemy_type: EnemyType) -> Self {
        let shoot_timer = match enemy_type {
            EnemyType::Skeleton => Some(Timer::from_seconds(2.0, TimerMode::Repeating)), // Shoot every 2 seconds
            _ => None,
        };

        Self {
            enemy_type,
            shoot_timer,
        }
    }
}

impl EnemyType {
    pub fn max_health(&self) -> u32 {
        match self {
            EnemyType::Zombie => 3,
            EnemyType::Skeleton => 2,
            EnemyType::Pigman => 3,
            EnemyType::Enderman => 3,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            EnemyType::Zombie => Color::srgb(0.2, 0.6, 0.2),  // Green
            EnemyType::Skeleton => Color::srgb(0.9, 0.9, 0.9), // White/bone
            EnemyType::Pigman => Color::srgb(0.9, 0.6, 0.6),   // Pink
            EnemyType::Enderman => Color::srgb(0.1, 0.0, 0.1), // Dark purple/black
        }
    }
}

#[derive(Component)]
pub struct Ghast {
    pub shoot_timer: Timer,
}
