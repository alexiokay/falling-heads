use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use crate::resources::inventory::Inventory;
use crate::resources::settings::GameSettings;
use crate::states::GameState;
use crate::systems::pause_menu_system::*;
use crate::systems::settings_menu_system::*;

pub struct UiPlugin;

#[derive(Component)]
struct InventoryText;

#[derive(Component)]
struct FpsText;

#[derive(Resource)]
struct FpsUpdateTimer {
    timer: Timer,
}

impl Default for FpsUpdateTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.5, TimerMode::Repeating), // Update twice per second
        }
    }
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<FpsUpdateTimer>()
            .add_systems(OnEnter(GameState::Playing), setup_ui)
            .add_systems(Update, (
                update_inventory_display,
                update_fps_display,
                update_fps_visibility,
                toggle_pause,
            ).run_if(in_state(GameState::Playing)))
            .add_systems(Update, (
                toggle_pause,
                handle_menu_buttons,
            ).run_if(in_state(GameState::Paused)))
            .add_systems(OnEnter(GameState::Paused), setup_pause_menu)
            .add_systems(OnExit(GameState::Paused), cleanup_pause_menu)
            .add_systems(Update, handle_settings_buttons.run_if(in_state(GameState::Settings)))
            .add_systems(OnEnter(GameState::Settings), setup_settings_menu)
            .add_systems(OnExit(GameState::Settings), cleanup_settings_menu);

        info!("UiPlugin loaded");
    }
}

fn setup_ui(mut commands: Commands) {
    // Spawn UI root for inventory (top left)
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            // Inventory display
            parent.spawn((
                Text::new("Resources: "),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                InventoryText,
            ));
        });

    // Spawn FPS counter (top right)
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::FlexStart,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("FPS: --"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.0, 1.0, 0.0)), // Green color
                FpsText,
            ));
        });

    info!("UI spawned");
}

fn update_inventory_display(
    inventory: Res<Inventory>,
    mut query: Query<&mut Text, With<InventoryText>>,
) {
    if !inventory.is_changed() {
        return;
    }

    for mut text in query.iter_mut() {
        **text = format!(
            "Resources: Coal:{} Iron:{} Gold:{} Diamond:{} Emerald:{} Redstone:{}",
            inventory.coal,
            inventory.iron,
            inventory.gold,
            inventory.diamond,
            inventory.emerald,
            inventory.redstone
        );
    }
}

fn update_fps_display(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<FpsText>>,
    mut timer: ResMut<FpsUpdateTimer>,
    time: Res<Time>,
    settings: Res<GameSettings>,
) {
    if !settings.show_fps {
        return;
    }

    // Only update FPS display every 0.5 seconds
    timer.timer.tick(time.delta());

    if !timer.timer.just_finished() {
        return;
    }

    for mut text in query.iter_mut() {
        if let Some(fps) = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|fps| fps.smoothed())
        {
            **text = format!("FPS: {:.0}", fps);
        }
    }
}

fn update_fps_visibility(
    settings: Res<GameSettings>,
    mut query: Query<&mut Visibility, With<FpsText>>,
) {
    if !settings.is_changed() {
        return;
    }

    for mut visibility in query.iter_mut() {
        *visibility = if settings.show_fps {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
