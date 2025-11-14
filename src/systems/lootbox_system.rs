use bevy::prelude::*;
use avian3d::prelude::*;
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::components::{
    hero::Hero,
    lootbox::{Lootbox, LootboxReward, LootboxUI, RouletteSpinner},
    block::Block,
    bundles::SpriteBundle3d,
    collision_layers::GameLayer,
};
use crate::resources::{
    asset_handles::BlockTextures,
    spatial_hash::SpatialHash,
};
use crate::states::GameState;
use crate::utils::constants::*;
use crate::systems::random_events::{RandomEventType, EventNotification};

/// Global flag to track if a lootbox is currently active (for preventing multiple simultaneous lootboxes)
static LOOTBOX_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Component wrapper for cleanup timer
#[derive(Component)]
struct CleanupTimer(Timer);

const LOOTBOX_SIZE: f32 = 64.0;
const LOOTBOX_INTERACTION_RANGE: f32 = 100.0;
const ITEM_WIDTH: f32 = 150.0;
const ITEM_HEIGHT: f32 = 100.0;


/// Check if a position is valid for lootbox (has space around it)
fn is_valid_lootbox_position(
    pos: Vec3,
    spatial_hash: &SpatialHash,
    block_query: &Query<&Transform, With<Block>>,
) -> bool {
    // Check if the position itself is free
    let nearby = spatial_hash.query_nearby(pos.x, pos.y);

    for entity in nearby {
        if let Ok(block_transform) = block_query.get(entity) {
            let distance = pos.distance(block_transform.translation);
            if distance < BLOCK_SIZE {
                return false; // Too close to a block
            }
        }
    }

    // Check sides and top are free (mini cave requirement)
    let check_positions = vec![
        Vec3::new(pos.x - BLOCK_SIZE, pos.y, pos.z), // Left
        Vec3::new(pos.x + BLOCK_SIZE, pos.y, pos.z), // Right
        Vec3::new(pos.x, pos.y + BLOCK_SIZE, pos.z), // Top
    ];

    for check_pos in check_positions {
        let nearby = spatial_hash.query_nearby(check_pos.x, check_pos.y);
        for entity in nearby {
            if let Ok(block_transform) = block_query.get(entity) {
                let distance = check_pos.distance(block_transform.translation);
                if distance < BLOCK_SIZE * 0.5 {
                    return false; // Not enough space
                }
            }
        }
    }

    true
}

/// Spawn a single lootbox
pub fn spawn_single_lootbox(
    commands: &mut Commands,
    position: Vec3,
    block_textures: &BlockTextures,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.6, 0.2), // Golden color
        base_color_texture: Some(block_textures.gold_block.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Lootbox::new(),
        SpriteBundle3d::new(
            meshes.add(Rectangle::new(LOOTBOX_SIZE, LOOTBOX_SIZE)),
            material,
            position,
        ),
        // NO collision components - these are just UI triggers, not physics objects!
        // Detection handled by distance check in lootbox_system.rs
    ));
}


/// Spawn roulette UI for command (loads icons from world)
pub fn spawn_roulette_ui_for_command(
    commands: &mut Commands,
    selected_reward: LootboxReward,
    asset_server: &AssetServer,
) -> bool {
    // Check if already active using atomic flag
    if LOOTBOX_ACTIVE.load(Ordering::Relaxed) {
        return false; // Lootbox already active, skip
    }

    // Mark as active immediately
    LOOTBOX_ACTIVE.store(true, Ordering::Relaxed);

    // Load icons directly via AssetServer as fallback
    use crate::resources::asset_handles::LootboxIcons;

    let fallback_icons = LootboxIcons {
        monster_horde: asset_server.load("textures/ui/lootbox/monster_horde.png"),
        // treasure_rain: asset_server.load("textures/ui/lootbox/treasure_rain.png"),  // REMOVED
        travelers_arrive: asset_server.load("textures/ui/lootbox/travelers_arrive.png"),
        elite_enemy: asset_server.load("textures/ui/lootbox/elite_enemy.png"),
        mega_treasure: asset_server.load("textures/ui/lootbox/mega_treasure.png"),
        magic_barrier: asset_server.load("textures/ui/lootbox/magic_barrier.png"),
        arrow_barrage: asset_server.load("textures/ui/lootbox/arrow_barrage.png"),
        thunder_strike: asset_server.load("textures/ui/lootbox/thunder_strike.png"),
        tnt_rain: asset_server.load("textures/ui/lootbox/tnt_rain.png"),
        laser_beam: asset_server.load("textures/ui/lootbox/laser_beam.png"),
        magnet: asset_server.load("textures/ui/lootbox/magnet.png"),
        nothing: asset_server.load("textures/ui/lootbox/nothing.png"),
    };

    spawn_roulette_ui_internal(commands, selected_reward, &fallback_icons);
    true // Successfully spawned
}

/// Spawn the roulette UI (regular version with icons parameter)
pub fn spawn_roulette_ui_public(
    commands: &mut Commands,
    selected_reward: LootboxReward,
    _asset_server: &AssetServer,
    lootbox_icons: &crate::resources::asset_handles::LootboxIcons,
) {
    spawn_roulette_ui_internal(commands, selected_reward, lootbox_icons);
}

/// Internal implementation of roulette UI spawning
fn spawn_roulette_ui_internal(
    commands: &mut Commands,
    selected_reward: LootboxReward,
    lootbox_icons: &crate::resources::asset_handles::LootboxIcons,
) {
    use rand::Rng;
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();

    let all_possible_rewards = LootboxReward::all_rewards();

    // Select 6 random items from all possible rewards
    // Make sure selected_reward is included
    let mut display_rewards: Vec<LootboxReward> = Vec::new();
    display_rewards.push(selected_reward);

    // Add 5 more random rewards (different from selected)
    let mut remaining: Vec<LootboxReward> = all_possible_rewards
        .into_iter()
        .filter(|r| *r != selected_reward)
        .collect();
    remaining.shuffle(&mut rng);
    display_rewards.extend(remaining.into_iter().take(5));

    // Shuffle all 6 items so selected isn't always first
    display_rewards.shuffle(&mut rng);

    // Find where selected_reward ended up after shuffle
    let selected_index = display_rewards.iter().position(|r| *r == selected_reward).unwrap_or(0);

    info!("🎰 Lootbox spinning for reward: {:?} (index {} in 6 items)", selected_reward, selected_index);

    // Calculate total distance to travel with multiple loops
    // Add some randomness to number of loops (3-5) to make it less predictable
    let num_loops = rng.gen_range(3..=5);
    let total_items = 6; // Always 6 items displayed
    let target_item_count = (num_loops * total_items) + selected_index;

    // Calculate speed needed to reach target in 3 seconds with deceleration
    // With quadratic deceleration: distance ≈ initial_speed * time * 0.667
    let target_speed = (target_item_count as f32) / 2.0; // items per second

    info!("🎰 Will make {} loops through {} items, target {} items, speed {:.1} items/sec", num_loops, total_items, target_item_count, target_speed);

    // Create UI container
    let ui_entity = commands
        .spawn((
            LootboxUI,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            ZIndex(1000),
        ))
        .id();

    // Create border container
    let border_entity = commands
        .spawn((
            Node {
                width: Val::Px(700.0),
                height: Val::Px(200.0),
                border: UiRect::all(Val::Px(5.0)),
                padding: UiRect::all(Val::Px(20.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            BorderColor::all(Color::srgb(0.8, 0.6, 0.2)),
        ))
        .id();

    // Create fixed items container - 6 items displayed in a row
    let items_container = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                ..default()
            },
        ))
        .id();

    // Spawn the 6 fixed items and collect their entities
    let mut item_entities = Vec::new();
    for (index, reward) in display_rewards.iter().enumerate() {
        let item_entity = commands
            .spawn((
                crate::components::lootbox::RouletteItem { index },
                Node {
                    width: Val::Px(100.0),
                    height: Val::Px(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),  // Border on parent
                    ..default()
                },
                BorderColor::all(Color::WHITE),  // White border initially
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),  // Dark background
            ))
            .with_children(|parent| {
                // Image as a child so border shows
                parent.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ImageNode::new(reward.icon(lootbox_icons)),
                ));

                // Add text label below the icon
                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: Val::Px(-20.0),
                        width: Val::Px(100.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    Text::new(reward.name()),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            })
            .id();

        item_entities.push(item_entity);
        commands.entity(items_container).add_child(item_entity);
    }

    commands.entity(border_entity).add_child(items_container);

    // Store the spinner component with the items container (not as a separate border)
    let mut spinner = RouletteSpinner::new_with_speed(display_rewards.clone(), selected_index, target_speed);
    spinner.item_entities = item_entities; // Store the item entities

    // Attach spinner to the items container so we can track it
    commands.entity(items_container).insert(spinner);

    commands.entity(ui_entity).add_child(border_entity);
}

/// Update roulette spinner animation (changes border color of items)
pub fn update_roulette_spinner(
    mut commands: Commands,
    mut spinner_query: Query<(Entity, &mut RouletteSpinner)>,
    mut item_query: Query<(&mut BorderColor, &mut Node), With<crate::components::lootbox::RouletteItem>>,
    hero_query: Query<&Transform, With<Hero>>,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
) {
    for (spinner_entity, mut spinner) in spinner_query.iter_mut() {
        if !spinner.is_spinning {
            continue;
        }

        spinner.spin_timer.tick(time.delta());

        // Calculate deceleration - slow down over time
        let progress = spinner.spin_timer.fraction();
        let speed_multiplier = 1.0 - (progress * progress); // Quadratic slowdown

        // Increment current offset (represents position through the animation)
        // Speed is in "items per second", so this counts through items
        spinner.current_offset += spinner.spin_speed * speed_multiplier * time.delta_secs();

        let total_items = spinner.items.len();

        // Calculate which item is currently selected (cycle 0-5)
        // Use round() instead of truncate to get closest item
        let current_item_index = (spinner.current_offset.round() as usize) % total_items;

        // Play tick sound when moving to a new item
        if current_item_index != spinner.last_item_index {
            spinner.last_item_index = current_item_index;

            // Play tick sound at higher volume
            commands.spawn((
                AudioPlayer::new(asset_server.load("audio/sfx/roulette/tick.ogg")),
                PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(2.0)), // 2x louder!
            ));
        }

        // Update border colors - highlight current item, white for others
        for (i, item_entity) in spinner.item_entities.iter().enumerate() {
            if let Ok((mut border_color, mut node)) = item_query.get_mut(*item_entity) {
                if i == current_item_index {
                    // Selected item: red border, thicker
                    *border_color = BorderColor::all(Color::srgb(1.0, 0.0, 0.0));
                    node.border = UiRect::all(Val::Px(5.0));
                } else {
                    // Unselected items: white border, normal thickness
                    *border_color = BorderColor::all(Color::WHITE);
                    node.border = UiRect::all(Val::Px(2.0));
                }
            }
        }

        // Check if spinning is done
        if spinner.spin_timer.is_finished() {
            spinner.is_spinning = false;

            let reward = spinner.items[spinner.selected_index];

            // Trigger the reward event
            trigger_lootbox_reward(&mut commands, reward, &hero_query);

            // Schedule UI cleanup after 2 seconds
            commands.entity(spinner_entity).insert(CleanupTimer(Timer::from_seconds(2.0, TimerMode::Once)));
        }
    }
}

/// Clean up roulette UI after delay
pub fn cleanup_roulette_ui(
    mut commands: Commands,
    mut ui_query: Query<(Entity, &mut CleanupTimer), With<RouletteSpinner>>,
    lootbox_ui_query: Query<Entity, With<LootboxUI>>,
    time: Res<Time>,
) {
    for (_entity, mut cleanup_timer) in ui_query.iter_mut() {
        cleanup_timer.0.tick(time.delta());
        if cleanup_timer.0.is_finished() {
            // Find and despawn the entire LootboxUI and all its children
            for ui_entity in lootbox_ui_query.iter() {
                commands.entity(ui_entity).despawn();
            }
            // Reset the atomic flag so new lootboxes can be opened
            LOOTBOX_ACTIVE.store(false, Ordering::Relaxed);
        }
    }
}

/// Trigger the reward from lootbox
fn trigger_lootbox_reward(
    commands: &mut Commands,
    reward: LootboxReward,
    hero_query: &Query<&Transform, With<Hero>>,
) {
    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let hero_pos = hero_transform.translation;

    match reward {
        LootboxReward::MonsterHorde => {
            // Trigger monster horde event
            commands.spawn(EventNotification {
                event_type: RandomEventType::MobSpawn,
                timer: Timer::from_seconds(5.0, TimerMode::Once),
            });
            info!("Lootbox triggered Monster Horde!");
        }
        // LootboxReward::TreasureRain => {
        //     // REMOVED - Trigger treasure rain event
        //     commands.spawn(EventNotification {
        //         event_type: RandomEventType::LootDrop,
        //         timer: Timer::from_seconds(5.0, TimerMode::Once),
        //     });
        //     info!("Lootbox triggered Treasure Rain!");
        // }
        LootboxReward::TravelersArrive => {
            // Trigger villager spawn event
            commands.spawn(EventNotification {
                event_type: RandomEventType::VillagerSpawn,
                timer: Timer::from_seconds(5.0, TimerMode::Once),
            });
            info!("Lootbox triggered Travelers Arrive!");
        }
        LootboxReward::EliteEnemy => {
            // Trigger elite enemy event
            commands.spawn(EventNotification {
                event_type: RandomEventType::EliteMobSpawn,
                timer: Timer::from_seconds(5.0, TimerMode::Once),
            });
            info!("Lootbox triggered Elite Enemy!");
        }
        LootboxReward::MegaTreasure => {
            // Trigger double treasure rain
            commands.spawn(EventNotification {
                event_type: RandomEventType::LootDrop,
                timer: Timer::from_seconds(5.0, TimerMode::Once),
            });
            commands.spawn(EventNotification {
                event_type: RandomEventType::LootDrop,
                timer: Timer::from_seconds(5.5, TimerMode::Once),
            });
            info!("Lootbox triggered Mega Treasure!");
        }
        LootboxReward::MagicBarrier => {
            // Trigger magic barrier power
            use crate::resources::command_queue::PowerCommand;
            use crate::resources::command_queue::PowerCommandType;
            use crate::plugins::external_api_plugin::get_command_queue;

            if let Some(queue) = get_command_queue() {
                if let Ok(mut q) = queue.lock() {
                    q.push_back(PowerCommand::new(PowerCommandType::Mirror, "Lootbox".to_string()));
                }
            }
            info!("Lootbox triggered Magic Barrier!");
        }
        LootboxReward::ArrowBarrage => {
            // Trigger arrow barrage power
            use crate::resources::command_queue::PowerCommand;
            use crate::resources::command_queue::PowerCommandType;
            use crate::plugins::external_api_plugin::get_command_queue;

            if let Some(queue) = get_command_queue() {
                if let Ok(mut q) = queue.lock() {
                    q.push_back(PowerCommand::new(PowerCommandType::Arrow, "Lootbox".to_string()));
                }
            }
            info!("Lootbox triggered Arrow Barrage!");
        }
        LootboxReward::ThunderStrike => {
            // Trigger thunder strike power
            use crate::resources::command_queue::PowerCommand;
            use crate::resources::command_queue::PowerCommandType;
            use crate::plugins::external_api_plugin::get_command_queue;

            if let Some(queue) = get_command_queue() {
                if let Ok(mut q) = queue.lock() {
                    q.push_back(PowerCommand::new(PowerCommandType::Thunder, "Lootbox".to_string()));
                }
            }
            info!("Lootbox triggered Thunder Strike!");
        }
        LootboxReward::TntRain => {
            // Trigger TNT rain power
            use crate::resources::command_queue::PowerCommand;
            use crate::resources::command_queue::PowerCommandType;
            use crate::plugins::external_api_plugin::get_command_queue;

            if let Some(queue) = get_command_queue() {
                if let Ok(mut q) = queue.lock() {
                    q.push_back(PowerCommand::new(PowerCommandType::Tnt, "Lootbox".to_string()));
                }
            }
            info!("Lootbox triggered TNT Rain!");
        }
        LootboxReward::LaserBeam => {
            // Trigger laser beam power
            use crate::resources::command_queue::PowerCommand;
            use crate::resources::command_queue::PowerCommandType;
            use crate::plugins::external_api_plugin::get_command_queue;

            if let Some(queue) = get_command_queue() {
                if let Ok(mut q) = queue.lock() {
                    q.push_back(PowerCommand::new(PowerCommandType::Laser, "Lootbox".to_string()));
                }
            }
            info!("Lootbox triggered Laser Beam!");
        }
        LootboxReward::Magnet => {
            // Trigger magnet power
            use crate::resources::command_queue::PowerCommand;
            use crate::resources::command_queue::PowerCommandType;
            use crate::plugins::external_api_plugin::get_command_queue;

            if let Some(queue) = get_command_queue() {
                if let Ok(mut q) = queue.lock() {
                    q.push_back(PowerCommand::new(PowerCommandType::Magnet, "Lootbox".to_string()));
                }
            }
            info!("Lootbox triggered Magnet!");
        }
        LootboxReward::Nothing => {
            info!("Lootbox gave nothing!");
        }
    }
}

/// Plugin for lootbox system
pub struct LootboxPlugin;

impl Plugin for LootboxPlugin {
    fn build(&self, app: &mut App) {
        // DISABLED: Lootboxes should NOT spawn as physical entities in the world
        // They are UI-only events triggered by commands/dev panel
        // app.add_systems(OnEnter(GameState::Playing), spawn_lootboxes_in_world)
        app.add_systems(
                Update,
                (
                    update_roulette_spinner,
                    cleanup_roulette_ui,
                )
                    .run_if(in_state(GameState::Playing)),
            );

        info!("LootboxPlugin loaded");
    }
}
