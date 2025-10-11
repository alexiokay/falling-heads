use bevy::prelude::*;
use avian3d::prelude::{LinearVelocity, Restitution, Mass};
use crate::components::frozen::FrozenStacks;
use crate::components::projectile::Snowball;
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::block::Block;
use crate::resources::spatial_hash::SpatialHash;
use crate::states::GameState;

/// Marker component for snowball splash particles
#[derive(Component)]
struct SplashParticle {
    lifetime: Timer,
}

/// Marker component for freeze overlay visual on entities
#[derive(Component)]
struct FreezeOverlay {
    parent_entity: Entity,
}

pub struct FrozenPlugin;

impl Plugin for FrozenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            snowball_collision_system,
            frozen_stacks_update_system,
            apply_freeze_movement_penalty,
            update_splash_particles,
            update_freeze_overlay,
        ).run_if(in_state(GameState::Playing)));

        info!("FrozenPlugin loaded");
    }
}

/// Detect snowball collisions with hero/villagers and apply freeze stacks
fn snowball_collision_system(
    mut commands: Commands,
    spatial_hash: Res<SpatialHash>,
    snowball_query: Query<(Entity, &Transform), With<Snowball>>,
    block_query: Query<&Transform, With<Block>>,
    mut target_query: Query<(Entity, &Transform, Option<&mut FrozenStacks>, &mut LinearVelocity), Or<(With<Hero>, With<Villager>)>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    const SNOWBALL_HIT_RADIUS: f32 = 40.0;  // Collision radius for entities
    const SNOWBALL_RADIUS: f32 = 12.0;      // Snowball size (24x24, use half)
    const BLOCK_RADIUS: f32 = 32.0;         // Block size (64x64, use half)
    const PUSH_FORCE: f32 = 50.0;           // Small push on hit

    for (snowball_entity, snowball_transform) in snowball_query.iter() {
        let snowball_pos = snowball_transform.translation;
        let mut snowball_hit_something = false;

        // Check collision with blocks using spatial hash (snowballs STOP at walls like arrows)
        let nearby_blocks = spatial_hash.query_nearby(snowball_pos.x, snowball_pos.y);
        for block_entity in nearby_blocks {
            if let Ok(block_transform) = block_query.get(block_entity) {
                let block_pos = block_transform.translation;
                // Use 2D distance (ignore Z) for collision detection
                let dx = snowball_pos.x - block_pos.x;
                let dy = snowball_pos.y - block_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < (SNOWBALL_RADIUS + BLOCK_RADIUS) {
                    // Snowball hit wall - spawn splash particles and despawn
                    info!("SNOWBALL HIT BLOCK: entity={:?}, distance={:.1}, snowball_pos=({:.0},{:.0}), block_pos=({:.0},{:.0})",
                        snowball_entity, distance, snowball_pos.x, snowball_pos.y, block_pos.x, block_pos.y);

                    // Play snowball hit sound
                    commands.spawn((
                        AudioPlayer::new(asset_server.load("audio/sfx/projectiles/snowball/hurt1.ogg")),
                        PlaybackSettings::DESPAWN,
                    ));

                    // Spawn white splash particles at block hit position
                    for i in 0..8 {
                        let angle = (i as f32 / 8.0) * std::f32::consts::TAU;
                        let offset = Vec3::new(angle.cos() * 20.0, angle.sin() * 20.0, 5.0);

                        commands.spawn((
                            Mesh3d(meshes.add(Circle::new(6.0))),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgba(1.0, 1.0, 1.0, 0.9),
                                unlit: true,
                                alpha_mode: AlphaMode::Blend,
                                ..default()
                            })),
                            Transform::from_translation(snowball_pos + offset),
                            SplashParticle { lifetime: Timer::from_seconds(0.3, TimerMode::Once) },
                        ));
                    }

                    commands.entity(snowball_entity).despawn();
                    snowball_hit_something = true;
                    break; // This snowball is done, move to next snowball
                }
            }
        }

        if snowball_hit_something {
            continue; // Skip to next snowball
        }

        // Check collision with hero/villagers
        for (target_entity, target_transform, frozen_stacks, mut velocity) in target_query.iter_mut() {
            let target_pos = target_transform.translation;
            let distance = snowball_pos.distance(target_pos);

            if distance < SNOWBALL_HIT_RADIUS {
                // Hit! Despawn snowball
                commands.entity(snowball_entity).despawn();

                // Play snowball hit sound
                commands.spawn((
                    AudioPlayer::new(asset_server.load("audio/sfx/projectiles/snowball/hurt1.ogg")),
                    PlaybackSettings::DESPAWN,
                ));

                // Spawn white splash particles (8 particles radiating outward)
                for i in 0..8 {
                    let angle = (i as f32 / 8.0) * std::f32::consts::TAU;
                    let offset = Vec3::new(angle.cos() * 20.0, angle.sin() * 20.0, 5.0);

                    commands.spawn((
                        Mesh3d(meshes.add(Circle::new(6.0))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgba(1.0, 1.0, 1.0, 0.9),
                            unlit: true,
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        })),
                        Transform::from_translation(target_pos + offset),
                        SplashParticle { lifetime: Timer::from_seconds(0.3, TimerMode::Once) },
                    ));
                }

                // Add freeze stack
                if let Some(mut stacks) = frozen_stacks {
                    stacks.add_stack();
                    info!("Snowball hit! Freeze stacks: {}/6", stacks.stack_count());
                } else {
                    // First hit - add FrozenStacks component
                    let mut new_stacks = FrozenStacks::new();
                    new_stacks.add_stack();
                    commands.entity(target_entity).insert(new_stacks);
                    info!("First snowball hit! Freeze stack: 1/6");
                }

                // Small push back effect
                let push_direction = (target_pos - snowball_pos).normalize();
                velocity.x += push_direction.x * PUSH_FORCE;
                velocity.y += push_direction.y * PUSH_FORCE;

                break;  // Snowball can only hit one target
            }
        }
    }
}

/// Update frozen stack timers and remove expired stacks
fn frozen_stacks_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut frozen_query: Query<(Entity, &mut FrozenStacks)>,
) {
    for (entity, mut frozen) in frozen_query.iter_mut() {
        // Tick all stack timers
        frozen.stacks.retain_mut(|stack| {
            stack.timer.tick(time.delta());
            !stack.timer.is_finished()  // Keep stacks that haven't expired
        });

        // If all stacks expired, remove FrozenStacks component
        if frozen.stacks.is_empty() {
            commands.entity(entity).remove::<FrozenStacks>();
            info!("All freeze stacks expired");
        }
    }
}

/// Update splash particles and despawn when lifetime expires
fn update_splash_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particle_query: Query<(Entity, &mut SplashParticle, &mut Transform)>,
) {
    for (entity, mut particle, mut transform) in particle_query.iter_mut() {
        particle.lifetime.tick(time.delta());

        // Fade out and move particles outward
        let progress = particle.lifetime.fraction();
        transform.scale = Vec3::splat(1.0 - progress); // Shrink over time

        if particle.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Apply movement speed penalty based on freeze level
fn apply_freeze_movement_penalty(
    mut frozen_query: Query<(&FrozenStacks, &mut LinearVelocity, &mut Restitution, &mut Mass), Or<(With<Hero>, With<Villager>)>>,
) {
    for (frozen, mut velocity, mut restitution, mut mass) in frozen_query.iter_mut() {
        // If fully frozen (6 stacks), become ice cube - no bouncing, heavy, no horizontal movement
        if frozen.is_fully_frozen() {
            velocity.x = 0.0;  // Stop all horizontal movement
            velocity.y = velocity.y.min(0.0);  // Only allow downward movement
            *restitution = Restitution::new(0.0);  // No bouncing
            mass.0 = 5.0;  // 5x heavier - falls much faster
        } else {
            // Normal movement when not fully frozen
            *restitution = Restitution::new(1.5);  // Normal bounce (HERO_BOUNCE)
            mass.0 = 1.0;  // Normal mass
        }
    }
}

/// Create/update/remove freeze overlays based on freeze stack count
fn update_freeze_overlay(
    mut commands: Commands,
    frozen_query: Query<(Entity, &FrozenStacks, &Transform), Or<(With<Hero>, With<Villager>)>>,
    mut overlay_query: Query<(Entity, &FreezeOverlay, &mut Transform), Without<FrozenStacks>>,
    mut overlay_material_query: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Track which entities currently have freeze stacks
    let mut frozen_entities = std::collections::HashSet::new();

    for (entity, frozen, transform) in frozen_query.iter() {
        frozen_entities.insert(entity);
        let stack_count = frozen.stack_count();

        // Check if this entity already has an overlay
        let existing_overlay = overlay_query.iter_mut()
            .find(|(_, overlay, _)| overlay.parent_entity == entity)
            .map(|(e, _, t)| (e, t));

        if let Some((overlay_entity, mut overlay_transform)) = existing_overlay {
            // Update overlay position to follow parent
            overlay_transform.translation = transform.translation + Vec3::new(0.0, 0.0, 2.0);

            // Update overlay color based on stack count
            if let Ok(material_handle) = overlay_material_query.get_mut(overlay_entity) {
                let color = get_freeze_color(stack_count);
                if let Some(material) = materials.get_mut(&material_handle.0) {
                    material.base_color = color;
                }
            }
        } else {
            // Create new overlay
            let color = get_freeze_color(stack_count);
            let overlay_pos = transform.translation + Vec3::new(0.0, 0.0, 2.0);

            commands.spawn((
                Mesh3d(meshes.add(Circle::new(32.0))),  // Match character size
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: color,
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform::from_translation(overlay_pos),
                FreezeOverlay { parent_entity: entity },
            ));
        }
    }

    // Remove overlays for entities that no longer have freeze stacks
    for (overlay_entity, overlay, _) in overlay_query.iter() {
        if !frozen_entities.contains(&overlay.parent_entity) {
            commands.entity(overlay_entity).despawn();
        }
    }
}

/// Get freeze overlay color based on stack count (1-6)
fn get_freeze_color(stack_count: usize) -> Color {
    match stack_count {
        1 => Color::srgba(0.7, 0.9, 1.0, 0.2),  // Very light blue, barely visible
        2 => Color::srgba(0.6, 0.85, 1.0, 0.35), // Light blue
        3 => Color::srgba(0.5, 0.8, 1.0, 0.5),   // Medium blue
        4 => Color::srgba(0.4, 0.7, 0.95, 0.65), // Stronger blue
        5 => Color::srgba(0.3, 0.6, 0.9, 0.8),   // Deep blue
        _ => Color::srgba(0.2, 0.5, 0.85, 0.9),  // Full freeze - very opaque dark blue
    }
}

/// Update snowball lifetime and despawn expired ones
pub fn update_snowballs(
    mut commands: Commands,
    time: Res<Time>,
    mut snowball_query: Query<(Entity, &mut Snowball)>,
) {
    for (entity, mut snowball) in snowball_query.iter_mut() {
        snowball.lifetime.tick(time.delta());

        if snowball.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
