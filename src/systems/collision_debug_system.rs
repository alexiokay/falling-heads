use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use avian3d::prelude::*;
use crate::components::collision_debug::{CollisionDebugBorder, CollisionDebugSettings};
use crate::components::block::{Block, BlockType};
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::enemy::Enemy;
use crate::components::resource::Resource;
use crate::components::explosive::Tnt;
use crate::components::projectile::{Arrow, Snowball, Fireball};
use crate::components::lootbox::{RandomBox, Lootbox};
use crate::states::GameState;
use crate::utils::constants::*;

pub struct CollisionDebugPlugin;

impl Plugin for CollisionDebugPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<CollisionDebugSettings>()
            .add_systems(Update, (
                toggle_collision_debug,
                spawn_debug_borders,
                update_debug_border_positions,
                update_debug_border_visibility,
                cleanup_debug_borders,
                debug_hover_info,
            ).run_if(in_state(GameState::Playing)));

        info!("CollisionDebugPlugin loaded - Press F3 to toggle collision borders");
    }
}

/// Toggle collision debug with F3 key
fn toggle_collision_debug(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<CollisionDebugSettings>,
) {
    if keyboard.just_pressed(KeyCode::F3) {
        settings.toggle();
        if settings.enabled {
            info!("🔍 Collision debug borders ENABLED");
        } else {
            info!("🔍 Collision debug borders DISABLED");
        }
    }
}

/// Spawn debug border entities for objects that don't have them yet
fn spawn_debug_borders(
    mut commands: Commands,
    settings: Res<CollisionDebugSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    // Query all entities that need debug borders
    block_query: Query<(Entity, &Transform), (With<Block>, Without<CollisionDebugBorder>)>,
    character_query: Query<(Entity, &Transform), (Or<(With<Hero>, With<Villager>, With<Enemy>)>, Without<CollisionDebugBorder>)>,
    resource_query: Query<(Entity, &Transform), (With<Resource>, Without<CollisionDebugBorder>)>,
    tnt_query: Query<(Entity, &Transform), (With<Tnt>, Without<CollisionDebugBorder>)>,
    projectile_query: Query<(Entity, &Transform), (Or<(With<Arrow>, With<Snowball>, With<Fireball>)>, Without<CollisionDebugBorder>)>,
    randombox_query: Query<(Entity, &Transform), (Or<(With<RandomBox>, With<Lootbox>)>, Without<CollisionDebugBorder>)>,
) {
    if !settings.enabled {
        return;
    }

    // Create shared wireframe material (green semi-transparent)
    let border_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 1.0, 0.0, 0.5),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    // Spawn borders for blocks (64x64)
    if settings.show_blocks {
        for (entity, transform) in block_query.iter().take(20) {  // Limit to 20 per frame to prevent freezes
            let border_mesh = meshes.add(Cuboid::new(BLOCK_SIZE, BLOCK_SIZE, BLOCK_SIZE));

            // Spawn border at block position (NOT as child to avoid transform issues)
            commands.spawn((
                CollisionDebugBorder {
                    parent_entity: entity,
                    size: Vec3::splat(BLOCK_SIZE),
                },
                Mesh3d(border_mesh),
                MeshMaterial3d(border_material.clone()),
                Transform::from_translation(transform.translation),
            ));
        }
    }

    // Spawn borders for characters (64x64)
    if settings.show_characters {
        for (entity, transform) in character_query.iter() {
            let border_mesh = meshes.add(Cuboid::new(HERO_SIZE, HERO_SIZE, HERO_SIZE));

            commands.spawn((
                CollisionDebugBorder {
                    parent_entity: entity,
                    size: Vec3::splat(HERO_SIZE),
                },
                Mesh3d(border_mesh),
                MeshMaterial3d(border_material.clone()),
                Transform::from_translation(transform.translation),
            ));
        }
    }

    // Spawn borders for resources (32x32)
    if settings.show_resources {
        for (entity, transform) in resource_query.iter() {
            let border_mesh = meshes.add(Cuboid::new(RESOURCE_SIZE, RESOURCE_SIZE, RESOURCE_SIZE));

            commands.spawn((
                CollisionDebugBorder {
                    parent_entity: entity,
                    size: Vec3::splat(RESOURCE_SIZE),
                },
                Mesh3d(border_mesh),
                MeshMaterial3d(border_material.clone()),
                Transform::from_translation(transform.translation),
            ));
        }
    }

    // Spawn borders for TNT (64x64)
    if settings.show_tnt {
        for (entity, transform) in tnt_query.iter() {
            let border_mesh = meshes.add(Cuboid::new(BLOCK_SIZE, BLOCK_SIZE, BLOCK_SIZE));

            commands.spawn((
                CollisionDebugBorder {
                    parent_entity: entity,
                    size: Vec3::splat(BLOCK_SIZE),
                },
                Mesh3d(border_mesh),
                MeshMaterial3d(border_material.clone()),
                Transform::from_translation(transform.translation),
            ));
        }
    }

    // Spawn borders for projectiles (small, 16x16)
    if settings.show_projectiles {
        for (entity, transform) in projectile_query.iter() {
            let border_mesh = meshes.add(Cuboid::new(16.0, 16.0, 16.0));

            commands.spawn((
                CollisionDebugBorder {
                    parent_entity: entity,
                    size: Vec3::splat(16.0),
                },
                Mesh3d(border_mesh),
                MeshMaterial3d(border_material.clone()),
                Transform::from_translation(transform.translation),
            ));
        }
    }

    // Spawn borders for randomboxes (48x48)
    if settings.show_randomboxes {
        for (entity, transform) in randombox_query.iter() {
            let border_mesh = meshes.add(Cuboid::new(48.0, 48.0, 48.0));

            commands.spawn((
                CollisionDebugBorder {
                    parent_entity: entity,
                    size: Vec3::splat(48.0),
                },
                Mesh3d(border_mesh),
                MeshMaterial3d(border_material.clone()),
                Transform::from_translation(transform.translation),
            ));
        }
    }
}

/// Update border positions to follow their parent entities
fn update_debug_border_positions(
    mut border_query: Query<(&CollisionDebugBorder, &mut Transform)>,
    entity_transforms: Query<&Transform, Without<CollisionDebugBorder>>,
) {
    for (border, mut border_transform) in border_query.iter_mut() {
        // Update border position to match parent
        if let Ok(parent_transform) = entity_transforms.get(border.parent_entity) {
            border_transform.translation = parent_transform.translation;
        }
    }
}

/// Update border visibility based on settings
fn update_debug_border_visibility(
    settings: Res<CollisionDebugSettings>,
    mut border_query: Query<&mut Visibility, With<CollisionDebugBorder>>,
) {
    let visibility = if settings.enabled {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for mut vis in border_query.iter_mut() {
        *vis = visibility;
    }
}

/// Clean up debug borders when parent entities are despawned
fn cleanup_debug_borders(
    mut commands: Commands,
    border_query: Query<(Entity, &CollisionDebugBorder)>,
    entity_check: Query<Entity>,
) {
    for (border_entity, border) in border_query.iter() {
        // If parent doesn't exist anymore, despawn the border
        if entity_check.get(border.parent_entity).is_err() {
            commands.entity(border_entity).despawn();
        }
    }
}

/// Show debug info when hovering over collision borders with mouse
/// Click on a border to log detailed information to console
fn debug_hover_info(
    settings: Res<CollisionDebugSettings>,
    border_query: Query<(&CollisionDebugBorder, &Transform)>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mouse_button: Res<ButtonInput<MouseButton>>,

    // Query all entity types for inspection
    block_query: Query<(&Transform, &BlockType, Option<&LinearVelocity>), With<Block>>,
    hero_query: Query<(&Transform, &LinearVelocity), With<Hero>>,
    villager_query: Query<(&Transform, &LinearVelocity), With<Villager>>,
    enemy_query: Query<(&Transform, &LinearVelocity), With<Enemy>>,
    resource_query: Query<(&Transform, &Resource, Option<&LinearVelocity>)>,
    tnt_query: Query<(&Transform, Option<&LinearVelocity>), With<Tnt>>,
    projectile_query: Query<(&Transform, &LinearVelocity), Or<(With<Arrow>, With<Snowball>, With<Fireball>)>>,
    randombox_query: Query<(&Transform, Option<&LinearVelocity>), Or<(With<RandomBox>, With<Lootbox>)>>,
) {
    if !settings.enabled {
        return;
    }

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Ok(window) = window_query.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Get 3D ray from cursor position
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    // Find closest border to mouse cursor
    let mut closest_border: Option<(Entity, f32)> = None;

    for (border, border_transform) in border_query.iter() {
        let border_pos = border_transform.translation;

        // Simple distance check (not perfect raycasting but good enough for debug)
        let distance_to_ray = (border_pos - ray.origin).length();

        if distance_to_ray < 100.0 {  // Within 100 units
            if closest_border.is_none() || distance_to_ray < closest_border.unwrap().1 {
                closest_border = Some((border.parent_entity, distance_to_ray));
            }
        }
    }

    if let Some((entity, _distance)) = closest_border {
        // Mouse is hovering over a border - check if clicked
        if mouse_button.just_pressed(MouseButton::Left) {
            info!("\n🔍 ============ COLLISION DEBUG INFO ============");
            info!("Entity: {:?}", entity);

            // Check what type of entity this is
            if let Ok((transform, _block_type, velocity)) = block_query.get(entity) {
                info!("Type: BLOCK");
                info!("Position: ({:.1}, {:.1}, {:.1})", transform.translation.x, transform.translation.y, transform.translation.z);
                if let Some(vel) = velocity {
                    info!("Velocity: ({:.1}, {:.1}, {:.1})", vel.x, vel.y, vel.z);
                }
            } else if let Ok((transform, velocity)) = hero_query.get(entity) {
                info!("Type: HERO");
                info!("Position: ({:.1}, {:.1}, {:.1})", transform.translation.x, transform.translation.y, transform.translation.z);
                info!("Velocity: ({:.1}, {:.1}, {:.1})", velocity.x, velocity.y, velocity.z);
            } else if let Ok((transform, velocity)) = villager_query.get(entity) {
                info!("Type: VILLAGER");
                info!("Position: ({:.1}, {:.1}, {:.1})", transform.translation.x, transform.translation.y, transform.translation.z);
                info!("Velocity: ({:.1}, {:.1}, {:.1})", velocity.x, velocity.y, velocity.z);
            } else if let Ok((transform, velocity)) = enemy_query.get(entity) {
                info!("Type: ENEMY");
                info!("Position: ({:.1}, {:.1}, {:.1})", transform.translation.x, transform.translation.y, transform.translation.z);
                info!("Velocity: ({:.1}, {:.1}, {:.1})", velocity.x, velocity.y, velocity.z);
            } else if let Ok((transform, _resource, velocity)) = resource_query.get(entity) {
                info!("Type: RESOURCE");
                info!("Position: ({:.1}, {:.1}, {:.1})", transform.translation.x, transform.translation.y, transform.translation.z);
                if let Some(vel) = velocity {
                    info!("Velocity: ({:.1}, {:.1}, {:.1})", vel.x, vel.y, vel.z);
                } else {
                    warn!("⚠️ GHOST RESOURCE: No velocity component! Resource exists but missing physics!");
                }
            } else if let Ok((transform, velocity)) = tnt_query.get(entity) {
                info!("Type: TNT");
                info!("Position: ({:.1}, {:.1}, {:.1})", transform.translation.x, transform.translation.y, transform.translation.z);
                if let Some(vel) = velocity {
                    info!("Velocity: ({:.1}, {:.1}, {:.1})", vel.x, vel.y, vel.z);
                }
            } else if let Ok((transform, velocity)) = projectile_query.get(entity) {
                info!("Type: PROJECTILE");
                info!("Position: ({:.1}, {:.1}, {:.1})", transform.translation.x, transform.translation.y, transform.translation.z);
                info!("Velocity: ({:.1}, {:.1}, {:.1})", velocity.x, velocity.y, velocity.z);
            } else if let Ok((transform, velocity)) = randombox_query.get(entity) {
                info!("Type: RANDOMBOX");
                info!("Position: ({:.1}, {:.1}, {:.1})", transform.translation.x, transform.translation.y, transform.translation.z);
                if let Some(vel) = velocity {
                    info!("Velocity: ({:.1}, {:.1}, {:.1})", vel.x, vel.y, vel.z);
                }
            } else {
                warn!("⚠️ GHOST ENTITY: Entity exists but has no recognized components!");
                warn!("This is likely a despawned entity with lingering collision!");
            }

            info!("============================================\n");
        }
    }
}
