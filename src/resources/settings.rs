use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const GAME_SETTINGS_FILE: &str = "game-settings.json";

/// Settings files live in the per-user config folder (%APPDATA%\MineHeads on
/// Windows), which stays writable when the game is installed under Program
/// Files. The old file next to the working directory is still read once.
pub fn settings_path(file: &str) -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    base.map_or_else(|| PathBuf::from(file), |b| b.join("MineHeads").join(file))
}

/// Reads a settings file from the user config folder, falling back to the
/// legacy copy in the working directory.
pub fn read_settings_file(file: &str) -> Option<String> {
    std::fs::read_to_string(settings_path(file))
        .or_else(|_| std::fs::read_to_string(file))
        .ok()
}

pub fn write_settings_file(file: &str, contents: &str) {
    let path = settings_path(file);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&path, contents) {
        warn!("Cannot save {}: {e}", path.display());
    }
}

/// Test, probe and benchmark launches change settings on purpose; they must
/// never overwrite the player's saved choices.
pub fn persistence_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        !std::env::args().skip(1).any(|a| {
            ["test", "check", "benchmark", "probe", "smoke", "export"]
                .iter()
                .any(|word| a.contains(word))
        })
    })
}

#[derive(Resource, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct GameSettings {
    pub vsync_enabled: bool,
    pub show_fps: bool,
    pub dev_mode: bool,      // Enable developer controls
    pub fps_limit: u32,      // FPS limit (0 = unlimited)
    pub effects_volume: f32, // Sound effects volume (0.0 to 1.0)
    pub music_volume: f32,   // Background music volume (0.0 to 1.0)
}

impl Default for GameSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl GameSettings {
    pub fn new() -> Self {
        Self {
            vsync_enabled: false, // Default to vsync off (use FPS limiter instead for less input lag)
            show_fps: true,       // Default to showing FPS
            dev_mode: false,      // Dev mode off by default
            fps_limit: 60,        // Default to 60 FPS (only active when VSync is off)
            effects_volume: 0.7,  // 70% effects volume by default
            music_volume: 0.5,    // 50% music volume by default
        }
    }

    /// Get effects volume (clamped 0.0-1.0)
    pub fn get_effects_volume(&self) -> f32 {
        self.effects_volume.clamp(0.0, 1.0)
    }

    /// Get music volume (clamped 0.0-1.0)
    pub fn get_music_volume(&self) -> f32 {
        self.music_volume.clamp(0.0, 1.0)
    }

    /// Set effects volume (clamped 0.0-1.0)
    pub fn set_effects_volume(&mut self, volume: f32) {
        self.effects_volume = volume.clamp(0.0, 1.0);
    }

    /// Set music volume (clamped 0.0-1.0)
    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume.clamp(0.0, 1.0);
    }
}

impl GameSettings {
    /// Saved settings, or the defaults for anything missing or unreadable.
    pub fn load() -> Self {
        read_settings_file(GAME_SETTINGS_FILE)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(s) = serde_json::to_string_pretty(self) {
            write_settings_file(GAME_SETTINGS_FILE, &s);
        }
    }
}

/// Saves game settings after they change, at most once a second (volume
/// sliders change them every frame while dragged).
pub fn save_game_settings(
    settings: Res<GameSettings>,
    time: Res<Time<Real>>,
    mut pending_since: Local<Option<f32>>,
) {
    if !persistence_enabled() {
        return;
    }
    let now = time.elapsed_secs();
    if settings.is_changed() && !settings.is_added() {
        pending_since.get_or_insert(now);
    }
    if pending_since.is_some_and(|since| now - since >= 1.0) {
        settings.save();
        *pending_since = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_keep_defaults_instead_of_resetting_everything() {
        let loaded: GameSettings = serde_json::from_str(r#"{"music_volume":0.2}"#).unwrap();
        assert_eq!(loaded.music_volume, 0.2);
        assert_eq!(loaded.fps_limit, GameSettings::new().fps_limit);
        assert_eq!(loaded.effects_volume, GameSettings::new().effects_volume);
    }
}
