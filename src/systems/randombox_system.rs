use bevy::prelude::*;
use avian3d::prelude::*;
use rand::Rng;

use crate::components::{
    hero::Hero,
    lootbox::RandomBox,
    bundles::SpriteBundle3d,
    collision_layers::GameLayer,
    block::Block,
};
use crate::resources::asset_handles::GameTextures;
use crate::resources::spatial_hash::SpatialHash;
use crate::systems::random_events::{RandomEventType, EventNotification};
use crate::states::GameState;
use crate::utils::constants::BLOCK_SIZE;

const RANDOMBOX_SIZE: f32 = 48.0; // Smaller than villagers (64px)
const RANDOMBOX_SPAWN_INTERVAL: f32 = 30.0; // Spawn every 30 seconds
const MAX_RANDOMBOXES: usize = 3; // Maximum number at once

/// Timer resource for spawning randomboxes
#[derive(Resource)]
pub struct RandomBoxSpawnTimer(pub Timer);

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
            base_pos.y + offset_y + 300.0, // Spawn 300px above to ensure it's in the air
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
                            // Check if we're too close to a block (within block size + randombox size)
                            if distance < (BLOCK_SIZE + RANDOMBOX_SIZE) * 1.5 {
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
    Some(Vec3::new(base_pos.x, base_pos.y + 400.0, 10.0))
}

/// Spawn randomboxes periodically
pub fn spawn_randombox_periodic(
    mut commands: Commands,
    textures: Res<GameTextures>,
    randombox_query: Query<&RandomBox>,
    hero_query: Query<&Transform, With<Hero>>,
    mut spawn_timer: ResMut<RandomBoxSpawnTimer>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_timer.0.tick(time.delta());

    if !spawn_timer.0.is_finished() {
        return;
    }

    spawn_timer.0.reset();

    // Don't spawn if we already have too many
    if randombox_query.iter().count() >= MAX_RANDOMBOXES {
        return;
    }

    // Get hero position to spawn nearby
    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let mut rng = rand::thread_rng();

    // Spawn randombox near hero (within 300-500 pixels)
    let distance = rng.gen_range(300.0..500.0);
    let angle = rng.gen_range(0.0..(2.0 * std::f32::consts::PI));

    let spawn_pos = Vec3::new(
        hero_transform.translation.x + distance * angle.cos(),
        hero_transform.translation.y + 200.0, // Spawn above
        hero_transform.translation.z,
    );

    spawn_randombox(&mut commands, spawn_pos, &textures, &mut meshes, &mut materials);

    info!("📦 RandomBox spawned at {:?}", spawn_pos);
}

/// Spawn a single randombox at a specific position
pub fn spawn_randombox(
    commands: &mut Commands,
    position: Vec3,
    textures: &GameTextures,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    spawn_randombox_internal(commands, position, textures, meshes, materials);
}

/// Spawn a single randombox with block collision checking
pub fn spawn_randombox_safe(
    commands: &mut Commands,
    hero_pos: Vec3,
    textures: &GameTextures,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    spatial_hash: &SpatialHash,
    block_query: &Query<&Transform, With<Block>>,
) {
    let mut rng = rand::thread_rng();

    // Try to find an empty position within 500-700px radius
    let spawn_pos = if let Some(pos) = find_empty_spawn_position(
        hero_pos,
        600.0, // radius
        spatial_hash,
        block_query,
        &mut rng,
    ) {
        pos
    } else {
        // Fallback to position directly above hero
        Vec3::new(hero_pos.x, hero_pos.y + 400.0, 10.0)
    };

    spawn_randombox_internal(commands, spawn_pos, textures, meshes, materials);
}

/// Internal function to spawn a randombox
fn spawn_randombox_internal(
    commands: &mut Commands,
    position: Vec3,
    textures: &GameTextures,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(textures.randombox.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });

    let mut rng = rand::thread_rng();
    let initial_velocity = Vec2::new(
        rng.gen_range(-50.0..50.0),
        rng.gen_range(-100.0..0.0),
    );

    commands.spawn((
        RandomBox,
        SpriteBundle3d::new(
            meshes.add(Rectangle::new(RANDOMBOX_SIZE, RANDOMBOX_SIZE)),
            material,
            position,
        ),
        RigidBody::Dynamic,
        LinearVelocity(initial_velocity.extend(0.0)),
        GravityScale(1.5), // Same as villagers
        Mass(1.0), // Same as villagers
        Collider::cuboid(RANDOMBOX_SIZE, RANDOMBOX_SIZE, RANDOMBOX_SIZE),
        // NO Sensor - they're solid objects that bounce!
        Restitution::new(1.0), // Same bounce as villagers
        Friction::new(0.0), // No friction like villagers
        LockedAxes::new().lock_rotation_x().lock_rotation_y().lock_rotation_z().lock_translation_z(),
        CollisionLayers::new(
            GameLayer::Character,  // Same as villagers - they're dynamic entities
            GameLayer::Character.filters(),
        ),
        CollisionEventsEnabled,
    ));
}

/// Handle hero collision with randomboxes using collision events
pub fn handle_randombox_collision(
    mut commands: Commands,
    mut collision_events: MessageReader<CollisionStart>,
    randombox_query: Query<Entity, With<RandomBox>>,
    hero_query: Query<Entity, With<Hero>>,
) {
    let Ok(hero_entity) = hero_query.single() else {
        return;
    };

    // Check collision events for hero touching randombox
    for event in collision_events.read() {
        let (entity1, entity2) = (event.collider1, event.collider2);

        // Check if hero collides with randombox (either order)
        let randombox_entity = if entity1 == hero_entity {
            Some(entity2)
        } else if entity2 == hero_entity {
            Some(entity1)
        } else {
            None
        };

        if let Some(randombox_entity) = randombox_entity {
            // Verify it's actually a randombox
            if randombox_query.get(randombox_entity).is_ok() {
                // Trigger random event by spawning notification
                let event_type = RandomEventType::random();

                info!("🎲 RandomBox opened via collision! Triggering event: {}", event_type.name());

                // Spawn event notification (will be handled by existing systems)
                commands.spawn(EventNotification {
                    event_type,
                    timer: Timer::from_seconds(3.0, TimerMode::Once),
                });

                // Despawn the randombox
                commands.entity(randombox_entity).despawn();

                break; // Only one per frame
            }
        }
    }
}

/// Plugin for randombox system
pub struct RandomBoxPlugin;

impl Plugin for RandomBoxPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(RandomBoxSpawnTimer(Timer::from_seconds(RANDOMBOX_SPAWN_INTERVAL, TimerMode::Repeating)))
            .add_systems(
                Update,
                (
                    spawn_randombox_periodic,
                    handle_randombox_collision,
                )
                    .run_if(in_state(GameState::Playing)),
            );

        info!("RandomBoxPlugin loaded");
    }
}
