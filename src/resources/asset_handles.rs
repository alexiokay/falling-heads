use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct GameTextures {
    pub hero: Handle<Image>,
    pub tnt: Handle<Image>,
}
