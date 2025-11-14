use bevy::prelude::*;
use bevy::ecs::schedule::common_conditions::resource_exists;
use rand::Rng;
use avian3d::prelude::*;

use crate::components::{
    hero::Hero,
    enemy::{Enemy, EnemyType, Health, EnemyBehavior, Flying},
    buff::Buffable,
    bundles::{SpriteBundle3d, avian_physics},
    resource::{Resource, ResourceType, RotatingResource},
    collision_layers::GameLayer,
};
use avian3d::prelude::CollisionLayers;
use crate::resources::{
    asset_handles::{GameTextures, EnemyTextures, ResourceTextures},
    mesh_cache::MeshCache,
    spatial_hash::SpatialHash,
};
use crate::states::GameState;
use crate::utils::constants::*;
use crate::systems::entity_spawn::spawn_villager;
use crate::components::block::Block;

/// Resource to track random event timing
#[derive(Resource)]
pub struct RandomEventTimer {
    pub timer: Timer,
    pub next_event_delay: f32,
}

impl Default for RandomEventTimer {
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        let initial_delay = rng.gen_range(30.0..60.0);

        Self {
            timer: Timer::from_seconds(initial_delay, TimerMode::Once),
            next_event_delay: initial_delay,
        }
    }
}

/// Different types of random events that can occur
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RandomEventType {
    MobSpawn,           // Spawn enemies near hero
    LootDrop,           // Drop resources around hero
    VillagerSpawn,      // Spawn friendly villagers
    EliteMobSpawn,      // Spawn stronger enemies
    GrowBuff,           // Apply Grow buff to hero
}

impl RandomEventType {
    /// Get a random event based on weighted probabilities
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let roll = rng.gen_range(0..100);

        match roll {
            0..30 => RandomEventType::MobSpawn,          // 30% chance
            30..55 => RandomEventType::LootDrop,         // 25% chance
            55..70 => RandomEventType::VillagerSpawn,    // 15% chance
            70..85 => RandomEventType::GrowBuff,         // 15% chance (NEW!)
            _ => RandomEventType::EliteMobSpawn,         // 15% chance
        }
    }

    /// Get display name for the event
    pub fn name(&self) -> &str {
        match self {
            RandomEventType::MobSpawn => "Monster Horde",
            RandomEventType::LootDrop => "Treasure Rain",
            RandomEventType::VillagerSpawn => "Travelers Arrive",
            RandomEventType::EliteMobSpawn => "Elite Enemy Appears",
            RandomEventType::GrowBuff => "Giant's Blessing",
        }
    }
}

/// Event notification component (for UI display)
#[derive(Component)]
pub struct EventNotification {
    pub event_type: RandomEventType,
    pub timer: Timer,
}

/// Helper function to find an empty spawn position (not inside blocks)
fn find_empty_spawn_position(
    base_pos: Vec3,
    radius: f32,
    spatial_hash: &SpatialHash,
    block_query: &Query<&Transform, With<Block>>,
    rng: &mut impl Rng,
) -> Option<Vec3> {
    // Try multiple times to find an empty position
    for _ in 0..30 {
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let distance = rng.gen_range(radius * 0.7..radius);
        let offset_x = angle.cos() * distance;
        let offset_y = angle.sin() * distance;

        let test_pos = Vec3::new(
            base_pos.x + offset_x,
            base_pos.y + offset_y,
            10.0,
        );

        // Check if this position collides with any blocks
        let chunk_coords = spatial_hash.world_to_chunk(test_pos.x, test_pos.y);
        let mut is_blocked = false;

        // Check this chunk and adjacent chunks for blocks
        for dx in -1..=1 {
            for dy in -1..=1 {
                let check_chunk = (chunk_coords.0 + dx, chunk_coords.1 + dy);
                if let Some(entities) = spatial_hash.chunks.get(&check_chunk) {
                    for &entity in entities {
                        if let Ok(block_transform) = block_query.get(entity) {
                            let distance = test_pos.distance(block_transform.translation);
                            // Check if we're too close to a block (within block size)
                            if distance < BLOCK_SIZE * 1.5 {
                                is_blocked = true;
                                break;
                            }
                        }
                    }
                    if is_blocked {
                        break;
                    }
                }
            }
            if is_blocked {
                break;
            }
        }

        if !is_blocked {
            return Some(test_pos);
        }
    }

    // Fallback: spawn above hero position (should be safe)
    Some(Vec3::new(base_pos.x, base_pos.y + 300.0, 10.0))
}

/// Main system that checks and triggers random events
pub fn random_event_system(
    mut commands: Commands,
    mut event_timer: ResMut<RandomEventTimer>,
    time: Res<Time>,
    hero_query: Query<&Transform, With<Hero>>,
) {
    event_timer.timer.tick(time.delta());

    if event_timer.timer.is_finished() {
        // Get hero position for event spawning
        let hero_pos = if let Ok(hero_transform) = hero_query.single() {
            hero_transform.translation
        } else {
            return; // No hero, skip events
        };

        // Choose random event
        let event_type = RandomEventType::random();

        info!("Random Event Triggered: {}", event_type.name());

        // Spawn notification entity (can be used by UI system)
        commands.spawn(EventNotification {
            event_type,
            timer: Timer::from_seconds(3.0, TimerMode::Once),
        });

        // Reset timer with new random delay
        let mut rng = rand::thread_rng();
        let next_delay = rng.gen_range(30.0..60.0);
        event_timer.timer = Timer::from_seconds(next_delay, TimerMode::Once);
        event_timer.next_event_delay = next_delay;
    }
}

/// Handle mob spawning event
pub fn handle_mob_spawn_event(
    mut commands: Commands,
    hero_query: Query<&Transform, With<Hero>>,
    event_query: Query<(Entity, &EventNotification), Added<EventNotification>>,
    enemy_textures: Res<EnemyTextures>,
    mesh_cache: Res<MeshCache>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    spatial_hash: Res<SpatialHash>,
    block_query: Query<&Transform, With<Block>>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
) {
    for (notification_entity, notification) in event_query.iter() {
        if notification.event_type != RandomEventType::MobSpawn {
            continue;
        }

        let Ok(hero_transform) = hero_query.single() else {
            commands.entity(notification_entity).despawn();
            continue;
        };

        let hero_pos = hero_transform.translation;
        let mut rng = rand::thread_rng();

        // Spawn 3-5 enemies around the hero in empty spaces
        let num_enemies = rng.gen_range(3..=5);
        let radius = 800.0; // Spawn at medium distance
        let mut spawned = 0;

        for _ in 0..num_enemies {
            // Find an empty position
            if let Some(spawn_pos) = find_empty_spawn_position(hero_pos, radius, &spatial_hash, &block_query, &mut rng) {
                // Choose appropriate enemy type based on hero Y position (biome)
                let enemy_type = if hero_pos.y > 150000.0 {
                    // Iceland biome
                    EnemyType::Yeti
                } else if hero_pos.y < 100000.0 {
                    // Nether biome
                    if rng.gen_bool(0.3) {
                        EnemyType::Ghast
                    } else {
                        EnemyType::Pigman
                    }
                } else {
                    // Overworld biomes
                    if rng.gen_bool(0.5) {
                        EnemyType::Skeleton
                    } else {
                        EnemyType::Zombie
                    }
                };

                spawn_enemy(
                    &mut commands,
                    spawn_pos,
                    enemy_type,
                    &enemy_textures,
                    &mesh_cache,
                    &mut materials,
                    &mut meshes,
                    &asset_server,
                );
                spawned += 1;
            }
        }

        info!("Spawned {} enemies around hero in empty spaces", spawned);
    }
}

/// Handle elite mob spawning event
pub fn handle_elite_mob_spawn_event(
    mut commands: Commands,
    hero_query: Query<&Transform, With<Hero>>,
    event_query: Query<(Entity, &EventNotification), Added<EventNotification>>,
    enemy_textures: Res<EnemyTextures>,
    mesh_cache: Res<MeshCache>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
) {
    for (notification_entity, notification) in event_query.iter() {
        if notification.event_type != RandomEventType::EliteMobSpawn {
            continue;
        }

        let Ok(hero_transform) = hero_query.single() else {
            commands.entity(notification_entity).despawn();
            continue;
        };

        let hero_pos = hero_transform.translation;
        let mut rng = rand::thread_rng();

        // Spawn 1-2 elite enemies (Ghast or Dragon)
        let num_elites = rng.gen_range(1..=2);
        let offset_x = rng.gen_range(-600.0..600.0);
        let offset_y = rng.gen_range(400.0..800.0); // Above hero

        let spawn_pos = Vec3::new(
            hero_pos.x + offset_x,
            hero_pos.y + offset_y,
            10.0,
        );

        // Choose elite enemy type (only Ghast - Dragon is experimental)
        let enemy_type = EnemyType::Ghast;

        for _ in 0..num_elites {
            spawn_enemy(
                &mut commands,
                spawn_pos,
                enemy_type,
                &enemy_textures,
                &mesh_cache,
                &mut materials,
                &mut meshes,
                &asset_server,
            );
        }

        info!("Spawned {} elite Ghasts", num_elites);
    }
}

/// Handle loot drop event
pub fn handle_loot_drop_event(
    mut commands: Commands,
    hero_query: Query<&Transform, With<Hero>>,
    event_query: Query<(Entity, &EventNotification), Added<EventNotification>>,
    resource_textures: Res<ResourceTextures>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    spatial_hash: Res<SpatialHash>,
    block_query: Query<&Transform, With<Block>>,
) {
    for (_notification_entity, notification) in event_query.iter() {
        if notification.event_type != RandomEventType::LootDrop {
            continue;
        }

        let Ok(hero_transform) = hero_query.single() else {
            continue;
        };

        let hero_pos = hero_transform.translation;
        let mut rng = rand::thread_rng();

        // Drop 5-10 random resources around the hero in empty spaces
        let num_drops = rng.gen_range(5..=10);
        let mut dropped = 0;

        for _ in 0..num_drops {
            // Find an empty position for the resource
            if let Some(drop_pos) = find_empty_spawn_position(hero_pos, 400.0, &spatial_hash, &block_query, &mut rng) {

            // Random resource type (uses actual ResourceType enum variants)
            let resource_type = match rng.gen_range(0..7) {
                0 => ResourceType::Coal,
                1 => ResourceType::Iron,
                2 => ResourceType::Gold,
                3 => ResourceType::Diamond,
                4 => ResourceType::Emerald,
                5 => ResourceType::Redstone,
                _ => ResourceType::Netherite,
            };

                spawn_resource(
                    &mut commands,
                    drop_pos,
                    resource_type,
                    &resource_textures,
                    &mut meshes,
                    &mut materials,
                );
                dropped += 1;
            }
        }

        info!("Dropped {} resources around hero in empty spaces", dropped);
    }
}

/// Handle villager spawn event
pub fn handle_villager_spawn_event(
    mut commands: Commands,
    hero_query: Query<&Transform, With<Hero>>,
    event_query: Query<(Entity, &EventNotification), Added<EventNotification>>,
    textures: Res<GameTextures>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    spatial_hash: Res<SpatialHash>,
    block_query: Query<&Transform, With<Block>>,
) {
    for (_notification_entity, notification) in event_query.iter() {
        if notification.event_type != RandomEventType::VillagerSpawn {
            continue;
        }

        let Ok(hero_transform) = hero_query.single() else {
            continue;
        };

        let hero_pos = hero_transform.translation;
        let mut rng = rand::thread_rng();

        // Spawn 2-4 villagers around the hero in empty spaces
        let num_villagers = rng.gen_range(2..=4);
        let mut spawned = 0;

        for _ in 0..num_villagers {
            // Find an empty position for the villager
            if let Some(villager_pos) = find_empty_spawn_position(hero_pos, 500.0, &spatial_hash, &block_query, &mut rng) {
                spawn_villager(
                    &mut commands,
                    villager_pos,
                    &textures,
                    &mut meshes,
                    &mut materials,
                    None,
                    &asset_server,
                );
                spawned += 1;
            }
        }

        info!("Spawned {} villagers near hero in empty spaces", spawned);
    }
}

/// Handle Grow buff event - applies Grow buff to hero
pub fn handle_grow_buff_event(
    mut commands: Commands,
    hero_query: Query<Entity, With<Hero>>,
    event_query: Query<(Entity, &EventNotification), Added<EventNotification>>,
    mut buff_query: Query<&mut crate::components::buff::Buff>,
) {
    for (notification_entity, notification) in event_query.iter() {
        if notification.event_type != RandomEventType::GrowBuff {
            continue;
        }

        let Ok(hero_entity) = hero_query.single() else {
            commands.entity(notification_entity).despawn();
            continue;
        };

        // Check if hero already has a Grow buff
        let has_grow_buff = if let Ok(buff) = buff_query.get(hero_entity) {
            buff.buff_type == crate::components::buff::BuffType::Grow
        } else {
            false
        };

        // Apply Grow buff (10 second duration, stacks up to 2 times)
        if has_grow_buff {
            // Stack with existing buff
            if let Ok(mut buff) = buff_query.get_mut(hero_entity) {
                crate::systems::buffs::buff_core::apply_buff(
                    &mut commands,
                    hero_entity,
                    crate::components::buff::BuffType::Grow,
                    10.0,
                    Some(&mut *buff),
                );
            }
        } else {
            // No existing buff, create new one
            crate::systems::buffs::buff_core::apply_buff(
                &mut commands,
                hero_entity,
                crate::components::buff::BuffType::Grow,
                10.0,
                None,
            );
        }

        info!("🌟 Giant's Blessing applied to hero!");
    }
}

/// Clean up expired event notifications
pub fn cleanup_event_notifications(
    mut commands: Commands,
    mut query: Query<(Entity, &mut EventNotification)>,
    time: Res<Time>,
) {
    for (entity, mut notification) in query.iter_mut() {
        notification.timer.tick(time.delta());

        if notification.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Helper function to spawn an enemy
fn spawn_enemy(
    commands: &mut Commands,
    position: Vec3,
    enemy_type: EnemyType,
    enemy_textures: &EnemyTextures,
    _mesh_cache: &MeshCache,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    asset_server: &AssetServer,
) {
    let mut rng = rand::thread_rng();

    // Special handling for Ghast - flying enemy with proper physics
    if enemy_type == EnemyType::Ghast {
        let ghast_texture = asset_server.load("textures/entities/enemy/ghast/ghast.png");

        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(ghast_texture),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        let ghast_entity = commands.spawn((
            Enemy::new(EnemyType::Ghast),
            Flying,  // Makes Ghast float and ignore block collisions
            Health::new(EnemyType::Ghast.max_health()),
            Buffable,
            SpriteBundle3d::new(
                meshes.add(Rectangle::new(80.0, 80.0)),
                material,
                position,
            ),
        )).id();

        // Add custom flying physics components
        commands.entity(ghast_entity).insert((
            RigidBody::Dynamic,
            LinearVelocity(Vec3::ZERO),
            GravityScale(0.0),  // No gravity - ghasts float
            LinearDamping(2.0),
            Mass(CHARACTER_MASS),
            Collider::cuboid(80.0, 80.0, 80.0),
            Restitution::new(ENEMY_BOUNCE),
            Friction::new(0.0),
            LockedAxes::ROTATION_LOCKED,
            CollisionLayers::new(GameLayer::Enemy, GameLayer::Enemy.filters()),
            CollisionEventsEnabled,
        ));

        info!("👻 Ghast spawned at {:?} via random event", position);
        return;
    }

    // Regular enemies (Skeleton, Yeti, Zombie, etc.)
    let texture = match enemy_type {
        EnemyType::Zombie => &enemy_textures.skeleton,
        EnemyType::Skeleton => &enemy_textures.skeleton,
        EnemyType::Pigman => &enemy_textures.skeleton2,
        EnemyType::Enderman => &enemy_textures.skeleton2,
        EnemyType::Yeti => &enemy_textures.yeti,
        _ => &enemy_textures.skeleton, // Fallback
    };

    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(texture.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    // Initial velocity for spawn (drop from above)
    let initial_velocity = Vec3::new(
        rng.gen_range(-50.0..50.0),
        rng.gen_range(50.0..150.0),
        0.0,
    );

    commands.spawn((
        Enemy::new(enemy_type),
        Health::new(enemy_type.max_health()),
        Buffable,
        avian_physics::enemy_with_velocity(initial_velocity),
        SpriteBundle3d::new(meshes.add(Rectangle::new(ENEMY_SIZE, ENEMY_SIZE)), material, position),
    ));
}

/// Helper function to spawn a resource
fn spawn_resource(
    commands: &mut Commands,
    position: Vec3,
    resource_type: ResourceType,
    resource_textures: &ResourceTextures,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let texture = match resource_type {
        ResourceType::Coal => &resource_textures.coal,
        ResourceType::Iron => &resource_textures.iron_ingot,
        ResourceType::Gold => &resource_textures.gold_ingot,
        ResourceType::Diamond => &resource_textures.diamond,
        ResourceType::Emerald => &resource_textures.emerald,
        ResourceType::Redstone => &resource_textures.redstone,
        ResourceType::Netherite => &resource_textures.netherite_ingot,
    };

    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(texture.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });

    let mut rng = rand::thread_rng();

    // Random upward velocity for dramatic effect
    let upward_velocity = rng.gen_range(100.0..200.0);

    // Resources should be at Z=5.0 (behind hero at Z=10.0, in front of blocks at Z=-10.0)
    // Add random offset to prevent Z-fighting
    let z_offset = rng.gen_range(0.0..2.0);
    let resource_position = Vec3::new(position.x, position.y, 5.0 + z_offset);

    commands.spawn((
        Resource { resource_type },
        RotatingResource::default(),
        avian_physics::resource(Vec3::new(0.0, upward_velocity, 0.0)),
        SpriteBundle3d::new(
            meshes.add(Rectangle::new(RESOURCE_SIZE, RESOURCE_SIZE)),
            material,
            resource_position,
        ),
    ));
}

/// Plugin to manage random events
pub struct RandomEventPlugin;

impl Plugin for RandomEventPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<RandomEventTimer>()
            .add_systems(
                Update,
                (
                    random_event_system,
                    handle_mob_spawn_event,
                    handle_elite_mob_spawn_event,
                    handle_loot_drop_event,
                    handle_villager_spawn_event,
                    handle_grow_buff_event,
                    cleanup_event_notifications,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<MeshCache>)
                    .run_if(resource_exists::<EnemyTextures>)
                    .run_if(resource_exists::<ResourceTextures>),
            );

        info!("RandomEventPlugin loaded");
    }
}
