use bevy::prelude::*;
use crate::systems::audio::mob_sounds::MobSoundPlugin;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MobSoundPlugin);
        info!("AudioPlugin loaded");
    }
}
