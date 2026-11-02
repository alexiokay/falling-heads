use crate::states::GameState;
use bevy::prelude::*;

#[derive(Component)]
pub struct CommandsPanel;

pub struct CommandsPanelPlugin;

impl Plugin for CommandsPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_commands_panel)
            .add_systems(OnExit(GameState::Playing), cleanup_commands_panel)
            .add_systems(
                Update,
                sync_commands_visibility.run_if(in_state(GameState::Playing)),
            );

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
                row_gap: Val::Px(0.0), // No gap between blue and black boxes
                ..default()
            },
            CommandsPanel,
        ))
        .with_children(|parent| {
            // Blue header container with "COMMANDS" title
            parent
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(10.0)),
                        width: Val::Px(260.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.4, 0.8)), // Blue background
                ))
                .with_children(|header| {
                    header.spawn((
                        Text::new("COMMANDS"),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::WHITE), // White title text
                    ));
                });

            // Semi-transparent black container with command list
            parent
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(8.0)),
                        width: Val::Px(260.0),
                        // Two columns so the longer list stays compact.
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        row_gap: Val::Px(3.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)), // Semi-transparent black
                ))
                .with_children(|list| {
                    // Power commands (just the command names)
                    for cmd in [
                        "!TNT",
                        "!ARROW",
                        "!LASER",
                        "!BURST",
                        "!THUNDER",
                        "!MIRROR",
                        "!MAGNET",
                        "!LAUNCHER",
                        "!TELEPORT",
                        "!WATERBUCKET",
                        "!LAVABUCKET",
                        "!VILLAGERS",
                        "!LOOTBOX",
                        "!RANDOMBOX", //"!DRAGON",
                        "!STYLE",
                        "!FEATHER",
                        "!ANVIL",
                        "!GOLEM",
                        "!PEARL",
                        "!QUAKE",
                        "!SHRINK",
                        "!WIND",
                        "!BLACKHOLE",
                        "!ICEAGE",
                        "!BEES",
                        "!BOLT",
                        "!FUSS",
                        "!PRISON",
                    ] {
                        list.spawn((
                            Text::new(cmd),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(Color::WHITE), // White text
                            Node {
                                width: Val::Percent(50.0),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                        ));
                    }
                });
        });
}

/// Cleanup commands panel
fn cleanup_commands_panel(mut commands: Commands, panel_query: Query<Entity, With<CommandsPanel>>) {
    for entity in panel_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn sync_commands_visibility(
    settings: Res<crate::resources::settings::GameSettings>,
    mut roots: Query<&mut Node, With<CommandsPanel>>,
) {
    for mut node in &mut roots {
        node.display = if settings.dev_mode {
            Display::None
        } else {
            Display::Flex
        };
    }
}
