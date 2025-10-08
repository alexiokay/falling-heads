use bevy::prelude::*;
use rand::Rng;
use crate::components::{
    villager::Villager,
    enemy::Health,
    buff::Buffable,
    bundles::{SpriteBundle3d, PhysicsBundle},
    username_label::UsernameLabel,
};
use crate::resources::asset_handles::GameTextures;
use crate::utils::constants::*;
use crate::systems::username_label_system::spawn_username_label;

/// Spawns a single villager at the specified position
pub fn spawn_villager(
    commands: &mut Commands,
    position: Vec3,
    textures: &Res<GameTextures>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    username: Option<String>,
    asset_server: &Res<AssetServer>,
) -> Entity {
    let mut rng = rand::thread_rng();
    let initial_direction = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };

    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(textures.villager.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    let villager_entity = commands.spawn((
        Villager {
            ai_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            move_direction: initial_direction,
            direction_change_timer: Timer::from_seconds(VILLAGER_DIRECTION_CHANGE_INTERVAL, TimerMode::Repeating),
        },
        Health::new(6),
        Buffable,
        PhysicsBundle::new(Vec2::ZERO, STANDARD_GRAVITY, VILLAGER_BOUNCE),
        SpriteBundle3d::new(
            meshes.add(Rectangle::new(VILLAGER_SIZE, VILLAGER_SIZE)),
            material,
            position,
        ),
    )).id();

    // Add username label if provided
    if let Some(username) = username {
        spawn_username_label(
            commands,
            villager_entity,
            position,
            username,
            40.0,  // Below hearts (hearts are at ~60-80 units above)
            meshes,
            materials,
            asset_server,
        );
    }

    villager_entity
}

/// Spawns multiple villagers around a center position with random offsets
pub fn spawn_villagers_around_position(
    commands: &mut Commands,
    center_pos: Vec3,
    count: usize,
    offset_range_x: (f32, f32),
    offset_range_y: (f32, f32),
    textures: &Res<GameTextures>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    username: Option<String>,
    asset_server: &Res<AssetServer>,
) {
    let mut rng = rand::thread_rng();

    for _ in 0..count {
        let offset_x = rng.gen_range(offset_range_x.0..offset_range_x.1);
        let offset_y = rng.gen_range(offset_range_y.0..offset_range_y.1);
        let position = Vec3::new(center_pos.x + offset_x, center_pos.y + offset_y, 10.0);

        spawn_villager(commands, position, textures, meshes, materials, username.clone(), asset_server);
    }
}
