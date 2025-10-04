use bevy::prelude::*;
use bevy::window::WindowResolution;

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
use resources::asset_handles::GameTextures;
use states::GameState;

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
        .add_plugins((
            PhysicsPlugin,
            GameplayPlugin,
            PowersPlugin,
            RenderingPlugin,
            UiPlugin,
            AudioPlugin,
            ExternalApiPlugin,
        ))
        .add_systems(Startup, setup)
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
    };

    commands.insert_resource(textures);

    info!("Falling Heads initialized with mixed 2D/3D isometric view!");
}
