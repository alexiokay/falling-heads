use crate::components::{
    buff::Buffable, bundles::avian_physics, enemy::Health, villager::Villager,
};
use crate::resources::asset_handles::GameTextures;
use crate::systems::ui::username_label::spawn_username_label;
use crate::utils::constants::*;
use bevy::prelude::*;
use rand::RngExt;

#[derive(Resource)]
struct VillagerFace {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}
#[derive(Component)]
pub(crate) struct SharedVillagerFace;
#[derive(Component)]
pub(crate) struct RestoreFace(pub Handle<StandardMaterial>);

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
    let mut rng = rand::rng();
    let initial_direction = if rng.random_bool(0.5) { 1.0 } else { -1.0 };

    let texture = textures.villager.clone();

    let villager_entity = commands
        .spawn((
            Villager {
                ai_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
                move_direction: initial_direction,
                direction_change_timer: Timer::from_seconds(
                    VILLAGER_DIRECTION_CHANGE_INTERVAL,
                    TimerMode::Repeating,
                ),
            },
            Health::new(6),
            Buffable,
            avian_physics::character(), // Avian physics for villagers
            SharedVillagerFace,
            Transform::from_translation(position),
        ))
        .id();

    // All heads share one geometry/BLAS; damage can still tint each face independently.
    commands.queue(move |world: &mut World| {
        if !world.contains_resource::<VillagerFace>() {
            let material = world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color_texture: Some(texture),
                    unlit: true,
                    alpha_mode: AlphaMode::Mask(0.5),
                    ..default()
                });
            let mesh = world
                .resource_mut::<Assets<Mesh>>()
                .add(Rectangle::new(VILLAGER_SIZE, VILLAGER_SIZE));
            world.insert_resource(VillagerFace { mesh, material });
        }
        let art = world.resource::<VillagerFace>();
        let material = art.material.clone();
        let mesh = art.mesh.clone();
        if let Ok(mut entity) = world.get_entity_mut(villager_entity) {
            entity.insert((Mesh3d(mesh), MeshMaterial3d(material)));
        }
    });

    // Add username label if provided
    if let Some(username) = username {
        spawn_username_label(
            commands,
            villager_entity,
            position,
            username,
            40.0, // Below hearts (hearts are at ~60-80 units above)
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
    let mut rng = rand::rng();

    for _ in 0..count {
        let offset_x = rng.random_range(offset_range_x.0..offset_range_x.1);
        let offset_y = rng.random_range(offset_range_y.0..offset_range_y.1);
        let position = Vec3::new(center_pos.x + offset_x, center_pos.y + offset_y, 10.0);

        spawn_villager(
            commands,
            position,
            textures,
            meshes,
            materials,
            username.clone(),
            asset_server,
        );
    }
}
