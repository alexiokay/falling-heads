use bevy::prelude::*;

#[derive(Component)]
pub struct AnimationState {
    pub current_frame: usize,
    pub timer: Timer,
}

#[derive(Component)]
pub struct ParticleEmitter;
