use bevy::prelude::*;
use crate::components::{
    hero::Hero,
    block::{Block, BlockType},
    physics::{Velocity, Gravity, Bouncy},
    resource::{Resource, ResourceType, RotatingResource},
    power_effect::SpeedModifier,
    villager::Villager,
    chunk::LoadedChunks,
    enemy::Health,
    buff::Buffable,
};
use crate::events::block_events::{BlockDamaged, BlockDestroyed};
use crate::events::resource_events::ResourceCollected;
use crate::events::damage_events::{EntityDamaged, EntityDied};
use crate::resources::inventory::Inventory;
use crate::resources::material_cache::MaterialCache;
use crate::resources::asset_handles::GameTextures;
use crate::states::GameState;
use crate::systems::world_generation::{generate_chunks_around_hero, despawn_distant_chunks};
use crate::systems::health_system::{handle_entity_damage, handle_entity_death};
use crate::systems::health_ui_system::{spawn_health_bars, update_health_bars};
use crate::systems::background_system::update_background;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_message::<BlockDamaged>()
            .add_message::<BlockDestroyed>()
            .add_message::<ResourceCollected>()
            .add_message::<EntityDamaged>()
            .add_message::<EntityDied>()
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
                // Run chunk management every 0.1s for balance of performance and responsiveness
                (generate_chunks_around_hero, despawn_distant_chunks).run_if(|time: Res<Time>| {
                    time.elapsed_secs() % 0.1 < time.delta_secs()
                }),
                // Clean up unused materials every 5 seconds
                cleanup_unused_materials.run_if(|time: Res<Time>| {
                    time.elapsed_secs() % 5.0 < time.delta_secs()
                }),
                update_speed_modifiers,
                handle_entity_damage,
                handle_entity_death,
                spawn_health_bars,
                update_health_bars,
                // Background only updates when biome changes (already has internal check)
                update_background,
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
        Health::new(3),  // Hero has 3 hearts
        Buffable,  // Hero can receive buffs
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
        // Spawn in Nether: Y < -20 chunks = Y < -20 * 16 * 64 = -20,480
        Transform::from_xyz(0.0, -25000.0, 10.0),  // Deep underground in Nether
    ));

    info!("Hero spawned in Nether biome at Y=-25000");
}

fn spawn_villagers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    hero_query: Query<&Transform, With<Hero>>,
    textures: Res<GameTextures>,
) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Get hero position to spawn villagers nearby
    let hero_y = if let Ok(hero_transform) = hero_query.single() {
        hero_transform.translation.y
    } else {
        -25000.0  // Default to nether depth if hero not found
    };

    // Spawn 5 villagers around the hero
    for _i in 0..5 {
        let offset_x = rng.gen_range(-128.0..128.0);
        let offset_y = rng.gen_range(-64.0..64.0);

        commands.spawn((
            Villager {
                ai_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            },
            Health::new(6),  // Villagers have 6 health (3 hearts)
            Buffable,  // Villagers can receive buffs
            Velocity { x: 0.0, y: 0.0 },
            Gravity { force: -500.0 },
            Bouncy { restitution: 0.8 },
            Mesh3d(meshes.add(Rectangle::new(56.0, 56.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(textures.villager.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_xyz(offset_x, hero_y + offset_y, 10.0),  // Spawn near hero
        ));
    }

    info!("Spawned 5 villagers near hero at Y={}", hero_y);
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
    mut commands: Commands,
    query: Query<(Entity, &Block, &BlockType), Changed<Block>>,
    material_cache: Res<crate::resources::material_cache::MaterialCache>,
) {
    // Use shared cached materials only - no unique materials per damaged block
    // This prevents creating thousands of materials that cause lag
    for (entity, _block, block_type) in query.iter() {
        // Always use the cached material regardless of damage state
        if let Some(cached_material) = material_cache.get(*block_type) {
            commands.entity(entity).try_insert(MeshMaterial3d(cached_material));
        }
    }
}

fn spawn_resources_from_destroyed_blocks(
    mut commands: Commands,
    mut destroy_events: MessageReader<BlockDestroyed>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    resource_textures: Res<crate::resources::asset_handles::ResourceTextures>,
) {
    for event in destroy_events.read() {
        let (resource_type, texture) = match event.block_type {
            BlockType::Coal => (ResourceType::Coal, resource_textures.coal.clone()),
            BlockType::Iron => (ResourceType::Iron, resource_textures.iron_ingot.clone()),
            BlockType::Gold => (ResourceType::Gold, resource_textures.gold_ingot.clone()),
            BlockType::Diamond => (ResourceType::Diamond, resource_textures.diamond.clone()),
            BlockType::Emerald => (ResourceType::Emerald, resource_textures.emerald.clone()),
            BlockType::Redstone => (ResourceType::Redstone, resource_textures.redstone.clone()),
            BlockType::Netherite | BlockType::AncientDebris => (ResourceType::Netherite, resource_textures.netherite_ingot.clone()),
            // Terrain and decorative blocks don't drop resources
            _ => continue,
        };

        // Use 2D rectangle with double-sided rendering (no culling) to avoid UV mapping issues
        commands.spawn((
            Resource { resource_type },
            RotatingResource::default(), // Add rotation component for visual effect (Y-axis spin)
            Velocity { x: 0.0, y: 50.0 }, // Small upward velocity when dropped
            Gravity { force: -500.0 },  // Physics gravity
            // NO BOUNCY COMPONENT - resources settle on ground without bouncing
            Mesh3d(meshes.add(Rectangle::new(32.0, 32.0))),  // 2D rectangle for clean rendering
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(texture),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,  // Render both sides so visible during rotation
                ..default()
            })),
            Transform::from_translation(event.position).with_translation(Vec3::new(
                event.position.x,
                event.position.y,
                5.0,  // Z=5 to render above blocks but below UI
            )),
        ));
    }
}

/// Rotate resources on the Y axis (coin-flip rotation for 2D sprites)
fn rotate_resources(
    time: Res<Time>,
    mut query: Query<(&RotatingResource, &mut Transform)>,
) {
    for (rotating, mut transform) in query.iter_mut() {
        let rotation_radians = rotating.rotation_speed.to_radians() * time.delta_secs();
        transform.rotate_y(-rotation_radians); // Y-axis rotation in opposite direction (negative)
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
                ResourceType::Netherite => inventory.netherite += 1,
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

/// Clean up unused materials to prevent memory leaks from damaged block materials
fn cleanup_unused_materials(
    materials: ResMut<Assets<StandardMaterial>>,
) {
    // Note: Bevy 0.17 automatically handles unused asset cleanup
    // The Assets<T> resource will clean up unreferenced materials automatically
    // This function serves as a periodic trigger point if needed in future
    let material_count = materials.len();
    if material_count > 1000 {
        info!("Material count: {} (cleanup running automatically)", material_count);
    }
}
