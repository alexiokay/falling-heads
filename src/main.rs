use bevy::prelude::*;
use bevy::window::{WindowResolution, PresentMode};
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::winit::WinitSettings;
use avian3d::{math::*, prelude::*};
use std::time::{Duration, Instant};

mod components;
mod resources;
mod systems;
mod events;
mod plugins;
mod api;
mod utils;
mod config;
mod states;

use plugins::{
    physics_plugin::PhysicsPlugin,
    gameplay_plugin::GameplayPlugin,
    powers_plugin::PowersPlugin,
    rendering_plugin::RenderingPlugin,
    ui_plugin::UiPlugin,
    audio_plugin::AudioPlugin,
    external_api_plugin::ExternalApiPlugin,
    asset_loader_plugin::AssetLoaderPlugin,
};
// Buff systems
use systems::buffs::buff_core::BuffPlugin;
use systems::buffs::frozen_visual::FrozenPlugin;
use systems::buffs::burning_visual::BurningVisualPlugin;

// Weather systems
use systems::weather::weather_core::WeatherSystemPlugin;
use systems::weather::snow_accumulation::SnowAccumulationPlugin;

// UI systems
use systems::ui::heart_ui::HeartUIPlugin;
use systems::ui::username_label::UsernameLabelPlugin;
use systems::ui::dev_panel::DevPanelPlugin;

// Other systems
use systems::projectile_system::ProjectilePlugin;
use systems::enemy_ai_system::EnemyAIPlugin;
use systems::damage_visual_system::DamageVisualPlugin;
use systems::block_damage_overlay_system::BlockDamageOverlayPlugin;
use systems::death_particle_system::DeathParticlePlugin;
use resources::asset_handles::BlockTextures;
use resources::material_cache::MaterialCache;
use resources::mesh_cache::MeshCache;
use resources::settings::GameSettings;
use resources::world_config::WorldConfig;
use states::GameState;
use systems::performance_diagnostics::{PerformanceTracker, log_performance_metrics};
use utils::system_sets::configure_system_sets;

fn main() {
    let mut app = App::new();
    let settings = GameSettings::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Falling Heads - Minecraft Block Breaker".to_string(),
                resolution: WindowResolution::new(1280, 720),
                present_mode: PresentMode::AutoNoVsync, // Disable VSync for uncapped FPS and smooth movement
                ..default()
            }),
            ..default()
        }))
        .insert_resource(WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Continuous,
            unfocused_mode: bevy::winit::UpdateMode::Continuous, // Keep rendering when unfocused
        })
        .insert_resource(FpsLimiter::new())
        .init_state::<GameState>()
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        // Enable interpolation for ALL rigid bodies by default for smooth movement at any framerate
        .add_plugins(PhysicsPlugins::default()
            .with_length_unit(64.0)
            .build()
            .disable::<PhysicsInterpolationPlugin>()
            .add(PhysicsInterpolationPlugin::interpolate_all()))
        .insert_resource(Gravity(Vector::NEG_Y * 500.0))  // -500 gravity (matching old constant)
        .insert_resource(SubstepCount(1))  // Reduce substeps from default 6 to 1 for performance
        .add_systems(OnEnter(GameState::Paused), pause_physics)
        .add_systems(OnEnter(GameState::Playing), resume_physics)
        .add_plugins(AssetLoaderPlugin)  // Load assets first
        .add_plugins((
            PhysicsPlugin,  // Keep custom physics plugin for migration
            GameplayPlugin,
            PowersPlugin,
            RenderingPlugin,
            UiPlugin,
            AudioPlugin,
            ExternalApiPlugin,
            BuffPlugin,
        ))
        .add_plugins((
            ProjectilePlugin,
            EnemyAIPlugin,
            HeartUIPlugin,
            UsernameLabelPlugin,
            DamageVisualPlugin,
            BlockDamageOverlayPlugin,
            WeatherSystemPlugin,
            FrozenPlugin,
            DevPanelPlugin,
            DeathParticlePlugin,
            SnowAccumulationPlugin,
            BurningVisualPlugin,
        ))
        .init_resource::<MaterialCache>()
        .init_resource::<PerformanceTracker>()
        .init_resource::<WorldConfig>()
        .init_resource::<resources::command_queue::CommandQueue>()
        .insert_resource(settings)
        .add_systems(Startup, setup)
        .add_systems(Update, initialize_mesh_cache)
        .add_systems(Update, initialize_material_cache)
        .add_systems(Update, log_performance_metrics)
        .add_systems(Update, apply_fps_limit);

    // Configure system execution order
    configure_system_sets(&mut app);

    app.run();
}

/// Pause Avian physics when entering Paused state
fn pause_physics(mut time: ResMut<Time<Physics>>) {
    time.pause();
}

/// Resume Avian physics when entering Playing state
fn resume_physics(mut time: ResMut<Time<Physics>>) {
    time.unpause();
}

/// FPS Limiter resource - tracks frame timing for manual FPS limiting
#[derive(Resource)]
struct FpsLimiter {
    last_frame: Instant,
}

impl FpsLimiter {
    fn new() -> Self {
        Self {
            last_frame: Instant::now(),
        }
    }
}

/// Apply FPS limit based on settings
/// VSync takes priority - when enabled, FPS limiter is disabled
fn apply_fps_limit(
    settings: Res<GameSettings>,
    mut limiter: ResMut<FpsLimiter>,
) {
    // Skip if VSync is enabled (let VSync handle frame limiting)
    if settings.vsync_enabled {
        limiter.last_frame = Instant::now();
        return;
    }

    // Skip if no FPS limit (unlimited)
    if settings.fps_limit == 0 {
        limiter.last_frame = Instant::now();
        return;
    }

    // Calculate target frame time
    let target_frame_time = Duration::from_secs_f64(1.0 / settings.fps_limit as f64);
    let elapsed = limiter.last_frame.elapsed();

    // Sleep if we're ahead of schedule
    if elapsed < target_frame_time {
        let sleep_time = target_frame_time - elapsed;
        std::thread::sleep(sleep_time);
    }

    limiter.last_frame = Instant::now();
}

fn setup(mut commands: Commands) {
    // Spawn 3D camera - slightly angled top-down with perspective for depth
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0, // Render 3D world first
            clear_color: ClearColorConfig::Custom(Color::srgb(0.1, 0.1, 0.15)),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: std::f32::consts::FRAC_PI_4,  // 45 degree field of view
            ..default()
        }),
        Transform::from_xyz(30.0, 80.0, 1500.0)  // Far back to see large 64x64x64 blocks
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    // Spawn 2D camera for Text2d rendering
    commands.spawn((
        Camera2d,
        Camera {
            order: 1, // Render after 3D
            clear_color: ClearColorConfig::None, // Don't clear 3D scene
            ..default()
        },
    ));

    // Add ambient light for 3D blocks
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 1500.0,
        affects_lightmapped_meshes: false,
    });

    // Add directional light for 3D depth
    commands.spawn((
        DirectionalLight {
            illuminance: 25000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, -0.5, 0.0)),
    ));

    info!("Falling Heads initialized with mixed 2D/3D isometric view!");
}

/// Initialize shared mesh for all blocks - CRITICAL PERFORMANCE FIX
/// All blocks share the same 64x64 mesh to prevent mesh asset accumulation
fn initialize_mesh_cache(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    cache: Option<Res<MeshCache>>,
) {
    // Only run once
    if cache.is_some() {
        return;
    }

    let mesh_cache = MeshCache::new(&mut meshes);
    commands.insert_resource(mesh_cache);
    info!("Mesh cache initialized with shared block mesh");
}

/// Initialize shared materials for all block types to reduce memory usage
/// Runs in Update to ensure textures are loaded first
fn initialize_material_cache(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<MaterialCache>,
    block_textures: Option<Res<BlockTextures>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Only run once
    if !cache.materials.is_empty() {
        return;
    }

    // Wait for BlockTextures resource to be available
    let Some(block_textures) = block_textures else {
        return;
    };

    // Wait for all textures to load before generating atlases
    let pumpkin_top = images.get(&block_textures.pumpkin_top);
    let pumpkin_side = images.get(&block_textures.pumpkin_side);
    let pumpkin_front = images.get(&block_textures.pumpkin_front);
    let oak_log = images.get(&block_textures.oak_log);
    let oak_log_top = images.get(&block_textures.oak_log_top);
    let oak_leaves = images.get(&block_textures.oak_leaves);
    let melon_top = images.get(&block_textures.melon_top);
    let melon_side = images.get(&block_textures.melon_side);
    let tnt_top = images.get(&block_textures.tnt_top);
    let tnt_bottom = images.get(&block_textures.tnt_bottom);
    let tnt_side = images.get(&block_textures.tnt_side);
    let cactus_top = images.get(&block_textures.cactus_top);
    let cactus_bottom = images.get(&block_textures.cactus_bottom);
    let cactus_side = images.get(&block_textures.cactus_side);

    if pumpkin_top.is_none() || pumpkin_side.is_none() || pumpkin_front.is_none()
        || oak_log.is_none() || oak_log_top.is_none() || oak_leaves.is_none()
        || melon_top.is_none() || melon_side.is_none()
        || tnt_top.is_none() || tnt_bottom.is_none() || tnt_side.is_none()
        || cactus_top.is_none() || cactus_bottom.is_none() || cactus_side.is_none() {
        return; // Textures not loaded yet, wait for next frame
    }

    // Generate atlases inline here
    let pumpkin_atlas = crate::systems::atlas_generator::create_atlas_4x1(
        pumpkin_top.unwrap(),
        pumpkin_top.unwrap(),
        pumpkin_side.unwrap(),
        pumpkin_front.unwrap(),
    );

    let oak_log_atlas = crate::systems::atlas_generator::create_atlas_4x1(
        oak_log_top.unwrap(),
        oak_log_top.unwrap(),
        oak_log.unwrap(),
        oak_log.unwrap(),
    );

    let oak_leaves_atlas = crate::systems::atlas_generator::create_atlas_4x1(
        oak_leaves.unwrap(),
        oak_leaves.unwrap(),
        oak_leaves.unwrap(),
        oak_leaves.unwrap(),
    );

    let melon_atlas = crate::systems::atlas_generator::create_atlas_4x1(
        melon_top.unwrap(),
        melon_top.unwrap(),
        melon_side.unwrap(),
        melon_side.unwrap(),
    );

    let tnt_atlas = crate::systems::atlas_generator::create_atlas_4x1(
        tnt_top.unwrap(),
        tnt_bottom.unwrap(),
        tnt_side.unwrap(),
        tnt_side.unwrap(),
    );

    // Create sandstone atlas (top, bottom, side)
    let sandstone_top = images.get(&block_textures.sandstone_top);
    let sandstone_bottom = images.get(&block_textures.sandstone_bottom);
    let sandstone_side = images.get(&block_textures.sandstone);

    let sandstone_atlas = crate::systems::atlas_generator::create_atlas_4x1(
        sandstone_top.unwrap(),
        sandstone_bottom.unwrap(),
        sandstone_side.unwrap(),
        sandstone_side.unwrap(),
    );

    // Create cactus atlas (top, bottom, side)
    let cactus_atlas = crate::systems::atlas_generator::create_atlas_4x1(
        cactus_top.unwrap(),
        cactus_bottom.unwrap(),
        cactus_side.unwrap(),
        cactus_side.unwrap(),
    );

    let pumpkin_atlas_handle = images.add(pumpkin_atlas);
    let oak_log_atlas_handle = images.add(oak_log_atlas);
    let oak_leaves_atlas_handle = images.add(oak_leaves_atlas);
    let melon_atlas_handle = images.add(melon_atlas);
    let tnt_atlas_handle = images.add(tnt_atlas);
    let sandstone_atlas_handle = images.add(sandstone_atlas);
    let cactus_atlas_handle = images.add(cactus_atlas);

    info!("Generated block atlases: pumpkin, oak_log, oak_leaves, melon, tnt, sandstone, cactus");

    // Store atlases as a resource
    commands.insert_resource(crate::resources::asset_handles::BlockAtlases {
        pumpkin: pumpkin_atlas_handle.clone(),
        oak_log: oak_log_atlas_handle.clone(),
        oak_leaves: oak_leaves_atlas_handle.clone(),
        melon: melon_atlas_handle.clone(),
        tnt: tnt_atlas_handle,
        sandstone: sandstone_atlas_handle.clone(),
        cactus: cactus_atlas_handle.clone(),
    });

    use crate::components::block::BlockType;
    use crate::systems::world_generation::get_block_color;

    info!("Initializing material cache with {} block types...", 29);

    // Create one material per block type
    let block_types = [
        BlockType::Stone, BlockType::Dirt, BlockType::Grass, BlockType::Sand,
        BlockType::Sandstone, BlockType::Snow, BlockType::Coal, BlockType::Iron,
        BlockType::Gold, BlockType::Diamond, BlockType::Emerald, BlockType::Redstone,
        BlockType::Wood, BlockType::Leaves, BlockType::Pumpkin, BlockType::Melon,
        BlockType::Cactus, BlockType::DeadBush, BlockType::Mushroom, BlockType::Vines,
        BlockType::Netherrack, BlockType::Netherite, BlockType::AncientDebris,
        BlockType::SoulSand, BlockType::Glowstone, BlockType::EndStone,
        BlockType::Obsidian, BlockType::ChorusFruit, BlockType::Purpur,
    ];

    for block_type in block_types {
        let material = match block_type {
            BlockType::AncientDebris => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.ancient_debris_side.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Netherrack => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.netherrack.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Obsidian => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.obsidian.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Gold => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.gold_block.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Leaves => {
                info!("Creating Leaves material with atlas handle: {:?}", oak_leaves_atlas_handle);
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.4, 0.8, 0.4),  // Tint leaves green (Minecraft-style biome tinting)
                    base_color_texture: Some(oak_leaves_atlas_handle.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,  // Support transparency for leaf gaps
                    cull_mode: None,  // Render both sides for better leaf appearance
                    double_sided: true,
                    ..default()
                })
            }
            BlockType::Wood => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(oak_log_atlas_handle.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    cull_mode: None,
                    double_sided: true,
                    ..default()
                })
            }
            BlockType::Pumpkin => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(pumpkin_atlas_handle.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    cull_mode: None,
                    double_sided: true,
                    ..default()
                })
            }
            BlockType::Melon => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(melon_atlas_handle.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    cull_mode: None,
                    double_sided: true,
                    ..default()
                })
            }
            BlockType::Grass => {
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.4, 0.8, 0.4),  // Green tint for leaves
                    base_color_texture: Some(oak_leaves_atlas_handle.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    cull_mode: None,
                    double_sided: true,
                    ..default()
                })
            }
            BlockType::Dirt => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.dirt.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Stone => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.stone.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Coal => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.coal_ore.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Iron => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.coal_ore.clone()),  // Reusing coal texture for iron (no iron_ore texture)
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Diamond => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.diamond_ore.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Emerald => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.emerald_ore.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Redstone => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.coal_ore.clone()),  // Reusing coal texture for redstone (no redstone_ore texture)
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Sand => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.sand.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Sandstone => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(sandstone_atlas_handle.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    cull_mode: None,
                    double_sided: true,
                    ..default()
                })
            }
            BlockType::EndStone => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.end_stone.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Snow => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.snow.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Cactus => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(cactus_atlas_handle.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    cull_mode: None,
                    double_sided: true,
                    ..default()
                })
            }
            BlockType::SoulSand => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.soul_sand.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Netherite => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.netherite_block.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            BlockType::Glowstone => {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(block_textures.glowstone.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
            _ => {
                materials.add(StandardMaterial {
                    base_color: get_block_color(block_type),
                    unlit: true,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            }
        };

        cache.materials.insert(block_type, material);
    }

    // CRITICAL PERFORMANCE FIX: Initialize resource materials too (prevents unique material creation)
    use crate::components::resource::ResourceType;

    let resource_types = [
        ResourceType::Coal, ResourceType::Iron, ResourceType::Gold,
        ResourceType::Diamond, ResourceType::Emerald, ResourceType::Redstone,
        ResourceType::Netherite,
    ];

    for resource_type in resource_types {
        let resource_color = match resource_type {
            ResourceType::Coal => Color::srgb(0.2, 0.2, 0.2),
            ResourceType::Iron => Color::srgb(0.8, 0.8, 0.8),
            ResourceType::Gold => Color::srgb(1.0, 0.84, 0.0),
            ResourceType::Diamond => Color::srgb(0.0, 0.8, 1.0),
            ResourceType::Emerald => Color::srgb(0.0, 1.0, 0.3),
            ResourceType::Redstone => Color::srgb(1.0, 0.0, 0.0),
            ResourceType::Netherite => Color::srgb(0.27, 0.19, 0.19),
        };

        let material = materials.add(StandardMaterial {
            base_color: resource_color,
            emissive: (resource_color.to_linear() * 0.3).into(),
            perceptual_roughness: 0.3,
            metallic: 0.5,
            ..default()
        });

        cache.resource_materials.insert(resource_type, material);
    }

    info!("Material cache initialized with {} block materials and {} resource materials",
          cache.materials.len(), cache.resource_materials.len());
}
