use bevy::prelude::*;
use crate::components::buff::{Buff, BuffType};
use crate::components::bundles::SpriteBundle3d;
use crate::resources::asset_handles::EffectTextures;

pub struct BurningVisualPlugin;

impl Plugin for BurningVisualPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            add_fire_overlay_to_burning_entities,
            update_fire_overlay_animation,
            remove_fire_overlay_when_buff_ends,
        ));

        info!("BurningVisualPlugin loaded");
    }
}

/// Marker component for fire overlay visual with sprite animation
#[derive(Component)]
struct FireOverlay {
    parent_entity: Entity,
    animation_timer: Timer,
    current_frame: usize, // 0-7 for 8 frames
}

/// Add fire overlay sprite to entities with Burning buff
fn add_fire_overlay_to_burning_entities(
    mut commands: Commands,
    burning_query: Query<(Entity, &Transform), (With<Buff>, Without<FireOverlay>)>,
    buff_query: Query<&Buff>,
    existing_overlays: Query<&FireOverlay>,
    effect_textures: Res<EffectTextures>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, transform) in burning_query.iter() {
        // Check if entity has Burning buff
        let has_burning = buff_query.iter_many([entity])
            .any(|buff| buff.buff_type == BuffType::Burning);

        if !has_burning {
            continue;
        }

        // Check if fire overlay already exists for this entity
        let already_has_overlay = existing_overlays.iter()
            .any(|overlay| overlay.parent_entity == entity);

        if already_has_overlay {
            continue;
        }

        // Create fire overlay sprite with UV for first frame
        // Sprite sheet has 8 frames vertically stacked, each frame is 1/8 of the texture height
        // Size matches entity size (64x64)
        let fire_mesh = meshes.add(
            Mesh::from(Rectangle::new(64.0, 64.0))
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_UV_0,
                    vec![
                        [0.0, 0.0],      // Bottom-left
                        [1.0, 0.0],      // Bottom-right
                        [1.0, 0.125],    // Top-right (1/8 of texture)
                        [0.0, 0.125],    // Top-left (1/8 of texture)
                    ]
                )
        );

        let fire_material = materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.8),
            base_color_texture: Some(effect_textures.fire.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        // Position on top of entity (in front on Z axis)
        let fire_pos = Vec3::new(
            transform.translation.x,
            transform.translation.y,
            transform.translation.z + 2.0, // In FRONT (higher Z = in front)
        );

        commands.spawn((
            FireOverlay {
                parent_entity: entity,
                animation_timer: Timer::from_seconds(0.08, TimerMode::Repeating), // ~12 FPS animation
                current_frame: 0,
            },
            SpriteBundle3d::new(fire_mesh, fire_material, fire_pos),
        ));
    }
}

/// Animate fire overlay (8-frame sprite sheet animation)
fn update_fire_overlay_animation(
    time: Res<Time>,
    entity_query: Query<&Transform, Without<FireOverlay>>,
    mut fire_query: Query<(&mut FireOverlay, &mut Transform, &Mesh3d), Without<Buff>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (mut fire, mut fire_transform, mesh_handle) in fire_query.iter_mut() {
        fire.animation_timer.tick(time.delta());

        // Update position to follow parent entity
        if let Ok(parent_transform) = entity_query.get(fire.parent_entity) {
            fire_transform.translation.x = parent_transform.translation.x;
            fire_transform.translation.y = parent_transform.translation.y;
            fire_transform.translation.z = parent_transform.translation.z + 2.0; // In FRONT (higher Z = in front)

            // Advance animation frame
            if fire.animation_timer.just_finished() {
                fire.current_frame = (fire.current_frame + 1) % 8; // Loop through 8 frames

                // Update UV coordinates for current frame
                // Each frame is 1/8 of the texture height (0.125)
                let frame_height = 0.125;
                let v_start = fire.current_frame as f32 * frame_height;
                let v_end = v_start + frame_height;

                if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
                    mesh.insert_attribute(
                        Mesh::ATTRIBUTE_UV_0,
                        vec![
                            [0.0, v_start],      // Bottom-left
                            [1.0, v_start],      // Bottom-right
                            [1.0, v_end],        // Top-right
                            [0.0, v_end],        // Top-left
                        ]
                    );
                }
            }
        }
    }
}

/// Remove fire overlay when Burning buff ends or entity dies
fn remove_fire_overlay_when_buff_ends(
    mut commands: Commands,
    fire_query: Query<(Entity, &FireOverlay)>,
    buff_query: Query<&Buff>,
    entity_query: Query<Entity>,
) {
    for (fire_entity, fire_overlay) in fire_query.iter() {
        // Check if parent entity still exists
        if entity_query.get(fire_overlay.parent_entity).is_err() {
            commands.entity(fire_entity).despawn();
            continue;
        }

        // Check if parent still has Burning buff
        let still_burning = buff_query.iter_many([fire_overlay.parent_entity])
            .any(|buff| buff.buff_type == BuffType::Burning);

        if !still_burning {
            commands.entity(fire_entity).despawn();
        }
    }
}
