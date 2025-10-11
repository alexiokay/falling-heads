use bevy::prelude::*;
use bevy::scene::SceneRoot;
use crate::resources::command_queue::{PowerCommand, PowerCommandType};
use crate::resources::asset_handles::{GameTextures, ProjectileTextures, DragonAnimation};
use crate::events::power_events::PowerActivated;
use crate::components::hero::Hero;
use crate::components::power_effect::SpeedModifier;
use crate::plugins::powers_plugin::{spawn_tnt, spawn_laser, spawn_arrows, spawn_magic_barrier};
use crate::systems::entity_spawn::spawn_villagers_around_position;
use crate::components::enemy::{Enemy, EnemyType, Health, Dragon, DragonState, Flying, EnemyBehavior, DragonAnimationClip};
use crate::components::bundles::PhysicsBundle;
use crate::resources::asset_handles::EnemyModels;
use crate::components::collision_layers::GameLayer;
use crate::utils::constants::{CHARACTER_MASS, ENEMY_BOUNCE};
use avian3d::prelude::*;

pub struct ExternalApiPlugin;

// Global channel for commands (simplified approach)
use std::sync::{Arc, Mutex, OnceLock};
use std::collections::VecDeque;

static COMMAND_QUEUE: OnceLock<Arc<Mutex<VecDeque<PowerCommand>>>> = OnceLock::new();

/// Get access to the global command queue (for dev panel and other systems)
pub fn get_command_queue() -> Option<Arc<Mutex<VecDeque<PowerCommand>>>> {
    COMMAND_QUEUE.get().cloned()
}

impl Plugin for ExternalApiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_message::<PowerActivated>()
            .add_systems(Startup, start_api_server)
            .add_systems(Update, process_commands);

        info!("ExternalApiPlugin loaded - API will start on http://localhost:3000");
    }
}

fn start_api_server() {
    // Initialize the global queue
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    COMMAND_QUEUE.set(queue.clone()).ok();

    use axum::{
        routing::post,
        Router, Json,
        extract::State,
        response::IntoResponse,
        http::StatusCode,
    };
    use serde::{Deserialize, Serialize};
    use tokio::runtime::Runtime;

    #[derive(Deserialize)]
    struct CommandRequest {
        action: String,
        #[serde(default = "default_username")]
        username: String,
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
            "TNT" => Some(PowerCommandType::Tnt),
            "FAST" => Some(PowerCommandType::Fast),
            "SLOW" => Some(PowerCommandType::Slow),
            "FUSS" => Some(PowerCommandType::Fuss),
            "BOLT" => Some(PowerCommandType::Bolt),
            "ARROW" => Some(PowerCommandType::Arrow),
            "LASER" => Some(PowerCommandType::Laser),
            "GHAST" => Some(PowerCommandType::Ghast),
            "BOX" => Some(PowerCommandType::Box),
            "MAGNET" => Some(PowerCommandType::Magnet),
            "VILLAGERS" => Some(PowerCommandType::Villagers),
            "MIRROR" => Some(PowerCommandType::Mirror),
            "DRAGON" => Some(PowerCommandType::Dragon),
            _ => None,
        };

        if let Some(cmd_type) = command_type {
            let power_cmd = PowerCommand::new(cmd_type, cmd.username.clone());
            if let Ok(mut q) = queue.lock() {
                q.push_back(power_cmd);
                (
                    StatusCode::OK,
                    Json(CommandResponse {
                        success: true,
                        message: format!("Power '{}' queued for user '{}'!", cmd.action, cmd.username),
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
                    message: format!("Unknown command: {}. Available: TNT, FAST, SLOW, LASER, ARROW, BOLT, GHAST, BOX, MAGNET, FUSS, VILLAGERS, MIRROR", cmd.action),
                }),
            )
        }
    }

    let app_router = Router::new()
        .route("/command", post(handle_command))
        .with_state(queue.clone());

    // Spawn API server in background thread
    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:3029")
                .await
                .expect("Failed to bind to port 3029");
            println!("🌐 API Server running on http://127.0.0.1:3029");
            println!("📡 Try: curl -X POST http://localhost:3029/command -H \"Content-Type: application/json\" -d '{{\"action\":\"TNT\",\"username\":\"viewer123\"}}'");
            axum::serve(listener, app_router).await.unwrap();
        });
    });
}

fn process_commands(
    mut commands: Commands,
    hero_query: Query<(Entity, &Transform), With<Hero>>,
    textures: Res<GameTextures>,
    projectile_textures: Option<Res<ProjectileTextures>>,
    enemy_models: Res<EnemyModels>,
    dragon_animation: Option<Res<DragonAnimation>>,
    asset_server: Res<AssetServer>,
    block_atlases: Option<Res<crate::resources::asset_handles::BlockAtlases>>,
    mesh_cache: Option<Res<crate::resources::mesh_cache::MeshCache>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut power_events: MessageWriter<PowerActivated>,
) {
    let Some(queue) = COMMAND_QUEUE.get() else {
        return;
    };

    let Ok(mut q) = queue.lock() else {
        return;
    };

    while let Some(command) = q.pop_front() {
        power_events.write(PowerActivated { command: command.clone() });

        // Execute power immediately
        match command.command_type {
            PowerCommandType::Tnt => {
                if let (Ok((_, hero_transform)), Some(atlases), Some(mesh_cache)) =
                    (hero_query.single(), block_atlases.as_ref(), mesh_cache.as_ref()) {
                    let spawn_pos = hero_transform.translation + Vec3::new(0.0, 50.0, 0.0);  // 2D scale
                    spawn_tnt(&mut commands, spawn_pos, &atlases, &mesh_cache, &mut meshes, &mut materials, Some(command.username.clone()), Some(&asset_server));
                    info!("🧨 TNT power activated by {}!", command.username);
                }
            }
            PowerCommandType::Fast => {
                if let Ok((hero_entity, _)) = hero_query.single() {
                    commands.entity(hero_entity).insert(SpeedModifier {
                        multiplier: 2.0,
                        duration: Timer::from_seconds(5.0, TimerMode::Once),
                    });
                    info!("⚡ FAST power activated by {}! Speed x2 for 5 seconds", command.username);
                }
            }
            PowerCommandType::Slow => {
                if let Ok((hero_entity, _)) = hero_query.single() {
                    commands.entity(hero_entity).insert(SpeedModifier {
                        multiplier: 0.5,
                        duration: Timer::from_seconds(5.0, TimerMode::Once),
                    });
                    info!("🐌 SLOW power activated by {}! Speed x0.5 for 5 seconds", command.username);
                }
            }
            PowerCommandType::Laser => {
                if let Ok((_, hero_transform)) = hero_query.single() {
                    spawn_laser(&mut commands, hero_transform.translation, &mut meshes, &mut materials);
                    info!("💜 LASER power activated by {}!", command.username);
                }
            }
            PowerCommandType::Arrow => {
                if let Ok((hero_entity, _)) = hero_query.single() {
                    if let Some(ref projectile_textures) = projectile_textures {
                        spawn_arrows(&mut commands, hero_entity, &mut meshes, &mut materials, projectile_textures);
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
                        &mut meshes,
                        &mut materials,
                    );
                    info!("🪞 MIRROR barrier activated by {}!", command.username);
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
            PowerCommandType::Dragon => {
                if let Ok((_, hero_transform)) = hero_query.single() {
                    if let Some(dragon_anim) = dragon_animation.as_ref() {
                        spawn_dragon(
                            &mut commands,
                            hero_transform.translation + Vec3::new(300.0, 200.0, 0.0), // Spawn above and to the right
                            &enemy_models,
                            dragon_anim,
                            &mut meshes,
                            &mut materials,
                        );
                        info!("🐉 DRAGON spawned by {}!", command.username);
                    }
                }
            }
            _ => {
                info!("⚡ Power {:?} activated by {} (not yet implemented)", command.command_type, command.username);
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
        Flying,  // Ghasts float and ignore block collisions
        Health::new(EnemyType::Ghast.max_health()),
        // Custom physics for flying - no gravity!
        RigidBody::Dynamic,
        LinearVelocity(Vec3::ZERO),
        GravityScale(0.0),  // No gravity - ghasts float
        LinearDamping(2.0),  // Slow down naturally - allows Avian collision response to work
        Mass(CHARACTER_MASS),
        Collider::cuboid(80.0, 80.0, 80.0),  // 160x160 collider - matches 2x pattern used by other entities (prevents overlap)
        Restitution::new(ENEMY_BOUNCE),  // Same bounce as other enemies
        Friction::new(0.0),
        LockedAxes::ROTATION_LOCKED,
        CollisionLayers::new(GameLayer::Enemy, GameLayer::Enemy.filters()),
        CollisionEventsEnabled,
        SpriteBundle3d::new(
            meshes.add(Rectangle::new(80.0, 80.0)),  // Bigger size
            materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(ghast_texture),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            position,
        ),
    ));

    info!("👻 Ghast spawned at {:?}", position);
}

fn spawn_dragon(
    commands: &mut Commands,
    position: Vec3,
    enemy_models: &Res<EnemyModels>,
    dragon_animation: &DragonAnimation,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Spawn dragon in front of camera to avoid background clipping
    // Keep hero's Y (height), but move forward in Z and offset in X
    let spawn_pos = Vec3::new(position.x + 400.0, position.y, 200.0);

    // First target: fly to the left side of the screen
    let target_position = Vec3::new(position.x - 600.0, position.y + 300.0, 200.0);

    // Spawn dragon boss enemy using GLB model with animation
    commands.spawn((
        Enemy::new(EnemyType::Dragon),
        Dragon {
            state: DragonState::Emerging,
            state_timer: 0.0,
            hover_offset: 0.0,
            hover_timer: 0.0,
            target_position,
            spawn_position: position,
        },
        Flying,  // Dragons ignore block collisions
        Health::new(EnemyType::Dragon.max_health()),
        PhysicsBundle::dragon(),
        SceneRoot(enemy_models.dragon.clone()),
        AnimationPlayer::default(),
        DragonAnimationClip(dragon_animation.fly.clone()),  // Store animation clip handle
        Transform::from_translation(spawn_pos)
            .with_scale(Vec3::new(80.0, 80.0, 80.0))  // Much bigger!
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        Visibility::Visible,
    ));
}

