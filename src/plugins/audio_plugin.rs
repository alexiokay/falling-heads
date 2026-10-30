use crate::systems::audio::mob_sounds::MobSoundPlugin;
use bevy::prelude::*;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MobSoundPlugin,
            crate::systems::audio::effects::EffectsVolumePlugin,
        ));
        info!("AudioPlugin loaded");
    }
}
