use bevy::prelude::*;
use crate::components::heart_ui::HeartUI;
use crate::components::enemy::{Enemy, Health};
use crate::components::villager::Villager;
use crate::resources::asset_handles::HeartTextures;

pub struct HeartUIPlugin;

impl Plugin for HeartUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_hearts)
           .add_systems(PostUpdate, update_hearts); // Run after physics updates
    }
}

/// Spawn hearts above entities that have Health (enemies and villagers)
fn spawn_hearts(
    mut commands: Commands,
    entity_query: Query<(Entity, &Health), Or<(With<Enemy>, With<Villager>)>>,
    heart_query: Query<&HeartUI>,
    heart_textures: Option<Res<HeartTextures>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(heart_textures) = heart_textures else {
        return;
    };

    // For each entity with Health, check if hearts already exist
    for (entity, health) in entity_query.iter() {
        // Check if hearts already spawned for this entity
        let has_hearts = heart_query.iter().any(|heart| heart.owner == entity);

        if !has_hearts {
            let num_hearts = (health.max / 2) as usize; // 2 health = 1 heart

            // Spawn hearts above the entity
            for i in 0..num_hearts {
                commands.spawn((
                    HeartUI {
                        owner: entity,
                        heart_index: i,
                    },
                    Mesh3d(meshes.add(Rectangle::new(24.0, 24.0))), // Bigger hearts (16 → 24)
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        base_color_texture: Some(heart_textures.container.clone()),
                        unlit: true,
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    })),
                    Transform::from_xyz(0.0, 0.0, 15.0), // Will be updated in update_hearts
                ));
            }
        }
    }
}

/// Update heart positions and textures based on entity health
fn update_hearts(
    mut commands: Commands,
    entity_query: Query<(Entity, &Transform, &Health), Or<(With<Enemy>, With<Villager>)>>,
    mut heart_query: Query<(Entity, &HeartUI, &mut Transform, &MeshMaterial3d<StandardMaterial>), Without<Health>>,
    heart_textures: Option<Res<HeartTextures>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(heart_textures) = heart_textures else {
        return;
    };

    // First pass: collect all hearts per entity and mark for despawn
    let mut hearts_by_entity: std::collections::HashMap<Entity, Vec<(Entity, usize)>> = std::collections::HashMap::new();
    let mut to_despawn = Vec::new();

    for (heart_entity, heart_ui, _transform, _material) in heart_query.iter() {
        if let Ok((_owner_entity, _owner_transform, health)) = entity_query.get(heart_ui.owner) {
            let heart_value = heart_ui.heart_index * 2;

            if health.current <= heart_value as u32 {
                to_despawn.push(heart_entity);
            } else {
                hearts_by_entity.entry(heart_ui.owner)
                    .or_insert_with(Vec::new)
                    .push((heart_entity, heart_ui.heart_index));
            }
        } else {
            to_despawn.push(heart_entity);
        }
    }

    // Despawn marked hearts
    for entity in to_despawn {
        commands.entity(entity).despawn();
    }

    // Sort hearts by index
    for hearts in hearts_by_entity.values_mut() {
        hearts.sort_by_key(|(_, idx)| *idx);
    }

    // Second pass: update positions and textures
    for (heart_entity, heart_ui, mut heart_transform, material_3d) in heart_query.iter_mut() {
        if let Ok((_owner_entity, owner_transform, health)) = entity_query.get(heart_ui.owner) {
            let heart_value = heart_ui.heart_index * 2;

            if health.current <= heart_value as u32 {
                continue; // Already marked for despawn
            }

            // Determine texture
            let texture = if health.current > (heart_value + 1) as u32 {
                heart_textures.full.clone()
            } else {
                heart_textures.half.clone()
            };

            // Get this entity's hearts and find position
            if let Some(entity_hearts) = hearts_by_entity.get(&heart_ui.owner) {
                let num_hearts = entity_hearts.len();
                let position_index = entity_hearts.iter()
                    .position(|(e, _)| *e == heart_entity)
                    .unwrap_or(0);

                // Center the hearts
                let heart_spacing = 26.0;
                let total_width = (num_hearts.saturating_sub(1)) as f32 * heart_spacing;
                let start_x = -(total_width / 2.0);
                let x_offset = start_x + (position_index as f32 * heart_spacing);

                // Update transform
                heart_transform.translation.x = owner_transform.translation.x + x_offset;
                heart_transform.translation.y = owner_transform.translation.y + 60.0;
                heart_transform.translation.z = 15.0;
            }

            // Update texture
            if let Some(material) = materials.get_mut(&material_3d.0) {
                material.base_color_texture = Some(texture);
            }
        }
    }
}
