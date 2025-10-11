use bevy::prelude::*;
use bevy::ecs::system::ParamSet;
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

            // FPS Limit Button (disabled when VSync is on)
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
                    BackgroundColor(if settings.vsync_enabled {
                        Color::srgb(0.2, 0.2, 0.2) // Grayed out when VSync is on
                    } else {
                        Color::srgb(0.3, 0.6, 0.3)
                    }),
                    FpsLimitButton,
                ))
                .with_children(|parent| {
                    let fps_text = if settings.fps_limit == 0 {
                        "FPS Limit: Unlimited".to_string()
                    } else {
                        format!("FPS Limit: {}", settings.fps_limit)
                    };
                    parent.spawn((
                        Text::new(fps_text),
                        TextFont {
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(if settings.vsync_enabled {
                            Color::srgb(0.5, 0.5, 0.5) // Grayed out text
                        } else {
                            Color::WHITE
                        }),
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
    mut param_set: ParamSet<(
        Query<
            (
                Entity,
                &Interaction,
                &mut BackgroundColor,
                &Children,
                Option<&VsyncToggleButton>,
                Option<&FpsToggleButton>,
                Option<&FpsLimitButton>,
                Option<&DevModeToggleButton>,
                Option<&BackToMenuButton>,
            ),
            Changed<Interaction>,
        >,
        Query<(&mut BackgroundColor, &Children), With<FpsLimitButton>>,
    )>,
    mut settings: ResMut<GameSettings>,
    mut next_state: ResMut<NextState<GameState>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut text_query: Query<&mut Text, With<ToggleButtonText>>,
    mut text_color_query: Query<&mut TextColor>,
    mouse_button: Res<ButtonInput<MouseButton>>,
) {
    for (_entity, interaction, mut color, children, vsync_btn, fps_btn, fps_limit_btn, dev_btn, back_btn) in &mut param_set.p0() {
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

                    // Update VSync button text
                    for child in children.iter() {
                        if let Ok(mut text) = text_query.get_mut(child) {
                            **text = if settings.vsync_enabled { "VSync: ON" } else { "VSync: OFF" }.to_string();
                        }
                    }
                } else if fps_btn.is_some() {
                    // Toggle FPS counter
                    settings.show_fps = !settings.show_fps;

                    // Update button text
                    for child in children.iter() {
                        if let Ok(mut text) = text_query.get_mut(child) {
                            **text = if settings.show_fps { "FPS Counter: ON" } else { "FPS Counter: OFF" }.to_string();
                        }
                    }
                } else if fps_limit_btn.is_some() {
                    // Only allow changing FPS limit when VSync is off
                    if !settings.vsync_enabled {
                        // Check if right-click for backwards cycling
                        let cycle_backwards = mouse_button.just_pressed(MouseButton::Right);

                        if cycle_backwards {
                            // Cycle backwards: 30 <- 60 <- 120 <- 144 <- 240 <- Unlimited (0) <- 30
                            settings.fps_limit = match settings.fps_limit {
                                60 => 30,
                                120 => 60,
                                144 => 120,
                                240 => 144,
                                0 => 240,
                                _ => 0,
                            };
                        } else {
                            // Left-click: Cycle forwards: 30 -> 60 -> 120 -> 144 -> 240 -> Unlimited (0) -> 30
                            settings.fps_limit = match settings.fps_limit {
                                30 => 60,
                                60 => 120,
                                120 => 144,
                                144 => 240,
                                240 => 0,
                                _ => 30,
                            };
                        }

                        // Update button text
                        for child in children.iter() {
                            if let Ok(mut text) = text_query.get_mut(child) {
                                **text = if settings.fps_limit == 0 {
                                    "FPS Limit: Unlimited".to_string()
                                } else {
                                    format!("FPS Limit: {}", settings.fps_limit)
                                };
                            }
                        }
                    }
                } else if dev_btn.is_some() {
                    // Toggle Dev Mode
                    settings.dev_mode = !settings.dev_mode;

                    info!("🔧 Dev Mode {}", if settings.dev_mode { "ENABLED" } else { "DISABLED" });

                    // Update button text
                    for child in children.iter() {
                        if let Ok(mut text) = text_query.get_mut(child) {
                            **text = if settings.dev_mode { "Dev Mode: ON" } else { "Dev Mode: OFF" }.to_string();
                        }
                    }
                } else if back_btn.is_some() {
                    next_state.set(GameState::Paused);
                }
            }
            Interaction::Hovered => {
                // Don't highlight if it's the FPS limit button and VSync is enabled
                if fps_limit_btn.is_some() && settings.vsync_enabled {
                    // Keep it grayed out
                } else {
                    *color = BackgroundColor(Color::srgb(0.6, 0.6, 0.8));
                }
            }
            Interaction::None => {
                if vsync_btn.is_some() || fps_btn.is_some() {
                    *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.6));
                } else if fps_limit_btn.is_some() {
                    // Always reflect current VSync state
                    *color = BackgroundColor(if settings.vsync_enabled {
                        Color::srgb(0.2, 0.2, 0.2) // Keep grayed out when VSync is on
                    } else {
                        Color::srgb(0.3, 0.6, 0.3)
                    });
                } else if dev_btn.is_some() {
                    *color = BackgroundColor(Color::srgb(0.6, 0.3, 0.3));
                } else if back_btn.is_some() {
                    *color = BackgroundColor(Color::srgb(0.5, 0.5, 0.5));
                }
            }
        }
    }

    // Update FPS limit button appearance based on current VSync state
    if let Ok((mut fps_bg, fps_children)) = param_set.p1().single_mut() {
        *fps_bg = BackgroundColor(if settings.vsync_enabled {
            Color::srgb(0.2, 0.2, 0.2)
        } else {
            Color::srgb(0.3, 0.6, 0.3)
        });

        // Update text color
        for child in fps_children.iter() {
            if let Ok(mut text_color) = text_color_query.get_mut(child) {
                *text_color = TextColor(if settings.vsync_enabled {
                    Color::srgb(0.5, 0.5, 0.5)
                } else {
                    Color::WHITE
                });
            }
        }
    }
}
