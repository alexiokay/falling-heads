use crate::components::pause_menu::*;
use crate::states::GameState;
use bevy::prelude::*;
use bevy::{
    picking::{Pickable, hover::PickingInteraction},
    ui_widgets::Button,
};

pub fn toggle_pause(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match current_state.get() {
            GameState::Playing => next_state.set(GameState::Paused),
            GameState::Paused => next_state.set(GameState::Playing),
            GameState::Settings => next_state.set(GameState::Paused), // Go back to pause menu
            _ => {}
        }
    }
}

pub fn setup_pause_menu(mut commands: Commands) {
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
            BackgroundColor(Color::srgba(0.015, 0.02, 0.03, 0.45)),
            GlobalZIndex(200),
            PauseMenuRoot,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Pickable::IGNORE,
                Text::new("PAUSED"),
                TextFont {
                    font_size: FontSize::Px(80.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::all(Val::Px(20.0)),
                    ..default()
                },
            ));

            // Resume Button
            parent
                .spawn((
                    Button,
                    PickingInteraction::default(),
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(65.0),
                        margin: UiRect::all(Val::Px(10.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.6, 0.2)),
                    ResumeButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Pickable::IGNORE,
                        Text::new("Resume"),
                        TextFont {
                            font_size: FontSize::Px(32.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Settings Button
            parent
                .spawn((
                    Button,
                    PickingInteraction::default(),
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(65.0),
                        margin: UiRect::all(Val::Px(10.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.6)),
                    SettingsButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Pickable::IGNORE,
                        Text::new("Settings"),
                        TextFont {
                            font_size: FontSize::Px(32.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Quit Button
            parent
                .spawn((
                    Button,
                    PickingInteraction::default(),
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(65.0),
                        margin: UiRect::all(Val::Px(10.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.6, 0.2, 0.2)),
                    QuitButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Pickable::IGNORE,
                        Text::new("Quit"),
                        TextFont {
                            font_size: FontSize::Px(32.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

pub fn cleanup_pause_menu(mut commands: Commands, query: Query<Entity, With<PauseMenuRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn handle_menu_buttons(
    mut interaction_query: Query<
        (
            Entity,
            &PickingInteraction,
            &mut BackgroundColor,
            Option<&ResumeButton>,
            Option<&SettingsButton>,
            Option<&QuitButton>,
        ),
        (Changed<PickingInteraction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<bevy::app::AppExit>,
    mut presses: Local<crate::systems::settings_menu_system::PressTracker>,
) {
    for (entity, interaction, mut color, resume, settings, quit) in &mut interaction_query {
        // Only a click that started on the button (not a held drag onto it).
        let clicked = presses.pressed(entity, *interaction);
        match *interaction {
            PickingInteraction::Pressed if !clicked => {}
            PickingInteraction::Pressed => {
                if resume.is_some() {
                    next_state.set(GameState::Playing);
                } else if settings.is_some() {
                    next_state.set(GameState::Settings);
                } else if quit.is_some() {
                    exit.write(bevy::app::AppExit::Success);
                }
            }
            PickingInteraction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.7, 0.7, 0.7));
            }
            PickingInteraction::None => {
                if resume.is_some() {
                    *color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2));
                } else if settings.is_some() {
                    *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.6));
                } else if quit.is_some() {
                    *color = BackgroundColor(Color::srgb(0.6, 0.2, 0.2));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hovering_menu_backdrop_must_not_paint_it_opaque() {
        let mut app = App::new();
        app.insert_resource(NextState::<GameState>::default())
            .add_message::<AppExit>()
            .add_systems(Update, handle_menu_buttons);
        let color = Color::srgba(0.015, 0.02, 0.03, 0.45);
        let root = app
            .world_mut()
            .spawn((
                PauseMenuRoot,
                PickingInteraction::Hovered,
                BackgroundColor(color),
            ))
            .id();
        let button = app
            .world_mut()
            .spawn((
                Button,
                ResumeButton,
                PickingInteraction::Hovered,
                BackgroundColor(Color::BLACK),
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<BackgroundColor>(root).unwrap().0, color);
        assert_eq!(
            app.world().get::<BackgroundColor>(button).unwrap().0,
            Color::srgb(0.7, 0.7, 0.7)
        );
    }
}
