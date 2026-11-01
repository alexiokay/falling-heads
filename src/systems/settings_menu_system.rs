use crate::{components::settings_menu::*, resources::settings::GameSettings, states::GameState};
use bevy::ecs::system::ParamSet;
use bevy::{
    picking::{Pickable, hover::PickingInteraction},
    prelude::*,
    ui::RelativeCursorPosition,
    ui_widgets::Button,
    window::{PresentMode, PrimaryWindow},
};

pub fn menu_button(parent: &mut ChildSpawnerCommands, label: &str, marker: impl Bundle) {
    parent
        .spawn((
            marker,
            SettingsControl,
            Button,
            PickingInteraction::default(),
            Node {
                width: Val::Percent(100.),
                min_height: Val::Px(34.),
                padding: UiRect::all(Val::Px(7.)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.10, 0.16, 0.19)),
        ))
        .with_child((
            Pickable::IGNORE,
            ToggleButtonText,
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(16.),
                ..default()
            },
        ));
}
fn volume(parent: &mut ChildSpawnerCommands, label: &str, value: f32, marker: impl Bundle) {
    parent
        .spawn((
            marker,
            Node {
                width: Val::Percent(100.),
                height: Val::Px(66.),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                Pickable::IGNORE,
                VolumeSliderLabel,
                Text::new(format!("{label}: {:.0}%", value * 100.)),
                TextFont {
                    font_size: FontSize::Px(16.),
                    ..default()
                },
            ));
            p.spawn((
                VolumeSliderBar,
                PickingInteraction::default(),
                RelativeCursorPosition::default(),
                Node {
                    width: Val::Percent(100.),
                    height: Val::Px(16.),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.09, 0.11, 0.14)),
            ))
            .with_children(|p| {
                p.spawn((
                    Pickable::IGNORE,
                    Node {
                        width: Val::Percent(value * 100.),
                        height: Val::Percent(100.),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.22, 0.55, 0.61)),
                ));
                p.spawn((
                    Pickable::IGNORE,
                    VolumeSliderHandle,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(value * 100.),
                        width: Val::Px(5.),
                        height: Val::Percent(100.),
                        ..default()
                    },
                    BackgroundColor(Color::WHITE),
                ));
            });
        });
}
pub fn setup_settings_menu(mut commands: Commands, settings: Res<GameSettings>) {
    commands.insert_resource(SettingsNavigation::default());
    commands
        .spawn((
            SettingsMenuRoot,
            GlobalZIndex(200),
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.02, 0.03, 0.65)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(92.),
                    max_width: Val::Px(520.),
                    max_height: Val::Percent(96.),
                    padding: UiRect::all(Val::Px(18.)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.035, 0.055, 0.065)),
            ))
            .with_children(|p| {
                p.spawn((
                    Pickable::IGNORE,
                    Text::new("SETTINGS"),
                    TextFont {
                        font_size: FontSize::Px(26.),
                        ..default()
                    },
                ));
                p.spawn(Node {
                    width: Val::Percent(100.),
                    column_gap: Val::Px(8.),
                    ..default()
                })
                .with_children(|tabs| {
                    for (page, label) in [
                        (SettingsPage::General, "General"),
                        (SettingsPage::Graphics, "Graphics"),
                        (SettingsPage::GameModes, "Game modes"),
                        (SettingsPage::Character, "Character"),
                        (SettingsPage::Theme, "Theme"),
                    ] {
                        tabs.spawn(Node {
                            width: Val::Percent(20.),
                            ..default()
                        })
                        .with_children(|p| menu_button(p, label, SettingsTab(page)));
                    }
                });
                p.spawn((
                    SettingsPageRoot(SettingsPage::General),
                    Node {
                        width: Val::Percent(100.),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.),
                        ..default()
                    },
                ))
                .with_children(|p| {
                    menu_button(
                        p,
                        if settings.vsync_enabled {
                            "VSync: ON"
                        } else {
                            "VSync: OFF"
                        },
                        VsyncToggleButton,
                    );
                    menu_button(
                        p,
                        if settings.show_fps {
                            "FPS Counter: ON"
                        } else {
                            "FPS Counter: OFF"
                        },
                        FpsToggleButton,
                    );
                    menu_button(p, &fps_label(settings.fps_limit), FpsLimitButton);
                    volume(
                        p,
                        "Effects Volume",
                        settings.effects_volume,
                        EffectsVolumeSlider,
                    );
                    volume(p, "Music Volume", settings.music_volume, MusicVolumeSlider);
                    menu_button(
                        p,
                        if settings.dev_mode {
                            "Dev Mode: ON"
                        } else {
                            "Dev Mode: OFF"
                        },
                        DevModeToggleButton,
                    );
                });
                p.spawn((
                    SettingsPageRoot(SettingsPage::Graphics),
                    Node {
                        display: Display::None,
                        width: Val::Percent(100.),
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(6.),
                        row_gap: Val::Px(4.),
                        ..default()
                    },
                ));
                // Filled by `game_modes::fill_game_modes_page`.
                p.spawn((
                    SettingsPageRoot(SettingsPage::GameModes),
                    Node {
                        display: Display::None,
                        width: Val::Percent(100.),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.),
                        ..default()
                    },
                ));
                // Filled by `character_select::fill_character_page`.
                p.spawn((
                    SettingsPageRoot(SettingsPage::Character),
                    Node {
                        display: Display::None,
                        width: Val::Percent(100.),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.),
                        ..default()
                    },
                ));
                // Filled by `asset_packs::fill_theme_page`.
                p.spawn((
                    SettingsPageRoot(SettingsPage::Theme),
                    Node {
                        display: Display::None,
                        width: Val::Percent(100.),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.),
                        ..default()
                    },
                ));
                menu_button(p, "Back to pause menu", BackToMenuButton);
            });
        });
}
fn fps_label(fps: u32) -> String {
    if fps == 0 {
        "FPS Limit: Unlimited".into()
    } else {
        format!("FPS Limit: {fps}")
    }
}
pub fn cleanup_settings_menu(mut commands: Commands, roots: Query<Entity, With<SettingsMenuRoot>>) {
    for e in &roots {
        commands.entity(e).despawn();
    }
}
pub fn handle_settings_buttons(
    buttons: Query<
        (
            &PickingInteraction,
            Option<&VsyncToggleButton>,
            Option<&FpsToggleButton>,
            Option<&FpsLimitButton>,
            Option<&DevModeToggleButton>,
            Option<&BackToMenuButton>,
            Option<&SettingsTab>,
        ),
        (Changed<PickingInteraction>, With<SettingsControl>),
    >,
    mut settings: ResMut<GameSettings>,
    mut nav: ResMut<SettingsNavigation>,
    mut state: ResMut<NextState<GameState>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    for (i, vsync, fps, limit, dev, back, tab) in &buttons {
        if *i != PickingInteraction::Pressed {
            continue;
        }
        if let Some(tab) = tab {
            nav.0 = tab.0;
        } else if back.is_some() {
            state.set(GameState::Paused);
        } else if vsync.is_some() {
            settings.vsync_enabled = !settings.vsync_enabled;
            for mut w in &mut windows {
                w.present_mode = if settings.vsync_enabled {
                    PresentMode::AutoVsync
                } else {
                    PresentMode::AutoNoVsync
                };
            }
        } else if fps.is_some() {
            settings.show_fps = !settings.show_fps;
        } else if limit.is_some() && !settings.vsync_enabled {
            settings.fps_limit = match settings.fps_limit {
                30 => 60,
                60 => 120,
                120 => 144,
                144 => 240,
                240 => 0,
                _ => 30,
            };
        } else if dev.is_some() {
            settings.dev_mode = !settings.dev_mode;
        }
    }
}
pub fn settings_layout_and_style(
    nav: Res<SettingsNavigation>,
    settings: Res<GameSettings>,
    mut pages: Query<(&SettingsPageRoot, &mut Node)>,
    mut buttons: Query<
        (
            &PickingInteraction,
            &mut BackgroundColor,
            &Children,
            Option<&SettingsTab>,
            Has<FpsLimitButton>,
            Has<VsyncToggleButton>,
            Has<FpsToggleButton>,
            Has<DevModeToggleButton>,
        ),
        With<SettingsControl>,
    >,
    mut texts: Query<&mut Text, With<ToggleButtonText>>,
) {
    for (page, mut node) in &mut pages {
        node.display = if page.0 == nav.0 {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (interaction, mut color, children, tab, limit, vsync, fps, dev) in &mut buttons {
        let disabled = limit && settings.vsync_enabled;
        color.0 = if disabled {
            Color::srgb(0.08, 0.09, 0.10)
        } else if *interaction == PickingInteraction::Pressed {
            Color::srgb(0.20, 0.42, 0.46)
        } else if *interaction == PickingInteraction::Hovered {
            Color::srgb(0.16, 0.28, 0.31)
        } else if tab.is_some_and(|t| t.0 == nav.0) {
            Color::srgb(0.13, 0.32, 0.36)
        } else {
            Color::srgb(0.10, 0.16, 0.19)
        };
        let label = if vsync {
            Some(format!(
                "VSync: {}",
                if settings.vsync_enabled { "ON" } else { "OFF" }
            ))
        } else if fps {
            Some(format!(
                "FPS Counter: {}",
                if settings.show_fps { "ON" } else { "OFF" }
            ))
        } else if limit {
            Some(fps_label(settings.fps_limit))
        } else if dev {
            Some(format!(
                "Dev Mode: {}",
                if settings.dev_mode { "ON" } else { "OFF" }
            ))
        } else {
            None
        };
        if let Some(label) = label {
            for child in children {
                if let Ok(mut text) = texts.get_mut(*child) {
                    if **text != label {
                        **text = label.clone();
                    }
                }
            }
        }
    }
}

/// Handle volume slider interactions using RelativeCursorPosition
pub fn handle_volume_sliders(
    mut settings: ResMut<GameSettings>,
    container_query: Query<
        (
            &Children,
            Option<&EffectsVolumeSlider>,
            Option<&MusicVolumeSlider>,
        ),
        Or<(With<EffectsVolumeSlider>, With<MusicVolumeSlider>)>,
    >,
    slider_bar_query: Query<
        (&PickingInteraction, &RelativeCursorPosition, &Children),
        With<VolumeSliderBar>,
    >,
    mut label_query: Query<&mut Text, With<VolumeSliderLabel>>,
    mut param_set: ParamSet<(
        Query<
            &mut Node,
            (
                Without<VolumeSliderHandle>,
                Without<VolumeSliderBar>,
                Without<VolumeSliderLabel>,
            ),
        >,
        Query<
            &mut Node,
            (
                With<VolumeSliderHandle>,
                Without<VolumeSliderBar>,
                Without<VolumeSliderLabel>,
            ),
        >,
    )>,
    mouse_button: Res<ButtonInput<MouseButton>>,
) {
    // Only respond when mouse is pressed
    if !mouse_button.pressed(MouseButton::Left) {
        return;
    }

    // Check each slider container
    for (container_children, effects_marker, music_marker) in container_query.iter() {
        // Find the slider bar child
        for child in container_children.iter() {
            if let Ok((interaction, relative_cursor, bar_children)) = slider_bar_query.get(child) {
                // Check if mouse is over the slider bar and pressed
                if matches!(
                    interaction,
                    PickingInteraction::Pressed | PickingInteraction::Hovered
                ) {
                    // Get the normalized cursor position (relative to the bar)
                    if let Some(normalized_pos) = relative_cursor.normalized {
                        // RelativeCursorPosition uses range (-0.5, -0.5) to (0.5, 0.5)
                        // Convert to 0.0 to 1.0 range for volume
                        let volume = (normalized_pos.x + 0.5).clamp(0.0, 1.0);

                        // Update settings
                        if effects_marker.is_some() {
                            settings.set_effects_volume(volume);
                            info!("Effects volume: {}%", (volume * 100.0) as i32);
                        } else if music_marker.is_some() {
                            settings.set_music_volume(volume);
                            info!("Music volume: {}%", (volume * 100.0) as i32);
                        }

                        // Update label
                        for label_child in container_children.iter() {
                            if let Ok(mut text) = label_query.get_mut(label_child) {
                                let label = if effects_marker.is_some() {
                                    "Effects Volume"
                                } else {
                                    "Music Volume"
                                };
                                **text = format!("{}: {}%", label, (volume * 100.0) as i32);
                            }
                        }

                        // Update filled bar and handle
                        for bar_child in bar_children.iter() {
                            // Update filled portion width
                            if let Ok(mut filled_node) = param_set.p0().get_mut(bar_child) {
                                filled_node.width = Val::Percent(volume * 100.0);
                            }
                            // Update handle position
                            if let Ok(mut handle_node) = param_set.p1().get_mut(bar_child) {
                                handle_node.left = Val::Percent(volume * 100.0);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graphics_navigation_and_hover_reset_are_owned_by_settings() {
        let mut app = App::new();
        app.insert_resource(SettingsNavigation::default())
            .insert_resource(GameSettings::new())
            .add_systems(Update, settings_layout_and_style);
        let general = app
            .world_mut()
            .spawn((SettingsPageRoot(SettingsPage::General), Node::default()))
            .id();
        let graphics = app
            .world_mut()
            .spawn((SettingsPageRoot(SettingsPage::Graphics), Node::default()))
            .id();
        let button = app
            .world_mut()
            .spawn((
                SettingsControl,
                PickingInteraction::Hovered,
                BackgroundColor(Color::BLACK),
            ))
            .with_child((Text::new("Graphics control"), ToggleButtonText))
            .id();
        let other = app
            .world_mut()
            .spawn((
                Button,
                PickingInteraction::Hovered,
                BackgroundColor(Color::WHITE),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Node>(graphics).unwrap().display,
            Display::None
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(other).unwrap().0,
            Color::WHITE
        );
        *app.world_mut()
            .get_mut::<PickingInteraction>(button)
            .unwrap() = PickingInteraction::None;
        app.world_mut().resource_mut::<SettingsNavigation>().0 = SettingsPage::Graphics;
        app.update();
        assert_eq!(
            app.world().get::<Node>(general).unwrap().display,
            Display::None
        );
        assert_eq!(
            app.world().get::<Node>(graphics).unwrap().display,
            Display::Flex
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(button).unwrap().0,
            Color::srgb(0.10, 0.16, 0.19)
        );
    }
}
