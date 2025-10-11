use bevy::prelude::*;
use crate::components::damage_visual::{DamageVisual, DeathAnimation};
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::components::enemy::Enemy;
use crate::events::damage_events::EntityDamaged;
use crate::states::GameState;

pub struct DamageVisualPlugin;

impl Plugin for DamageVisualPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            apply_damage_visuals,
            update_damage_visuals,
        ).run_if(in_state(GameState::Playing)));

        info!("DamageVisualPlugin loaded");
    }
}

/// Marker component for damage border entity
#[derive(Component)]
struct DamageBorder;

/// Listen for damage events and add DamageVisual component + spawn red border
fn apply_damage_visuals(
    mut commands: Commands,
    mut damage_events: MessageReader<EntityDamaged>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    entity_query: Query<(Entity, Option<&MeshMaterial3d<StandardMaterial>>, Option<&DamageVisual>)>,
) {
    for event in damage_events.read() {
        // Check if entity still exists (might have been despawned mid-frame)
        let Ok((_, material_handle, existing_damage_visual)) = entity_query.get(event.entity) else {
            continue; // Entity doesn't exist or already despawned, skip
        };

        // Get original color before applying damage visual
        // IMPORTANT: If entity already has DamageVisual, use its stored original color!
        // Otherwise we'll capture the red-tinted color and make it progressively redder
        let original_color = if let Some(existing_dmg) = existing_damage_visual {
            existing_dmg.original_color // Reuse the already-stored original color
        } else if let Some(mat_handle) = material_handle {
            if let Some(material) = materials.get(&mat_handle.0) {
                material.base_color
            } else {
                Color::WHITE
            }
        } else {
            Color::WHITE
        };

        // Add/update damage visual effect with stored original color
        commands.entity(event.entity).try_insert(DamageVisual::with_original_color(1.0, original_color));

        // Spawn red border as child entity
        // Store entity ID for the closure
        let entity_id = event.entity;

        let border = commands.spawn((
            DamageBorder,
            Mesh3d(meshes.add(Rectangle::new(72.0, 72.0))), // Slightly larger than entity (64x64)
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.0, 0.0, 0.8), // Red with alpha
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.0, -1.0), // Behind the entity
        )).id();

        // Add border as child using a custom command that won't panic
        commands.queue(move |world: &mut World| {
            // Only add child if parent still exists at command flush time
            if let Ok(mut parent_entity) = world.get_entity_mut(entity_id) {
                parent_entity.add_child(border);
            } else {
                // Parent was despawned - clean up orphan border
                if let Ok(border_entity) = world.get_entity_mut(border) {
                    border_entity.despawn();
                }
            }
        });

        info!("Applied damage visual with border to entity {:?}", event.entity);
    }
}

/// Update damage visuals - change material color to red with fade out
fn update_damage_visuals(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DamageVisual, &MeshMaterial3d<StandardMaterial>, &Children, Option<&DeathAnimation>), Or<(With<Hero>, With<Villager>, With<Enemy>)>>,
    border_query: Query<(Entity, &MeshMaterial3d<StandardMaterial>), With<DamageBorder>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    for (entity, mut damage_visual, material_handle, children, death_animation) in query.iter_mut() {
        damage_visual.timer.tick(time.delta());

        if damage_visual.timer.is_finished() {
            // Effect finished - remove component and reset material to original color
            if let Some(material) = materials.get_mut(&material_handle.0) {
                material.base_color = damage_visual.original_color; // Reset to original color
            }

            // Despawn damage border child entities (only if not already despawned by parent death)
            for child in children.iter() {
                if border_query.get(child).is_ok() {
                    // Use get_entity to gracefully handle if already despawned
                    if let Ok(mut entity_commands) = commands.get_entity(child) {
                        entity_commands.despawn();
                    }
                }
            }

            commands.entity(entity).remove::<DamageVisual>();
        } else {
            // If entity is dead, keep damage visual at full intensity (no fade)
            let alpha = if death_animation.is_some() {
                1.0  // Keep at full red for dead entities
            } else {
                damage_visual.alpha()  // Fade out for living entities
            };

            if let Some(material) = materials.get_mut(&material_handle.0) {
                // Mix red color with white based on alpha (creates red overlay effect)
                let red_amount = alpha * 0.8; // 80% red at peak
                material.base_color = Color::srgba(1.0, 1.0 - red_amount, 1.0 - red_amount, 1.0);
            }

            // Update border alpha
            for child in children.iter() {
                if let Ok((_, border_material_handle)) = border_query.get(child) {
                    if let Some(border_material) = materials.get_mut(&border_material_handle.0) {
                        border_material.base_color = Color::srgba(1.0, 0.0, 0.0, alpha * 0.8);
                    }
                }
            }
        }
    }
}
