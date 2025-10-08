use bevy::prelude::*;
use bevy::window::{PresentMode, Window, PrimaryWindow};
use crate::components::settings_menu::*;
use crate::resources::settings::GameSettings;
use crate::states::GameState;

pub fn setup_settings_menu(
    mut commands: Commands,
    settings: Res<GameSettings>,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            SettingsMenuRoot,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("SETTINGS"),
                TextFont {
                    font_size: 60.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::all(Val::Px(30.0)),
                    ..default()
                },
            ));

            // VSync Toggle Button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Px(65.0),
                        margin: UiRect::all(Val::Px(10.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.6)),
                    VsyncToggleButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(if settings.vsync_enabled { "VSync: ON" } else { "VSync: OFF" }),
                        TextFont {
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        ToggleButtonText,
                    ));
                });

            // FPS Counter Toggle Button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Px(65.0),
                        margin: UiRect::all(Val::Px(10.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.6)),
                    FpsToggleButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(if settings.show_fps { "FPS Counter: ON" } else { "FPS Counter: OFF" }),
                        TextFont {
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        ToggleButtonText,
                    ));
                });

            // Dev Mode Toggle Button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Px(65.0),
                        margin: UiRect::all(Val::Px(10.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.6, 0.3, 0.3)),  // Red-ish for dev mode
                    DevModeToggleButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(if settings.dev_mode { "Dev Mode: ON" } else { "Dev Mode: OFF" }),
                        TextFont {
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        ToggleButtonText,
                    ));
                });

            // Back Button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(65.0),
                        margin: UiRect::top(Val::Px(30.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.5, 0.5, 0.5)),
                    BackToMenuButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Back"),
                        TextFont {
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

pub fn cleanup_settings_menu(mut commands: Commands, query: Query<Entity, With<SettingsMenuRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn handle_settings_buttons(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&VsyncToggleButton>,
            Option<&FpsToggleButton>,
            Option<&DevModeToggleButton>,
            Option<&BackToMenuButton>,
        ),
        Changed<Interaction>,
    >,
    mut settings: ResMut<GameSettings>,
    mut next_state: ResMut<NextState<GameState>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    vsync_button_query: Query<&Children, With<VsyncToggleButton>>,
    fps_button_query: Query<&Children, With<FpsToggleButton>>,
    dev_button_query: Query<&Children, With<DevModeToggleButton>>,
    mut text_query: Query<&mut Text, With<ToggleButtonText>>,
) {
    for (interaction, mut color, vsync_btn, fps_btn, dev_btn, back_btn) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                if vsync_btn.is_some() {
                    // Toggle VSync
                    settings.vsync_enabled = !settings.vsync_enabled;

                    // Apply VSync setting to window
                    if let Ok(mut window) = window_query.single_mut() {
                        window.present_mode = if settings.vsync_enabled {
                            PresentMode::AutoVsync
                        } else {
                            PresentMode::AutoNoVsync
                        };
                    }

                    // Update button text
                    if let Ok(children) = vsync_button_query.single() {
                        for child in children.iter() {
                            if let Ok(mut text) = text_query.get_mut(child) {
                                **text = if settings.vsync_enabled { "VSync: ON" } else { "VSync: OFF" }.to_string();
                            }
                        }
                    }
                } else if fps_btn.is_some() {
                    // Toggle FPS counter
                    settings.show_fps = !settings.show_fps;

                    // Update button text
                    if let Ok(children) = fps_button_query.single() {
                        for child in children.iter() {
                            if let Ok(mut text) = text_query.get_mut(child) {
                                **text = if settings.show_fps { "FPS Counter: ON" } else { "FPS Counter: OFF" }.to_string();
                            }
                        }
                    }
                } else if dev_btn.is_some() {
                    // Toggle Dev Mode
                    settings.dev_mode = !settings.dev_mode;

                    info!("🔧 Dev Mode {}", if settings.dev_mode { "ENABLED" } else { "DISABLED" });

                    // Update button text
                    if let Ok(children) = dev_button_query.single() {
                        for child in children.iter() {
                            if let Ok(mut text) = text_query.get_mut(child) {
                                **text = if settings.dev_mode { "Dev Mode: ON" } else { "Dev Mode: OFF" }.to_string();
                            }
                        }
                    }
                } else if back_btn.is_some() {
                    next_state.set(GameState::Paused);
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.6, 0.6, 0.8));
            }
            Interaction::None => {
                if vsync_btn.is_some() || fps_btn.is_some() {
                    *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.6));
                } else if dev_btn.is_some() {
                    *color = BackgroundColor(Color::srgb(0.6, 0.3, 0.3));
                } else if back_btn.is_some() {
                    *color = BackgroundColor(Color::srgb(0.5, 0.5, 0.5));
                }
            }
        }
    }
}
