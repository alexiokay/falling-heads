use avian3d::prelude::*;
use bevy::prelude::*;
use rand::RngExt;

use crate::components::{
    bundles::SpriteBundle3d, collision_layers::GameLayer, hero::Hero, lootbox::RandomBox,
};
use crate::resources::asset_handles::GameTextures;
use crate::systems::random_events::{EventNotification, RandomEventType};

const RANDOMBOX_SIZE: f32 = 48.0; // Smaller than villagers (64px)
const RANDOMBOX_SPAWN_INTERVAL: f32 = 30.0; // Spawn every 30 seconds
const MAX_RANDOMBOXES: usize = 3; // Maximum number at once

/// Timer resource for spawning randomboxes
#[derive(Resource)]
pub struct RandomBoxSpawnTimer(pub Timer);

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

    let mut rng = rand::rng();

    // Spawn randombox near hero (within 300-500 pixels)
    let distance = rng.random_range(300.0..500.0);
    let angle = rng.random_range(0.0..(2.0 * std::f32::consts::PI));

    let spawn_pos = Vec3::new(
        hero_transform.translation.x + distance * angle.cos(),
        hero_transform.translation.y + 200.0, // Spawn above
        hero_transform.translation.z,
    );

    spawn_randombox(
        &mut commands,
        spawn_pos,
        &textures,
        &mut meshes,
        &mut materials,
    );

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
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
        ..default()
    });

    let mut rng = rand::rng();
    let initial_velocity = Vec2::new(rng.random_range(-50.0..50.0), rng.random_range(-100.0..0.0));

    commands.spawn((
        RandomBox,
        crate::components::chunk::ChunkMember::at(position),
        SpriteBundle3d::new(
            meshes.add(Rectangle::new(RANDOMBOX_SIZE, RANDOMBOX_SIZE)),
            material,
            position,
        ),
        RigidBody::Dynamic,
        LinearVelocity(initial_velocity.extend(0.0)),
        GravityScale(1.5), // Same as villagers
        Mass(1.0),         // Same as villagers
        Collider::cuboid(RANDOMBOX_SIZE, RANDOMBOX_SIZE, RANDOMBOX_SIZE),
        // NO Sensor - they're solid objects that bounce!
        Restitution::new(1.0), // Same bounce as villagers
        Friction::new(0.0),    // No friction like villagers
        LockedAxes::new()
            .lock_rotation_x()
            .lock_rotation_y()
            .lock_rotation_z()
            .lock_translation_z(),
        CollisionLayers::new(
            GameLayer::Character, // Same as villagers - they're dynamic entities
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

                info!(
                    "🎲 RandomBox opened via collision! Triggering event: {}",
                    event_type.name()
                );

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
        app.insert_resource(RandomBoxSpawnTimer(Timer::from_seconds(
            RANDOMBOX_SPAWN_INTERVAL,
            TimerMode::Repeating,
        )))
        .add_systems(
            Update,
            (
                spawn_randombox_periodic.run_if(crate::systems::game_modes::mode_on(
                    crate::systems::game_modes::GameMode::RandomBoxes,
                )),
                handle_randombox_collision,
            )
                .run_if(in_state(crate::states::InGame)),
        );

        info!("RandomBoxPlugin loaded");
    }
}
