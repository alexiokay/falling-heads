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
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(10.0),
                padding: UiRect::all(Val::Px(15.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            CommandsPanel,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("COMMANDS"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.8, 0.2)),
                Node {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
            ));

            // Power commands (just the command names, no descriptions)
            for cmd in [
                "!TNT", "!ARROW", "!LASER", "!THUNDER",
                "!MIRROR", "!MAGNET", "!VILLAGERS",
                "!DRAGON", "!LOOTBOX",
            ] {
                parent.spawn((
                    Text::new(cmd),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgb(1.0, 1.0, 0.5)),
                ));
            }
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
