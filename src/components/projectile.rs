use bevy::prelude::*;

#[derive(Component)]
pub struct Arrow {
    pub damage: u32,
    pub lifetime: Timer,
}

impl Arrow {
    pub fn new(damage: u32) -> Self {
        Self {
            damage,
            lifetime: Timer::from_seconds(5.0, TimerMode::Once), // Despawn after 5 seconds
        }
    }
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
