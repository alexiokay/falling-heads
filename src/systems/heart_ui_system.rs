use bevy::prelude::*;
use crate::components::heart_ui::HeartUI;
use crate::components::enemy::{Enemy, Health};
use crate::components::villager::Villager;
use crate::resources::asset_handles::HeartTextures;

pub struct HeartUIPlugin;

impl Plugin for HeartUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_hearts, update_hearts));
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
                    Mesh3d(meshes.add(Rectangle::new(16.0, 16.0))),
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

    for (heart_entity, heart_ui, mut heart_transform, material_3d) in heart_query.iter_mut() {
        // Find the owner entity
        if let Ok((owner_entity, owner_transform, health)) = entity_query.get(heart_ui.owner) {
            // Position heart above the entity (centered)
            let num_hearts = (health.max / 2) as f32;
            let total_width = num_hearts * 18.0;
            let start_x = -(total_width / 2.0) + 9.0; // Center and offset by half heart width
            let x_offset = start_x + (heart_ui.heart_index as f32 * 18.0);

            heart_transform.translation.x = owner_transform.translation.x + x_offset;
            heart_transform.translation.y = owner_transform.translation.y + 40.0; // Above head
            heart_transform.translation.z = 15.0; // Front layer

            // Calculate which texture to use based on health
            let heart_value = heart_ui.heart_index * 2; // Each heart represents 2 health
            let texture = if health.current > (heart_value + 1) as u32 {
                heart_textures.full.clone() // Full heart
            } else if health.current == (heart_value + 1) as u32 {
                heart_textures.half.clone() // Half heart
            } else {
                heart_textures.container.clone() // Empty heart
            };

            // Update material texture
            if let Some(material) = materials.get_mut(&material_3d.0) {
                material.base_color_texture = Some(texture);
            }
        } else {
            // Owner entity despawned - remove heart
            commands.entity(heart_entity).despawn();
        }
    }
}
