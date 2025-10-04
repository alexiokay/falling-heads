use bevy::prelude::*;
use crate::components::{
    hero::Hero,
    block::{Block, BlockType},
    physics::{Velocity, Gravity, Bouncy},
    resource::{Resource, ResourceType, RotatingResource},
    power_effect::SpeedModifier,
    villager::Villager,
    chunk::LoadedChunks,
};
use crate::events::block_events::{BlockDamaged, BlockDestroyed};
use crate::events::resource_events::ResourceCollected;
use crate::resources::inventory::Inventory;
use crate::resources::asset_handles::GameTextures;
use crate::states::GameState;
use crate::systems::world_generation::{generate_chunks_around_hero, despawn_distant_chunks, get_block_color};

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_message::<BlockDamaged>()
            .add_message::<BlockDestroyed>()
            .add_message::<ResourceCollected>()
            .init_resource::<Inventory>()
            .init_resource::<LoadedChunks>()
            .add_systems(OnEnter(GameState::Playing), (spawn_hero, spawn_villagers))
            .add_systems(Startup, start_game)
            .add_systems(Update, (
                hero_movement,
                handle_block_damage,
                spawn_resources_from_destroyed_blocks,
                collect_resources,
                rotate_resources,
                update_block_visuals,
                camera_follow_hero,
                generate_chunks_around_hero,
                despawn_distant_chunks,
                update_speed_modifiers,
            ).run_if(in_state(GameState::Playing)));

        info!("GameplayPlugin loaded");
    }
}

fn start_game(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Playing);
}

fn spawn_hero(
    mut commands: Commands,
    textures: Res<GameTextures>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Use a 3D sprite (textured quad) instead of 2D sprite for consistent coordinate space
    commands.spawn((
        Hero,
        Velocity { x: 0.0, y: 0.0 },
        Gravity { force: -500.0 },
        Bouncy { restitution: 0.85 },
        Mesh3d(meshes.add(Rectangle::new(64.0, 64.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(textures.hero.clone()),
            unlit: true,  // No lighting for sprite-like appearance
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(0.0, 100.0, 10.0),  // Z=10 to render in front of blocks
    ));

    info!("Hero spawned at (0, 100)");
}

fn spawn_villagers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Spawn 5 villagers around the hero
    for _i in 0..5 {
        let offset_x = rng.gen_range(-128.0..128.0);
        let offset_y = rng.gen_range(-64.0..64.0);

        let color = Color::srgb(
            rng.gen_range(0.5..1.0),
            rng.gen_range(0.5..1.0),
            rng.gen_range(0.5..1.0),
        );

        commands.spawn((
            Villager {
                ai_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            },
            Velocity { x: 0.0, y: 0.0 },
            Gravity { force: -500.0 },
            Bouncy { restitution: 0.8 },
            Mesh3d(meshes.add(Rectangle::new(56.0, 56.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                unlit: true,
                ..default()
            })),
            Transform::from_xyz(offset_x, 100.0 + offset_y, 10.0),  // Z=10 in front of blocks
        ));
    }

    info!("Spawned 5 villagers");
}


fn hero_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Velocity, Option<&SpeedModifier>), With<Hero>>,
) {
    let Ok((mut velocity, speed_mod)) = query.single_mut() else {
        return;
    };

    let mut speed = 3.0;  // 3D scale

    // Apply speed modifier if present
    if let Some(modifier) = speed_mod {
        speed *= modifier.multiplier;
    }

    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        velocity.x = -speed;
    } else if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        velocity.x = speed;
    } else {
        velocity.x = 0.0;
    }
}

fn handle_block_damage(
    mut commands: Commands,
    mut damage_events: MessageReader<BlockDamaged>,
    mut destroy_events: MessageWriter<BlockDestroyed>,
    mut block_query: Query<(&mut Block, &BlockType, &Transform)>,
) {
    for event in damage_events.read() {
        if let Ok((mut block, block_type, transform)) = block_query.get_mut(event.entity) {
            block.health = block.health.saturating_sub(event.damage);

            if block.health == 0 {
                // Block destroyed!
                destroy_events.write(BlockDestroyed {
                    entity: event.entity,
                    block_type: *block_type,
                    position: transform.translation,
                });
                commands.entity(event.entity).despawn();
                info!("Block destroyed at {:?}", transform.translation);
            }
        }
    }
}

fn update_block_visuals(
    mut query: Query<(&Block, &BlockType, &MeshMaterial3d<StandardMaterial>), Changed<Block>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (block, block_type, material_handle) in query.iter_mut() {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            let base_color = get_block_color(*block_type);
            let health_percent = block.health as f32 / block.max_health as f32;

            // Darken as health decreases
            material.base_color = Color::srgb(
                base_color.to_srgba().red * (0.5 + health_percent * 0.5),
                base_color.to_srgba().green * (0.5 + health_percent * 0.5),
                base_color.to_srgba().blue * (0.5 + health_percent * 0.5),
            );
        }
    }
}

fn spawn_resources_from_destroyed_blocks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut destroy_events: MessageReader<BlockDestroyed>,
) {
    for event in destroy_events.read() {
        let resource_type = match event.block_type {
            BlockType::Coal => ResourceType::Coal,
            BlockType::Iron => ResourceType::Iron,
            BlockType::Gold => ResourceType::Gold,
            BlockType::Diamond => ResourceType::Diamond,
            BlockType::Emerald => ResourceType::Emerald,
            BlockType::Redstone => ResourceType::Redstone,
            // Terrain and decorative blocks don't drop resources
            _ => continue,
        };

        let resource_color = match resource_type {
            ResourceType::Coal => Color::srgb(0.2, 0.2, 0.2),
            ResourceType::Iron => Color::srgb(0.8, 0.8, 0.8),
            ResourceType::Gold => Color::srgb(1.0, 0.84, 0.0),
            ResourceType::Diamond => Color::srgb(0.0, 0.8, 1.0),
            ResourceType::Emerald => Color::srgb(0.0, 1.0, 0.3),
            ResourceType::Redstone => Color::srgb(1.0, 0.0, 0.0),
        };

        commands.spawn((
            Resource { resource_type },
            RotatingResource::default(), // Add rotation component
            Velocity { x: 0.0, y: 0.0 },
            Gravity { force: -3.0 },  // Adjusted for 3D scale
            Mesh3d(meshes.add(Sphere::new(0.15))),  // Small 3D sphere for resources
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: resource_color,
                emissive: (resource_color.to_linear() * 0.3).into(),  // Make them glow slightly
                perceptual_roughness: 0.3,
                metallic: 0.5,
                ..default()
            })),
            Transform::from_translation(event.position),
        ));
    }
}

/// Rotate resources on the X axis
fn rotate_resources(
    time: Res<Time>,
    mut query: Query<(&RotatingResource, &mut Transform)>,
) {
    for (rotating, mut transform) in query.iter_mut() {
        let rotation_radians = rotating.rotation_speed.to_radians() * time.delta_secs();
        transform.rotate_x(rotation_radians);
    }
}

fn collect_resources(
    mut commands: Commands,
    hero_query: Query<&Transform, With<Hero>>,
    resource_query: Query<(Entity, &Transform, &Resource)>,
    mut inventory: ResMut<Inventory>,
    mut collect_events: MessageWriter<ResourceCollected>,
) {
    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let collection_range = 1.0;  // Adjusted for 3D scale

    for (entity, resource_transform, resource) in resource_query.iter() {
        let distance = hero_transform.translation.distance(resource_transform.translation);

        if distance < collection_range {
            // Collect resource
            match resource.resource_type {
                ResourceType::Coal => inventory.coal += 1,
                ResourceType::Iron => inventory.iron += 1,
                ResourceType::Gold => inventory.gold += 1,
                ResourceType::Diamond => inventory.diamond += 1,
                ResourceType::Emerald => inventory.emerald += 1,
                ResourceType::Redstone => inventory.redstone += 1,
            }

            collect_events.write(ResourceCollected {
                resource_type: resource.resource_type,
                amount: 1,
            });

            commands.entity(entity).despawn();
        }
    }
}

fn camera_follow_hero(
    hero_query: Query<&Transform, With<Hero>>,
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<Hero>)>,
) {
    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let target_y = hero_transform.translation.y;

    // Follow with 3D camera (keep isometric angle)
    if let Ok(mut camera_transform) = camera_query.single_mut() {
        let offset_y = 80.0;  // Keep camera above hero
        camera_transform.translation.y += (target_y + offset_y - camera_transform.translation.y) * 0.1;
    }
}

fn update_speed_modifiers(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut SpeedModifier), With<Hero>>,
) {
    for (entity, mut modifier) in query.iter_mut() {
        modifier.duration.tick(time.delta());

        if modifier.duration.is_finished() {
            commands.entity(entity).remove::<SpeedModifier>();
            info!("Speed modifier expired");
        }
    }
}
