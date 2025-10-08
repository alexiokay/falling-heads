use bevy::prelude::*;
use crate::components::dev_panel::*;
use crate::components::weather::{WeatherType, ActiveWeather};
use crate::resources::settings::GameSettings;
use crate::states::GameState;

pub struct DevPanelPlugin;

impl Plugin for DevPanelPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Playing), setup_dev_panel)
            .add_systems(OnExit(GameState::Playing), cleanup_dev_panel)
            .add_systems(Update, (
                toggle_dev_panel_visibility,
                handle_dev_panel_buttons,
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

/// Handle dev panel button clicks
fn handle_dev_panel_buttons(
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
