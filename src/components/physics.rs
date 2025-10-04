use bevy::prelude::*;

#[derive(Component)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct Gravity {
    pub force: f32,
}

#[derive(Component)]
pub struct Bouncy {
    pub restitution: f32,
}
