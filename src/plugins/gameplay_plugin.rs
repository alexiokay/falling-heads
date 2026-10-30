use crate::components::{
    background::BackgroundCache,
    block::{Block, BlockType},
    buff::Buffable,
    bundles::{SpriteBundle3d, avian_physics},
    chunk::LoadedChunks,
    enemy::{Enemy, Health},
    hero::Hero,
    power_effect::SpeedModifier,
    projectile::{Arrow, Fireball, Snowball},
    resource::{Resource, ResourceType, RotatingResource},
    villager::Villager,
};
use crate::events::block_events::{BlockDamaged, BlockDestroyed};
use crate::events::damage_events::{EntityDamaged, EntityDied};
use crate::events::resource_events::ResourceCollected;
use crate::resources::action_center::{ActionCenter, ActionWeights};
use crate::resources::asset_handles::GameTextures;
use crate::resources::biome_music::BiomeMusicPlayer;
use crate::resources::inventory::Inventory;
use crate::states::GameState;
use crate::systems::background_system::update_background;
use crate::systems::biome_music_system::update_biome_music;
use crate::systems::entity_spawn::spawn_villagers_around_position;
use crate::systems::health_system::{
    cleanup_marked_entities, handle_entity_damage, handle_entity_death, update_death_animations,
};
use crate::systems::ui::health_ui::{spawn_health_bars, update_health_bars};
use crate::systems::world_generation::{despawn_distant_chunks, generate_chunks_around_hero};
use crate::utils::constants::*;
use avian3d::prelude::*;
use bevy::ecs::schedule::SystemCondition;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;

#[derive(Resource, Default, PartialEq)]
struct EntitiesSpawned(bool);

/// Tracks the spawn area that should be kept clear of blocks
#[derive(Resource)]
pub struct SpawnArea {
    pub center: Vec3,
    pub radius: f32, // Radius in world units
}

impl Default for SpawnArea {
    fn default() -> Self {
        Self {
            center: Vec3::new(0.0, 130000.0, 10.0), // Forest biome spawn
            radius: 200.0,                          // 200 units radius = ~3 blocks
        }
    }
}

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BlockDamaged>()
            .add_message::<BlockDestroyed>()
            .add_message::<ResourceCollected>()
            .add_message::<EntityDamaged>()
            .add_message::<EntityDied>()
            .init_resource::<Inventory>()
            .init_resource::<LoadedChunks>()
            .init_resource::<SpawnArea>()
            .init_resource::<BackgroundCache>()
            .init_resource::<ActionCenter>()
            .init_resource::<BiomeMusicPlayer>()
            .init_resource::<crate::resources::lootbox_state::LootboxState>()
            .add_systems(
                OnEnter(GameState::Playing),
                (spawn_hero, spawn_villagers).run_if(resource_equals(EntitiesSpawned(false))),
            )
            .add_systems(Startup, start_game)
            .init_resource::<EntitiesSpawned>()
            .add_systems(
                Update,
                (
                    handle_block_damage,
                    spawn_resources_from_destroyed_blocks,
                    collect_resources,
                    rotate_resources,
                    update_block_visuals,
                    // Run chunk management every 0.1s for balance of performance and responsiveness
                    (generate_chunks_around_hero, despawn_distant_chunks).run_if(
                        on_timer(std::time::Duration::from_millis(100)).or_else(
                            |teleport: Res<crate::systems::biome_teleport::BiomeTeleport>| {
                                teleport.target.is_some()
                            },
                        ),
                    ),
                    // Clean up unused materials every 5 seconds
                    cleanup_unused_materials.run_if(on_timer(std::time::Duration::from_secs(5))),
                    update_speed_modifiers,
                    handle_entity_damage,
                    handle_entity_death,
                    update_death_animations,
                    spawn_health_bars,
                    update_health_bars,
                    // Reconcile only off-screen tile edges after the camera moves.

                    // Biome music system updates music based on hero position
                    // Music starts once the loading screen is gone.
                    update_biome_music.run_if(not(resource_exists::<
                        crate::systems::ui::loading_screen::LoadingActive,
                    >)),
                )
                    // The world keeps running behind the Slime Rift.
                    .run_if(in_state(crate::states::InGame)),
            )
            .add_systems(
                Update,
                // Hero keyboard control and the camera belong to normal play; the
                // Rift steers its own runner and camera.
                (
                    hero_movement,
                    calculate_action_center,
                    camera_follow_action_center.after(calculate_action_center),
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                PostUpdate,
                update_background
                    .after(crate::systems::biome_teleport::finish_biome_teleport)
                    .before(TransformSystems::Propagate)
                    // Also in the Slime Rift, whose camera shows the real world behind the course.
                    .run_if(in_state(crate::states::InGame)),
            )
            .add_systems(
                Update,
                (
                    // Cobweb collision detection
                    crate::systems::cobweb_system::cobweb_collision_system
                        .before(crate::systems::cobweb_system::apply_cobweb_slow),
                )
                    .run_if(in_state(crate::states::InGame)),
            )
            .add_systems(
                Update,
                // Cobweb slow must run BEFORE physics collision to prevent clipping
                crate::systems::cobweb_system::apply_cobweb_slow
                    .after(crate::plugins::physics_plugin::PhysicsSet::AI)
                    .before(crate::plugins::physics_plugin::PhysicsSet::Collision)
                    .run_if(in_state(crate::states::InGame)),
            )
            // Cleanup dead entities in PostUpdate after all damage processing is done
            .add_systems(
                PostUpdate,
                cleanup_marked_entities.run_if(in_state(crate::states::InGame)),
            );

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
        alpha_mode: AlphaMode::Mask(0.5),
        ..default()
    });

    commands.spawn((
        Hero {
            move_direction: 1.0, // Start moving right
            direction_change_timer: Timer::from_seconds(
                VILLAGER_DIRECTION_CHANGE_INTERVAL,
                TimerMode::Repeating,
            ),
        },
        Health::new(3),
        Buffable,
        crate::components::experience::Experience::default(), // Start at level 1 with 0 XP
        avian_physics::character(), // Avian physics: RigidBody, Collider, LinearVelocity, etc.
        SpriteBundle3d::new(
            meshes.add(Rectangle::new(HERO_SIZE, HERO_SIZE)),
            material,
            Vec3::new(0.0, 130000.0, 10.0), // Spawn in middle of Forest biome
        ),
    ));

    info!("Hero spawned in Forest biome (Y=130000) with XP system");
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
        Vec3::new(0.0, 130000.0, 10.0) // Default to Forest biome spawn
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
        None, // No username for initial spawned villagers
        &asset_server,
    );

    info!("Spawned 5 villagers near hero at {:?}", hero_pos);
}

/// Optional player control - ADDS to automatic AI movement
/// Hero moves automatically like villagers, but player can override/boost with A/D keys
fn hero_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<
        (
            &mut LinearVelocity,
            &mut Hero,
            Option<&SpeedModifier>,
            Option<&crate::components::buff::Buff>,
            Option<&crate::components::buff::GrowthPower>,
            Option<&crate::components::surface_grip::SurfaceGrip>,
        ),
        (
            With<Hero>,
            Without<crate::systems::launcher::LauncherPassenger>,
        ),
    >,
    time: Res<Time>,
    fuss: Option<Res<crate::systems::extra_powers::Fuss>>,
) {
    let Ok((mut velocity, mut hero, speed_mod, buff_opt, growth, grip)) = query.single_mut() else {
        return;
    };
    // FUSS power: left and right are swapped while it lasts.
    let mirror = fuss.map_or(1.0, |f| f.steer_sign(time.elapsed_secs()));

    let mut acceleration = 800.0; // Stronger player override acceleration (was 500)

    // Calculate total speed multiplier from both SpeedModifier (powers) and Buff
    let mut speed_multiplier = 1.0;

    // Apply SpeedModifier from powers (e.g., !FAST command)
    if let Some(modifier) = speed_mod {
        speed_multiplier *= modifier.multiplier;
    }

    // Apply Buff velocity multiplier (e.g., Grow, Speed, Slowness buffs)
    if let Some(buff) = buff_opt {
        use crate::components::buff::BuffBehavior;
        speed_multiplier *= buff.buff_type.velocity_multiplier();
    }

    if growth.is_some()
        && buff_opt.is_none_or(|b| b.buff_type != crate::components::buff::BuffType::Grow)
    {
        speed_multiplier *= 1.3;
    }
    acceleration *= speed_multiplier;

    // ADD to velocity (preserves automatic AI movement + physics)
    let steer = if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        -mirror
    } else if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        mirror
    } else {
        0.0
    };
    if steer != 0.0 {
        velocity.x += steer
            * acceleration
            * time.delta_secs()
            * grip.map_or(1., |g| g.steering(velocity.x, steer, time.elapsed_secs()));
        hero.move_direction = steer; // Override AI direction when player presses key
    }

    // Cap max horizontal velocity (higher than AI clamp so player can boost past it)
    let max_velocity = 600.0 * speed_multiplier;
    velocity.x = velocity.x.clamp(-max_velocity, max_velocity);
}

pub(crate) fn handle_block_damage(
    mut commands: Commands,
    mut damage_events: MessageReader<BlockDamaged>,
    mut destroy_events: MessageWriter<BlockDestroyed>,
    mut block_query: Query<(
        &mut Block,
        &BlockType,
        &Transform,
        Option<&crate::systems::biome_content::Tile>,
    )>,
) {
    for event in damage_events.read() {
        if let Ok((mut block, block_type, transform, tile)) = block_query.get_mut(event.entity) {
            if block.health == 0 {
                continue;
            }
            // Cobwebs are indestructible by hero collision (only TNT/explosions can destroy them)
            if *block_type == BlockType::Cobweb {
                continue;
            }

            block.health = block.health.saturating_sub(event.damage);

            if block.health == 0 {
                // Block destroyed!
                destroy_events.write(BlockDestroyed {
                    tile: tile.copied(),
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

/// Break a block outside the damage path (explosions, beams, thunder). Sends the
/// same `BlockDestroyed` as mining so drops, XP, sounds and biome reactions
/// happen. `broken` stops overlapping blasts destroying one block twice.
pub(crate) fn break_block(
    commands: &mut Commands,
    destroyed: &mut MessageWriter<BlockDestroyed>,
    broken: &mut std::collections::HashSet<Entity>,
    entity: Entity,
    block_type: BlockType,
    position: Vec3,
    tile: Option<crate::systems::biome_content::Tile>,
) {
    if !broken.insert(entity) {
        return;
    }
    destroyed.write(BlockDestroyed {
        tile,
        entity,
        block_type,
        position,
    });
    commands.entity(entity).try_despawn();
}

fn update_block_visuals(
    mut commands: Commands,
    query: Query<
        (Entity, &BlockType),
        (
            With<Block>,
            Without<MeshMaterial3d<StandardMaterial>>,
            Without<MeshMaterial3d<crate::systems::terrain_cracks::Cracked>>,
        ),
    >,
    material_cache: Res<crate::resources::material_cache::MaterialCache>,
) {
    // Fill missing visuals only. Generated species and grass-topped ground already
    // have shared materials; replacing those on damage loses their appearance.
    for (entity, block_type) in query.iter() {
        if let Some(cached_material) = material_cache.get(*block_type) {
            commands
                .entity(entity)
                .try_insert(MeshMaterial3d(cached_material));
        }
    }
}

fn spawn_resources_from_destroyed_blocks(
    mut commands: Commands,
    mut destroy_events: MessageReader<BlockDestroyed>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    resource_textures: Res<crate::resources::asset_handles::ResourceTextures>,
    mut drop_materials: Local<std::collections::HashMap<ResourceType, Handle<StandardMaterial>>>,
    mut drop_mesh: Local<Option<Handle<Mesh>>>,
) {
    for event in destroy_events.read() {
        let (resource_type, texture) = match event.block_type {
            BlockType::Coal => (ResourceType::Coal, resource_textures.coal.clone()),
            BlockType::Iron => (ResourceType::Iron, resource_textures.iron_ingot.clone()),
            BlockType::Gold => (ResourceType::Gold, resource_textures.gold_ingot.clone()),
            BlockType::Diamond => (ResourceType::Diamond, resource_textures.diamond.clone()),
            BlockType::Emerald => (ResourceType::Emerald, resource_textures.emerald.clone()),
            BlockType::Redstone => (ResourceType::Redstone, resource_textures.redstone.clone()),
            BlockType::Netherite | BlockType::AncientDebris => (
                ResourceType::Netherite,
                resource_textures.netherite_ingot.clone(),
            ),
            // Terrain and decorative blocks don't drop resources
            _ => continue,
        };

        // One shared material and mesh per drop type instead of new assets per drop.
        let material = drop_materials
            .entry(resource_type)
            .or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(texture),
                    unlit: true,
                    alpha_mode: AlphaMode::Mask(0.5),
                    cull_mode: None,
                    ..default()
                })
            })
            .clone();
        let mesh = drop_mesh
            .get_or_insert_with(|| meshes.add(Rectangle::new(RESOURCE_SIZE, RESOURCE_SIZE)))
            .clone();

        // Random Z offset to prevent Z-fighting when resources stack
        let z_offset = rand::random::<f32>() * 2.0; // 0.0 to 2.0 range
        let drop_position = Vec3::new(event.position.x, event.position.y, 5.0 + z_offset);

        commands.spawn((
            Resource { resource_type },
            RotatingResource::default(),
            // Uncollected drops unload with the chunk they fell in.
            crate::components::chunk::ChunkMember::at(drop_position),
            avian_physics::resource(Vec3::new(0.0, RESOURCE_DROP_VELOCITY_Y, 0.0)), // Avian physics for gravity/collisions
            SpriteBundle3d::new(mesh, material, drop_position),
        ));
    }
}

/// Rotate resources on the Y axis (coin-flip rotation for 2D sprites)
fn rotate_resources(time: Res<Time>, mut query: Query<(&RotatingResource, &mut Transform)>) {
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
    let collection_range = (HERO_SIZE / 2.0) + (RESOURCE_SIZE / 2.0); // 32.0 + 16.0 = 48.0

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

/// Calculate the center of action based on hero, combat, villagers, and projectiles
/// This determines where the camera should focus
fn calculate_action_center(
    mut action_center: ResMut<ActionCenter>,
    hero_query: Query<&Transform, With<Hero>>,
    enemy_query: Query<&Transform, With<Enemy>>,
    villager_query: Query<&Transform, With<Villager>>,
    projectile_query: Query<&Transform, Or<(With<Arrow>, With<Snowball>, With<Fireball>)>>,
    damage_events: MessageReader<EntityDamaged>,
    time: Res<Time>,
) {
    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let hero_pos = hero_transform.translation;

    // Detect combat: count recent damage events and nearby enemies
    let damage_count = damage_events.len();
    let nearby_enemies = enemy_query
        .iter()
        .filter(|t| t.translation.distance(hero_pos) < 800.0)
        .count();

    // Calculate combat intensity (0.0 to 1.0)
    let recent_combat = if damage_count > 0 || nearby_enemies > 0 {
        time.elapsed_secs()
    } else {
        action_center.last_combat_time
    };

    // Combat intensity decays over time (stays high for 3 seconds after last combat)
    let time_since_combat = time.elapsed_secs() - recent_combat;
    let combat_intensity = (1.0 - (time_since_combat / 3.0)).clamp(0.0, 1.0);

    action_center.last_combat_time = recent_combat;
    action_center.combat_intensity = combat_intensity;

    // Get combat-adjusted weights
    let weights = ActionWeights::default().adjust_for_combat(combat_intensity);

    // Calculate weighted center positions
    let mut total_weight = weights.hero;
    let mut weighted_center = hero_pos * weights.hero;

    // Add combat positions (enemies within range)
    let combat_positions: Vec<Vec3> = enemy_query
        .iter()
        .filter(|t| t.translation.distance(hero_pos) < 1000.0)
        .map(|t| t.translation)
        .collect();

    if !combat_positions.is_empty() {
        let combat_center = combat_positions.iter().sum::<Vec3>() / combat_positions.len() as f32;
        weighted_center += combat_center * weights.combat;
        total_weight += weights.combat;
    }

    // Add villager positions (nearby villagers)
    let villager_positions: Vec<Vec3> = villager_query
        .iter()
        .filter(|t| t.translation.distance(hero_pos) < 800.0)
        .map(|t| t.translation)
        .collect();

    if !villager_positions.is_empty() {
        let villager_center =
            villager_positions.iter().sum::<Vec3>() / villager_positions.len() as f32;
        weighted_center += villager_center * weights.villagers;
        total_weight += weights.villagers;
    }

    // Add projectile positions (active projectiles)
    let projectile_positions: Vec<Vec3> = projectile_query
        .iter()
        .filter(|t| t.translation.distance(hero_pos) < 1000.0)
        .map(|t| t.translation)
        .collect();

    if !projectile_positions.is_empty() {
        let projectile_center =
            projectile_positions.iter().sum::<Vec3>() / projectile_positions.len() as f32;
        weighted_center += projectile_center * weights.projectiles;
        total_weight += weights.projectiles;
    }

    // Normalize by total weight to get raw center
    let new_raw_center = weighted_center / total_weight;

    // CRITICAL: Clamp maximum movement per frame to prevent teleportation
    // Limit is per second (500 units per frame at 60 FPS) so it holds at any frame rate
    let max_jump = 30_000.0 * time.delta_secs();
    let distance_from_current = new_raw_center.distance(action_center.raw_center);

    if distance_from_current > max_jump {
        // Huge jump detected - clamp it to prevent camera teleport
        let direction = (new_raw_center - action_center.raw_center).normalize_or_zero();
        action_center.raw_center = action_center.raw_center + direction * max_jump;
    } else {
        action_center.raw_center = new_raw_center;
    }

    // Apply heavy smoothing to prevent camera jumps
    // Use very slow lerp (2.0) so camera transitions are extremely gradual
    let smoothing_speed = 2.0; // Much slower than before
    let smooth_factor = 1.0 - (-smoothing_speed * time.delta_secs()).exp();

    action_center.center = action_center
        .center
        .lerp(action_center.raw_center, smooth_factor);
}

/// Camera follows the action center with smooth transitions
/// Focuses on where the action is happening, not just the hero
fn camera_follow_action_center(
    action_center: Res<ActionCenter>,
    hero_query: Query<&Transform, With<Hero>>,
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<Hero>)>,
    time: Res<Time>,
) {
    let Ok(_hero_transform) = hero_query.single() else {
        return;
    };

    // DEBUG: Log frame freezes (delta time > 50ms = freeze)
    if time.delta_secs() > 0.05 {
        warn!(
            "⚠️ CAMERA FREEZE DETECTED: Frame took {:.1}ms (target: 16.7ms)",
            time.delta_secs() * 1000.0
        );
    }

    let target_x = action_center.center.x;
    let target_y = action_center.center.y;

    // Follow with 3D camera (keep isometric angle) - follow both X and Y
    if let Ok(mut camera_transform) = camera_query.single_mut() {
        let offset_x = 30.0; // Keep camera slightly offset
        let offset_y = 80.0; // Keep camera above action
        let offset_z = 1500.0; // Keep camera at fixed Z distance for consistent perspective

        // Use consistent slow camera speed to prevent jumpy movement
        // Much slower than original (was 30.0, now 8.0) for smooth, stable camera
        let camera_lerp_speed = 8.0;
        let lerp_factor = (camera_lerp_speed * time.delta_secs()).min(1.0);

        let target_camera_x = target_x + offset_x;
        let target_camera_y = target_y + offset_y;

        camera_transform.translation.x +=
            (target_camera_x - camera_transform.translation.x) * lerp_factor;
        camera_transform.translation.y +=
            (target_camera_y - camera_transform.translation.y) * lerp_factor;
        camera_transform.translation.z = offset_z;

        // Update camera rotation to always look at action center (Z=0 plane)
        // CRITICAL FIX: Use look_to instead of looking_at to avoid position snapping
        let look_target = Vec3::new(target_x, target_y, 0.0);
        let direction = (look_target - camera_transform.translation).normalize_or_zero();
        if direction != Vec3::ZERO {
            camera_transform.look_to(direction, Vec3::Y);
        }
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
fn cleanup_unused_materials(materials: ResMut<Assets<StandardMaterial>>) {
    // Note: Bevy 0.17 automatically handles unused asset cleanup
    // The Assets<T> resource will clean up unreferenced materials automatically
    // This function serves as a periodic trigger point if needed in future
    let material_count = materials.len();
    if material_count > 1000 {
        info!(
            "Material count: {} (cleanup running automatically)",
            material_count
        );
    }
}

#[cfg(test)]
mod species_visual_regression {
    use super::*;
    #[test]
    fn block_health_changes_preserve_shared_species_and_surface_materials() {
        let mut app = App::new();
        app.init_resource::<crate::resources::material_cache::MaterialCache>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Update, update_block_visuals);
        let base = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let species = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        app.world_mut()
            .resource_mut::<crate::resources::material_cache::MaterialCache>()
            .materials
            .insert(BlockType::Wood, base.clone());
        let tree = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                BlockType::Wood,
                MeshMaterial3d(species.clone()),
            ))
            .id();
        let missing = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                BlockType::Wood,
            ))
            .id();
        let cracked = app
            .world_mut()
            .spawn((
                Block {
                    health: 1,
                    max_health: 3,
                },
                BlockType::Wood,
                MeshMaterial3d::<crate::systems::terrain_cracks::Cracked>(default()),
            ))
            .id();
        app.update();
        app.world_mut().get_mut::<Block>(tree).unwrap().health = 2;
        app.update();
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(tree)
                .unwrap()
                .0,
            species
        );
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(missing)
                .unwrap()
                .0,
            base
        );
        assert!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(cracked)
                .is_none()
        );
    }
}

#[cfg(test)]
mod surface_control_tests {
    use super::*;
    use crate::components::surface_grip::SurfaceGrip;
    #[test]
    fn player_can_counter_steer_but_snow_and_ice_preserve_more_momentum() {
        for (grip, expected) in [
            (None, 70.),
            (Some(SurfaceGrip::snow(0.)), 106.),
            (Some(SurfaceGrip::ice(0.)), 134.),
            (Some(SurfaceGrip::snow(-1.)), 70.),
        ] {
            let mut app = App::new();
            let mut clock = Time::<()>::default();
            clock.advance_by(std::time::Duration::from_secs_f32(0.1));
            let mut keys = ButtonInput::<KeyCode>::default();
            keys.press(KeyCode::KeyA);
            app.insert_resource(clock)
                .insert_resource(keys)
                .add_systems(Update, hero_movement);
            let e = app
                .world_mut()
                .spawn((
                    Hero {
                        move_direction: 1.,
                        direction_change_timer: Timer::default(),
                    },
                    LinearVelocity(Vec3::new(150., 400., 0.)),
                ))
                .id();
            if let Some(grip) = grip {
                app.world_mut().entity_mut(e).insert(grip);
            }
            app.update();
            let v = app.world().get::<LinearVelocity>(e).unwrap();
            assert!((v.x - expected).abs() < 0.01);
            assert_eq!(v.y, 400.);
        }
    }
}
