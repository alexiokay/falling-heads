//! One shared multi-quad row per healthy actor. An odd half-heart uses one
//! temporary child; health loss/healing never allocates per-heart materials.
use crate::{
    components::{
        enemy::{Enemy, Health},
        heart_ui::HeartUI,
        villager::Villager,
    },
    resources::asset_handles::HeartTextures,
};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
pub struct HeartUIPlugin;
#[derive(Component)]
struct HeartsSpawned;
#[derive(Component, Default)]
struct RowState {
    health: Option<u32>,
    half: Option<Entity>,
}
#[derive(Component)]
struct HalfHeart;
struct Art {
    mesh: Handle<Mesh>,
    full: Handle<StandardMaterial>,
    half: Handle<StandardMaterial>,
}
#[derive(Component)]
struct HeartArt {
    single: Handle<Mesh>,
    full: Handle<StandardMaterial>,
    half: Handle<StandardMaterial>,
}
impl Plugin for HeartUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_hearts).add_systems(
            PostUpdate,
            update_hearts.before(TransformSystems::Propagate),
        );
    }
}
fn spawn_hearts(
    mut commands: Commands,
    owners: Query<
        Entity,
        (
            Or<(With<Enemy>, With<Villager>)>,
            With<Health>,
            Without<HeartsSpawned>,
            Without<crate::systems::dragon_flyover::EndDragon>,
        ),
    >,
    textures: Option<Res<HeartTextures>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut art: Local<Option<Art>>,
) {
    let Some(textures) = textures else {
        return;
    };
    let art = art.get_or_insert_with(|| Art {
        mesh: meshes.add(Rectangle::new(24., 24.)),
        full: materials.add(StandardMaterial {
            base_color_texture: Some(textures.full.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        half: materials.add(StandardMaterial {
            base_color_texture: Some(textures.half.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
    });
    for e in &owners {
        commands.entity(e).try_insert(HeartsSpawned);
        commands.spawn((
            HeartUI { owner: e },
            RowState::default(),
            HeartArt {
                single: art.mesh.clone(),
                full: art.full.clone(),
                half: art.half.clone(),
            },
            Mesh3d(art.mesh.clone()),
            MeshMaterial3d(art.full.clone()),
            Transform::default(),
            Visibility::Hidden,
            bevy::light::NotShadowCaster,
        ));
    }
}
fn row_mesh(health: u32) -> Mesh {
    let full = health / 2;
    let total = health.div_ceil(2);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uv = Vec::new();
    let mut indices = Vec::new();
    for i in 0..full {
        let x = (i as f32 - (total.saturating_sub(1)) as f32 * 0.5) * 26.;
        let base = positions.len() as u32;
        positions.extend([
            [x - 12., -12., 0.],
            [x + 12., -12., 0.],
            [x + 12., 12., 0.],
            [x - 12., 12., 0.],
        ]);
        normals.extend([[0., 0., 1.]; 4]);
        uv.extend([[0., 1.], [1., 1.], [1., 0.], [0., 0.]]);
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(indices))
}
fn update_hearts(
    mut commands: Commands,
    owners: Query<(&Transform, &Health, Option<&Mesh3d>), Or<(With<Enemy>, With<Villager>)>>,
    mut meshes: ResMut<Assets<Mesh>>,
    cameras: Query<&Transform, (With<Camera3d>, Without<HeartUI>, Without<Health>)>,
    mut bounds_cache: Local<
        std::collections::HashMap<AssetId<Mesh>, crate::systems::head_geometry::FaceBounds>,
    >,
    mut rows: Local<std::collections::HashMap<u32, Handle<Mesh>>>,
    mut hearts: Query<
        (
            Entity,
            &HeartUI,
            &HeartArt,
            &mut RowState,
            &mut Transform,
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Visibility,
        ),
        Without<Health>,
    >,
) {
    let camera = cameras.iter().next();
    let (_right, up, rotation) = camera.map_or((Vec3::X, Vec3::Y, Quat::IDENTITY), |c| {
        (*c.right(), *c.up(), c.rotation)
    });
    for (e, heart, art, mut state, mut t, mut row_mesh_handle, mut mat, mut visibility) in
        &mut hearts
    {
        let Ok((owner, h, mesh)) = owners.get(heart.owner) else {
            commands.entity(e).try_despawn();
            continue;
        };
        let current = h.current.min(h.max);
        let desired = if current > 0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != desired {
            *visibility = desired;
        }
        if state.health != Some(current) {
            if let Some(child) = state.half.take() {
                commands.entity(child).try_despawn();
            }
            if current > 0 {
                if current == 1 {
                    row_mesh_handle.0 = art.single.clone();
                    mat.0 = art.half.clone();
                } else {
                    row_mesh_handle.0 = rows
                        .entry(current)
                        .or_insert_with(|| meshes.add(row_mesh(current)))
                        .clone();
                    mat.0 = art.full.clone();
                    if current % 2 == 1 {
                        let x = (current.div_ceil(2) - 1) as f32 * 13.;
                        let half = commands
                            .spawn((
                                HalfHeart,
                                Mesh3d(art.single.clone()),
                                MeshMaterial3d(art.half.clone()),
                                Transform::from_xyz(x, 0., 0.),
                                bevy::light::NotShadowCaster,
                                ChildOf(e),
                            ))
                            .id();
                        state.half = Some(half);
                    }
                }
            }
            state.health = Some(current);
        }
        if current == 0 {
            continue;
        }
        let face = mesh
            .map(|m| {
                *bounds_cache
                    .entry(m.0.id())
                    .or_insert_with(|| crate::systems::head_geometry::bounds(&meshes, Some(m)))
            })
            .unwrap_or_default();
        let scale = crate::systems::head_geometry::effect_scale(face, owner)
            .min_element()
            .clamp(0.6, 1.5);
        let center = owner.transform_point(face.center + Vec3::Z * (face.size.z * 0.5));
        let top = center + up * (face.size.y * owner.scale.y.abs() * 0.5);
        let offset = up * 28. * scale;
        let toward = camera.map_or(Vec3::Z, |c| {
            (c.translation - top - offset).normalize_or_zero()
        });
        t.translation = top + offset + toward * 0.5;
        t.rotation = rotation;
        t.scale = Vec3::splat(scale);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hundred_villagers_share_assets_and_health_changes_only_switch_handles() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .insert_resource(HeartTextures::default())
            .add_systems(Update, (spawn_hearts, update_hearts).chain());
        let mut owners = Vec::new();
        for _ in 0..100 {
            owners.push(
                app.world_mut()
                    .spawn((
                        Villager {
                            ai_timer: Timer::default(),
                            move_direction: 1.,
                            direction_change_timer: Timer::default(),
                        },
                        Health::new(6),
                        Transform::default(),
                    ))
                    .id(),
            );
        }
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&HeartUI>()
                .iter(app.world())
                .count(),
            100
        );
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
        assert_eq!(app.world().resource::<Assets<StandardMaterial>>().len(), 2);
        app.world_mut()
            .get_mut::<Health>(owners[0])
            .unwrap()
            .current = 1;
        app.update();
        let visible = app
            .world_mut()
            .query::<(&HeartUI, &Visibility)>()
            .iter(app.world())
            .filter(|(h, v)| h.owner == owners[0] && **v != Visibility::Hidden)
            .count();
        assert_eq!(visible, 1);
        app.world_mut()
            .get_mut::<Health>(owners[0])
            .unwrap()
            .current = 5;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&HalfHeart>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut()
            .get_mut::<Health>(owners[0])
            .unwrap()
            .current = 6;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&HalfHeart>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().despawn(owners[0]);
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&HeartUI>()
                .iter(app.world())
                .count(),
            99
        );
        assert_eq!(
            app.world_mut()
                .query::<&HeartUI>()
                .iter(app.world())
                .count(),
            99
        );
        assert_eq!(app.world().resource::<Assets<StandardMaterial>>().len(), 2);
    }
    #[test]
    fn hearts_stay_screen_centered_with_an_oblique_camera_and_small_head() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .insert_resource(HeartTextures::default())
            .add_systems(Update, (spawn_hearts, update_hearts).chain());
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Rectangle::new(32., 32.));
        let center = Vec3::new(500., 100., 30.);
        app.world_mut().spawn((
            Villager {
                ai_timer: Timer::default(),
                move_direction: 1.,
                direction_change_timer: Timer::default(),
            },
            Health::new(2),
            Mesh3d(mesh),
            Transform::from_translation(center).with_scale(Vec3::splat(0.5)),
        ));
        let camera = Transform::from_xyz(800., 350., 1500.).looking_at(Vec3::ZERO, Vec3::Y);
        app.world_mut().spawn((Camera3d::default(), camera));
        app.update();
        let heart = app
            .world_mut()
            .query_filtered::<&Transform, With<HeartUI>>()
            .single(app.world())
            .unwrap();
        let inverse = camera.to_matrix().inverse();
        let a = inverse.transform_point3(center);
        let b = inverse.transform_point3(heart.translation);
        assert!((a.x / a.z - b.x / b.z).abs() < 0.00001);
        assert!(heart.scale.x < 1.);
    }
}

#[cfg(test)]
mod row_tests {
    use super::*;
    #[test]
    fn rows_keep_spacing_and_leave_the_half_heart_slot() {
        for health in [2, 3, 5, 6, 20] {
            let mesh = row_mesh(health);
            assert_eq!(mesh.count_vertices(), (health / 2 * 4) as usize);
            let bevy::mesh::VertexAttributeValues::Float32x3(vertices) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!()
            };
            let first = (vertices[0][0] + vertices[1][0]) * 0.5;
            assert_eq!(first, -((health.div_ceil(2) - 1) as f32) * 13.);
            for quad in vertices.chunks_exact(4) {
                assert_eq!(quad[1][0] - quad[0][0], 24.);
            }
        }
    }
    #[test]
    fn mass_removal_cleans_rows_and_half_children() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .insert_resource(HeartTextures::default())
            .add_systems(Update, (spawn_hearts, update_hearts).chain());
        let owners: Vec<_> = (0..370)
            .map(|_| {
                app.world_mut()
                    .spawn((
                        Villager {
                            ai_timer: Timer::default(),
                            move_direction: 1.,
                            direction_change_timer: Timer::default(),
                        },
                        Health { current: 5, max: 6 },
                        Transform::default(),
                    ))
                    .id()
            })
            .collect();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&HeartUI>()
                .iter(app.world())
                .count(),
            370
        );
        assert_eq!(
            app.world_mut()
                .query::<&HalfHeart>()
                .iter(app.world())
                .count(),
            370
        );
        for e in owners {
            app.world_mut().despawn(e);
        }
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&HeartUI>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(
            app.world_mut()
                .query::<&HalfHeart>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(app.world().resource::<Assets<StandardMaterial>>().len(), 2);
    }
}
