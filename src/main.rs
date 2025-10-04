use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;

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
};
use systems::buff_system::BuffPlugin;
use systems::projectile_system::ProjectilePlugin;
use systems::enemy_ai_system::EnemyAIPlugin;
use systems::heart_ui_system::HeartUIPlugin;
use resources::asset_handles::{GameTextures, BlockTextures, ResourceTextures, EnemyTextures, ProjectileTextures, HeartTextures};
use resources::material_cache::MaterialCache;
use resources::mesh_cache::MeshCache;
use states::GameState;
use systems::performance_diagnostics::{PerformanceTracker, log_performance_metrics};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Falling Heads - Minecraft Block Breaker".to_string(),
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins((
            PhysicsPlugin,
            GameplayPlugin,
            PowersPlugin,
            RenderingPlugin,
            UiPlugin,
            AudioPlugin,
            ExternalApiPlugin,
            BuffPlugin,
            ProjectilePlugin,
            EnemyAIPlugin,
            HeartUIPlugin,
        ))
        .init_resource::<MaterialCache>()
        .init_resource::<PerformanceTracker>()
        .add_systems(Startup, setup)
        .add_systems(Update, (initialize_mesh_cache, initialize_material_cache, log_performance_metrics))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Spawn BOTH 2D and 3D cameras for mixed rendering
    // 3D camera - slightly angled top-down with perspective for depth
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0, // Render 3D blocks first
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

    // No separate 2D camera - everything renders in 3D world space

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

    // Load textures
    let textures = GameTextures {
        hero: asset_server.load("textures/entities/hero.jpg"),
        tnt: asset_server.load("textures/effects/tnt.png"),
        nether_bg: asset_server.load("textures/blocks/nether/nether.png"),
        villager: asset_server.load("textures/entities/villager.png"),
    };

    commands.insert_resource(textures);

    // Load block textures
    let block_textures = BlockTextures {
        ancient_debris_top: asset_server.load("textures/blocks/nether/ancient_debris_top.png"),
        ancient_debris_side: asset_server.load("textures/blocks/nether/ancient_debris_side.png"),
        netherrack: asset_server.load("textures/blocks/nether/netherrack.png"),
        obsidian: asset_server.load("textures/blocks/nether/obsidian.png"),
        gold_block: asset_server.load("textures/blocks/nether/gold_block.png"),
    };

    commands.insert_resource(block_textures);

    // Load resource textures
    let resource_textures = ResourceTextures {
        coal: asset_server.load("textures/resources/coal.png"),
        diamond: asset_server.load("textures/resources/diamond.png"),
        emerald: asset_server.load("textures/resources/diamond.png"), // Using diamond as fallback for emerald
        gold_ingot: asset_server.load("textures/resources/gold_ingot.png"),
        iron_ingot: asset_server.load("textures/resources/iron_ingot.png"),
        redstone: asset_server.load("textures/resources/redstone.png"),
        netherite_ingot: asset_server.load("textures/resources/netherite_ingot.png"),
    };

    commands.insert_resource(resource_textures);

    // Load enemy textures
    let enemy_textures = EnemyTextures {
        skeleton: asset_server.load("textures/entities/enemy/skeleton.png"),
        skeleton2: asset_server.load("textures/entities/enemy/skeleton2.png"),
    };

    commands.insert_resource(enemy_textures);

    // Load projectile textures
    let projectile_textures = ProjectileTextures {
        arrow: asset_server.load("textures/projectiles/arrow.png"),
    };

    commands.insert_resource(projectile_textures);

    // Load heart textures
    let heart_textures = HeartTextures {
        container: asset_server.load("textures/effects/heart/container.png"),
        half: asset_server.load("textures/effects/heart/half.png"),
        full: asset_server.load("textures/effects/heart/full.png"),
    };

    commands.insert_resource(heart_textures);

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
) {
    // Only run once
    if !cache.materials.is_empty() {
        return;
    }

    // Wait for BlockTextures resource to be available
    let Some(block_textures) = block_textures else {
        return;
    };

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
