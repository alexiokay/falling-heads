//! Switches for optional parts of the game (Settings > Game modes). Saved per
//! user in `%APPDATA%\FallingHeads\game-modes.json`; everything is on by default.
//! Each part checks its switch where it starts (spawning, triggering).
use crate::components::settings_menu::{SettingsPage, SettingsPageRoot, ToggleButtonText};
use crate::resources::settings::{persistence_enabled, read_settings_file, write_settings_file};
use crate::states::GameState;
use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

const GAME_MODES_FILE: &str = "game-modes.json";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameMode {
    RandomEvents,
    RandomBoxes,
    Weather,
    Dragon,
    Machines,
    LandmarkEnemies,
    ChatPowers,
}

impl GameMode {
    pub const ALL: [GameMode; 7] = [
        Self::RandomEvents,
        Self::RandomBoxes,
        Self::Weather,
        Self::Dragon,
        Self::Machines,
        Self::LandmarkEnemies,
        Self::ChatPowers,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::RandomEvents => "Random events",
            Self::RandomBoxes => "Random boxes",
            Self::Weather => "Weather",
            Self::Dragon => "Dragon flyovers",
            Self::Machines => "Machines (doors, pistons)",
            Self::LandmarkEnemies => "Landmark enemies",
            Self::ChatPowers => "Chat powers (API)",
        }
    }
}

#[derive(Resource, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct GameModes {
    pub random_events: bool,
    pub random_boxes: bool,
    pub weather: bool,
    pub dragon: bool,
    pub machines: bool,
    pub landmark_enemies: bool,
    pub chat_powers: bool,
}

impl Default for GameModes {
    fn default() -> Self {
        Self {
            random_events: true,
            random_boxes: true,
            weather: true,
            dragon: true,
            machines: true,
            landmark_enemies: true,
            chat_powers: true,
        }
    }
}

impl GameModes {
    pub fn enabled(&self, mode: GameMode) -> bool {
        match mode {
            GameMode::RandomEvents => self.random_events,
            GameMode::RandomBoxes => self.random_boxes,
            GameMode::Weather => self.weather,
            GameMode::Dragon => self.dragon,
            GameMode::Machines => self.machines,
            GameMode::LandmarkEnemies => self.landmark_enemies,
            GameMode::ChatPowers => self.chat_powers,
        }
    }

    pub fn toggle(&mut self, mode: GameMode) {
        let flag = match mode {
            GameMode::RandomEvents => &mut self.random_events,
            GameMode::RandomBoxes => &mut self.random_boxes,
            GameMode::Weather => &mut self.weather,
            GameMode::Dragon => &mut self.dragon,
            GameMode::Machines => &mut self.machines,
            GameMode::LandmarkEnemies => &mut self.landmark_enemies,
            GameMode::ChatPowers => &mut self.chat_powers,
        };
        *flag = !*flag;
    }

    /// Saved switches, or all on. Test and probe runs always start with all on.
    pub fn load() -> Self {
        if !persistence_enabled() {
            return Self::default();
        }
        read_settings_file(GAME_MODES_FILE)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
}

/// True when a mode is on. Missing resource (unit tests) counts as on.
pub fn is_on(modes: Option<&GameModes>, mode: GameMode) -> bool {
    modes.is_none_or(|m| m.enabled(mode))
}

/// Run condition: `.run_if(mode_on(GameMode::RandomEvents))`.
pub fn mode_on(mode: GameMode) -> impl FnMut(Option<Res<GameModes>>) -> bool + Clone {
    move |modes: Option<Res<GameModes>>| is_on(modes.as_deref(), mode)
}

/// Mirror of the chat switch for the HTTP thread, which has no ECS access.
static CHAT_POWERS_ON: AtomicBool = AtomicBool::new(true);

pub fn chat_powers_on() -> bool {
    CHAT_POWERS_ON.load(Ordering::Relaxed)
}

#[derive(Component)]
pub struct GameModeToggle(pub GameMode);

pub struct GameModesPlugin;

impl Plugin for GameModesPlugin {
    fn build(&self, app: &mut App) {
        let modes = GameModes::load();
        CHAT_POWERS_ON.store(modes.chat_powers, Ordering::Relaxed);
        app.insert_resource(modes)
            .add_systems(Update, save_game_modes)
            .add_systems(
                Update,
                (fill_game_modes_page, handle_game_mode_toggles)
                    .chain()
                    .run_if(in_state(GameState::Settings)),
            );
    }
}

/// Saves after a change (at most once a second) and keeps the API flag in sync.
fn save_game_modes(
    modes: Res<GameModes>,
    time: Res<Time<Real>>,
    mut pending_since: Local<Option<f32>>,
) {
    let now = time.elapsed_secs();
    if modes.is_changed() && !modes.is_added() {
        CHAT_POWERS_ON.store(modes.chat_powers, Ordering::Relaxed);
        pending_since.get_or_insert(now);
    }
    if pending_since.is_some_and(|since| now - since >= 1.0) {
        *pending_since = None;
        if persistence_enabled()
            && let Ok(json) = serde_json::to_string_pretty(&*modes)
        {
            write_settings_file(GAME_MODES_FILE, &json);
        }
    }
}

fn toggle_label(mode: GameMode, on: bool) -> String {
    format!("{}: {}", mode.label(), if on { "ON" } else { "OFF" })
}

/// Fills the Game modes settings page with one toggle per mode.
fn fill_game_modes_page(
    mut commands: Commands,
    pages: Query<(Entity, &SettingsPageRoot), Added<SettingsPageRoot>>,
    modes: Res<GameModes>,
) {
    for (page, root) in &pages {
        if root.0 != SettingsPage::GameModes {
            continue;
        }
        commands.entity(page).with_children(|p| {
            p.spawn((
                bevy::picking::Pickable::IGNORE,
                Text::new("Turn parts of the game on or off. Saved for next time."),
                TextFont {
                    font_size: FontSize::Px(14.),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.75, 0.78)),
            ));
            for mode in GameMode::ALL {
                crate::systems::settings_menu_system::menu_button(
                    p,
                    &toggle_label(mode, modes.enabled(mode)),
                    GameModeToggle(mode),
                );
            }
            // Where chat bots and VortiDeck send commands (the port can change
            // when Windows reserves the default one).
            p.spawn((
                bevy::picking::Pickable::IGNORE,
                Text::new(format!(
                    "Viewer commands: {}",
                    crate::plugins::external_api_plugin::api_url()
                )),
                TextFont {
                    font_size: FontSize::Px(13.),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.75, 0.78)),
            ));
        });
    }
}

fn handle_game_mode_toggles(
    mut modes: ResMut<GameModes>,
    buttons: Query<
        (Entity, &PickingInteraction, &GameModeToggle, &Children),
        Changed<PickingInteraction>,
    >,
    all: Query<(&GameModeToggle, &Children)>,
    mut texts: Query<&mut Text, With<ToggleButtonText>>,
    mut presses: Local<crate::systems::settings_menu_system::PressTracker>,
) {
    for (entity, interaction, toggle, _) in &buttons {
        if presses.pressed(entity, *interaction) {
            modes.toggle(toggle.0);
            info!(
                "Game mode {} turned {}",
                toggle.0.label(),
                if modes.enabled(toggle.0) { "on" } else { "off" }
            );
        }
    }
    if !modes.is_changed() {
        return;
    }
    for (toggle, children) in &all {
        let label = toggle_label(toggle.0, modes.enabled(toggle.0));
        for child in children {
            if let Ok(mut text) = texts.get_mut(*child)
                && text.0 != label
            {
                text.0 = label.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_default_on_toggle_and_survive_missing_fields() {
        let mut modes = GameModes::default();
        assert!(GameMode::ALL.iter().all(|m| modes.enabled(*m)));
        modes.toggle(GameMode::Dragon);
        assert!(!modes.enabled(GameMode::Dragon));
        assert!(modes.enabled(GameMode::Weather));
        // An older save without some switches keeps the rest on.
        // A save from before the Slime Rift was removed still loads.
        let loaded: GameModes =
            serde_json::from_str(r#"{"weather":false,"slime_rift":false}"#).unwrap();
        assert!(!loaded.weather && loaded.dragon && loaded.chat_powers);
        assert!(is_on(None, GameMode::Dragon));
    }
}
