use crate::components::block::{Block, BlockType};
use crate::components::damage_overlay::{BlockDamageOverlay, get_damage_stage};
use crate::events::block_events::BlockDamaged;
use crate::resources::asset_handles::{DamageOverlayCache, DamageStageTextures};
use bevy::prelude::*;
use bevy::{reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef};
use std::collections::HashSet;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CrackMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[uniform(2)]
    pub variant: Vec4,
}
impl Material for CrackMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/damage_cracks.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
#[derive(Component)]
struct CrackVariant(usize);
pub struct BlockDamageOverlayPlugin;

impl Plugin for BlockDamageOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CrackMaterial>::default())
            .init_resource::<DamageOverlayCache>()
            .add_systems(
                Update,
                (
                    spawn_damage_overlays,
                    update_damage_overlays,
                    cleanup_orphan_overlays,
                )
                    .chain()
                    .after(crate::plugins::gameplay_plugin::handle_block_damage)
                    .run_if(in_state(crate::states::InGame)),
            );

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
    mut materials: ResMut<Assets<CrackMaterial>>,
    damage_textures: Res<DamageStageTextures>,
    mut overlay_cache: ResMut<DamageOverlayCache>,
) {
    // PERFORMANCE FIX: Initialize cache if empty (prevents 7k+ mesh/material leak!)
    if overlay_cache.mesh.is_none() {
        overlay_cache.mesh = Some(meshes.add(Cuboid::new(1.0, 1.0, 1.0)));
    }

    if overlay_cache.materials.is_empty() {
        for stage_texture in damage_textures.stages.iter() {
            for variant in 0..3 {
                overlay_cache.materials.push(materials.add(CrackMaterial {
                    texture: stage_texture.clone(),
                    variant: Vec4::new(variant as f32, 0., 0., 0.),
                }));
            }
        }
    }

    let mut spawned = HashSet::new();
    // Process blocks that were just damaged
    for event in damage_events.read() {
        let Ok((block_transform, block_type, block, children)) = block_query.get(event.entity)
        else {
            continue; // Block was destroyed
        };

        // Check if this block already has a damage overlay
        let has_overlay = if let Some(children) = children {
            children
                .iter()
                .any(|child| overlay_query.get(child).is_ok())
        } else {
            false
        };

        if !has_overlay && spawned.insert(event.entity) {
            // Determine initial visibility based on current damage
            let damage_stage = get_damage_stage(block.health, block.max_health);
            let initial_visibility = if damage_stage > 0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            // A shared six-face shell follows both unit meshes and 64-unit atlas meshes.
            // A small world-space separation avoids z fighting without a visible floating rim.
            let size = if *block_type == BlockType::Cactus {
                Vec3::new(57.02, 65.02, 57.02)
            } else {
                Vec3::splat(65.02)
            };
            let overlay_scale = size / block_transform.scale.abs().max(Vec3::splat(0.001));

            // Get initial material based on current damage stage
            let cell = (block_transform.translation.xy() / 64.).round().as_ivec2();
            let variant = ((cell.x as u32).wrapping_mul(73856093)
                ^ (cell.y as u32).wrapping_mul(19349663)) as usize
                % 3;
            let initial_material = overlay_cache
                .materials
                .get(damage_stage * 3 + variant)
                .unwrap_or(&overlay_cache.materials[0])
                .clone();

            // Spawn damage overlay with correct initial state
            let overlay = commands
                .spawn((
                    BlockDamageOverlay,
                    CrackVariant(variant),
                    Mesh3d(overlay_cache.mesh.clone().unwrap()), // Shared 1x1 mesh
                    MeshMaterial3d(initial_material), // Start with correct damage stage material
                    Transform::from_scale(overlay_scale),
                    initial_visibility, // Start visible if block is already damaged
                ))
                .id();

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
    mut overlay_query: Query<
        (
            Entity,
            &ChildOf,
            &mut Visibility,
            &MeshMaterial3d<CrackMaterial>,
            &CrackVariant,
        ),
        With<BlockDamageOverlay>,
    >,
    block_query: Query<&Block>,
    overlay_cache: Res<DamageOverlayCache>,
) {
    for (overlay_entity, child_of, mut visibility, current_material, variant) in
        overlay_query.iter_mut()
    {
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
                if let Some(material) = overlay_cache.materials.get(damage_stage * 3 + variant.0) {
                    // Only update if the material actually changed
                    if current_material.0 != *material {
                        // Use try_insert to gracefully handle if entity was despawned mid-frame
                        commands
                            .entity(overlay_entity)
                            .try_insert(MeshMaterial3d(material.clone()));
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

#[cfg(test)]
mod regression_tests {
    use super::*;
    #[test]
    fn duplicate_damage_messages_create_one_overlay() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<BlockDamaged>()
            .add_message::<crate::events::block_events::HeadHitBlock>()
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<CrackMaterial>::default())
            .insert_resource(DamageStageTextures {
                stages: std::array::from_fn(|_| Handle::default()),
            })
            .init_resource::<DamageOverlayCache>()
            .add_systems(Update, spawn_damage_overlays);
        let block = app
            .world_mut()
            .spawn((
                Block {
                    health: 2,
                    max_health: 3,
                },
                BlockType::Stone,
                Transform::from_scale(Vec3::splat(64.)),
            ))
            .id();
        for _ in 0..2 {
            app.world_mut().write_message(BlockDamaged {
                entity: block,
                damage: 1,
                remaining_health: 2,
            });
        }
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<BlockDamageOverlay>>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(app.world().get::<Children>(block).unwrap().len(), 1);
        let child = app.world().get::<Children>(block).unwrap()[0];
        let transform = app.world().get::<Transform>(child).unwrap();
        assert!(transform.translation.length() < 0.001);
        assert!((transform.scale * 64.0 - Vec3::splat(65.02)).length() < 0.001);
        let handle = app.world().get::<Mesh3d>(child).unwrap();
        let mesh = app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&handle.0)
            .unwrap();
        let bevy::mesh::VertexAttributeValues::Float32x3(normals) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap()
        else {
            panic!("missing face normals")
        };
        for direction in [
            Vec3::X,
            Vec3::NEG_X,
            Vec3::Y,
            Vec3::NEG_Y,
            Vec3::Z,
            Vec3::NEG_Z,
        ] {
            assert!(
                normals
                    .iter()
                    .any(|normal| Vec3::from(*normal).dot(direction) > 0.99)
            );
        }
    }
}
