use bevy::prelude::*;

#[derive(Component)]
pub struct Arrow {
    pub angle: f32,
    pub speed: f32,
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
