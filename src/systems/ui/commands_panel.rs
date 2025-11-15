use bevy::prelude::*;
use crate::states::GameState;

#[derive(Component)]
pub struct CommandsPanel;

pub struct CommandsPanelPlugin;

impl Plugin for CommandsPanelPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Playing), setup_commands_panel)
            .add_systems(OnExit(GameState::Playing), cleanup_commands_panel);

        info!("CommandsPanelPlugin loaded");
    }
}

/// Setup commands help panel UI (always visible in top-right)
fn setup_commands_panel(mut commands: Commands) {
    // Main container (transparent, just for positioning)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(50.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(0.0),  // No gap between blue and black boxes
                ..default()
            },
            CommandsPanel,
        ))
        .with_children(|parent| {
            // Blue header container with "COMMANDS" title
            parent.spawn((
                Node {
                    padding: UiRect::all(Val::Px(10.0)),
                    width: Val::Px(150.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.4, 0.8)),  // Blue background
            ))
            .with_children(|header| {
                header.spawn((
                    Text::new("COMMANDS"),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),  // White title text
                ));
            });

            // Semi-transparent black container with command list
            parent.spawn((
                Node {
                    padding: UiRect::all(Val::Px(12.0)),
                    width: Val::Px(150.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(5.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),  // Semi-transparent black
            ))
            .with_children(|list| {
                // Power commands (just the command names)
                for cmd in [
                    "!TNT", "!ARROW", "!LASER", "!THUNDER",
                    "!MIRROR", "!MAGNET", "!VILLAGERS",
                    "!DRAGON", "!LOOTBOX", "!RANDOMBOX",
                ] {
                    list.spawn((
                        Text::new(cmd),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(Color::WHITE),  // White text
                    ));
                }
            });
        });
}

/// Cleanup commands panel
fn cleanup_commands_panel(
    mut commands: Commands,
    panel_query: Query<Entity, With<CommandsPanel>>,
) {
    for entity in panel_query.iter() {
        commands.entity(entity).despawn();
    }
}
