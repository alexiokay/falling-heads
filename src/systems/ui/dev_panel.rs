use bevy::prelude::*;
use crate::components::dev_panel::*;
use crate::components::weather::{WeatherType, ActiveWeather};
use crate::components::block::BiomeType;
use crate::components::hero::Hero;
use crate::resources::settings::GameSettings;
use crate::resources::command_queue::PowerCommandType;
use crate::states::GameState;
use crate::plugins::external_api_plugin::get_command_queue;

pub struct DevPanelPlugin;

impl Plugin for DevPanelPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Playing), setup_dev_panel)
            .add_systems(OnExit(GameState::Playing), cleanup_dev_panel)
            .add_systems(Update, (
                toggle_dev_panel_visibility,
                toggle_dev_panel_with_tab,
                handle_weather_buttons,
                handle_power_buttons,
                handle_biome_teleport_buttons,
                update_dev_panel_info,
            ).run_if(in_state(GameState::Playing)));

        info!("DevPanelPlugin loaded");
    }
}

/// Setup dev panel UI (hidden by default)
fn setup_dev_panel(
    mut commands: Commands,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(10.0),
                width: Val::Px(300.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            Visibility::Hidden,  // Hidden by default
            DevPanelRoot,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("🔧 DEV CONTROLS"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.5, 0.5)),
                Node {
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                },
            ));

            // Weather label
            parent.spawn((
                Text::new("Weather Control:"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Weather buttons
            let weather_types = [
                (WeatherType::Clear, "Clear"),
                (WeatherType::Rain, "Rain"),
                (WeatherType::Snow, "Snow"),
                (WeatherType::Sandstorm, "Sandstorm"),
                (WeatherType::AshStorm, "Ash Storm"),
                (WeatherType::FireRain, "Fire Rain"),
            ];

            for (weather_type, label) in weather_types {
                parent
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.3, 0.3, 0.5)),
                        WeatherControlButton { weather_type },
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Text::new(label),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
            }

            // Current weather info
            parent.spawn((
                Text::new("Current: Clear"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                Node {
                    margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                },
                ParameterLabel,
            ));

            // Powers label
            parent.spawn((
                Text::new("Powers:"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::top(Val::Px(15.0)),
                    ..default()
                },
            ));

            // Power buttons
            let power_types = [
                (PowerCommandType::Tnt, "TNT"),
                (PowerCommandType::Fast, "FAST"),
                (PowerCommandType::Slow, "SLOW"),
                (PowerCommandType::Arrow, "ARROW"),
                (PowerCommandType::Laser, "LASER"),
                (PowerCommandType::Mirror, "MIRROR"),
                (PowerCommandType::Magnet, "MAGNET"),
                (PowerCommandType::Villagers, "VILLAGERS"),
                (PowerCommandType::Ghast, "GHAST"),
                (PowerCommandType::Thunder, "THUNDER"),
                (PowerCommandType::Lootbox, "LOOTBOX"),
                (PowerCommandType::RandomBox, "RANDOMBOX"),
                (PowerCommandType::GrowBuff, "GROW"),
            ];

            for (power_type, label) in power_types {
                parent
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.5, 0.3, 0.3)),
                        PowerButton { power_type },
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Text::new(label),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
            }

            // Biome Teleport label
            parent.spawn((
                Text::new("Biome Teleport:"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::top(Val::Px(15.0)),
                    ..default()
                },
            ));

            // Biome teleport buttons
            let biome_types = [
                (BiomeType::Plains, "Plains"),
                (BiomeType::Forest, "Forest"),
                (BiomeType::Desert, "Desert"),
                (BiomeType::Mountains, "Mountains"),
                (BiomeType::Underground, "Underground"),
                (BiomeType::Nether, "Nether"),
                (BiomeType::End, "End"),
                (BiomeType::Iceland, "Iceland"),
            ];

            for (biome_type, label) in biome_types {
                parent
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.3, 0.5, 0.3)),
                        BiomeTeleportButton { biome_type },
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Text::new(label),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
            }
        });
}

fn cleanup_dev_panel(
    mut commands: Commands,
    query: Query<Entity, With<DevPanelRoot>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Toggle dev panel visibility based on dev_mode setting
fn toggle_dev_panel_visibility(
    settings: Res<GameSettings>,
    mut query: Query<&mut Visibility, With<DevPanelRoot>>,
) {
    if settings.is_changed() {
        for mut visibility in query.iter_mut() {
            *visibility = if settings.dev_mode {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// Toggle dev panel with Tab key
fn toggle_dev_panel_with_tab(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<GameSettings>,
) {
    if keyboard.just_pressed(KeyCode::Tab) {
        settings.dev_mode = !settings.dev_mode;
        info!("🔧 Dev panel toggled: {}", if settings.dev_mode { "ON" } else { "OFF" });
    }
}

/// Handle weather control button clicks
fn handle_weather_buttons(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &WeatherControlButton),
        Changed<Interaction>,
    >,
    mut weather: ResMut<ActiveWeather>,
) {
    for (interaction, mut color, weather_btn) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                // Change weather immediately
                weather.weather_type = weather_btn.weather_type;
                weather.intensity = if weather_btn.weather_type == WeatherType::Clear { 0.0 } else { 1.0 };
                weather.duration_remaining = 9999.0; // Keep it for a long time
                weather.transition_timer = 0.0;

                info!("🌦️ Dev: Weather changed to {:?}", weather_btn.weather_type);
                *color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2)); // Green flash
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.5, 0.5, 0.7));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.5));
            }
        }
    }
}

/// Handle power button clicks
fn handle_power_buttons(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &PowerButton),
        Changed<Interaction>,
    >,
) {
    for (interaction, mut color, power_btn) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                // Add power to global command queue (same as API)
                if let Some(queue) = get_command_queue() {
                    if let Ok(mut q) = queue.lock() {
                        q.push_back(crate::resources::command_queue::PowerCommand::new(
                            power_btn.power_type,
                            "DevPanel".to_string()
                        ));
                        info!("⚡ Dev: Power triggered {:?}", power_btn.power_type);
                        *color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2)); // Green flash
                    }
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.7, 0.5, 0.5));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.5, 0.3, 0.3));
            }
        }
    }
}

/// Handle biome teleport button clicks
fn handle_biome_teleport_buttons(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &BiomeTeleportButton),
        Changed<Interaction>,
    >,
    mut hero_query: Query<&mut Transform, With<Hero>>,
) {
    for (interaction, mut color, biome_btn) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                // Teleport hero to a Y position where this biome exists
                // Each biome appears in specific Y ranges based on biome_zone calculation
                // biome_zone = chunk_y / 4, chunk_y = y / (64.0 * 64.0) = y / 4096
                // So we need to find a Y that produces the right biome_seed

                let target_y = match biome_btn.biome_type {
                    BiomeType::Plains => 0.0,           // biome_seed % 8 = 0
                    BiomeType::Forest => 4096.0,        // biome_seed % 8 = 1
                    BiomeType::Desert => 8192.0,        // biome_seed % 8 = 2
                    BiomeType::Mountains => 12288.0,    // biome_seed % 8 = 3
                    BiomeType::Underground => 16384.0,  // biome_seed % 8 = 4
                    BiomeType::Nether => 20480.0,       // biome_seed % 8 = 5
                    BiomeType::End => 24576.0,          // biome_seed % 8 = 6
                    BiomeType::Iceland => 28672.0,      // biome_seed % 8 = 7
                };

                if let Ok(mut hero_transform) = hero_query.single_mut() {
                    hero_transform.translation.y = target_y;
                    info!("🌍 Dev: Teleported to {:?} biome (Y={})", biome_btn.biome_type, target_y);
                    *color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2)); // Green flash
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.5, 0.7, 0.5));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.3, 0.5, 0.3));
            }
        }
    }
}

/// Update dev panel info text
fn update_dev_panel_info(
    weather: Res<ActiveWeather>,
    mut query: Query<&mut Text, With<ParameterLabel>>,
) {
    if weather.is_changed() {
        for mut text in query.iter_mut() {
            **text = format!("Current: {:?} ({:.0}%)", weather.weather_type, weather.intensity * 100.0);
        }
    }
}
