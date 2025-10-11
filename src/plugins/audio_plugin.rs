use bevy::prelude::*;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, _app: &mut App) {
        info!("AudioPlugin loaded");
        // TODO: Add audio systems
    }
}
