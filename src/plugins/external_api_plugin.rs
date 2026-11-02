use crate::components::enemy::{Enemy, EnemyBehavior, EnemyType, Flying, Health};
use crate::components::hero::Hero;
use crate::components::power_effect::SpeedModifier;
use crate::events::power_events::PowerActivated;
use crate::plugins::powers_plugin::{
    spawn_arrows, spawn_laser, spawn_laser_burst, spawn_magic_barrier, spawn_magnet_field,
    spawn_tnt,
};
use crate::resources::asset_handles::{GameTextures, ProjectileTextures};
use crate::resources::command_queue::{PowerCommand, PowerCommandType};
use crate::systems::entity_spawn::spawn_villagers_around_position;
use bevy::prelude::*;

use crate::components::collision_layers::GameLayer;
use crate::utils::constants::{CHARACTER_MASS, ENEMY_BOUNCE};
use avian3d::prelude::*;

pub struct ExternalApiPlugin;

#[derive(serde::Deserialize)]
struct LauncherApiOptions {
    #[serde(default)]
    direction: crate::resources::command_queue::LaunchDirection,
    #[serde(default = "one_passenger")]
    villagers: u8,
}
fn one_passenger() -> u8 {
    1
}
#[cfg(test)]
mod launcher_api_tests {
    use super::*;
    #[test]
    fn api_defaults_and_passenger_limits() {
        let defaults: LauncherApiOptions = serde_json::from_str("{}").unwrap();
        assert_eq!(defaults.validated().unwrap().villagers, 1);
        for count in [0, 1, 8] {
            let options: LauncherApiOptions =
                serde_json::from_value(serde_json::json!({"direction":"left","villagers":count}))
                    .unwrap();
            assert_eq!(
                options.validated().unwrap().direction,
                crate::resources::command_queue::LaunchDirection::Left
            );
        }
        let invalid: LauncherApiOptions = serde_json::from_str("{\"villagers\":9}").unwrap();
        assert!(invalid.validated().is_err());
        for json in [
            "{\"villagers\":-1}",
            "{\"villagers\":1.5}",
            "{\"direction\":\"up\"}",
        ] {
            assert!(serde_json::from_str::<LauncherApiOptions>(json).is_err());
        }
    }
}
impl LauncherApiOptions {
    fn validated(&self) -> Result<crate::resources::command_queue::LauncherOptions, &'static str> {
        if self.villagers > 8 {
            return Err("Launcher villagers must be between 0 and 8");
        }
        Ok(crate::resources::command_queue::LauncherOptions {
            direction: self.direction,
            villagers: self.villagers,
        })
    }
}

// Global channel for commands (simplified approach)
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};

/// Preferred API port, used whenever it is free.
const DEFAULT_API_PORT: u16 = 3029;
/// Tried in order when the default is taken. Hyper-V, WSL and Docker reserve
/// port ranges that can change at every boot (on some PCs anywhere above 1024).
const FALLBACK_API_PORTS: [u16; 5] = [7329, 17329, 27329, 37329, 47329];
/// Settings file with the address in use, for VortiDeck and other tools.
const API_INFO_FILE: &str = "api.json";

/// Ports to try in order: only FALLING_HEADS_API_PORT when it is set, else the
/// default, the fallbacks, then 0 (any free port Windows offers).
fn api_ports() -> Vec<u16> {
    match std::env::var("FALLING_HEADS_API_PORT")
        .ok()
        .and_then(|p| p.trim().parse::<u16>().ok())
    {
        Some(port) => vec![port],
        None => std::iter::once(DEFAULT_API_PORT)
            .chain(FALLBACK_API_PORTS)
            .chain(std::iter::once(0))
            .collect(),
    }
}

/// Where viewer commands go once the server is listening ("off" if it failed).
static API_URL: OnceLock<String> = OnceLock::new();

/// The command address for the settings page, or why there is none.
pub fn api_url() -> String {
    API_URL
        .get()
        .cloned()
        .unwrap_or_else(|| "starting...".to_string())
}

/// STYLE needs a valid style; VILLAGERS may carry one; other commands ignore it.
fn requested_style(
    command: PowerCommandType,
    style: Option<&str>,
) -> Result<Option<crate::components::play_style::PlayStyle>, String> {
    use crate::components::play_style::PlayStyle;
    let names = || {
        PlayStyle::ALL
            .map(|s| s.name().to_ascii_lowercase())
            .join(", ")
    };
    match (command, style) {
        (PowerCommandType::Style | PowerCommandType::Villagers, Some(name)) => {
            PlayStyle::parse(name)
                .map(Some)
                .ok_or_else(|| format!("Unknown style '{name}'. Styles: {}", names()))
        }
        (PowerCommandType::Style, None) => Err(format!("STYLE needs \"style\": {}", names())),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod port_tests {
    use super::*;

    #[test]
    fn default_port_first_then_fallbacks_then_any_free_port() {
        if std::env::var("FALLING_HEADS_API_PORT").is_ok() {
            return; // An explicit port replaces the list.
        }
        let ports = api_ports();
        assert_eq!(ports.first(), Some(&DEFAULT_API_PORT));
        assert_eq!(ports.last(), Some(&0));
        assert_eq!(ports.len(), FALLBACK_API_PORTS.len() + 2);
    }
}

#[cfg(test)]
mod style_api_tests {
    use super::*;
    use crate::components::play_style::PlayStyle;
    #[test]
    fn style_is_required_for_style_optional_for_villagers_ignored_elsewhere() {
        assert_eq!(
            requested_style(PowerCommandType::Style, Some("Heavy")),
            Ok(Some(PlayStyle::Heavy))
        );
        assert!(requested_style(PowerCommandType::Style, None).is_err());
        assert!(requested_style(PowerCommandType::Style, Some("rocket")).is_err());
        assert_eq!(requested_style(PowerCommandType::Villagers, None), Ok(None));
        assert_eq!(
            requested_style(PowerCommandType::Villagers, Some("sticky")),
            Ok(Some(PlayStyle::Sticky))
        );
        assert_eq!(
            requested_style(PowerCommandType::Tnt, Some("rocket")),
            Ok(None)
        );
    }
}

static COMMAND_QUEUE: OnceLock<Arc<Mutex<VecDeque<PowerCommand>>>> = OnceLock::new();

/// Get access to the global command queue (for dev panel and other systems)
pub fn get_command_queue() -> Option<Arc<Mutex<VecDeque<PowerCommand>>>> {
    COMMAND_QUEUE.get().cloned()
}

impl Plugin for ExternalApiPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PowerActivated>()
            .add_systems(Startup, start_api_server)
            .add_systems(
                Update,
                process_commands.run_if(in_state(crate::states::InGame)),
            )
            .add_systems(Update, update_thunder_bolts);

        info!(
            "ExternalApiPlugin loaded - API ports to try: {:?}",
            api_ports()
        );
    }
}

fn start_api_server() {
    // Initialize the global queue
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    COMMAND_QUEUE.set(queue.clone()).ok();
    if std::env::args().any(|arg| {
        arg == "--smoke-test"
            || arg == "--vfx-smoke-test"
            || arg == "--ui-smoke-test"
            || arg == "--gpu-smoke-test"
            || arg == "--fg-smoke-test"
            || arg == "--visual-smoke-test"
    }) {
        return;
    }

    use axum::{
        Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post,
    };
    use serde::{Deserialize, Serialize};
    use tokio::runtime::Runtime;

    #[derive(Deserialize)]
    struct CommandRequest {
        action: String,
        #[serde(default = "default_username")]
        username: String,
        #[serde(flatten)]
        launcher: LauncherApiOptions,
        /// Play style for STYLE (required) or VILLAGERS (optional)
        #[serde(default)]
        style: Option<String>,
    }

    fn default_username() -> String {
        "Anonymous".to_string()
    }

    #[derive(Serialize)]
    struct CommandResponse {
        success: bool,
        message: String,
    }

    async fn handle_command(
        State(queue): State<Arc<Mutex<VecDeque<PowerCommand>>>>,
        Json(cmd): Json<CommandRequest>,
    ) -> impl IntoResponse {
        let command_type = match cmd.action.to_uppercase().as_str() {
            "WATERBUCKET" => Some(PowerCommandType::WaterBucket),
            "LAVABUCKET" => Some(PowerCommandType::LavaBucket),
            "TELEPORT" => Some(PowerCommandType::Teleport),
            "LAUNCHER" | "ROCKET" => Some(PowerCommandType::Launcher),
            "RIFT" => Some(PowerCommandType::Rift),
            "REWIND" => Some(PowerCommandType::Rewind),
            "TNT" => Some(PowerCommandType::Tnt),
            "FAST" => Some(PowerCommandType::Fast),
            "SLOW" => Some(PowerCommandType::Slow),
            "FUSS" => Some(PowerCommandType::Fuss),
            "BOLT" => Some(PowerCommandType::Bolt),
            "ARROW" => Some(PowerCommandType::Arrow),
            "BURST" => Some(PowerCommandType::LaserBurst), // Renamed from LASERBURST to BURST
            "LASER" => Some(PowerCommandType::Laser),
            "GHAST" => Some(PowerCommandType::Ghast),
            "BOX" => Some(PowerCommandType::Box),
            "MAGNET" => Some(PowerCommandType::Magnet),
            "VILLAGERS" => Some(PowerCommandType::Villagers),
            "MIRROR" => Some(PowerCommandType::Mirror),
            "DRAGON" => Some(PowerCommandType::Dragon),
            "THUNDER" => Some(PowerCommandType::Thunder),
            "LOOTBOX" => Some(PowerCommandType::Lootbox),
            "STYLE" => Some(PowerCommandType::Style),
            "FEATHER" => Some(PowerCommandType::Feather),
            "ANVIL" => Some(PowerCommandType::Anvil),
            "GOLEM" => Some(PowerCommandType::Golem),
            "PEARL" | "ENDERPEARL" => Some(PowerCommandType::Pearl),
            "QUAKE" | "EARTHQUAKE" => Some(PowerCommandType::Quake),
            "SHRINK" => Some(PowerCommandType::Shrink),
            "WIND" => Some(PowerCommandType::Wind),
            "BLACKHOLE" | "BLACK_HOLE" | "HOLE" => Some(PowerCommandType::BlackHole),
            "ICEAGE" | "ICE_AGE" | "ICE" => Some(PowerCommandType::IceAge),
            "BEES" | "BEE" => Some(PowerCommandType::Bees),
            _ => None,
        };

        if !crate::systems::game_modes::chat_powers_on() {
            return (
                StatusCode::FORBIDDEN,
                Json(CommandResponse {
                    success: false,
                    message: "Chat powers are turned off in the game (Settings > Game modes)"
                        .into(),
                }),
            );
        }
        if let Some(cmd_type) = command_type {
            let mut power_cmd = PowerCommand::new(cmd_type, cmd.username.clone());
            match requested_style(cmd_type, cmd.style.as_deref()) {
                Ok(style) => power_cmd.style = style,
                Err(message) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(CommandResponse {
                            success: false,
                            message,
                        }),
                    );
                }
            }
            if cmd_type == PowerCommandType::Launcher {
                match cmd.launcher.validated() {
                    Ok(options) => power_cmd.launcher = options,
                    Err(message) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(CommandResponse {
                                success: false,
                                message: message.into(),
                            }),
                        );
                    }
                }
            }
            if let Ok(mut q) = queue.lock() {
                q.push_back(power_cmd);
                (
                    StatusCode::OK,
                    Json(CommandResponse {
                        success: true,
                        message: format!(
                            "Power '{}' queued for user '{}'!",
                            cmd.action, cmd.username
                        ),
                    }),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(CommandResponse {
                        success: false,
                        message: "Failed to queue command".to_string(),
                    }),
                )
            }
        } else {
            (
                StatusCode::BAD_REQUEST,
                Json(CommandResponse {
                    success: false,
                    message: format!(
                        "Unknown command: {}. Available: TNT, FAST, SLOW, LASER, BURST, ARROW, BOLT, GHAST, BOX, MAGNET, FUSS, VILLAGERS, MIRROR, DRAGON, THUNDER, LOOTBOX, STYLE, FEATHER, ANVIL, GOLEM, PEARL, QUAKE, SHRINK, WIND, BLACKHOLE, ICEAGE, BEES, GROW",
                        cmd.action
                    ),
                }),
            )
        }
    }

    let app_router = Router::new()
        .route("/command", post(handle_command))
        .with_state(queue.clone());

    // Spawn API server in background thread
    let ports = api_ports();
    std::thread::spawn(move || {
        let rt = match Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error!("API server disabled: cannot start async runtime: {e}");
                return;
            }
        };
        rt.block_on(async {
            let mut bound = None;
            for port in &ports {
                match tokio::net::TcpListener::bind(("127.0.0.1", *port)).await {
                    Ok(listener) => {
                        bound = Some(listener);
                        break;
                    }
                    // Reserved by Windows (permission denied) or used by another app.
                    Err(e) => warn!("API port {port} unavailable: {e}"),
                }
            }
            let Some(listener) = bound else {
                error!(
                    "API server disabled: no free port among {ports:?}. Set FALLING_HEADS_API_PORT \
                     to a free port (see `netsh interface ipv4 show excludedportrange protocol=tcp`)."
                );
                API_URL.set("off (no free port)".to_string()).ok();
                return;
            };
            let api_addr = listener
                .local_addr()
                .map_or_else(|_| "127.0.0.1:?".to_string(), |a| a.to_string());
            let url = format!("http://{api_addr}/command");
            API_URL.set(url.clone()).ok();
            if crate::resources::settings::persistence_enabled() {
                let info = serde_json::json!({ "url": url, "port": listener.local_addr().map(|a| a.port()).ok() });
                crate::resources::settings::write_settings_file(API_INFO_FILE, &info.to_string());
            }
            info!("API server running on http://{api_addr}");
            info!("Try: curl -X POST http://{api_addr}/command -H \"Content-Type: application/json\" -d '{{\"action\":\"TNT\",\"username\":\"viewer123\"}}'");
            if let Err(e) = axum::serve(listener, app_router).await {
                error!("API server stopped: {e}");
            }
        });
    });
}

#[allow(clippy::too_many_arguments)] // Bevy system parameters
/// The cooldown settings, when each power is ready, the on-screen notice and the clock.
type Cooldowns<'w> = (
    Res<'w, crate::systems::power_cooldowns::PowerCooldowns>,
    ResMut<'w, crate::systems::power_cooldowns::PowerReadyAt>,
    ResMut<'w, crate::systems::world_portals::PowerNotice>,
    Res<'w, Time>,
);

#[allow(clippy::too_many_arguments)] // Bevy system parameters
fn process_commands(
    mut commands: Commands,
    hero_query: Query<(Entity, &Transform), With<Hero>>,
    textures: Res<GameTextures>,
    projectile_textures: Option<Res<ProjectileTextures>>,
    asset_server: Res<AssetServer>,
    block_atlases: Option<Res<crate::resources::asset_handles::BlockAtlases>>,
    mesh_cache: Option<Res<crate::resources::mesh_cache::MeshCache>>,
    spatial_hash: Res<crate::resources::spatial_hash::SpatialHash>,
    mut queries: ParamSet<(
        Query<
            (
                Entity,
                &Transform,
                &crate::components::block::BlockType,
                Option<&crate::systems::biome_content::Tile>,
            ),
            With<crate::components::block::Block>,
        >,
        Query<
            (Entity, &Transform, &crate::components::enemy::Health),
            Or<(
                With<Hero>,
                With<crate::components::villager::Villager>,
                With<crate::components::enemy::Enemy>,
            )>,
        >,
        Query<&mut crate::components::buff::Buff>,
    )>,
    camera_query: Query<&Projection, With<Camera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut power_events: MessageWriter<PowerActivated>,
    mut damage_events: MessageWriter<crate::events::damage_events::EntityDamaged>,
    mut destroyed: MessageWriter<crate::events::block_events::BlockDestroyed>,
    (cooldowns, mut ready_at, mut notice, time): Cooldowns,
) {
    let Some(queue) = COMMAND_QUEUE.get() else {
        return;
    };

    let Ok(mut q) = queue.lock() else {
        return;
    };

    while let Some(command) = q.pop_front() {
        // Per-power cooldowns from Settings > Cooldowns (0 = none).
        if let Err(left) = ready_at.try_use(&cooldowns, command.command_type, time.elapsed_secs()) {
            notice.text = format!(
                "{} recharging: {:.0}s",
                crate::systems::power_cooldowns::names(command.command_type).1,
                left.ceil()
            );
            notice.remaining = 3.;
            continue;
        }
        power_events.write(PowerActivated {
            command: command.clone(),
        });

        // Execute power immediately
        match command.command_type {
            PowerCommandType::Rift
            | PowerCommandType::Rewind
            | PowerCommandType::Launcher
            | PowerCommandType::Teleport
            | PowerCommandType::WaterBucket
            | PowerCommandType::LavaBucket
            | PowerCommandType::Style
            | PowerCommandType::Fuss
            | PowerCommandType::Bolt
            | PowerCommandType::Feather
            | PowerCommandType::Anvil
            | PowerCommandType::Golem
            | PowerCommandType::Pearl
            | PowerCommandType::Quake
            | PowerCommandType::Shrink
            | PowerCommandType::Wind
            | PowerCommandType::BlackHole
            | PowerCommandType::IceAge
            | PowerCommandType::Bees => {} // Dedicated systems consume PowerActivated.
            PowerCommandType::Tnt => {
                if let (Ok((_, hero_transform)), Some(atlases), Some(mesh_cache)) = (
                    hero_query.single(),
                    block_atlases.as_ref(),
                    mesh_cache.as_ref(),
                ) {
                    let spawn_pos = hero_transform.translation + Vec3::new(0.0, 50.0, 0.0); // 2D scale
                    spawn_tnt(
                        &mut commands,
                        spawn_pos,
                        &atlases,
                        &mesh_cache,
                        &mut meshes,
                        &mut materials,
                        Some(command.username.clone()),
                        Some(&asset_server),
                    );
                    info!("🧨 TNT power activated by {}!", command.username);
                }
            }
            PowerCommandType::Fast => {
                if let Ok((hero_entity, _)) = hero_query.single() {
                    commands.entity(hero_entity).insert(SpeedModifier {
                        multiplier: 2.0,
                        duration: Timer::from_seconds(5.0, TimerMode::Once),
                    });
                    info!(
                        "⚡ FAST power activated by {}! Speed x2 for 5 seconds",
                        command.username
                    );
                }
            }
            PowerCommandType::Slow => {
                if let Ok((hero_entity, _)) = hero_query.single() {
                    commands.entity(hero_entity).insert(SpeedModifier {
                        multiplier: 0.5,
                        duration: Timer::from_seconds(5.0, TimerMode::Once),
                    });
                    info!(
                        "🐌 SLOW power activated by {}! Speed x0.5 for 5 seconds",
                        command.username
                    );
                }
            }
            PowerCommandType::Laser => {
                if let Ok((_, hero_transform)) = hero_query.single() {
                    spawn_laser(
                        &mut commands,
                        hero_transform.translation,
                        &mut meshes,
                        &mut materials,
                        &asset_server,
                    );
                    info!("💜 LASER power activated by {}!", command.username);
                }
            }
            PowerCommandType::LaserBurst => {
                if let Ok((hero_entity, _)) = hero_query.single() {
                    spawn_laser_burst(
                        &mut commands,
                        hero_entity,
                        &mut meshes,
                        &mut materials,
                        &asset_server,
                    );
                    info!("⚡ LASER BURST activated by {}!", command.username);
                }
            }
            PowerCommandType::Arrow => {
                if let Ok((hero_entity, _)) = hero_query.single() {
                    if let Some(ref projectile_textures) = projectile_textures {
                        spawn_arrows(
                            &mut commands,
                            hero_entity,
                            &mut meshes,
                            &mut materials,
                            projectile_textures,
                        );
                        info!("🏹 ARROW barrage activated by {}!", command.username);
                    } else {
                        info!("⚠️ ARROW power waiting for textures to load...");
                    }
                }
            }
            PowerCommandType::Villagers => {
                if let Ok((_, hero_transform)) = hero_query.single() {
                    spawn_villagers_around_position(
                        &mut commands,
                        hero_transform.translation,
                        5,
                        (-200.0, 200.0),
                        (-50.0, 50.0),
                        &textures,
                        &mut meshes,
                        &mut materials,
                        Some(command.username.clone()),
                        &asset_server,
                    );
                    info!("👥 VILLAGERS spawned by {}!", command.username);
                }
            }
            PowerCommandType::Mirror => {
                if let Ok((hero_entity, hero_transform)) = hero_query.single() {
                    spawn_magic_barrier(
                        &mut commands,
                        hero_entity,
                        hero_transform.translation,
                        &asset_server,
                    );
                    info!("🪞 MIRROR barrier activated by {}!", command.username);
                }
            }
            PowerCommandType::Magnet => {
                if let Ok((hero_entity, hero_transform)) = hero_query.single() {
                    spawn_magnet_field(
                        &mut commands,
                        hero_entity,
                        hero_transform.translation,
                        &mut meshes,
                        &mut materials,
                        &asset_server,
                    );
                    info!("🧲 MAGNET field activated by {}!", command.username);
                }
            }
            PowerCommandType::Ghast => {
                if let Ok((_, hero_transform)) = hero_query.single() {
                    spawn_ghast(
                        &mut commands,
                        hero_transform.translation + Vec3::new(200.0, 150.0, 0.0), // Spawn above and to the right
                        &mut meshes,
                        &mut materials,
                        &asset_server,
                    );
                    info!("👻 GHAST spawned by {}!", command.username);
                }
            }
            PowerCommandType::Dragon => {} // Shared natural/power dragon encounter.
            PowerCommandType::Thunder => {
                if let Ok((_, hero_transform)) = hero_query.single() {
                    spawn_thunder_strikes(
                        &mut commands,
                        hero_transform.translation,
                        &spatial_hash,
                        &mut queries,
                        &camera_query,
                        &mut meshes,
                        &mut materials,
                        &asset_server,
                        &mut damage_events,
                        &mut destroyed,
                    );
                    info!("⚡ THUNDER strikes spawned by {}!", command.username);
                }
            }
            PowerCommandType::Lootbox => {
                // Directly trigger lootbox roulette UI
                use crate::components::lootbox::LootboxReward;
                let selected_reward = LootboxReward::random();

                use crate::systems::lootbox_system::spawn_roulette_ui_for_command;
                if spawn_roulette_ui_for_command(&mut commands, selected_reward, &asset_server) {
                    info!(
                        "📦 LOOTBOX reward triggered by {}: {:?}",
                        command.username, selected_reward
                    );
                } else {
                    info!(
                        "📦 LOOTBOX already active, skipping request from {}",
                        command.username
                    );
                }
            }
            PowerCommandType::RandomBox => {
                // Spawn a bouncing randombox near the hero in a free area
                if let Ok((_, hero_transform)) = hero_query.single() {
                    use crate::systems::randombox_system::spawn_randombox;
                    use rand::RngExt;

                    // Every spawn is validated by the common drop/terrain resolver before rendering.
                    let hero_pos = hero_transform.translation;
                    let mut rng = rand::rng();
                    let final_pos =
                        hero_pos + Vec3::new(rng.random_range(-350.0..350.0), 200.0, 0.0);

                    spawn_randombox(
                        &mut commands,
                        final_pos,
                        &textures,
                        &mut meshes,
                        &mut materials,
                    );
                    info!(
                        "🎲 RANDOMBOX spawned by {} at {:?}!",
                        command.username, final_pos
                    );
                }
            }
            PowerCommandType::GrowBuff => {
                // Apply Grow buff to hero (makes them bigger and faster)
                if let Ok((hero_entity, _)) = hero_query.single() {
                    crate::systems::buffs::buff_core::activate_growth(&mut commands, hero_entity);
                    info!(
                        "🔼 GROW buff activated by {}! Hero grows 20% bigger and 30% faster",
                        command.username
                    );
                }
            }
            _ => {
                info!(
                    "⚡ Power {:?} activated by {} (not yet implemented)",
                    command.command_type, command.username
                );
            }
        }
    }
}

fn spawn_ghast(
    commands: &mut Commands,
    position: Vec3,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
) {
    use crate::components::bundles::SpriteBundle3d;

    // Spawn Ghast as flying enemy above hero
    let ghast_texture = asset_server.load("textures/entities/enemy/ghast/ghast.png");

    commands.spawn((
        Enemy::new(EnemyType::Ghast),
        Flying, // Ghasts float and ignore block collisions
        Health::new(EnemyType::Ghast.max_health()),
        // Custom physics for flying - no gravity!
        RigidBody::Dynamic,
        LinearVelocity(Vec3::ZERO),
        GravityScale(0.0),  // No gravity - ghasts float
        LinearDamping(2.0), // Slow down naturally - allows Avian collision response to work
        Mass(CHARACTER_MASS),
        Collider::cuboid(80.0, 80.0, 80.0), // 160x160 collider - matches 2x pattern used by other entities (prevents overlap)
        Restitution::new(ENEMY_BOUNCE),     // Same bounce as other enemies
        Friction::new(0.0),
        LockedAxes::ROTATION_LOCKED,
        CollisionLayers::new(GameLayer::Enemy, GameLayer::Enemy.filters()),
        CollisionEventsEnabled,
        SpriteBundle3d::new(
            meshes.add(Rectangle::new(80.0, 80.0)), // Bigger size
            materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(ghast_texture),
                unlit: true,
                alpha_mode: AlphaMode::Mask(0.5),
                ..default()
            }),
            position,
        ),
    ));

    info!("👻 Ghast spawned at {:?}", position);
}

#[allow(clippy::too_many_arguments)] // Bevy system parameters
fn spawn_thunder_strikes(
    commands: &mut Commands,
    hero_pos: Vec3,
    spatial_hash: &Res<crate::resources::spatial_hash::SpatialHash>,
    queries: &mut ParamSet<(
        Query<
            (
                Entity,
                &Transform,
                &crate::components::block::BlockType,
                Option<&crate::systems::biome_content::Tile>,
            ),
            With<crate::components::block::Block>,
        >,
        Query<
            (Entity, &Transform, &crate::components::enemy::Health),
            Or<(
                With<crate::components::hero::Hero>,
                With<crate::components::villager::Villager>,
                With<crate::components::enemy::Enemy>,
            )>,
        >,
        Query<&mut crate::components::buff::Buff>,
    )>,
    camera_query: &Query<&Projection, With<Camera>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    damage_events: &mut MessageWriter<crate::events::damage_events::EntityDamaged>,
    destroyed: &mut MessageWriter<crate::events::block_events::BlockDestroyed>,
) {
    use crate::components::bundles::SpriteBundle3d;
    use crate::utils::constants::BLOCK_SIZE;
    use rand::RngExt;

    const THUNDER_COUNT: usize = 3;
    const EXPLOSION_RADIUS: f32 = BLOCK_SIZE * 3.0; // 3 blocks radius for bigger impact
    const LIGHTNING_WIDTH: f32 = 24.0;
    const LIGHTNING_CORE_WIDTH: f32 = 12.0;
    const LIGHTNING_HEIGHT: f32 = 1400.0;
    const LIGHTNING_DURATION: f32 = 0.25;
    const LIGHTNING_SEGMENTS: usize = 14;
    const LIGHTNING_ZIGZAG: f32 = 80.0; // More dramatic zigzag
    const BRANCH_CHANCE: f32 = 0.3; // 30% chance per segment to create branch
    const BRANCH_LENGTH_RATIO: f32 = 0.4; // Branch length compared to main bolt

    let mut broken = std::collections::HashSet::new(); // Shared by all strikes this command
    let mut rng = rand::rng();

    // Get screen dimensions from camera
    let (screen_width, screen_height) = if let Ok(projection) = camera_query.single() {
        match projection {
            Projection::Orthographic(ortho) => (ortho.area.width(), ortho.area.height()),
            _ => (1920.0, 1080.0),
        }
    } else {
        (1920.0, 1080.0) // Fallback
    };

    // Instead of targeting specific blocks, spawn randomly across ENTIRE screen (width AND height)
    for _ in 0..THUNDER_COUNT {
        // Random X position across the screen
        let strike_x = hero_pos.x + rng.random_range(-screen_width / 2.0..screen_width / 2.0);

        // Random Y position across the screen (where the bottom of lightning hits)
        let target_y = hero_pos.y + rng.random_range(-screen_height / 2.0..screen_height / 2.0);

        // Lightning goes from well above target to well below target
        let top_y = target_y + LIGHTNING_HEIGHT;
        let bottom_y = target_y - 400.0;
        let total_height = top_y - bottom_y;
        let segment_height = total_height / LIGHTNING_SEGMENTS as f32;

        // Outer glow material (blue-white)
        let outer_material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.7, 0.8, 1.0, 0.6),
            emissive: Color::srgb(30.0, 40.0, 80.0).into(),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        // Inner core material (very bright white)
        let core_material = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 1.0, 1.0),
            emissive: Color::srgb(200.0, 200.0, 255.0).into(),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        // Generate zigzag path points with more random variation
        let mut points: Vec<Vec2> = Vec::new();
        points.push(Vec2::new(strike_x, top_y));

        let mut current_x = strike_x;
        for i in 1..=LIGHTNING_SEGMENTS {
            let offset = rng.random_range(-LIGHTNING_ZIGZAG..LIGHTNING_ZIGZAG);
            current_x += offset;
            let y = top_y - (i as f32 * segment_height);
            points.push(Vec2::new(current_x, y));
        }

        // Get both endpoints where lightning starts and ends
        let start_point = points[0];
        let end_point = points[points.len() - 1];

        let start_pos = Vec3::new(start_point.x, start_point.y, 0.0);
        let impact_pos = Vec3::new(end_point.x, end_point.y, 0.0);

        // Destroy blocks and damage characters at BOTH start and end points (lightning damages along entire path)
        for explosion_pos in [start_pos, impact_pos] {
            // Damage blocks
            let nearby = spatial_hash.query_nearby(explosion_pos.x, explosion_pos.y);
            for block_entity in nearby {
                if let Ok((entity, transform, kind, tile)) = queries.p0().get(block_entity) {
                    let dist = explosion_pos.distance(transform.translation);
                    if dist <= EXPLOSION_RADIUS {
                        crate::plugins::gameplay_plugin::break_block(
                            commands,
                            destroyed,
                            &mut broken,
                            entity,
                            *kind,
                            transform.translation,
                            tile.copied(),
                        );
                    }
                }
            }

            // Damage characters (hero, villagers, enemies)
            for (char_entity, char_transform, _health) in queries.p1().iter() {
                let dist = explosion_pos.distance(char_transform.translation);
                if dist <= EXPLOSION_RADIUS {
                    // Apply 2 hearts damage via event system
                    damage_events.write(crate::events::damage_events::EntityDamaged {
                        entity: char_entity,
                        damage: 2,
                    });
                    info!(
                        "⚡ Thunder explosion damaged character at distance {:.1}",
                        dist
                    );
                }
            }
        }

        // Helper function to draw a segment
        let mut draw_segment = |start: Vec2, end: Vec2, width_scale: f32| {
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let length = (dx * dx + dy * dy).sqrt();
            let angle = dy.atan2(dx);

            let mid_x = (start.x + end.x) / 2.0;
            let mid_y = (start.y + end.y) / 2.0;

            // Outer glow
            let mut outer_transform = Transform::from_xyz(mid_x, mid_y, 200.0);
            outer_transform.rotation = Quat::from_rotation_z(angle - std::f32::consts::FRAC_PI_2);

            let mut outer_bundle = SpriteBundle3d::new(
                meshes.add(Rectangle::new(LIGHTNING_WIDTH * width_scale, length + 4.0)),
                outer_material.clone(),
                Vec3::new(mid_x, mid_y, 200.0),
            );
            outer_bundle.transform = outer_transform;

            commands.spawn((
                ThunderBolt {
                    lifetime: Timer::from_seconds(LIGHTNING_DURATION, TimerMode::Once),
                },
                outer_bundle,
            ));

            // Inner core
            let mut core_transform = Transform::from_xyz(mid_x, mid_y, 201.0);
            core_transform.rotation = Quat::from_rotation_z(angle - std::f32::consts::FRAC_PI_2);

            let mut core_bundle = SpriteBundle3d::new(
                meshes.add(Rectangle::new(
                    LIGHTNING_CORE_WIDTH * width_scale,
                    length + 4.0,
                )),
                core_material.clone(),
                Vec3::new(mid_x, mid_y, 201.0),
            );
            core_bundle.transform = core_transform;

            commands.spawn((
                ThunderBolt {
                    lifetime: Timer::from_seconds(LIGHTNING_DURATION, TimerMode::Once),
                },
                core_bundle,
            ));
        };

        // Draw main bolt segments and create branches
        for i in 0..points.len() - 1 {
            let start = points[i];
            let end = points[i + 1];

            // Draw main segment
            draw_segment(start, end, 1.0);

            // Random chance to create a branch
            if rng.random_range(0.0..1.0) < BRANCH_CHANCE && i > 2 {
                // Branch goes sideways from this point
                let branch_direction = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                let branch_length = LIGHTNING_HEIGHT * BRANCH_LENGTH_RATIO;
                let branch_segments = (LIGHTNING_SEGMENTS as f32 * BRANCH_LENGTH_RATIO) as usize;

                let mut branch_points: Vec<Vec2> = vec![start];
                let mut branch_x = start.x;

                for j in 1..=branch_segments {
                    let offset = rng.random_range(-LIGHTNING_ZIGZAG * 0.5..LIGHTNING_ZIGZAG * 0.5);
                    branch_x +=
                        branch_direction * (branch_length / branch_segments as f32 * 0.5) + offset;
                    let branch_y = start.y - (j as f32 * (branch_length / branch_segments as f32));
                    branch_points.push(Vec2::new(branch_x, branch_y));
                }

                // Draw branch segments (thinner)
                for j in 0..branch_points.len() - 1 {
                    draw_segment(branch_points[j], branch_points[j + 1], 0.6);
                }
            }
        }

        // Same thunder crack as the BOLT power, heard from where it strikes.
        commands.spawn((
            AudioPlayer::new(asset_server.load("audio/powers/bolt.ogg")),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.75)),
            crate::systems::audio::effects::SoundAt(impact_pos.xy()),
        ));
    }
}

#[derive(Component)]
struct ThunderBolt {
    lifetime: Timer,
}

fn update_thunder_bolts(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut ThunderBolt)>,
) {
    for (entity, mut bolt) in query.iter_mut() {
        bolt.lifetime.tick(time.delta());
        if bolt.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
