use bevy::prelude::*;
use crate::utils::constants::ARROW_LIFETIME;

#[derive(Component)]
pub struct Arrow {
    pub damage: u32,
    pub lifetime: Timer,
    pub stuck_to: Option<Entity>, // Entity this arrow is stuck to
}

impl Arrow {
    pub fn new(damage: u32) -> Self {
        Self {
            damage,
            lifetime: Timer::from_seconds(ARROW_LIFETIME, TimerMode::Once),
            stuck_to: None,
        }
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
