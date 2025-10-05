use bevy::prelude::*;
use crate::components::block::Block;
use crate::components::damage_overlay::{BlockDamageOverlay, get_damage_stage};
use crate::resources::asset_handles::{DamageStageTextures, DamageOverlayCache};
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
            ).run_if(in_state(GameState::Playing)));

        info!("BlockDamageOverlayPlugin loaded");
    }
}

/// Spawn damage overlay child entities for blocks that don't have one yet
fn spawn_damage_overlays(
    mut commands: Commands,
    block_query: Query<(Entity, Option<&Children>), With<Block>>,
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

    for (block_entity, children) in block_query.iter() {
        // Check if this block already has a damage overlay
        let has_overlay = if let Some(children) = children {
            children.iter().any(|child| overlay_query.get(child).is_ok())
        } else {
            false
        };

        if !has_overlay {
            // Spawn damage overlay using cached mesh and material (stage 0)
            let overlay = commands.spawn((
                BlockDamageOverlay,
                Mesh3d(overlay_cache.mesh.clone().unwrap()),  // Shared mesh
                MeshMaterial3d(overlay_cache.materials[0].clone()),  // Stage 0 material
                Transform::from_xyz(0.0, 0.0, 0.51), // Just in front of block face
                Visibility::Hidden, // Initially hidden (no damage)
            )).id();

            // Attach overlay as child of block (inherits transform/scale)
            // Use get_entity to gracefully handle if block was despawned
            if let Ok(mut block_cmd) = commands.get_entity(block_entity) {
                block_cmd.add_child(overlay);
            } else {
                // Block was despawned - clean up the orphan overlay we just created
                commands.entity(overlay).despawn();
            }
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
            commands.entity(overlay_entity).despawn();
            orphans_cleaned += 1;
        }
    }

    // Log every 5 seconds if we cleaned orphans
    if orphans_cleaned > 0 && time.elapsed_secs() % 5.0 < time.delta_secs() {
        info!("Cleaned {} orphan damage overlays", orphans_cleaned);
    }
}
