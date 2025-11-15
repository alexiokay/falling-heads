use bevy::prelude::*;
use crate::components::block::{Block, BlockType};
use crate::components::damage_overlay::{BlockDamageOverlay, get_damage_stage};
use crate::resources::asset_handles::{DamageStageTextures, DamageOverlayCache};
use crate::events::block_events::BlockDamaged;
use crate::states::GameState;

pub struct BlockDamageOverlayPlugin;

impl Plugin for BlockDamageOverlayPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<DamageOverlayCache>()
            .add_systems(Update, (
                spawn_damage_overlays,
                update_damage_overlays,
                cleanup_orphan_overlays,
            ).chain().run_if(in_state(GameState::Playing)));

        info!("BlockDamageOverlayPlugin loaded");
    }
}

/// Spawn damage overlay child entities when blocks are damaged (event-driven)
fn spawn_damage_overlays(
    mut commands: Commands,
    mut damage_events: MessageReader<BlockDamaged>,
    block_query: Query<(&Transform, &BlockType, &Block, Option<&Children>), With<Block>>,
    overlay_query: Query<&BlockDamageOverlay>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    damage_textures: Res<DamageStageTextures>,
    mut overlay_cache: ResMut<DamageOverlayCache>,
) {
    // PERFORMANCE FIX: Initialize cache if empty (prevents 7k+ mesh/material leak!)
    if overlay_cache.mesh.is_none() {
        overlay_cache.mesh = Some(meshes.add(Rectangle::new(1.0, 1.0)));
    }

    if overlay_cache.materials.is_empty() {
        for stage_texture in damage_textures.stages.iter() {
            overlay_cache.materials.push(materials.add(StandardMaterial {
                base_color_texture: Some(stage_texture.clone()),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }));
        }
    }

    // Process blocks that were just damaged
    for event in damage_events.read() {
        let Ok((block_transform, _block_type, block, children)) = block_query.get(event.entity) else {
            continue; // Block was destroyed
        };

        // Check if this block already has a damage overlay
        let has_overlay = if let Some(children) = children {
            children.iter().any(|child| overlay_query.get(child).is_ok())
        } else {
            false
        };

        if !has_overlay {
            // Determine initial visibility based on current damage
            let damage_stage = get_damage_stage(block.health, block.max_health);
            let initial_visibility = if damage_stage > 0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            // Determine overlay scale based on parent block's scale
            // If parent is scaled (stone/coal use 64.0), overlay stays 1.0
            // If parent is not scaled (melon/wood use 1.0), overlay needs 64.0
            let overlay_scale = if block_transform.scale.x > 1.5 {
                1.0 // Parent is already scaled, keep overlay at 1.0
            } else {
                64.0 // Parent not scaled, scale overlay to match block size
            };

            // Z offset needs to be adjusted based on scale
            // For scale 1.0: z_offset = 0.51 (normal)
            // For scale 64.0: Use a larger offset since the mesh is already 64x64x64 in world space
            let z_offset = if overlay_scale > 1.5 {
                // For scaled overlays (melon/wood): use larger offset in local space
                32.51  // This will be 32.51 in local coords, but overlay is at front of 64-unit block
            } else {
                // For unscaled overlays (stone/coal): use normal offset
                0.51
            };

            // Get initial material based on current damage stage
            let initial_material = overlay_cache.materials.get(damage_stage)
                .unwrap_or(&overlay_cache.materials[0])
                .clone();

            // Spawn damage overlay with correct initial state
            let overlay = commands.spawn((
                BlockDamageOverlay,
                Mesh3d(overlay_cache.mesh.clone().unwrap()),  // Shared 1x1 mesh
                MeshMaterial3d(initial_material),  // Start with correct damage stage material
                Transform::from_xyz(0.0, 0.0, z_offset).with_scale(Vec3::splat(overlay_scale)),
                initial_visibility, // Start visible if block is already damaged
            )).id();

            // Use queue closure to safely handle parent-child relationship
            let block_entity = event.entity;
            commands.queue(move |world: &mut World| {
                // Only add child if parent still exists at command flush time
                if let Ok(mut parent_entity) = world.get_entity_mut(block_entity) {
                    parent_entity.add_child(overlay);
                } else {
                    // Block was despawned - clean up orphan overlay
                    if let Ok(overlay_entity) = world.get_entity_mut(overlay) {
                        overlay_entity.despawn();
                    }
                }
            });
        }
    }
}

/// Update damage overlay visibility and material based on block health
fn update_damage_overlays(
    mut commands: Commands,
    mut overlay_query: Query<(Entity, &ChildOf, &mut Visibility, &MeshMaterial3d<StandardMaterial>), With<BlockDamageOverlay>>,
    block_query: Query<&Block>,
    overlay_cache: Res<DamageOverlayCache>,
) {
    for (overlay_entity, child_of, mut visibility, current_material) in overlay_query.iter_mut() {
        // Get the parent block's health
        if let Ok(block) = block_query.get(child_of.parent()) {
            let damage_stage = get_damage_stage(block.health, block.max_health);

            if damage_stage == 0 {
                // No damage - hide overlay
                *visibility = Visibility::Hidden;
            } else {
                // Show overlay with appropriate cached material
                *visibility = Visibility::Visible;

                // PERFORMANCE FIX: Switch to cached material only if it changed
                if let Some(material) = overlay_cache.materials.get(damage_stage) {
                    // Only update if the material actually changed
                    if current_material.0 != *material {
                        // Use try_insert to gracefully handle if entity was despawned mid-frame
                        commands.entity(overlay_entity).try_insert(MeshMaterial3d(material.clone()));
                    }
                }
            }
        }
    }
}

/// Cleanup orphan overlays whose parent blocks were despawned
fn cleanup_orphan_overlays(
    mut commands: Commands,
    overlay_query: Query<(Entity, &ChildOf), With<BlockDamageOverlay>>,
    block_query: Query<&Block>,
    time: Res<Time>,
) {
    let mut orphans_cleaned = 0;

    for (overlay_entity, child_of) in overlay_query.iter() {
        // If parent block doesn't exist anymore, despawn the orphan overlay
        if block_query.get(child_of.parent()).is_err() {
            // Use get_entity to avoid errors if already despawned
            if let Ok(mut entity_commands) = commands.get_entity(overlay_entity) {
                entity_commands.despawn();
                orphans_cleaned += 1;
            }
        }
    }

    // Log every 5 seconds if we cleaned orphans
    if orphans_cleaned > 0 && time.elapsed_secs() % 5.0 < time.delta_secs() {
        info!("Cleaned {} orphan damage overlays", orphans_cleaned);
    }
}
