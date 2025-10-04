use bevy::prelude::*;

#[derive(Component)]
pub struct Tnt {
    pub timer: Timer,
}

#[derive(Component)]
pub struct Explosion {
    pub radius: f32,
}
