use crate::components::experience::*;
use crate::states::GameState;
use bevy::prelude::*;

pub struct XpBarUIPlugin;

impl Plugin for XpBarUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_xp_bar_ui)
            .add_systems(OnExit(GameState::Playing), despawn_xp_bar_ui);

        info!("XpBarUIPlugin loaded");
    }
}

/// Spawn the XP bar UI at the bottom of the screen
fn spawn_xp_bar_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Container for XP bar (bottom center of screen)
    commands
        .spawn((
            XpBar,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Vh(2.0),
                left: Val::Percent(50.0),
                width: Val::Px(600.0), // Fixed 600px width (works on all aspect ratios)
                max_width: Val::Percent(90.0), // But max 90% of screen width on narrow displays
                height: Val::Px(40.0), // Fixed 40px height
                // Center horizontally
                margin: UiRect {
                    left: Val::Px(-300.0), // Half of width to center
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|parent| {
            // Background bar
            parent.spawn((
                ImageNode::new(
                    asset_server.load("textures/effects/exp/experience_bar_background.png"),
                ),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
            ));

            // Container for progress bar (clips the overflow)
            parent
                .spawn((
                    Node {
                        width: Val::Percent(0.0), // This will be updated by experience_system
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    XpBarProgress,
                ))
                .with_children(|progress_parent| {
                    // Progress bar image (always full width, but parent clips it)
                    progress_parent.spawn((
                        ImageNode::new(
                            asset_server.load("textures/effects/exp/experience_bar_progress.png"),
                        ),
                        Node {
                            width: Val::Px(600.0), // Match parent width (600px)
                            height: Val::Percent(100.0),
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                    ));
                });

            // Level text (on top of bar)
            parent.spawn((
                LevelText,
                Text::new("Level 1 (0/200)"),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0), // Center vertically
                    margin: UiRect {
                        left: Val::Px(-70.0), // Center horizontally
                        top: Val::Px(-10.0),  // Offset by half font size (20/2 = 10)
                        ..default()
                    },
                    ..default()
                },
            ));
        });

    info!("XP bar UI spawned");
}

/// Despawn XP bar UI when leaving Playing state
fn despawn_xp_bar_ui(mut commands: Commands, xp_bar_query: Query<Entity, With<XpBar>>) {
    for entity in xp_bar_query.iter() {
        commands.entity(entity).despawn();
    }
}
