//! Owner-locked magic shell with animated surface currents and a Fresnel rim.
use crate::components::projectile::MagicBarrier;
use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};

#[derive(Component)]
pub struct MirrorVisual;
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct MirrorMaterial {
    #[uniform(0)]
    parameters: Vec4,
}
impl Material for MirrorMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/mirror_shell.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
#[derive(Resource)]
struct ShellMesh(Handle<Mesh>);
pub struct MirrorVfxPlugin;
impl Plugin for MirrorVfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<MirrorMaterial>::default())
            .add_systems(
                Startup,
                |mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>| {
                    commands
                        .insert_resource(ShellMesh(meshes.add(Sphere::new(1.0).mesh().uv(48, 32))));
                },
            )
            .add_systems(
                PostUpdate,
                (attach, sync_to_owner, animate)
                    .chain()
                    .after(crate::systems::biome_teleport::finish_biome_teleport)
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(crate::states::InGame)),
            );
    }
}
fn attach(
    mut commands: Commands,
    visuals: Query<Entity, Added<MirrorVisual>>,
    mesh: Res<ShellMesh>,
    mut materials: ResMut<Assets<MirrorMaterial>>,
) {
    for entity in &visuals {
        commands.entity(entity).insert((
            Mesh3d(mesh.0.clone()),
            MeshMaterial3d(materials.add(MirrorMaterial {
                parameters: Vec4::ZERO,
            })),
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
            bevy::picking::Pickable::IGNORE,
        ));
    }
}
fn animate(
    time: Res<Time>,
    visuals: Query<(&ChildOf, &MeshMaterial3d<MirrorMaterial>), With<MirrorVisual>>,
    barriers: Query<&MagicBarrier>,
    mut materials: ResMut<Assets<MirrorMaterial>>,
) {
    for (parent, handle) in &visuals {
        let Ok(barrier) = barriers.get(parent.parent()) else {
            continue;
        };
        if let Some(mut material) = materials.get_mut(&handle.0) {
            let fade = (barrier.lifetime.elapsed_secs() / 0.2).min(1.)
                * (barrier.lifetime.remaining_secs() / 0.6).min(1.);
            material.parameters = Vec4::new(time.elapsed_secs(), fade, 0., 0.);
        }
    }
}

/// Copy the final rendered owner transform after gameplay and teleport, never lerp it.
fn sync_to_owner(
    mut barriers: Query<(&MagicBarrier, &mut Transform)>,
    owners: Query<&Transform, Without<MagicBarrier>>,
) {
    for (barrier, mut transform) in &mut barriers {
        if let Ok(owner) = owners.get(barrier.owner_entity) {
            transform.translation = owner.translation + Vec3::Z * 5.0;
        }
    }
}
#[cfg(test)]
mod follow_tests {
    use super::*;
    #[test]
    fn barrier_tracks_large_owner_motion_in_the_same_frame() {
        let mut app = App::new();
        app.add_systems(Update, sync_to_owner);
        let owner = app
            .world_mut()
            .spawn(Transform::from_xyz(100.0, 200.0, 10.0))
            .id();
        let barrier = app
            .world_mut()
            .spawn((
                MagicBarrier {
                    owner_entity: owner,
                    radius: 150.0,
                    lifetime: Timer::from_seconds(10.0, TimerMode::Once),
                },
                Transform::IDENTITY,
            ))
            .id();
        for point in [
            Vec3::new(400.0, 200.0, 10.0),
            Vec3::new(-500.0, 40000.0, 10.0),
        ] {
            app.world_mut()
                .get_mut::<Transform>(owner)
                .unwrap()
                .translation = point;
            app.update();
            assert_eq!(
                app.world().get::<Transform>(barrier).unwrap().translation,
                point + Vec3::Z * 5.0
            );
        }
    }
}
