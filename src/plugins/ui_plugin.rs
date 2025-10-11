use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use crate::resources::inventory::Inventory;
use crate::resources::settings::GameSettings;
use crate::resources::world_config::WorldConfig;
use crate::resources::day_night_cycle::DayNightCycle;
use crate::states::GameState;
use crate::systems::pause_menu_system::*;
use crate::systems::settings_menu_system::*;
use crate::components::hero::Hero;

pub struct UiPlugin;

#[derive(Component)]
struct InventoryText;

#[derive(Component)]
struct ResourceCount {
    resource_type: String,
}

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct YCoordinateText;

#[derive(Component)]
struct DifficultyText;

#[derive(Component)]
struct DayNightText;

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
            .init_resource::<DayNightCycle>()
            .add_systems(OnEnter(GameState::Playing), setup_ui)
            .add_systems(Update, (
                update_day_night_cycle,
                update_inventory_display,
                update_fps_display,
                update_fps_visibility,
                update_y_coordinate,
                update_difficulty_display,
                update_day_night_display,
                toggle_pause,
            ).run_if(in_state(GameState::Playing)))
            .add_systems(Update, (
                toggle_pause,
                handle_menu_buttons,
            ).run_if(in_state(GameState::Paused)))
            .add_systems(OnEnter(GameState::Paused), setup_pause_menu)
            .add_systems(OnExit(GameState::Paused), cleanup_pause_menu)
            .add_systems(Update, (
                toggle_pause,
                handle_settings_buttons,
            ).run_if(in_state(GameState::Settings)))
            .add_systems(OnEnter(GameState::Settings), setup_settings_menu)
            .add_systems(OnExit(GameState::Settings), cleanup_settings_menu);

        info!("UiPlugin loaded");
    }
}

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn UI root for inventory (top left, vertical column)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Vh(1.0),
                left: Val::Vw(1.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Vh(1.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            InventoryText,
        ))
        .with_children(|parent| {
            // Helper macro to spawn resource rows
            macro_rules! resource_row {
                ($name:expr, $texture:expr) => {
                    parent.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Vw(0.6),
                        ..default()
                    }).with_children(|row| {
                        row.spawn((
                            ImageNode::new(asset_server.load($texture)),
                            Node {
                                width: Val::Vw(2.5),
                                height: Val::Vw(2.5),
                                ..default()
                            },
                        ));
                        row.spawn((
                            Text::new("0"),
                            TextFont {
                                font_size: 28.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            ResourceCount {
                                resource_type: $name.to_string(),
                            },
                        ));
                    });
                };
            }

            // Spawn all resource rows
            resource_row!("coal", "textures/resources/coal.png");
            resource_row!("iron", "textures/resources/iron_ingot.png");
            resource_row!("gold", "textures/resources/gold_ingot.png");
            resource_row!("diamond", "textures/resources/diamond.png");
            resource_row!("emerald", "textures/resources/emerald.png");
            resource_row!("redstone", "textures/resources/redstone.png");
            resource_row!("netherite", "textures/resources/netherite_ingot.png");

            // Y coordinate
            parent.spawn((
                Text::new("Y: 0"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect {
                        top: Val::Px(10.0),
                        ..default()
                    },
                    ..default()
                },
                YCoordinateText,
            ));

            // Difficulty
            parent.spawn((
                Text::new("Difficulty: 1.0"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.5, 0.0)), // Orange
                DifficultyText,
            ));

            // Day/Night
            parent.spawn((
                Text::new("Day"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.0)), // Yellow
                DayNightText,
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
    mut query: Query<(&mut Text, &ResourceCount)>,
) {
    if !inventory.is_changed() {
        return;
    }

    for (mut text, resource_count) in query.iter_mut() {
        let count = match resource_count.resource_type.as_str() {
            "coal" => inventory.coal,
            "iron" => inventory.iron,
            "gold" => inventory.gold,
            "diamond" => inventory.diamond,
            "emerald" => inventory.emerald,
            "redstone" => inventory.redstone,
            "netherite" => inventory.netherite,
            _ => 0,
        };
        **text = format!("{}", count);
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

fn update_y_coordinate(
    hero_query: Query<&Transform, With<Hero>>,
    mut text_query: Query<&mut Text, With<YCoordinateText>>,
) {
    if let Ok(hero_transform) = hero_query.single() {
        for mut text in text_query.iter_mut() {
            let block_y = hero_transform.translation.y / crate::utils::constants::BLOCK_SIZE;
            **text = format!("Y: {:.0}", block_y);
        }
    }
}

fn update_difficulty_display(
    world_config: Res<WorldConfig>,
    mut query: Query<&mut Text, With<DifficultyText>>,
) {
    for mut text in query.iter_mut() {
        let new_text = format!("Difficulty: {:.1}", world_config.difficulty_scale);
        if **text != new_text {
            **text = new_text;
        }
    }
}

fn update_day_night_cycle(
    time: Res<Time>,
    mut cycle: ResMut<DayNightCycle>,
) {
    cycle.update(time.delta_secs());
}

fn update_day_night_display(
    cycle: Res<DayNightCycle>,
    mut query: Query<(&mut Text, &mut TextColor), With<DayNightText>>,
) {
    for (mut text, mut color) in query.iter_mut() {
        let (new_text, new_color) = if cycle.is_day() {
            ("Day", Color::srgb(1.0, 1.0, 0.0)) // Yellow
        } else {
            ("Night", Color::srgb(0.3, 0.3, 0.8)) // Dark blue
        };

        if **text != new_text {
            **text = new_text.to_string();
            *color = TextColor(new_color);
        }
    }
}
