use crate::{
    components::hero::Hero,
    systems::biome_teleport::{
        BiomeTeleport, CHARGE_SECONDS, REVEAL_SECONDS, TeleportPhase, finish_biome_teleport,
    },
};
use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct PortalMaterial {
    #[uniform(0)]
    effect: Vec4,
}
impl Material for PortalMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/teleport.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
}
#[derive(Component)]
struct Portal;
#[derive(Component)]
struct Veil;
#[derive(Component)]
struct Status;
pub struct TeleportVfxPlugin;
impl Plugin for TeleportVfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<PortalMaterial>::default())
            .add_systems(Startup, setup)
            .add_systems(
                PostUpdate,
                (animate, world_transition)
                    .chain()
                    .after(finish_biome_teleport)
                    .after(crate::systems::biome_content::ContentArtReady)
                    .before(TransformSystems::Propagate),
            );
    }
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PortalMaterial>>,
) {
    commands.spawn((
        Portal,
        bevy::light::NotShadowCaster,
        Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial3d(materials.add(PortalMaterial { effect: Vec4::ZERO })),
        Transform::IDENTITY,
        Visibility::Hidden,
    ));
    commands
        .spawn((
            Veil,
            Pickable::IGNORE,
            GlobalZIndex(500),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_child((
            Status,
            Pickable::IGNORE,
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.95, 1.0)),
            Node {
                margin: UiRect::top(Val::Px(180.0)),
                ..default()
            },
        ));
}
fn animate(
    teleport: Res<BiomeTeleport>,
    time: Res<Time>,
    hero: Query<&Transform, (With<Hero>, Without<Portal>)>,
    mut portals: Query<
        (
            &mut Transform,
            &mut Visibility,
            &MeshMaterial3d<PortalMaterial>,
        ),
        With<Portal>,
    >,
    mut materials: ResMut<Assets<PortalMaterial>>,
    mut veils: Query<(&mut Node, &mut BackgroundColor), With<Veil>>,
    mut labels: Query<&mut Text, With<Status>>,
) {
    let active = teleport.phase != TeleportPhase::Idle;
    let reveal = teleport.phase == TeleportPhase::Revealing;
    let amount = if reveal {
        (1.0 - teleport.elapsed / REVEAL_SECONDS).clamp(0.0, 1.0)
    } else {
        (teleport.elapsed / CHARGE_SECONDS).min(1.0)
    };
    for (mut node, mut color) in &mut veils {
        node.display = if active { Display::Flex } else { Display::None };
        color.0 = Color::srgba(0.025, 0.02, 0.06, amount * 0.35);
    }
    for mut text in &mut labels {
        **text = match teleport.phase {
            TeleportPhase::Charging => format!(
                "TELEPORTING  {:.0}%",
                teleport
                    .progress
                    .min((teleport.elapsed / CHARGE_SECONDS).min(1.0))
                    * 100.0
            ),
            TeleportPhase::Failed => "No safe landing found - stayed here".into(),
            _ => String::new(),
        };
    }
    for (mut transform, mut visibility, handle) in &mut portals {
        *visibility = if active && teleport.phase != TeleportPhase::Failed {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Ok(owner) = hero.single() {
            transform.translation = owner.translation + Vec3::Z * 80.0;
            transform.scale = Vec3::splat(if reveal {
                260.0 + teleport.elapsed * 700.0
            } else {
                360.0 - amount * 100.0
            });
        }
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.effect = Vec4::new(time.elapsed_secs(), amount, 0.0, 0.0);
        }
    }
}

use bevy::post_process::effect_stack::{ChromaticAberration, LensDistortion};
#[derive(Component)]
pub(crate) struct CompressedHead {
    mesh: Handle<Mesh>,
    echo: Entity,
}
#[derive(Component)]
pub(crate) struct HeadEcho;
fn transition_strength(phase: TeleportPhase, elapsed: f32) -> f32 {
    match phase {
        TeleportPhase::Charging => (elapsed / CHARGE_SECONDS).clamp(0., 1.),
        TeleportPhase::Revealing => (1. - elapsed / REVEAL_SECONDS).clamp(0., 1.),
        _ => 0.,
    }
}
pub(crate) fn world_transition(
    settings: Option<Res<crate::resources::graphics_settings::GraphicsSettings>>,
    mut commands: Commands,
    teleport: Res<BiomeTeleport>,
    time: Res<Time>,
    cameras: Query<Entity, With<Camera3d>>,
    heads: Query<
        (Entity, &Mesh3d, &MeshMaterial3d<StandardMaterial>),
        (
            With<crate::components::enemy::Health>,
            Without<CompressedHead>,
        ),
    >,
    compressed: Query<(Entity, &CompressedHead)>,
    mut echoes: Query<&mut Transform, With<HeadEcho>>,
    mut damage_borders: Query<
        &mut Visibility,
        With<crate::systems::damage_visual_system::DamageBorder>,
    >,
) {
    let amount = transition_strength(teleport.phase, teleport.elapsed);
    let active = amount > 0.;
    for mut visibility in &mut damage_borders {
        *visibility = if active {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for camera in &cameras {
        if active && settings.as_ref().is_none_or(|s| s.distortion) {
            commands.entity(camera).insert((
                ChromaticAberration {
                    intensity: amount * 0.055,
                    max_samples: 16,
                    ..default()
                },
                LensDistortion {
                    intensity: -0.24 * amount,
                    scale: 1. + amount * 0.08,
                    multiplier: Vec2::new(1. + 0.15 * (time.elapsed_secs() * 15.).sin(), 1.),
                    ..default()
                },
            ));
        } else {
            commands
                .entity(camera)
                .remove::<(ChromaticAberration, LensDistortion)>();
        }
    }
    if active {
        for (entity, mesh, material) in &heads {
            let mesh = mesh.0.clone();
            let material = material.0.clone();
            // Streaming/death may remove a head before deferred commands flush.
            // Snapshot and restore the visual as one guarded operation.
            commands.queue(move |world: &mut World| {
                if world.get_entity(entity).is_err() {
                    return;
                }
                let echo = world
                    .spawn((
                        HeadEcho,
                        bevy::light::NotShadowCaster,
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::IDENTITY,
                        ChildOf(entity),
                    ))
                    .id();
                world
                    .entity_mut(entity)
                    .remove::<Mesh3d>()
                    .insert(CompressedHead { mesh, echo });
            });
        }
        for mut t in &mut echoes {
            t.scale = Vec3::new(1. - amount * 0.90, 1. - amount * 0.98, 1.);
            t.rotation = Quat::from_rotation_z(amount * 0.25);
        }
    } else {
        for (entity, original) in &compressed {
            let mesh = original.mesh.clone();
            let echo = original.echo;
            commands.queue(move |world: &mut World| {
                if let Ok(mut owner) = world.get_entity_mut(entity) {
                    if !owner.contains::<Mesh3d>() {
                        owner.insert(Mesh3d(mesh));
                    }
                    // Bevy drops the render-material binding when Mesh3d is removed.
                    // Re-extract the CURRENT material; never restore the stale fallback.
                    if let Some(current) = owner.get::<MeshMaterial3d<StandardMaterial>>().cloned()
                    {
                        owner.insert(current);
                    }
                    owner.remove::<CompressedHead>();
                }
                if let Ok(e) = world.get_entity_mut(echo) {
                    e.despawn();
                }
            });
        }
    }
}
#[cfg(test)]
mod transition_tests {
    use super::*;
    #[test]
    fn collapse_restores_mesh_without_scaling_physics_owner() {
        let mut app = App::new();
        let mut teleport = BiomeTeleport::default();
        teleport.phase = TeleportPhase::Charging;
        teleport.elapsed = CHARGE_SECONDS * 0.8;
        app.insert_resource(teleport)
            .insert_resource(Time::<()>::default())
            .add_systems(Update, world_transition);
        let transform = Transform::from_xyz(60., 120., 0.).with_scale(Vec3::splat(0.8));
        let e = app
            .world_mut()
            .spawn((
                crate::components::enemy::Health::new(5),
                Mesh3d::default(),
                MeshMaterial3d::<StandardMaterial>::default(),
                transform,
            ))
            .id();
        app.update();
        app.update();
        let echo = app.world().get::<CompressedHead>(e).unwrap().echo;
        assert!(app.world().get::<Mesh3d>(e).is_none());
        assert_eq!(
            app.world().get::<Transform>(e).unwrap().scale,
            transform.scale
        );
        assert!(app.world().get::<Transform>(echo).unwrap().scale.y < 0.3);
        // Streaming attaches the final species art while the old fallback is compressed.
        app.init_resource::<Assets<StandardMaterial>>();
        let final_face = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        app.world_mut()
            .entity_mut(e)
            .insert(MeshMaterial3d(final_face.clone()));
        app.world_mut().resource_mut::<BiomeTeleport>().phase = TeleportPhase::Idle;
        app.update();
        assert!(app.world().get::<Mesh3d>(e).is_some());
        assert!(app.world().get::<CompressedHead>(e).is_none());
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(e)
                .unwrap()
                .0,
            final_face
        );
        assert!(app.world().get_entity(echo).is_err());
        assert_eq!(
            app.world().get::<Transform>(e).unwrap().translation,
            transform.translation
        );
    }
    #[test]
    fn charge_and_reveal_restore_visual_strength() {
        assert_eq!(transition_strength(TeleportPhase::Charging, 0.), 0.);
        assert_eq!(
            transition_strength(TeleportPhase::Charging, CHARGE_SECONDS),
            1.
        );
        assert_eq!(
            transition_strength(TeleportPhase::Revealing, REVEAL_SECONDS),
            0.
        );
        assert_eq!(transition_strength(TeleportPhase::Failed, 1.), 0.);
    }
}
