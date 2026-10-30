use super::entity_spawn::{RestoreFace, SharedVillagerFace};
use crate::components::damage_visual::{DamageVisual, DeathAnimation};
use crate::components::enemy::Enemy;
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::events::damage_events::EntityDamaged;
use bevy::prelude::*;

pub struct DamageVisualPlugin;

impl Plugin for DamageVisualPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (apply_damage_visuals, update_damage_visuals).run_if(in_state(crate::states::InGame)),
        );

        info!("DamageVisualPlugin loaded");
    }
}

/// Marker component for damage border entity
#[derive(Component)]
pub(crate) struct DamageBorder;

/// Listen for damage events and add DamageVisual component + spawn red border
fn apply_damage_visuals(
    mut commands: Commands,
    mut damage_events: MessageReader<EntityDamaged>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    entity_query: Query<(
        Entity,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&DamageVisual>,
        Has<SharedVillagerFace>,
        Option<&Mesh3d>,
    )>,
) {
    let mut bordered = std::collections::HashSet::new();
    for event in damage_events.read() {
        // Check if entity still exists (might have been despawned mid-frame)
        let Ok((_, material_handle, existing_damage_visual, shared, face_mesh)) =
            entity_query.get(event.entity)
        else {
            continue; // Entity doesn't exist or already despawned, skip
        };
        // Articulated bosses have child meshes and their own health display;
        // a head-sized border on their meshless root would never be updated.
        if material_handle.is_none() {
            continue;
        }

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
        commands
            .entity(event.entity)
            .try_insert(DamageVisual::with_original_color(1.0, original_color));

        if existing_damage_visual.is_some() || !bordered.insert(event.entity) {
            continue;
        }

        // Copy only the hit face. Healthy villagers keep batching together, and
        // a single red flash must never tint the rest of a shared-material crowd.
        if shared {
            let original = material_handle.unwrap().0.clone();
            if let Some(source) = materials.get(&original).cloned() {
                let private = materials.add(source);
                commands
                    .entity(event.entity)
                    .insert((MeshMaterial3d(private), RestoreFace(original)))
                    .remove::<SharedVillagerFace>();
            }
        }

        // Spawn red border as child entity
        // Store entity ID for the closure
        let entity_id = event.entity;

        let face = super::head_geometry::bounds(&meshes, face_mesh);
        let border = commands
            .spawn((
                DamageBorder,
                bevy::light::NotShadowCaster,
                Mesh3d(meshes.add(Rectangle::new(face.size.x * 1.125, face.size.y * 1.125))), // Proportional to the actual mesh; child inherits root scale.
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 0.0, 0.0, 0.8), // Red with alpha
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform::from_translation(face.center + Vec3::Z * (face.size.z * 0.5 - 0.5)), // Behind its front face
            ))
            .id();

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

        info!(
            "Applied damage visual with border to entity {:?}",
            event.entity
        );
    }
}

/// Update damage visuals - change material color to red with fade out
fn update_damage_visuals(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut DamageVisual,
            &MeshMaterial3d<StandardMaterial>,
            &Children,
            Option<&DeathAnimation>,
            Option<&RestoreFace>,
        ),
        Or<(With<Hero>, With<Villager>, With<Enemy>)>,
    >,
    border_query: Query<(Entity, &MeshMaterial3d<StandardMaterial>), With<DamageBorder>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    for (entity, mut damage_visual, material_handle, children, death_animation, restore) in
        query.iter_mut()
    {
        damage_visual.timer.tick(time.delta());

        if damage_visual.timer.is_finished() {
            // Effect finished - remove component and reset material to original color
            if let Some(mut material) = materials.get_mut(&material_handle.0) {
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
            if let Some(original) = restore {
                commands
                    .entity(entity)
                    .insert((MeshMaterial3d(original.0.clone()), SharedVillagerFace))
                    .remove::<RestoreFace>();
            }
        } else {
            // If entity is dead, keep damage visual at full intensity (no fade)
            let alpha = if death_animation.is_some() {
                1.0 // Keep at full red for dead entities
            } else {
                damage_visual.alpha() // Fade out for living entities
            };

            if let Some(mut material) = materials.get_mut(&material_handle.0) {
                // Mix red color with white based on alpha (creates red overlay effect)
                let red_amount = alpha * 0.8; // 80% red at peak
                material.base_color = Color::srgba(1.0, 1.0 - red_amount, 1.0 - red_amount, 1.0);
            }

            // Update border alpha
            for child in children.iter() {
                if let Ok((_, border_material_handle)) = border_query.get(child) {
                    if let Some(mut border_material) = materials.get_mut(&border_material_handle.0)
                    {
                        border_material.base_color = Color::srgba(1.0, 0.0, 0.0, alpha * 0.8);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn damage_border_uses_the_small_mesh_and_inherits_parent_scale() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_message::<EntityDamaged>()
            .add_systems(Update, apply_damage_visuals);
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Rectangle::new(32., 20.));
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let e = app
            .world_mut()
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_scale(Vec3::splat(0.5)),
            ))
            .id();
        app.world_mut().write_message(EntityDamaged {
            entity: e,
            damage: 1,
        });
        app.update();
        let (border, parent) = app
            .world_mut()
            .query_filtered::<(&Mesh3d, &ChildOf), With<DamageBorder>>()
            .single(app.world())
            .unwrap();
        let face = crate::systems::head_geometry::bounds(
            app.world().resource::<Assets<Mesh>>(),
            Some(border),
        );
        assert_eq!(face.size.xy(), Vec2::new(36., 22.5));
        assert_eq!(parent.parent(), e);
    }
    #[test]
    fn shared_crowd_face_is_private_only_during_its_own_hit_flash() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .insert_resource(Time::<()>::default())
            .add_message::<EntityDamaged>()
            .add_systems(
                Update,
                (apply_damage_visuals, update_damage_visuals).chain(),
            );
        let face = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let hit = app
            .world_mut()
            .spawn((
                Enemy::new(crate::components::enemy::EnemyType::Skeleton),
                SharedVillagerFace,
                MeshMaterial3d(face.clone()),
            ))
            .id();
        let other = app.world_mut().spawn(MeshMaterial3d(face.clone())).id();
        app.world_mut().write_message(EntityDamaged {
            entity: hit,
            damage: 1,
        });
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.1));
        app.update();
        assert_ne!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(hit)
                .unwrap()
                .0,
            face
        );
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(other)
                .unwrap()
                .0,
            face
        );
        assert_eq!(
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&face)
                .unwrap()
                .base_color,
            Color::WHITE
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(2));
        app.update();
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(hit)
                .unwrap()
                .0,
            face
        );
        assert!(app.world().get::<SharedVillagerFace>(hit).is_some());
    }
    #[test]
    fn repeated_hits_share_one_border_instead_of_coplanar_layers() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_message::<EntityDamaged>()
            .add_systems(Update, apply_damage_visuals);
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let actor = app.world_mut().spawn(MeshMaterial3d(material)).id();
        for _ in 0..3 {
            app.world_mut().write_message(EntityDamaged {
                entity: actor,
                damage: 1,
            });
        }
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&DamageBorder>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut().write_message(EntityDamaged {
            entity: actor,
            damage: 1,
        });
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&DamageBorder>()
                .iter(app.world())
                .count(),
            1
        );
    }
}
