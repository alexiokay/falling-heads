use crate::components::hero::Hero;
use crate::resources::biome_music::{BiomeMusicPlayer, BiomeMusicTracks};
use crate::resources::settings::GameSettings;
use crate::utils::biome_utils::determine_biome_at_position;
use bevy::asset::{Assets, LoadState};
use bevy::prelude::*;

const FADE_DURATION: f32 = 1.0; // 1 second crossfade

/// System that updates biome music based on hero's position
pub fn update_biome_music(
    mut commands: Commands,
    hero_query: Query<&Transform, With<Hero>>,
    mut music_player: ResMut<BiomeMusicPlayer>,
    music_tracks: Res<BiomeMusicTracks>,
    settings: Res<GameSettings>,
    time: Res<Time>,
    mut audio_sinks: Query<&mut AudioSink>,
    asset_server: Res<AssetServer>,
    audio_assets: Res<Assets<AudioSource>>,
) {
    // Get hero position
    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    // Determine current biome based on hero position
    let current_biome =
        determine_biome_at_position(hero_transform.translation.x, hero_transform.translation.y);

    // Check if biome has changed (or if this is the first biome detection)
    let biome_changed = music_player.current_biome != Some(current_biome);

    if biome_changed {
        info!(
            "🎵 Biome changed from {:?} to {:?}",
            music_player.current_biome, current_biome
        );
    }

    // Handle fade-out for old music
    let mut fade_completed = false;
    if let Some(fade_timer) = &mut music_player.fade_timer {
        fade_timer.tick(time.delta());
        let fade_progress = fade_timer.fraction();
        let fade_finished = fade_timer.is_finished();

        // Fade out the OLD music (fading_entity)
        if let Some(fading_entity) = music_player.fading_entity {
            if let Ok(mut sink) = audio_sinks.get_mut(fading_entity) {
                let volume = settings.get_music_volume() * (1.0 - fade_progress);
                sink.set_volume(bevy::audio::Volume::Linear(volume));
            }

            // When fade completes, despawn old music
            if fade_finished {
                commands.entity(fading_entity).despawn();
                fade_completed = true;
            }
        }
    }

    // Clear timer and fading entity after fade completes
    if fade_completed {
        music_player.fade_timer = None;
        music_player.fading_entity = None;
    }

    // Start new music if biome changed and we're not already crossfading
    if biome_changed && music_player.fade_timer.is_none() {
        // Move current music to fading entity (it will fade out)
        if let Some(old_entity) = music_player.music_entity.take() {
            music_player.fading_entity = Some(old_entity);
            music_player.fade_timer = Some(Timer::from_seconds(FADE_DURATION, TimerMode::Once));
        }

        // Try to spawn new music (check if asset is loaded first)
        let track = music_tracks.get_track(current_biome);

        // Check if the audio asset is loaded and valid
        let load_state = asset_server.get_load_state(track.id());

        match load_state {
            Some(LoadState::Loaded) => {
                // Asset is loaded - check if it's actually valid audio
                if audio_assets.get(track.id()).is_some() {
                    // Spawn new music - asset is valid
                    let new_music_entity = commands
                        .spawn((
                            crate::systems::audio::effects::MusicAudio,
                            AudioPlayer::new(track.clone()),
                            PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::Linear(
                                settings.get_music_volume(),
                            )),
                        ))
                        .id();

                    // Update music player state
                    music_player.current_biome = Some(current_biome);
                    music_player.music_entity = Some(new_music_entity);
                } else {
                    // Asset handle exists but audio is invalid
                    warn!(
                        "⚠️ Music file for biome {:?} is invalid or in unsupported format. Use .ogg files!",
                        current_biome
                    );
                    music_player.current_biome = Some(current_biome);
                }
            }
            Some(LoadState::Failed(_)) => {
                // Asset failed to load
                warn!(
                    "⚠️ Failed to load music for biome {:?}. Check if file exists at assets/audio/music/biomes/",
                    current_biome
                );
                music_player.current_biome = Some(current_biome);
            }
            Some(LoadState::Loading) => {
                // Still loading - wait for next frame
                info!("⏳ Loading music for biome {:?}...", current_biome);
            }
            Some(LoadState::NotLoaded) | None => {
                // Not loaded yet - wait
                warn!(
                    "⚠️ Music for biome {:?} not loaded. Check assets/audio/music/biomes/",
                    current_biome
                );
                music_player.current_biome = Some(current_biome);
            }
        }
    }

    // Always update volume for current music (not just when biome hasn't changed)
    // This allows real-time volume control from settings menu
    if music_player.fade_timer.is_none() {
        if let Some(music_entity) = music_player.music_entity {
            if let Ok(mut sink) = audio_sinks.get_mut(music_entity) {
                sink.set_volume(bevy::audio::Volume::Linear(settings.get_music_volume()));
            }
        }
    }
}

/// Simple system to update music volume even when in settings menu
/// This runs in Settings state to provide real-time volume feedback
pub fn update_music_volume_in_settings(
    music_player: Res<BiomeMusicPlayer>,
    settings: Res<GameSettings>,
    mut audio_sinks: Query<&mut AudioSink>,
) {
    if let Some(music_entity) = music_player.music_entity {
        if let Ok(mut sink) = audio_sinks.get_mut(music_entity) {
            sink.set_volume(bevy::audio::Volume::Linear(settings.get_music_volume()));
        }
    }
}
