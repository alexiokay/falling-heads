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
    all_entities: Query<Entity>,
) {
    for event in damage_events.read() {
        // Check if entity still exists (might have been despawned mid-frame)
        if all_entities.get(event.entity).is_err() {
            continue; // Entity already despawned, skip
        }

        // Add damage visual effect (1.0 second duration with fade out)
        commands.entity(event.entity).try_insert(DamageVisual::new(1.0));

        // Spawn red border as child entity
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

        // Use get_entity to gracefully handle if parent was despawned mid-frame
        if let Ok(mut parent_commands) = commands.get_entity(event.entity) {
            parent_commands.add_child(border);
            info!("Applied damage visual with border to entity {:?}", event.entity);
        } else {
            // Parent already despawned - clean up the orphan border we just created
            commands.entity(border).despawn();
        }
    }
}

/// Update damage visuals - change material color to red with fade out
fn update_damage_visuals(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DamageVisual, &MeshMaterial3d<StandardMaterial>, &Children, Option<&DeathAnimation>), Or<(With<Hero>, With<Villager>, With<Enemy>)>>,
    mut border_query: Query<(Entity, &MeshMaterial3d<StandardMaterial>), With<DamageBorder>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    for (entity, mut damage_visual, material_handle, children, death_animation) in query.iter_mut() {
        damage_visual.timer.tick(time.delta());

        if damage_visual.timer.is_finished() {
            // Effect finished - remove component and reset material
            if let Some(material) = materials.get_mut(&material_handle.0) {
                material.base_color = Color::WHITE; // Reset to white (shows original texture)
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
