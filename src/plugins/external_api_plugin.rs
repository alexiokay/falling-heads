use bevy::prelude::*;
use bevy::scene::SceneRoot;
use crate::resources::command_queue::{PowerCommand, PowerCommandType};
use crate::resources::asset_handles::{GameTextures, ProjectileTextures, DragonAnimation};
use crate::events::power_events::PowerActivated;
use crate::components::hero::Hero;
use crate::components::power_effect::SpeedModifier;
use crate::plugins::powers_plugin::{spawn_tnt, spawn_laser, spawn_arrows, spawn_magic_barrier, spawn_magnet_field};
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
            .add_systems(Update, process_commands)
            .add_systems(Update, update_thunder_bolts);

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
            "THUNDER" => Some(PowerCommandType::Thunder),
            "LOOTBOX" => Some(PowerCommandType::Lootbox),
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
    spatial_hash: Res<crate::resources::spatial_hash::SpatialHash>,
    mut queries: ParamSet<(
        Query<(Entity, &Transform, &crate::components::block::Block)>,
        Query<(Entity, &Transform, &crate::components::enemy::Health), Or<(With<Hero>, With<crate::components::villager::Villager>, With<crate::components::enemy::Enemy>)>>,
    )>,
    camera_query: Query<&Projection, With<Camera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut power_events: MessageWriter<PowerActivated>,
    mut damage_events: MessageWriter<crate::events::damage_events::EntityDamaged>,
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
                    spawn_laser(&mut commands, hero_transform.translation, &mut meshes, &mut materials, &asset_server);
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
                    info!("📦 LOOTBOX reward triggered by {}: {:?}", command.username, selected_reward);
                } else {
                    info!("📦 LOOTBOX already active, skipping request from {}", command.username);
                }
            }
            PowerCommandType::RandomBox => {
                // Spawn a bouncing randombox near the hero in a free area
                if let Ok((_, hero_transform)) = hero_query.single() {
                    use crate::systems::randombox_system::spawn_randombox;
                    use rand::Rng;

                    // Find a safe spawn position by checking blocks
                    let mut rng = rand::thread_rng();
                    let hero_pos = hero_transform.translation;
                    let mut spawn_pos = None;

                    // Try 30 times to find an empty position
                    for _ in 0..30 {
                        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                        let distance = rng.gen_range(420.0..600.0);
                        let offset_x = angle.cos() * distance;
                        let offset_y = angle.sin() * distance;

                        let test_pos = Vec3::new(
                            hero_pos.x + offset_x,
                            hero_pos.y + offset_y + 300.0,
                            10.0,
                        );

                        // Check if this position is clear of blocks
                        let chunk_coords = spatial_hash.world_to_chunk(test_pos.x, test_pos.y);
                        let mut is_blocked = false;

                        'chunk_check: for dx in -1..=1 {
                            for dy in -1..=1 {
                                let check_chunk = (chunk_coords.0 + dx, chunk_coords.1 + dy);
                                if let Some(entities) = spatial_hash.chunks.get(&check_chunk) {
                                    for &entity in entities {
                                        if let Ok((_, block_transform, _)) = queries.p0().get(entity) {
                                            let dist = test_pos.distance(block_transform.translation);
                                            if dist < 96.0 { // BLOCK_SIZE (64) + RANDOMBOX_SIZE (48) / 2
                                                is_blocked = true;
                                                break 'chunk_check;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if !is_blocked {
                            spawn_pos = Some(test_pos);
                            break;
                        }
                    }

                    // Use found position or fallback to above hero
                    let final_pos = spawn_pos.unwrap_or_else(|| Vec3::new(hero_pos.x, hero_pos.y + 400.0, 10.0));

                    spawn_randombox(&mut commands, final_pos, &textures, &mut meshes, &mut materials);
                    info!("🎲 RANDOMBOX spawned by {} at {:?}!", command.username, final_pos);
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

fn spawn_thunder_strikes(
    commands: &mut Commands,
    hero_pos: Vec3,
    spatial_hash: &Res<crate::resources::spatial_hash::SpatialHash>,
    queries: &mut ParamSet<(
        Query<(Entity, &Transform, &crate::components::block::Block)>,
        Query<(Entity, &Transform, &crate::components::enemy::Health), Or<(With<crate::components::hero::Hero>, With<crate::components::villager::Villager>, With<crate::components::enemy::Enemy>)>>,
    )>,
    camera_query: &Query<&Projection, With<Camera>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    damage_events: &mut MessageWriter<crate::events::damage_events::EntityDamaged>,
) {
    use rand::Rng;
    use crate::components::bundles::SpriteBundle3d;
    use crate::utils::constants::BLOCK_SIZE;

    const THUNDER_COUNT: usize = 3;
    const EXPLOSION_RADIUS: f32 = BLOCK_SIZE * 3.0;  // 3 blocks radius for bigger impact
    const LIGHTNING_WIDTH: f32 = 24.0;
    const LIGHTNING_CORE_WIDTH: f32 = 12.0;
    const LIGHTNING_HEIGHT: f32 = 1400.0;
    const LIGHTNING_DURATION: f32 = 0.25;
    const LIGHTNING_SEGMENTS: usize = 14;
    const LIGHTNING_ZIGZAG: f32 = 80.0;  // More dramatic zigzag
    const BRANCH_CHANCE: f32 = 0.3;  // 30% chance per segment to create branch
    const BRANCH_LENGTH_RATIO: f32 = 0.4;  // Branch length compared to main bolt

    let mut rng = rand::thread_rng();

    // Get screen dimensions from camera
    let (screen_width, screen_height) = if let Ok(projection) = camera_query.single() {
        match projection {
            Projection::Orthographic(ortho) => (ortho.area.width(), ortho.area.height()),
            _ => (1920.0, 1080.0),
        }
    } else {
        (1920.0, 1080.0)  // Fallback
    };

    // Instead of targeting specific blocks, spawn randomly across ENTIRE screen (width AND height)
    for _ in 0..THUNDER_COUNT {
        // Random X position across the screen
        let strike_x = hero_pos.x + rng.gen_range(-screen_width / 2.0..screen_width / 2.0);

        // Random Y position across the screen (where the bottom of lightning hits)
        let target_y = hero_pos.y + rng.gen_range(-screen_height / 2.0..screen_height / 2.0);

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
            let offset = rng.gen_range(-LIGHTNING_ZIGZAG..LIGHTNING_ZIGZAG);
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
                if let Ok((entity, transform, _block)) = queries.p0().get(block_entity) {
                    let dist = explosion_pos.distance(transform.translation);
                    if dist <= EXPLOSION_RADIUS {
                        commands.entity(entity).despawn();
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
                    info!("⚡ Thunder explosion damaged character at distance {:.1}", dist);
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
                meshes.add(Rectangle::new(LIGHTNING_CORE_WIDTH * width_scale, length + 4.0)),
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
            if rng.gen_range(0.0..1.0) < BRANCH_CHANCE && i > 2 {
                // Branch goes sideways from this point
                let branch_direction = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
                let branch_length = LIGHTNING_HEIGHT * BRANCH_LENGTH_RATIO;
                let branch_segments = (LIGHTNING_SEGMENTS as f32 * BRANCH_LENGTH_RATIO) as usize;

                let mut branch_points: Vec<Vec2> = vec![start];
                let mut branch_x = start.x;

                for j in 1..=branch_segments {
                    let offset = rng.gen_range(-LIGHTNING_ZIGZAG * 0.5..LIGHTNING_ZIGZAG * 0.5);
                    branch_x += branch_direction * (branch_length / branch_segments as f32 * 0.5) + offset;
                    let branch_y = start.y - (j as f32 * (branch_length / branch_segments as f32));
                    branch_points.push(Vec2::new(branch_x, branch_y));
                }

                // Draw branch segments (thinner)
                for j in 0..branch_points.len() - 1 {
                    draw_segment(branch_points[j], branch_points[j + 1], 0.6);
                }
            }
        }

        // Play thunder sound (using explosion sound)
        commands.spawn((
            AudioPlayer::new(asset_server.load("audio/sfx/explode/explode1.ogg")),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.8)),
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

