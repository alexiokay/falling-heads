//! All non-music audio uses one effects bus, including existing loops in menus.
use crate::resources::settings::GameSettings;
use bevy::{audio::Volume, prelude::*};
#[derive(Component)]
pub struct MusicAudio;
#[derive(Component)]
struct EffectGain(f32);
pub struct EffectsVolumePlugin;
impl Plugin for EffectsVolumePlugin {
    fn build(&self, app: &mut App) {
        // Bevy queues playback after transform propagation. Apply gain before the
        // first audio sample, even when a player was spawned late in Update.
        app.add_systems(
            PostUpdate,
            (classify, apply)
                .chain()
                .before(TransformSystems::Propagate),
        );
    }
}
/// World position of a sound (a slime jump, a block breaking, an arrow). Such
/// sounds fade with distance from the camera and are dropped when far off-screen,
/// so creatures you cannot see don't fill the mix with clicks.
#[derive(Component, Clone, Copy)]
pub struct SoundAt(pub Vec2);

/// Full volume within this distance of the camera centre...
const HEARING_FULL: f32 = 1000.0;
/// ...fading to silence here (beyond it the sound is not played at all).
const HEARING_MAX: f32 = 2000.0;

pub(crate) fn distance_gain(distance: f32) -> f32 {
    (1.0 - (distance - HEARING_FULL) / (HEARING_MAX - HEARING_FULL)).clamp(0.0, 1.0)
}

fn classify(
    mut commands: Commands,
    mut q: Query<
        (Entity, &mut PlaybackSettings, Option<&SoundAt>),
        (With<AudioPlayer>, Without<MusicAudio>, Without<EffectGain>),
    >,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    settings: Res<GameSettings>,
    loading: Option<Res<crate::systems::ui::loading_screen::LoadingActive>>,
) {
    // Nothing is heard behind the loading screen.
    if loading.is_some() {
        for (e, _, _) in &mut q {
            commands.entity(e).despawn();
        }
        return;
    }
    let listener = cameras.iter().next().map(|c| c.translation().xy());
    for (e, mut p, at) in &mut q {
        let falloff = match (at, listener) {
            (Some(at), Some(listener)) if at.0.is_finite() => {
                distance_gain(at.0.distance(listener))
            }
            _ => 1.0,
        };
        if falloff <= 0.0 {
            // Too far away to hear: never start the decoder.
            commands.entity(e).despawn();
            continue;
        }
        let gain = p.volume.to_linear() * falloff;
        commands.entity(e).insert(EffectGain(gain));
        p.volume = Volume::Linear(gain * settings.get_effects_volume());
    }
}
fn apply(
    settings: Res<GameSettings>,
    mut q: Query<
        (
            &EffectGain,
            &mut PlaybackSettings,
            Option<&mut AudioSink>,
            Option<&mut SpatialAudioSink>,
        ),
        Without<MusicAudio>,
    >,
) {
    for (gain, mut playback, sink, spatial) in &mut q {
        let volume = Volume::Linear(gain.0 * settings.get_effects_volume());
        if playback.volume != volume {
            playback.volume = volume;
        }
        if let Some(mut sink) = sink {
            if sink.volume() != volume {
                sink.set_volume(volume);
            }
        }
        if let Some(mut sink) = spatial {
            if sink.volume() != volume {
                sink.set_volume(volume);
            }
        }
    }
}
pub(crate) fn verify_live(world: &mut World, expected: f32) {
    let mut count = 0;
    for (gain, sink) in world.query::<(&EffectGain, &AudioSink)>().iter(world) {
        assert!((sink.volume().to_linear() - gain.0 * expected).abs() < 0.001);
        count += 1;
    }
    assert!(count > 0, "A real playing effects sink must be checked");
    eprintln!("[POLISH TEST] {count} live effects sinks at slider {expected}");
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn far_world_sounds_are_dropped_and_mid_range_ones_fade() {
        let mut app = App::new();
        app.insert_resource(GameSettings::new())
            .add_systems(Update, classify);
        app.world_mut()
            .spawn((Camera3d::default(), GlobalTransform::default()));
        let mut sound = |at: Vec2| {
            app.world_mut()
                .spawn((
                    AudioPlayer::<AudioSource>(Handle::default()),
                    PlaybackSettings::DESPAWN,
                    SoundAt(at),
                ))
                .id()
        };
        let near = sound(Vec2::new(300., 0.));
        let mid = sound(Vec2::new(1500., 0.));
        let far = sound(Vec2::new(5000., 0.));
        let unknown = sound(Vec2::NAN);
        app.update();
        let gain = |e| app.world().get::<EffectGain>(e).map(|g| g.0);
        assert_eq!(gain(near), Some(1.0));
        assert_eq!(gain(mid), Some(0.5));
        assert_eq!(gain(unknown), Some(1.0));
        assert!(
            app.world().get_entity(far).is_err(),
            "far sound never plays"
        );
    }
    #[test]
    fn mute_and_restore_keep_base_gains_and_leave_music_alone() {
        let mut app = App::new();
        app.insert_resource(GameSettings::new())
            .add_systems(Update, (classify, apply).chain());
        let a = app
            .world_mut()
            .spawn((
                AudioPlayer::<AudioSource>::new(Handle::default()),
                PlaybackSettings::LOOP.with_volume(Volume::Linear(0.3)),
            ))
            .id();
        let b = app
            .world_mut()
            .spawn((
                AudioPlayer::<AudioSource>::new(Handle::default()),
                PlaybackSettings::DESPAWN,
            ))
            .id();
        let music = app
            .world_mut()
            .spawn((
                MusicAudio,
                AudioPlayer::<AudioSource>::new(Handle::default()),
                PlaybackSettings::LOOP.with_volume(Volume::Linear(0.6)),
            ))
            .id();
        for v in [0., 0.2, 1., 0., 0.8] {
            app.world_mut()
                .resource_mut::<GameSettings>()
                .effects_volume = v;
            app.update();
            app.update();
            assert!(
                (app.world()
                    .get::<PlaybackSettings>(a)
                    .unwrap()
                    .volume
                    .to_linear()
                    - v * 0.3)
                    .abs()
                    < 0.0001
            );
            assert_eq!(
                app.world()
                    .get::<PlaybackSettings>(b)
                    .unwrap()
                    .volume
                    .to_linear(),
                v
            );
            assert_eq!(
                app.world()
                    .get::<PlaybackSettings>(music)
                    .unwrap()
                    .volume
                    .to_linear(),
                0.6
            );
        }
    }
}
