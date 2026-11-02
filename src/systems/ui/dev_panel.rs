use crate::components::block::BiomeType;
use crate::components::dev_panel::*;
use crate::components::hero::Hero;
use crate::components::weather::{ActiveWeather, WeatherType};
use crate::plugins::external_api_plugin::get_command_queue;
use crate::resources::command_queue::PowerCommandType;
use crate::resources::settings::GameSettings;
use crate::states::GameState;
use bevy::prelude::*;
use bevy::{
    picking::{Pickable, hover::PickingInteraction},
    ui_widgets::Button,
};

use crate::systems::biome_teleport::BiomeTeleport;
pub struct DevPanelPlugin;

impl Plugin for DevPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_dev_panel)
            .add_systems(OnExit(GameState::Playing), cleanup_dev_panel)
            .add_systems(
                Update,
                (
                    toggle_dev_panel_with_tab,
                    toggle_dev_panel_visibility,
                    handle_contact_toggle,
                    handle_weather_buttons,
                    handle_power_buttons,
                    handle_biome_teleport_buttons,
                    update_dev_panel_info,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );

        info!("DevPanelPlugin loaded");
    }
}

/// Compact dev panel: sections of small buttons in wrapping grids so the whole
/// panel fits the 720x1280 window (it used to be one 36-button column).
fn setup_dev_panel(mut commands: Commands, settings: Res<GameSettings>) {
    commands
        .spawn((
            Node {
                display: if settings.dev_mode {
                    Display::Flex
                } else {
                    Display::None
                },
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(48.0), // Below the FPS counter
                width: Val::Px(300.0),
                max_height: Val::Percent(96.0),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.02, 0.025, 0.94)),
            GlobalZIndex(100),
            DevPanelRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Pickable::IGNORE,
                Text::new("DEV CONTROLS  (Tab hides)"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.5, 0.5)),
            ));

            parent
                .spawn((
                    Button,
                    ContactToggle,
                    Node {
                        height: Val::Px(24.),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.25, 0.25, 0.3)),
                ))
                .with_children(|p| {
                    p.spawn((
                        Pickable::IGNORE,
                        ContactLabel,
                        Text::new("Final hit: BOUNCE (default)"),
                        TextFont {
                            font_size: FontSize::Px(12.),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            section_title(parent, "Hero style (C cycles)");
            button_grid(parent, |grid| {
                for style in crate::components::play_style::PlayStyle::ALL {
                    grid_button(
                        grid,
                        style.name(),
                        4,
                        HERO_STYLE_COLOR,
                        crate::systems::play_styles::HeroStyleButton(style),
                    );
                }
            });

            section_title(parent, "Weather");
            button_grid(parent, |grid| {
                for (weather_type, label) in [
                    (WeatherType::Clear, "Clear"),
                    (WeatherType::Rain, "Rain"),
                    (WeatherType::Snow, "Snow"),
                    (WeatherType::Sandstorm, "Sandstorm"),
                    (WeatherType::AshStorm, "Ash Storm"),
                    (WeatherType::FireRain, "Fire Rain"),
                ] {
                    grid_button(
                        grid,
                        label,
                        3,
                        Color::srgb(0.3, 0.3, 0.5),
                        WeatherControlButton { weather_type },
                    );
                }
            });
            parent.spawn((
                Pickable::IGNORE,
                Text::new("Current: Clear"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                ParameterLabel,
            ));

            section_title(parent, "Powers");
            button_grid(parent, |grid| {
                for (power_type, label) in [
                    (PowerCommandType::Tnt, "TNT"),
                    (PowerCommandType::Fast, "FAST"),
                    (PowerCommandType::Slow, "SLOW"),
                    (PowerCommandType::Arrow, "ARROW"),
                    (PowerCommandType::Laser, "LASER"),
                    (PowerCommandType::LaserBurst, "BURST"),
                    (PowerCommandType::Mirror, "MIRROR"),
                    (PowerCommandType::Magnet, "MAGNET"),
                    (PowerCommandType::Villagers, "VILLAGERS"),
                    (PowerCommandType::Ghast, "GHAST"),
                    (PowerCommandType::Dragon, "DRAGON"),
                    (PowerCommandType::Thunder, "THUNDER"),
                    (PowerCommandType::Lootbox, "LOOTBOX"),
                    (PowerCommandType::RandomBox, "RANDOMBOX"),
                    (PowerCommandType::Teleport, "TELEPORT"),
                    (PowerCommandType::Launcher, "LAUNCHER"),
                    (PowerCommandType::Rift, "RIFT (F8)"),
                    (PowerCommandType::WaterBucket, "WATER"),
                    (PowerCommandType::LavaBucket, "LAVA"),
                    (PowerCommandType::GrowBuff, "GROW"),
                    (PowerCommandType::Feather, "FEATHER"),
                    (PowerCommandType::Anvil, "ANVIL"),
                    (PowerCommandType::Golem, "GOLEM"),
                    (PowerCommandType::Pearl, "PEARL"),
                    (PowerCommandType::Quake, "QUAKE"),
                    (PowerCommandType::Shrink, "SHRINK"),
                    (PowerCommandType::Wind, "WIND"),
                    (PowerCommandType::BlackHole, "BLACK HOLE"),
                    (PowerCommandType::IceAge, "ICE AGE"),
                    (PowerCommandType::Bees, "BEES"),
                    (PowerCommandType::Bolt, "BOLT"),
                    (PowerCommandType::Fuss, "FUSS"),
                    (PowerCommandType::Prison, "PRISON"),
                ] {
                    grid_button(
                        grid,
                        label,
                        3,
                        Color::srgb(0.5, 0.3, 0.3),
                        PowerButton { power_type },
                    );
                }
            });

            section_title(parent, "Biome teleport");
            button_grid(parent, |grid| {
                for (biome_type, label) in [
                    (BiomeType::Plains, "Plains"),
                    (BiomeType::Forest, "Forest"),
                    (BiomeType::Desert, "Desert"),
                    (BiomeType::Mountains, "Mountains"),
                    (BiomeType::Underground, "Underground"),
                    (BiomeType::Nether, "Nether"),
                    (BiomeType::End, "End"),
                    (BiomeType::Iceland, "Iceland"),
                ] {
                    grid_button(
                        grid,
                        label,
                        3,
                        Color::srgb(0.3, 0.5, 0.3),
                        BiomeTeleportButton { biome_type },
                    );
                }
            });
        });
}

/// Base colour of the hero style buttons (the active one is highlighted).
pub(crate) const HERO_STYLE_COLOR: Color = Color::srgb(0.35, 0.3, 0.15);

fn section_title(parent: &mut ChildSpawnerCommands, title: &str) {
    parent.spawn((
        Pickable::IGNORE,
        Text::new(title),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.85)),
        Node {
            margin: UiRect::top(Val::Px(6.0)),
            ..default()
        },
    ));
}

fn button_grid(parent: &mut ChildSpawnerCommands, buttons: impl FnOnce(&mut ChildSpawnerCommands)) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(4.0),
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(buttons);
}

/// One small button taking 1/`columns` of the row.
fn grid_button(
    grid: &mut ChildSpawnerCommands,
    label: &str,
    columns: u16,
    color: Color,
    marker: impl Bundle,
) {
    // Leave room for the 4px gaps between columns.
    let width = 100.0 / columns as f32 - 2.0;
    grid.spawn((
        Button,
        PickingInteraction::default(),
        Node {
            width: Val::Percent(width),
            height: Val::Px(24.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(color),
        marker,
    ))
    .with_children(|button| {
        button.spawn((
            Pickable::IGNORE,
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    });
}

fn cleanup_dev_panel(mut commands: Commands, query: Query<Entity, With<DevPanelRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Toggle dev panel visibility based on dev_mode setting
fn toggle_dev_panel_visibility(
    settings: Res<GameSettings>,
    mut query: Query<&mut Node, With<DevPanelRoot>>,
) {
    if settings.is_changed() {
        for mut node in query.iter_mut() {
            node.display = if settings.dev_mode {
                Display::Flex
            } else {
                Display::None
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
        info!(
            "🔧 Dev panel toggled: {}",
            if settings.dev_mode { "ON" } else { "OFF" }
        );
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    #[test]
    fn dev_panel_restores_enabled_setting_and_tab_updates_layout_same_frame() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(GameSettings {
                dev_mode: true,
                ..GameSettings::new()
            })
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Startup, setup_dev_panel)
            .add_systems(
                Update,
                (toggle_dev_panel_with_tab, toggle_dev_panel_visibility).chain(),
            );
        app.update();
        let root = app
            .world_mut()
            .query_filtered::<Entity, With<DevPanelRoot>>()
            .single(app.world())
            .unwrap();
        assert_eq!(
            app.world().get::<Node>(root).unwrap().display,
            Display::Flex
        );
        assert_eq!(app.world().get::<GlobalZIndex>(root).unwrap().0, 100);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        assert_eq!(
            app.world().get::<Node>(root).unwrap().display,
            Display::None
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        assert_eq!(
            app.world().get::<Node>(root).unwrap().display,
            Display::Flex
        );
    }
}

/// Handle weather control button clicks
#[cfg(test)]
mod weather_button_tests {
    use super::*;
    #[test]
    fn fire_rain_button_starts_active_weather() {
        let mut app = App::new();
        app.init_resource::<ActiveWeather>()
            .add_systems(Update, handle_weather_buttons);
        app.world_mut().spawn((
            PickingInteraction::Pressed,
            BackgroundColor(Color::BLACK),
            WeatherControlButton {
                weather_type: WeatherType::FireRain,
            },
        ));
        app.update();
        let w = app.world().resource::<ActiveWeather>();
        assert_eq!(w.weather_type, WeatherType::FireRain);
        assert_eq!(w.intensity, 1.);
        assert!(w.duration_remaining > 100.);
    }
}
fn handle_weather_buttons(
    mut interaction_query: Query<
        (
            &PickingInteraction,
            &mut BackgroundColor,
            &WeatherControlButton,
        ),
        Changed<PickingInteraction>,
    >,
    mut weather: ResMut<ActiveWeather>,
) {
    for (interaction, mut color, weather_btn) in &mut interaction_query {
        match *interaction {
            PickingInteraction::Pressed => {
                // Change weather immediately
                weather.weather_type = weather_btn.weather_type;
                weather.category = crate::components::weather::WeatherCategory::Normal;
                weather.intensity = if weather_btn.weather_type == WeatherType::Clear {
                    0.0
                } else {
                    1.0
                };
                weather.duration_remaining = 9999.0; // Keep it for a long time
                weather.transition_timer = 0.0;

                info!("🌦️ Dev: Weather changed to {:?}", weather_btn.weather_type);
                *color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2)); // Green flash
            }
            PickingInteraction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.5, 0.5, 0.7));
            }
            PickingInteraction::None => {
                *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.5));
            }
        }
    }
}

/// Handle power button clicks
fn handle_power_buttons(
    mut interaction_query: Query<
        (&PickingInteraction, &mut BackgroundColor, &PowerButton),
        Changed<PickingInteraction>,
    >,
) {
    for (interaction, mut color, power_btn) in &mut interaction_query {
        match *interaction {
            PickingInteraction::Pressed => {
                // Add power to global command queue (same as API)
                if let Some(queue) = get_command_queue() {
                    if let Ok(mut q) = queue.lock() {
                        q.push_back(crate::resources::command_queue::PowerCommand::new(
                            power_btn.power_type,
                            "DevPanel".to_string(),
                        ));
                        info!("⚡ Dev: Power triggered {:?}", power_btn.power_type);
                        *color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2)); // Green flash
                    }
                }
            }
            PickingInteraction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.7, 0.5, 0.5));
            }
            PickingInteraction::None => {
                *color = BackgroundColor(Color::srgb(0.5, 0.3, 0.3));
            }
        }
    }
}

/// Handle biome teleport button clicks
fn handle_biome_teleport_buttons(
    mut interaction_query: Query<
        (
            &PickingInteraction,
            &mut BackgroundColor,
            &BiomeTeleportButton,
        ),
        Changed<PickingInteraction>,
    >,
    hero_query: Query<&Transform, With<Hero>>,
    mut teleport: ResMut<BiomeTeleport>,
) {
    for (interaction, mut color, biome_btn) in &mut interaction_query {
        match *interaction {
            PickingInteraction::Pressed => {
                if let Ok(hero) = hero_query.single() {
                    let y = crate::utils::biome_utils::nearest_biome_y(
                        hero.translation.y,
                        biome_btn.biome_type,
                    );
                    teleport.target = Some(Vec3::new(hero.translation.x, y, hero.translation.z));
                    *color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2));
                }
            }
            PickingInteraction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.5, 0.7, 0.5));
            }
            PickingInteraction::None => {
                *color = BackgroundColor(Color::srgb(0.3, 0.5, 0.3));
            }
        }
    }
}
fn update_dev_panel_info(
    weather: Res<ActiveWeather>,
    teleport: Res<BiomeTeleport>,
    mut labels: Query<&mut Text, With<ParameterLabel>>,
) {
    if weather.is_changed() || teleport.is_changed() {
        for mut label in &mut labels {
            **label = if teleport.target.is_some() {
                "Loading destination…".into()
            } else {
                format!(
                    "Current: {:?} ({:.0}%)",
                    weather.weather_type,
                    weather.intensity * 100.0
                )
            };
        }
    }
}

#[derive(Component)]
struct ContactToggle;
#[derive(Component)]
struct ContactLabel;
fn handle_contact_toggle(
    q: Query<&PickingInteraction, (Changed<PickingInteraction>, With<ContactToggle>)>,
    mut tuning: ResMut<crate::plugins::physics_plugin::ContactTuning>,
    mut labels: Query<&mut Text, With<ContactLabel>>,
) {
    for interaction in &q {
        if *interaction == PickingInteraction::Pressed {
            tuning.break_through = !tuning.break_through;
        }
    }
    for mut label in &mut labels {
        **label = if tuning.break_through {
            "Final hit: PASS THROUGH"
        } else {
            "Final hit: BOUNCE (default)"
        }
        .into();
    }
}
