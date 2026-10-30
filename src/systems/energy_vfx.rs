use crate::components::projectile::{BarrierImpact, Laser, LaserBurstBeam};
use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};
#[derive(Clone, Copy)]
pub enum EnergyKind {
    Burst,
    Laser,
    Impact,
}
#[derive(Component)]
pub struct EnergyVisual(pub EnergyKind);
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct EnergyMaterial {
    /// age, kind, simulation time, fade
    #[uniform(0)]
    parameters: Vec4,
}
impl Material for EnergyMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/energy.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
#[derive(Resource)]
struct EnergyAssets {
    quad: Handle<Mesh>,
}
pub struct EnergyVfxPlugin;
impl Plugin for EnergyVfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<EnergyMaterial>::default())
            .add_systems(Startup, load)
            .add_systems(
                PostUpdate,
                (attach, animate)
                    .chain()
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(crate::states::InGame)),
            );
    }
}
fn load(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(EnergyAssets {
        quad: meshes.add(Rectangle::new(1., 1.)),
    });
}
fn attach(
    mut commands: Commands,
    visuals: Query<(Entity, &EnergyVisual), Added<EnergyVisual>>,
    assets: Res<EnergyAssets>,
    mut materials: ResMut<Assets<EnergyMaterial>>,
) {
    for (entity, visual) in &visuals {
        let kind = match visual.0 {
            EnergyKind::Burst => 0.,
            EnergyKind::Laser => 1.,
            EnergyKind::Impact => 2.,
        };
        commands.entity(entity).insert((
            Mesh3d(assets.quad.clone()),
            MeshMaterial3d(materials.add(EnergyMaterial {
                parameters: Vec4::new(0., kind, 0., 1.),
            })),
            bevy::picking::Pickable::IGNORE,
        ));
    }
}
fn animate(
    time: Res<Time>,
    mut visuals: Query<(
        &EnergyVisual,
        &ChildOf,
        Option<&BarrierImpact>,
        &MeshMaterial3d<EnergyMaterial>,
        &mut Transform,
    )>,
    lasers: Query<(&Laser, Option<&LaserBurstBeam>)>,
    mut materials: ResMut<Assets<EnergyMaterial>>,
) {
    for (visual, parent, impact, handle, mut transform) in &mut visuals {
        let (age, fade) = match visual.0 {
            EnergyKind::Burst => {
                let Ok((laser, Some(beam))) = lasers.get(parent.parent()) else {
                    continue;
                };
                transform.scale = Vec3::new(beam.length, laser.width * 1.6, 1.);
                let age = laser.timer.fraction();
                (age, ((1. - age) / 0.3).clamp(0., 1.))
            }
            EnergyKind::Laser => {
                let Ok((laser, _)) = lasers.get(parent.parent()) else {
                    continue;
                };
                transform.scale =
                    Vec3::new(crate::utils::constants::LASER_HEIGHT, laser.width * 1.2, 1.);
                transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
                let age = laser.timer.fraction();
                (age, ((1. - age) / 0.3).clamp(0., 1.))
            }
            EnergyKind::Impact => {
                let Some(impact) = impact else {
                    continue;
                };
                let age = impact.lifetime.fraction();
                (age, 1. - age * age)
            }
        };
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.parameters.x = age;
            material.parameters.z = time.elapsed_secs();
            material.parameters.w = fade;
        }
    }
}
