use bevy::audio::Volume;
use bevy::prelude::*;
#[derive(Resource)]
pub struct RocketSounds {
    pub ignition: Handle<AudioSource>,
    pub engine: Handle<AudioSource>,
    pub crash: Handle<AudioSource>,
}
pub fn setup(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(RocketSounds {
        ignition: server.load("audio/rocket/ignition.ogg"),
        engine: server.load("audio/rocket/engine.ogg"),
        crash: server.load("audio/rocket/crash.ogg"),
    });
}
pub fn play(world: &mut World, kind: u8) -> Entity {
    let sound = world.resource::<RocketSounds>();
    let handle = match kind {
        0 => sound.ignition.clone(),
        1 => sound.engine.clone(),
        _ => sound.crash.clone(),
    };
    world
        .spawn((
            AudioPlayer::new(handle),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.65)),
        ))
        .id()
}
