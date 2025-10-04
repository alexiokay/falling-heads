use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct Score {
    pub points: u32,
    pub multiplier: f32,
}
