use avian3d::prelude::*;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use bevy::winit::WinitSettings;
use std::time::{Duration, Instant};

mod api;
mod components;
mod config;
mod diagnostics;
mod events;
#[cfg(feature = "heap-stats")]
mod heap_stats;
mod plugins;
mod resources;
mod smoke_test;
mod states;
mod systems;
mod utils;
mod world_preview;
// Test and probe launch modes, left out of player builds (--no-default-features)
#[cfg(feature = "probes")]
mod character_probe;
#[cfg(feature = "probes")]
mod combo_probe;
#[cfg(feature = "probes")]
mod crowd_probe;
#[cfg(feature = "probes")]
mod depth_probe;
#[cfg(feature = "probes")]
mod ecology_probe;
#[cfg(feature = "probes")]
mod fast_motion_probe;
#[cfg(feature = "probes")]
mod ghost_floor_probe;
#[cfg(feature = "probes")]
mod head_size_probe;
#[cfg(feature = "probes")]
mod memory_probe;
#[cfg(feature = "probes")]
mod night_probe;
#[cfg(feature = "probes")]
mod overhaul_probe;
#[cfg(feature = "probes")]
mod pack_switch_probe;
#[cfg(feature = "probes")]
mod polish_probe;
#[cfg(feature = "probes")]
mod powers_probe;
#[cfg(feature = "probes")]
mod refinement_probe;
#[cfg(feature = "probes")]
mod snow_ghast_probe;
#[cfg(feature = "probes")]
mod ui_smoke;
#[cfg(feature = "probes")]
mod visual_probe;
#[cfg(feature = "probes")]
mod water_probe;

use plugins::{
    asset_loader_plugin::AssetLoaderPlugin, audio_plugin::AudioPlugin,
    external_api_plugin::ExternalApiPlugin, gameplay_plugin::GameplayPlugin,
    physics_plugin::PhysicsPlugin, powers_plugin::PowersPlugin, rendering_plugin::RenderingPlugin,
    ui_plugin::UiPlugin,
};
// Buff systems
use systems::buffs::buff_core::BuffPlugin;
use systems::buffs::burning_visual::BurningVisualPlugin;
use systems::buffs::frozen_visual::FrozenPlugin;
use systems::buffs::status_effects::StatusEffectsPlugin;

// Weather systems
use systems::weather::snow_accumulation::SnowAccumulationPlugin;
use systems::weather::weather_core::WeatherSystemPlugin;

// UI systems
use systems::ui::commands_panel::CommandsPanelPlugin;
use systems::ui::dev_panel::DevPanelPlugin;
use systems::ui::heart_ui::HeartUIPlugin;
use systems::ui::username_label::UsernameLabelPlugin;
use systems::ui::xp_bar::XpBarUIPlugin;

// Progression systems
use systems::progression::experience::ExperiencePlugin;

// Other systems
use resources::asset_handles::BlockTextures;
use resources::material_cache::MaterialCache;
use resources::mesh_cache::MeshCache;
use resources::settings::GameSettings;
use resources::world_config::WorldConfig;
use states::GameState;
use systems::block_damage_overlay_system::BlockDamageOverlayPlugin;
use systems::collision_debug_system::CollisionDebugPlugin;
use systems::damage_visual_system::DamageVisualPlugin;
use systems::death_particle_system::DeathParticlePlugin;
use systems::enemy_ai_system::EnemyAIPlugin;
use systems::lootbox_system::LootboxPlugin;
use systems::performance_diagnostics::{
    PerformanceTracker, log_memory_metrics, log_performance_metrics,
};
use systems::projectile_system::ProjectilePlugin;
use systems::random_events::RandomEventPlugin;
use systems::randombox_system::RandomBoxPlugin;

fn main() {
    if std::env::args().any(|a| a == "--export-world-preview") {
        if let Err(e) = world_preview::run() {
            eprintln!("World preview: {e}");
            std::process::exit(1);
        }
        return;
    }
    let session = diagnostics::Session::start()
        .map_err(|e| eprintln!("Cannot create diagnostics: {e}"))
        .ok();
    let mut app = App::new();
    app.set_error_handler(command_errors_warn);
    if let Some(session) = &session {
        app.insert_resource(session.clone());
    }
    #[cfg(feature = "probes")]
    if std::env::args().any(|a| a == "--diagnostics-test-clean" || a == "--diagnostics-test-panic")
    {
        app.add_plugins(bevy::log::LogPlugin {
            custom_layer: diagnostics::layer,
            ..default()
        });
        info!("Diagnostics self-test log entry");
        if std::env::args().any(|a| a == "--diagnostics-test-panic") {
            panic!("Intentional diagnostics self-test panic");
        }
        if let Some(session) = session {
            session.finish(0);
        }
        return;
    }
    #[cfg(target_os = "windows")]
    systems::frame_generation::prepare(&mut app);
    // Player settings from the last session (test and probe runs use the defaults)
    let mut settings = if resources::settings::persistence_enabled() {
        GameSettings::load()
    } else {
        GameSettings::new()
    };
    // Test, probe and benchmark runs (or FALLING_HEADS_QUIET=1): music off, effects
    // at 5%, so automated checks are not loud.
    let quiet = !resources::settings::persistence_enabled()
        || std::env::var("FALLING_HEADS_QUIET").is_ok_and(|v| v != "0");
    if quiet {
        settings.music_volume = 0.0; // Music fully off in test runs
        settings.effects_volume = 0.05;
    }
    let smoke_test = std::env::args().any(|arg| {
        arg == "--smoke-test"
            || arg == "--vfx-smoke-test"
            || arg == "--ui-smoke-test"
            || arg == "--gpu-smoke-test"
            || arg == "--fg-smoke-test"
            || arg == "--visual-smoke-test"
    });

    // Theme packs: must be registered before DefaultPlugins adds the AssetPlugin.
    systems::asset_packs::install(&mut app);
    systems::native_gpu::prepare(&mut app);
    app.add_plugins(
        DefaultPlugins
            .set(bevy::log::LogPlugin {
                custom_layer: diagnostics::layer,
                ..default()
            })
            .set(systems::native_gpu::render_plugin())
            // Quiet runs also scale every sound, including ones without a volume setting.
            .set(bevy::audio::AudioPlugin {
                global_volume: bevy::audio::GlobalVolume::new(bevy::audio::Volume::Linear(
                    if quiet { 0.05 } else { 1.0 },
                )),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Falling Heads".to_string(),
                    visible: !smoke_test || std::env::args().any(|a| a == "--fg-visible-test"),
                    resolution: WindowResolution::new(720, 1280),
                    present_mode: if settings.vsync_enabled {
                        PresentMode::AutoVsync
                    } else {
                        PresentMode::AutoNoVsync
                    }, // Saved VSync choice (off by default; FPS limiter instead)
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(WinitSettings {
        focused_mode: bevy::winit::UpdateMode::Continuous,
        unfocused_mode: bevy::winit::UpdateMode::Continuous, // Keep rendering when unfocused
    })
    .insert_resource(FpsLimiter::new())
    .init_state::<GameState>()
    .add_computed_state::<states::InGame>()
    .add_plugins(FrameTimeDiagnosticsPlugin::default())
    // Enable interpolation for ALL rigid bodies by default for smooth movement at any framerate
    .add_plugins(
        PhysicsPlugins::default()
            .with_length_unit(64.0)
            .build()
            .disable::<PhysicsInterpolationPlugin>()
            .add(PhysicsInterpolationPlugin::interpolate_all()),
    )
    .insert_resource(Gravity(Vec3::NEG_Y * 500.0)) // -500 gravity (matching old constant)
    .insert_resource(SubstepCount(1)) // Reduce substeps from default 6 to 1 for performance
    .add_systems(OnEnter(GameState::Paused), pause_physics)
    .add_systems(OnEnter(GameState::Prison), pause_physics)
    .add_systems(OnEnter(GameState::Playing), resume_physics)
    .add_plugins(systems::biome_teleport::BiomeTeleportPlugin)
    .add_plugins(systems::biome_mist::BiomeMistPlugin)
    .add_plugins(systems::world_portals::WorldPortalPlugin)
    .add_plugins(systems::launcher::LauncherPlugin)
    .add_plugins(systems::spiders::SpiderPlugin)
    .add_plugins(systems::bats::BatPlugin)
    .add_plugins(systems::play_styles::PlayStylePlugin)
    .add_plugins(systems::game_modes::GameModesPlugin)
    .add_plugins(systems::asset_packs::AssetPacksPlugin)
    .add_plugins(systems::character_select::CharacterSelectPlugin)
    .add_plugins(systems::power_cooldowns::PowerCooldownsPlugin)
    .add_plugins(systems::combos::CombosPlugin)
    .add_plugins(systems::extra_powers::ExtraPowersPlugin)
    .add_plugins(systems::window_icon::WindowIconPlugin)
    .add_plugins(systems::ui::loading_screen::LoadingScreenPlugin)
    .add_plugins(systems::biome_encounters::BiomeEncountersPlugin)
    .add_plugins(systems::biome_content::BiomeContentPlugin)
    .add_plugins(systems::prison::PrisonPlugin)
    .add_plugins(systems::terrain_cracks::TerrainCracksPlugin)
    .add_plugins(systems::dragon_flyover::DragonPlugin)
    .add_plugins(systems::energy_vfx::EnergyVfxPlugin)
    .add_plugins(systems::mirror_vfx::MirrorVfxPlugin)
    .add_plugins(AssetLoaderPlugin) // Load assets first
    .add_plugins((
        PhysicsPlugin, // Avian actor contacts plus swept grid terrain
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
        systems::target_index::TargetIndexPlugin,
        HeartUIPlugin,
        UsernameLabelPlugin,
        XpBarUIPlugin,
        DamageVisualPlugin,
        systems::actor_depth::ActorDepthPlugin,
        BlockDamageOverlayPlugin,
        WeatherSystemPlugin,
        FrozenPlugin,
    ))
    .add_plugins((
        DevPanelPlugin,
        CommandsPanelPlugin,
        DeathParticlePlugin,
        SnowAccumulationPlugin,
        BurningVisualPlugin,
        StatusEffectsPlugin,
        RandomEventPlugin,
        CollisionDebugPlugin,
        ExperiencePlugin,
        LootboxPlugin,
        RandomBoxPlugin,
        systems::drop_terrain::DropTerrainPlugin,
        systems::crowd_slide::CrowdSlidePlugin,
        systems::fluids::FluidPlugin,
    ))
    .init_resource::<MaterialCache>()
    .add_plugins(systems::forest_trees::ForestTreesPlugin)
    .init_resource::<PerformanceTracker>()
    .init_resource::<WorldConfig>()
    .init_resource::<resources::world_seed::WorldSeed>()
    .insert_resource(settings)
    .add_systems(Update, resources::settings::save_game_settings)
    .add_systems(Startup, setup)
    .add_systems(Update, initialize_mesh_cache)
    .add_systems(Update, initialize_material_cache)
    .add_systems(Update, diagnostics::snapshot)
    .add_systems(Update, (log_performance_metrics, log_memory_metrics))
    .add_systems(Update, apply_fps_limit);

    systems::native_gpu::install(&mut app);
    #[cfg(target_os = "windows")]
    systems::frame_generation::install(&mut app);

    if std::env::args().any(|arg| arg == "--fg-smoke-test") {
    } else if std::env::args().any(|arg| arg == "--gpu-smoke-test") {
        app.add_plugins(systems::native_gpu::GpuSmokePlugin);
    } else if cfg!(feature = "probes") && std::env::args().any(|arg| arg == "--ui-smoke-test") {
        #[cfg(feature = "probes")]
        app.add_plugins(ui_smoke::UiSmokePlugin);
    } else if cfg!(feature = "probes") && std::env::args().any(|a| a == "--visual-smoke-test") {
        #[cfg(feature = "probes")]
        app.add_plugins(visual_probe::VisualProbePlugin);
    } else if smoke_test {
        app.add_plugins(smoke_test::SmokeTestPlugin);
    } else if cfg!(feature = "probes") && std::env::args().any(|a| a == "--memory-probe") {
        #[cfg(feature = "probes")]
        app.add_plugins(memory_probe::MemoryProbePlugin);
    }
    let exit = app.run();
    let code = match exit {
        AppExit::Success => 0,
        AppExit::Error(code) => code.get(),
    };
    if let Some(session) = session {
        session.finish(code);
    }
    if code != 0 {
        std::process::exit(code as i32);
    }
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
    #[cfg(target_os = "windows")] fg: Option<Res<systems::frame_generation::FrameGenerationStatus>>,
) {
    #[cfg(target_os = "windows")]
    if fg.is_some_and(|s| s.available) {
        limiter.last_frame = Instant::now();
        return;
    }
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
            fov: std::f32::consts::FRAC_PI_4, // 45 degree field of view
            ..default()
        }),
        Transform::from_xyz(30.0, 80.0, 1500.0) // Far back to see large 64x64x64 blocks
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    // Present the resolved world image, labels and UI at native output resolution.
    commands.spawn((
        Camera2d,
        Msaa::Off,
        IsDefaultUiCamera,
        Camera {
            order: 1,                                            // Render after 3D
            clear_color: ClearColorConfig::Custom(Color::BLACK), // World arrives via the compositor sprite
            ..default()
        },
    ));

    // Add ambient light for 3D blocks
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 1500.0,
        affects_lightmapped_meshes: false,
    });

    // Shadows must cover the play plane 1500 world units from the camera.
    commands.spawn((
        bevy::light::CascadeShadowConfigBuilder {
            num_cascades: 2,
            minimum_distance: 900.,
            first_cascade_far_bound: 1650.,
            maximum_distance: 2600.,
            ..default()
        }
        .build(),
        DirectionalLight {
            illuminance: 25000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        // Solari samples this finite disk for softer contact-shadow edges.
        bevy::light::SunDisk {
            angular_size: 0.12,
            intensity: 1.,
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
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<MaterialCache>,
    block_textures: Option<Res<BlockTextures>>,
    mut images: ResMut<Assets<Image>>,
    mut derived: Option<ResMut<systems::asset_packs::DerivedImages>>,
) {
    // Only run once
    if !cache.materials.is_empty() {
        return;
    }

    // Wait for BlockTextures resource to be available
    let Some(block_textures) = block_textures else {
        return;
    };

    // Wait for all textures to load before generating atlases. A texture that
    // failed to load gets a placeholder and an error instead of waiting forever.
    let t = &block_textures;
    if !crate::systems::atlas_generator::textures_ready_or_placeholder(
        &[
            &t.pumpkin_top,
            &t.pumpkin_side,
            &t.pumpkin_front,
            &t.oak_log,
            &t.oak_log_top,
            &t.oak_leaves,
            &t.melon_top,
            &t.melon_side,
            &t.tnt_top,
            &t.tnt_bottom,
            &t.tnt_side,
            &t.cactus_top,
            &t.cactus_bottom,
            &t.cactus_side,
            &t.sandstone_top,
            &t.sandstone_bottom,
            &t.sandstone,
        ],
        &mut images,
        &asset_server,
    ) {
        return;
    }
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

    let sandstone_top = images.get(&block_textures.sandstone_top);
    let sandstone_bottom = images.get(&block_textures.sandstone_bottom);
    let sandstone_side = images.get(&block_textures.sandstone);

    if pumpkin_top.is_none()
        || pumpkin_side.is_none()
        || pumpkin_front.is_none()
        || oak_log.is_none()
        || oak_log_top.is_none()
        || oak_leaves.is_none()
        || melon_top.is_none()
        || melon_side.is_none()
        || tnt_top.is_none()
        || tnt_bottom.is_none()
        || tnt_side.is_none()
        || cactus_top.is_none()
        || cactus_bottom.is_none()
        || cactus_side.is_none()
        || sandstone_top.is_none()
        || sandstone_bottom.is_none()
        || sandstone_side.is_none()
    {
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

    // Create sandstone atlas (top, bottom, side) - already loaded above
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

    // Rebuild these atlases in place when a theme pack switch reloads their faces.
    if let Some(derived) = derived.as_mut() {
        let t = &block_textures;
        for (atlas, faces) in [
            (
                &pumpkin_atlas_handle,
                [
                    &t.pumpkin_top,
                    &t.pumpkin_top,
                    &t.pumpkin_side,
                    &t.pumpkin_front,
                ],
            ),
            (
                &oak_log_atlas_handle,
                [&t.oak_log_top, &t.oak_log_top, &t.oak_log, &t.oak_log],
            ),
            (
                &oak_leaves_atlas_handle,
                [&t.oak_leaves, &t.oak_leaves, &t.oak_leaves, &t.oak_leaves],
            ),
            (
                &melon_atlas_handle,
                [&t.melon_top, &t.melon_top, &t.melon_side, &t.melon_side],
            ),
            (
                &tnt_atlas_handle,
                [&t.tnt_top, &t.tnt_bottom, &t.tnt_side, &t.tnt_side],
            ),
            (
                &sandstone_atlas_handle,
                [
                    &t.sandstone_top,
                    &t.sandstone_bottom,
                    &t.sandstone,
                    &t.sandstone,
                ],
            ),
            (
                &cactus_atlas_handle,
                [
                    &t.cactus_top,
                    &t.cactus_bottom,
                    &t.cactus_side,
                    &t.cactus_side,
                ],
            ),
        ] {
            derived.add(atlas.clone(), faces.map(Clone::clone).to_vec(), |f| {
                crate::systems::atlas_generator::create_atlas_4x1(f[0], f[1], f[2], f[3])
            });
        }
    }

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

    info!("Initializing material cache with {} block types...", 30);

    // Create one material per block type
    let block_types = [
        BlockType::Stone,
        BlockType::Dirt,
        BlockType::Grass,
        BlockType::Sand,
        BlockType::Sandstone,
        BlockType::Snow,
        BlockType::Ice,
        BlockType::Coal,
        BlockType::Iron,
        BlockType::Gold,
        BlockType::Diamond,
        BlockType::Emerald,
        BlockType::Redstone,
        BlockType::Wood,
        BlockType::Leaves,
        BlockType::Pumpkin,
        BlockType::Melon,
        BlockType::Cactus,
        BlockType::DeadBush,
        BlockType::Mushroom,
        BlockType::Vines,
        BlockType::Cobweb,
        BlockType::Netherrack,
        BlockType::Netherite,
        BlockType::AncientDebris,
        BlockType::SoulSand,
        BlockType::Glowstone,
        BlockType::EndStone,
        BlockType::Obsidian,
        BlockType::ChorusFruit,
        BlockType::Purpur,
    ];

    for block_type in block_types {
        let material = match block_type {
            BlockType::Purpur => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(
                    asset_server
                        .load_builder()
                        .with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
                            s.sampler = bevy::image::ImageSampler::nearest()
                        })
                        .load("textures/blocks/end/purpur_block.png"),
                ),
                perceptual_roughness: 0.95,
                ..default()
            }),
            BlockType::AncientDebris => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.ancient_debris_side.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Netherrack => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.netherrack.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Obsidian => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.obsidian.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Gold => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.gold_block.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Leaves => {
                info!(
                    "Creating Leaves material with atlas handle: {:?}",
                    oak_leaves_atlas_handle
                );
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.4, 0.8, 0.4), // Tint leaves green (Minecraft-style biome tinting)
                    base_color_texture: Some(oak_leaves_atlas_handle.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Mask(0.5),
                    cull_mode: Some(bevy::render::render_resource::Face::Back),
                    double_sided: false,
                    ..default()
                })
            }
            BlockType::Wood => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(oak_log_atlas_handle.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                cull_mode: None,
                double_sided: true,
                ..default()
            }),
            BlockType::Pumpkin => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(pumpkin_atlas_handle.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                cull_mode: None,
                double_sided: true,
                ..default()
            }),
            BlockType::Melon => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(melon_atlas_handle.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                cull_mode: None,
                double_sided: true,
                ..default()
            }),
            BlockType::Grass => {
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.4, 0.8, 0.4), // Green tint for leaves
                    base_color_texture: Some(oak_leaves_atlas_handle.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Mask(0.5),
                    cull_mode: Some(bevy::render::render_resource::Face::Back),
                    double_sided: false,
                    ..default()
                })
            }
            BlockType::Dirt => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.dirt.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Stone => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.stone.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Coal => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.coal_ore.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Iron => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.iron_ore.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Diamond => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.diamond_ore.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Emerald => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.emerald_ore.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Redstone => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.redstone_ore.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Sand => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.sand.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Sandstone => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(sandstone_atlas_handle.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                cull_mode: None,
                double_sided: true,
                ..default()
            }),
            BlockType::EndStone => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.end_stone.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Ice => materials.add(StandardMaterial {
                base_color: Color::srgb(0.65, 0.85, 1.),
                base_color_texture: Some(asset_server.load("textures/blocks/iceland/ice.png")),
                perceptual_roughness: 0.2,
                ..default()
            }),
            BlockType::Snow => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.snow.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Cactus => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(cactus_atlas_handle.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Mask(0.5),
                cull_mode: Some(bevy::render::render_resource::Face::Back),
                double_sided: false,
                ..default()
            }),
            BlockType::Cobweb => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.cobweb.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Mask(0.5),
                cull_mode: None,
                double_sided: true,
                ..default()
            }),
            BlockType::SoulSand => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.soul_sand.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Netherite => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.netherite_block.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            BlockType::Mushroom => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.95,
                ..default()
            }),
            BlockType::DeadBush => materials.add(StandardMaterial {
                base_color: Color::srgb(0.50, 0.30, 0.13),
                base_color_texture: Some(block_textures.dirt.clone()),
                perceptual_roughness: 0.95,
                ..default()
            }),
            BlockType::Glowstone => materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(block_textures.glowstone.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
            _ => materials.add(StandardMaterial {
                base_color: get_block_color(block_type),
                unlit: true,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            }),
        };

        cache.materials.insert(block_type, material);
    }

    // CRITICAL PERFORMANCE FIX: Initialize resource materials too (prevents unique material creation)
    use crate::components::resource::ResourceType;

    let resource_types = [
        ResourceType::Coal,
        ResourceType::Iron,
        ResourceType::Gold,
        ResourceType::Diamond,
        ResourceType::Emerald,
        ResourceType::Redstone,
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

    info!(
        "Material cache initialized with {} block materials and {} resource materials",
        cache.materials.len(),
        cache.resource_materials.len()
    );
}

/// A command aimed at an entity despawned earlier in the frame (a mob killed, a
/// chunk unloaded) only logs a warning instead of crashing the game. Anything
/// else, such as a panicking system, logs the error and exits with a failure
/// code: re-panicking here is caught again by Bevy and leaves the game hanging.
fn command_errors_warn(error: bevy::ecs::error::BevyError, ctx: bevy::ecs::error::ErrorContext) {
    match ctx {
        bevy::ecs::error::ErrorContext::Command { .. } => bevy::ecs::error::warn(error, ctx),
        _ => {
            bevy::ecs::error::error(error, ctx);
            std::process::exit(101);
        }
    }
}
