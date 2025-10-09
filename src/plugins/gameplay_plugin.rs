use bevy::prelude::*;
use crate::components::{
    hero::Hero,
    block::{Block, BlockType},
    physics::Velocity,
    resource::{Resource, ResourceType, RotatingResource},
    power_effect::SpeedModifier,
    chunk::LoadedChunks,
    enemy::Health,
    buff::Buffable,
    bundles::{SpriteBundle3d, PhysicsBundle},
    background::BackgroundCache,
};
use crate::events::block_events::{BlockDamaged, BlockDestroyed};
use crate::events::resource_events::ResourceCollected;
use crate::events::damage_events::{EntityDamaged, EntityDied};
use crate::resources::inventory::Inventory;
use crate::resources::asset_handles::GameTextures;
use crate::states::GameState;
use crate::systems::world_generation::{generate_chunks_around_hero, despawn_distant_chunks};
use crate::systems::health_system::{handle_entity_damage, handle_entity_death, update_death_animations, cleanup_marked_entities};
use crate::systems::health_ui_system::{spawn_health_bars, update_health_bars};
use crate::systems::background_system::update_background;
use crate::systems::entity_spawn::spawn_villagers_around_position;
use crate::systems::dragon_ai_system::dragon_ai_system;
use crate::systems::dragon_animation_system::dragon_animation_system;
use crate::utils::constants::*;

#[derive(Resource, Default, PartialEq)]
struct EntitiesSpawned(bool);

/// Tracks the spawn area that should be kept clear of blocks
#[derive(Resource)]
pub struct SpawnArea {
    pub center: Vec3,
    pub radius: f32,  // Radius in world units
}

impl Default for SpawnArea {
    fn default() -> Self {
        Self {
            center: Vec3::new(0.0, 130000.0, 10.0),  // Forest biome spawn
            radius: 200.0,  // 200 units radius = ~3 blocks
        }
    }
}

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
            .init_resource::<SpawnArea>()
            .init_resource::<BackgroundCache>()
            .add_systems(OnEnter(GameState::Playing), (spawn_hero, spawn_villagers).run_if(resource_equals(EntitiesSpawned(false))))
            .add_systems(Startup, start_game)
            .init_resource::<EntitiesSpawned>()
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
                update_death_animations,
                spawn_health_bars,
                update_health_bars,
                dragon_ai_system,
                dragon_animation_system,
                // Background only updates when biome changes (already has internal check)
                update_background,
            ).run_if(in_state(GameState::Playing)))
            // Cleanup dead entities in PostUpdate after all damage processing is done
            .add_systems(PostUpdate, cleanup_marked_entities.run_if(in_state(GameState::Playing)));

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
    mut spawned: ResMut<EntitiesSpawned>,
) {
    spawned.0 = true;
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(textures.hero.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands.spawn((
        Hero {
            move_direction: 1.0,  // Start moving right
            direction_change_timer: Timer::from_seconds(VILLAGER_DIRECTION_CHANGE_INTERVAL, TimerMode::Repeating),
        },
        Health::new(3),
        Buffable,
        PhysicsBundle::new(Vec2::ZERO, STANDARD_GRAVITY, HERO_BOUNCE),
        SpriteBundle3d::new(
            meshes.add(Rectangle::new(HERO_SIZE, HERO_SIZE)),
            material,
            Vec3::new(0.0, 130000.0, 10.0),  // Spawn in middle of Forest biome
        ),
    ));

    info!("Hero spawned in Forest biome (Y=130000)");
}

fn spawn_villagers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    hero_query: Query<&Transform, With<Hero>>,
    textures: Res<GameTextures>,
    asset_server: Res<AssetServer>,
) {
    // Get hero position to spawn villagers nearby
    let hero_pos = if let Ok(hero_transform) = hero_query.single() {
        hero_transform.translation
    } else {
        Vec3::new(0.0, 130000.0, 10.0)  // Default to Forest biome spawn
    };

    // Spawn 5 villagers around the hero using shared spawning logic
    spawn_villagers_around_position(
        &mut commands,
        hero_pos,
        5,
        (-128.0, 128.0),
        (-64.0, 64.0),
        &textures,
        &mut meshes,
        &mut materials,
        None,  // No username for initial spawned villagers
        &asset_server,
    );

    info!("Spawned 5 villagers near hero at {:?}", hero_pos);
}


/// Optional player control - ADDS to automatic AI movement
/// Hero moves automatically like villagers, but player can override/boost with A/D keys
fn hero_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Velocity, &mut Hero, Option<&SpeedModifier>), With<Hero>>,
    time: Res<Time>,
) {
    let Ok((mut velocity, mut hero, speed_mod)) = query.single_mut() else {
        return;
    };

    let mut acceleration = 800.0;  // Stronger player override acceleration (was 500)

    // Apply speed modifier if present
    if let Some(modifier) = speed_mod {
        acceleration *= modifier.multiplier;
    }

    // ADD to velocity (preserves automatic AI movement + physics)
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        velocity.x += -acceleration * time.delta_secs();
        hero.move_direction = -1.0;  // Override AI direction when player presses key
    } else if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        velocity.x += acceleration * time.delta_secs();
        hero.move_direction = 1.0;  // Override AI direction when player presses key
    }

    // Cap max horizontal velocity (higher than AI clamp so player can boost past it)
    velocity.x = velocity.x.clamp(-600.0, 600.0);
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
                commands.entity(event.entity).despawn(); // Despawn children too (damage overlays, borders)
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

        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(texture),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        });

        commands.spawn((
            Resource { resource_type },
            RotatingResource::default(),
            PhysicsBundle::settling(Vec2::new(0.0, RESOURCE_DROP_VELOCITY_Y)),
            SpriteBundle3d::new(
                meshes.add(Rectangle::new(RESOURCE_SIZE, RESOURCE_SIZE)),
                material,
                Vec3::new(event.position.x, event.position.y, 5.0),
            ),
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

    // Collection range = (hero_half + resource_half) for overlap detection
    let collection_range = (HERO_SIZE / 2.0) + (RESOURCE_SIZE / 2.0);  // 32.0 + 16.0 = 48.0

    for (entity, resource_transform, resource) in resource_query.iter() {
        // Use 2D distance (ignore Z-depth) since hero is at Z=10 and resources at Z=5
        let hero_pos = hero_transform.translation.xy();
        let resource_pos = resource_transform.translation.xy();
        let distance = hero_pos.distance(resource_pos);

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
    time: Res<Time>,
) {
    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let target_x = hero_transform.translation.x;
    let target_y = hero_transform.translation.y;

    // Follow with 3D camera (keep isometric angle) - follow both X and Y
    if let Ok(mut camera_transform) = camera_query.single_mut() {
        let offset_x = 30.0;  // Keep camera slightly offset from hero
        let offset_y = 80.0;  // Keep camera above hero

        // Time-based lerp for consistent speed regardless of framerate
        let lerp_speed = 10.0;
        let lerp_factor = (lerp_speed * time.delta_secs()).min(1.0);

        camera_transform.translation.x += (target_x + offset_x - camera_transform.translation.x) * lerp_factor;
        camera_transform.translation.y += (target_y + offset_y - camera_transform.translation.y) * lerp_factor;
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
