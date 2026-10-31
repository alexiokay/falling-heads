//! Native Vulkan renderer: raster + DLSS SR, or Solari + DLSS RR.
//! Custom transparent effects stay forward-rendered; RT children never duplicate raster draws.
use crate::resources::graphics_settings::GraphicsSettings;
use bevy::{
    anti_alias::dlss::{
        Dlss, DlssPerfQualityMode, DlssProjectId, DlssRayReconstructionFeature,
        DlssRayReconstructionSupported, DlssSuperResolutionSupported,
    },
    camera::{CameraMainTextureUsages, visibility::VisibilitySystems},
    core_pipeline::prepass::{DeferredPrepass, MotionVectorPrepass},
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    pbr::{DefaultOpaqueRendererMethod, PreviousGlobalTransform},
    prelude::*,
    render::{
        render_resource::TextureUsages,
        settings::{Backends, RenderCreation, WgpuSettings},
    },
    solari::prelude::{RaytracingMesh3d, SolariLighting, SolariPlugins},
};
use bevy::{material::OpaqueRendererMethod, render::camera::TemporalJitter};
use std::collections::HashMap;
#[derive(Component)]
pub(crate) struct StatusLabel;
/// Declares ray-query support without a real GPU device (tests, headless probes).
#[derive(Resource)]
pub struct RayQuerySupported;
#[derive(Resource, Default)]
pub struct NativeStatus {
    pub sr: bool,
    pub rr: bool,
    /// Device has the ray-query features Solari needs
    pub ray_query: bool,
    pub mode: u8,
}
#[derive(Component)]
struct Prepared(AssetId<Mesh>, AssetId<StandardMaterial>);
#[derive(Component)]
struct RayInstance(Handle<Mesh>);
#[derive(Component)]
struct Emitter(Entity);
#[derive(Resource, Default)]
struct RayMeshes(HashMap<AssetId<Mesh>, Handle<Mesh>>);
#[derive(Resource)]
struct EmitMesh(Handle<Mesh>);
const EMITTER_RADIUS: f32 = 4.;

pub fn prepare(app: &mut App) {
    app.insert_resource(DlssProjectId(bevy::asset::uuid::Uuid::from_u128(
        0x2a6ed339f7f54a33940a5e7e9adc6137,
    )));
}
pub fn render_plugin() -> bevy::render::RenderPlugin {
    bevy::render::RenderPlugin {
        // Vulkan stays forced: Streamline frame generation needs Vulkan handles.
        // Ray-query features are NOT required here. Bevy's default (Functionality)
        // priority already enables every feature the adapter supports, so RTX
        // GPUs still get Solari, and GPUs without ray queries (GTX 10xx, most
        // integrated GPUs) start in raster/DLSS mode instead of failing in
        // request_device. Solari's plugins skip themselves when unsupported.
        render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
            backends: Some(Backends::VULKAN),
            ..default()
        })),
        ..default()
    }
}
pub fn install(app: &mut App) {
    app.add_plugins(SolariPlugins)
        .insert_resource(DefaultOpaqueRendererMethod::forward())
        .init_resource::<NativeStatus>()
        .init_resource::<RayMeshes>()
        .add_systems(Startup, setup_emitters)
        // Ray-traced copies, motion history and emitters only exist in mode 2;
        // raster and DLSS modes skip that per-frame work entirely.
        .add_systems(First, previous.run_if(ray_tracing))
        .add_systems(
            PostUpdate,
            drop_ray_proxies.run_if(resource_changed::<NativeStatus>),
        )
        .add_systems(Update, composite.before(configure))
        .add_systems(
            Update,
            (configure, materials, status_text)
                .chain()
                .after(super::graphics::camera_settings),
        )
        .add_systems(
            PostUpdate,
            reset_camera_history.before(bevy::transform::TransformSystems::Propagate),
        )
        .add_systems(
            PostUpdate,
            (attach, emitters)
                .chain()
                .run_if(ray_tracing)
                .before(bevy::transform::TransformSystems::Propagate),
        )
        .add_systems(
            PostUpdate,
            visibility
                .run_if(ray_tracing)
                .after(VisibilitySystems::VisibilityPropagate),
        );
}
fn ray_tracing(status: Res<NativeStatus>) -> bool {
    status.mode == 2
}
/// Leaving ray tracing: remove the RT proxies and emitters so raster/DLSS modes
/// carry no extra entities, meshes or acceleration structures. `attach` and
/// `emitters` rebuild them if mode 2 is selected again.
fn drop_ray_proxies(
    mut commands: Commands,
    status: Res<NativeStatus>,
    instances: Query<(Entity, &ChildOf), With<RayInstance>>,
    emitters: Query<Entity, With<Emitter>>,
    mut cache: ResMut<RayMeshes>,
) {
    if status.mode == 2 {
        return;
    }
    for (instance, parent) in &instances {
        commands.entity(instance).try_despawn();
        let owner = parent.parent();
        commands.queue(move |world: &mut World| {
            if let Ok(mut e) = world.get_entity_mut(owner) {
                e.remove::<Prepared>();
            }
        });
    }
    for emitter in &emitters {
        commands.entity(emitter).try_despawn();
    }
    cache.0.clear();
}
fn sr_quality(settings: &GraphicsSettings) -> DlssPerfQualityMode {
    match settings.quality_step() {
        0 => DlssPerfQualityMode::Quality,
        1 => DlssPerfQualityMode::Balanced,
        _ => DlssPerfQualityMode::Performance,
    }
}
#[allow(clippy::too_many_arguments)] // Bevy system parameters
fn configure(
    mut commands: Commands,
    settings: Res<GraphicsSettings>,
    sr: Option<Res<DlssSuperResolutionSupported>>,
    rr: Option<Res<DlssRayReconstructionSupported>>,
    device: Option<Res<bevy::render::renderer::RenderDevice>>,
    forced_ray_query: Option<Res<RayQuerySupported>>,
    mut status_res: ResMut<NativeStatus>,
    mut method: ResMut<DefaultOpaqueRendererMethod>,
    mut previous_mode: Local<Option<(u8, bool, u8, bool)>>,
    cameras: Query<
        (
            Entity,
            Has<SolariLighting>,
            Has<Dlss>,
            Has<Dlss<DlssRayReconstructionFeature>>,
        ),
        With<Camera3d>,
    >,
) {
    // Written every frame, but only flagged as changed when a value differs,
    // so systems gated on resource_changed::<NativeStatus> run only on switches.
    let status = status_res.bypass_change_detection();
    let before = (status.sr, status.rr, status.ray_query, status.mode);
    status.sr = sr.is_some();
    status.rr = rr.is_some();
    // Ray tracing also needs the device to actually have ray queries; without
    // them Solari is not loaded and mode 2 would render nothing.
    status.ray_query = forced_ray_query.is_some()
        || device.is_some_and(|d| {
            d.features()
                .contains(SolariPlugins::required_wgpu_features())
        });
    let mode = if settings.ray_tracing && settings.lighting && status.rr && status.ray_query {
        2
    } else if settings.dlss && status.sr {
        1
    } else {
        0
    };
    for (e, rt, dlss, denoised) in &cameras {
        if status.mode == mode
            && *previous_mode
                == Some((
                    mode,
                    settings.dlss,
                    if mode > 0 { settings.quality_step() } else { 0 },
                    settings.dlss_model_45 && mode == 1,
                ))
            && rt == (mode == 2)
            && dlss == (mode == 1)
            && denoised == (mode == 2)
        {
            continue;
        }
        let mut c = commands.entity(e);
        c.remove::<Dlss>()
            .remove::<Dlss<DlssRayReconstructionFeature>>()
            .remove::<SolariLighting>();
        if mode == 2 {
            method.set_to_deferred();
            c.insert((
                CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING),
                SolariLighting {
                    restir: true,
                    primary_di_samples: 16,
                    confidence_weight_cap: 4.,
                    world_cache_max_temporal_samples: 12.,
                    ..default()
                },
                Dlss::<DlssRayReconstructionFeature> {
                    perf_quality_mode: if settings.dlss {
                        sr_quality(&settings)
                    } else {
                        DlssPerfQualityMode::Dlaa
                    },
                    ..default()
                },
            ));
            c.remove::<bevy::pbr::ScreenSpaceAmbientOcclusion>()
                .remove::<bevy::anti_alias::fxaa::Fxaa>();
        } else {
            method.set_to_forward();
            c.remove::<DeferredPrepass>();
            if mode == 1 {
                c.insert(Dlss::<bevy::anti_alias::dlss::DlssSuperResolutionFeature> {
                    super_resolution_preset: if settings.dlss_model_45 { 13 } else { 11 },
                    // Model hints and input resolution are independent.
                    // Ray Reconstruction uses its own model and remains independent.
                    perf_quality_mode: sr_quality(&settings),
                    ..default()
                });
                c.remove::<bevy::anti_alias::fxaa::Fxaa>();
            } else {
                c.remove::<TemporalJitter>()
                    .remove::<bevy::render::camera::MipBias>()
                    .remove::<MotionVectorPrepass>()
                    .remove::<bevy::camera::MainPassResolutionOverride>();
            }
        }
        info!(
            "FALLING_HEADS GPU mode={mode} SR_supported={} RR_supported={}",
            status.sr, status.rr
        );
    }
    status.mode = mode;
    if (status.sr, status.rr, status.ray_query, status.mode) != before {
        status_res.set_changed();
    }
    *previous_mode = Some((
        mode,
        settings.dlss,
        if mode > 0 { settings.quality_step() } else { 0 },
        settings.dlss_model_45 && mode == 1,
    ));
}
fn materials(status: Res<NativeStatus>, mut assets: ResMut<Assets<StandardMaterial>>) {
    // iter_mut emits Modified for every asset even if its fields are untouched.
    // Inspect immutably so weather particles do not upload every material again
    // on every frame. New or renderer-switched materials still update normally.
    let updates: Vec<_> = assets
        .iter()
        .filter_map(|(id, m)| {
            let desired = if status.mode == 2 && primary_deferred(m) {
                OpaqueRendererMethod::Deferred
            } else {
                OpaqueRendererMethod::Forward
            };
            (m.opaque_render_method != desired).then_some((id, desired))
        })
        .collect();
    for (id, desired) in updates {
        if let Some(mut material) = assets.get_mut(id) {
            material.opaque_render_method = desired;
        }
    }
}
// Lit cutouts need the same depth/albedo/normal guides as opaque lit geometry.
// Unlit artwork stays in the forward pass; secondary-ray eligibility is separate.
fn primary_deferred(m: &StandardMaterial) -> bool {
    !m.unlit
        && matches!(m.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_))
        && m.specular_transmission == 0.
}
fn traceable(m: &StandardMaterial) -> bool {
    m.alpha_mode == AlphaMode::Opaque && !m.unlit && m.specular_transmission == 0.
}
fn status_text(
    status: Res<NativeStatus>,
    settings: Res<GraphicsSettings>,
    mut labels: Query<&mut Text, With<StatusLabel>>,
    #[cfg(target_os = "windows")] fg: Option<Res<super::frame_generation::FrameGenerationStatus>>,
) {
    for mut text in &mut labels {
        **text = if settings.ray_tracing && !status.ray_query {
            "Ray tracing unavailable: this GPU has no ray-query support".into()
        } else if settings.ray_tracing && !status.rr {
            "Ray tracing unavailable: check NVIDIA driver / DLLs".into()
        } else if settings.dlss && !status.sr {
            "DLSS unavailable: check NVIDIA driver / DLLs".into()
        } else {
            match status.mode {
                2 => "ACTIVE: Solari + DLSS RR",
                1 if settings.dlss_model_45 => "ACTIVE: DLSS 4.5",
                1 => "ACTIVE: DLSS 4",
                _ => "ACTIVE: native raster",
            }
            .into()
        };
        text.push_str(&format!("\nResolution: {}. Drag the slider; release to apply.\nRay tracing uses Ray Reconstruction; raster uses DLSS 4.5.",settings.quality_name()));
        #[cfg(target_os = "windows")]
        if let Some(fg) = &fg {
            text.push_str("\n");
            text.push_str(&fg.detail);
        }
    }
}
fn rt_mesh(source: &Mesh) -> Option<Mesh> {
    if source.primitive_topology() != PrimitiveTopology::TriangleList || source.count_vertices() < 3
    {
        return None;
    }
    let positions = source.attribute(Mesh::ATTRIBUTE_POSITION)?.clone();
    let ids: Vec<u32> = source
        .indices()
        .map(|v| v.iter().map(|i| i as u32).collect())
        .unwrap_or_else(|| (0..source.count_vertices() as u32).collect());
    if ids.is_empty()
        || ids.len() % 3 != 0
        || ids.iter().any(|i| *i as usize >= source.count_vertices())
    {
        return None;
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    for attr in [
        Mesh::ATTRIBUTE_NORMAL,
        Mesh::ATTRIBUTE_UV_0,
        Mesh::ATTRIBUTE_TANGENT,
    ] {
        if let Some(v) = source.attribute(attr) {
            mesh.insert_attribute(attr, v.clone());
        }
    }
    mesh.insert_indices(Indices::U32(ids));
    if !mesh.contains_attribute(Mesh::ATTRIBUTE_NORMAL) {
        mesh.compute_normals();
    }
    if !mesh.contains_attribute(Mesh::ATTRIBUTE_UV_0) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; mesh.count_vertices()]);
    }
    if !mesh.contains_attribute(Mesh::ATTRIBUTE_TANGENT) && mesh.generate_tangents().is_err() {
        let VertexAttributeValues::Float32x3(ns) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)? else {
            return None;
        };
        let ts: Vec<[f32; 4]> = ns
            .iter()
            .map(|n| {
                Vec3::from_array(*n)
                    .normalize_or(Vec3::Y)
                    .any_orthonormal_vector()
                    .extend(1.)
                    .to_array()
            })
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, ts);
    }
    mesh.enable_raytracing = true;
    Some(mesh)
}
fn attach(
    mut commands: Commands,
    sources: Query<
        (
            Entity,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            Option<&Prepared>,
            Option<&Children>,
        ),
        (
            Without<RayInstance>,
            Without<bevy::light::NotShadowCaster>,
            Without<Emitter>,
        ),
    >,
    instances: Query<(Entity, &ChildOf), With<RayInstance>>,
    owners: Query<(Has<Mesh3d>, Has<bevy::light::NotShadowCaster>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mats: Res<Assets<StandardMaterial>>,
    mut cache: ResMut<RayMeshes>,
    mut events: MessageReader<AssetEvent<Mesh>>,
) {
    // A removed raster mesh (teleport or replacement) must lose its old RT child.
    // Otherwise Solari keeps drawing the pre-transition skeleton/face indefinitely.
    for (instance, parent) in &instances {
        // Extended crack materials retain the same solid mesh and RT occluder.
        // Only absent geometry or an explicit non-caster invalidates the proxy.
        if !owners
            .get(parent.parent())
            .is_ok_and(|(mesh, no_shadow)| mesh && !no_shadow)
        {
            commands.entity(instance).try_despawn();
            let owner = parent.parent();
            commands.queue(move |world: &mut World| {
                if let Ok(mut e) = world.get_entity_mut(owner) {
                    e.remove::<Prepared>();
                }
            });
        }
    }
    for event in events.read() {
        match event {
            AssetEvent::Modified { id } => {
                if let Some(rt) = cache.0.get(id) {
                    if let Some(m) = meshes.get(*id).and_then(rt_mesh) {
                        let _ = meshes.insert(rt.id(), m);
                    }
                }
            }
            AssetEvent::Removed { id } => {
                cache.0.remove(id);
            }
            _ => {}
        }
    }
    for (e, mesh, mat, prepared, children) in &sources {
        if prepared.is_some_and(|p| p.0 == mesh.0.id() && p.1 == mat.0.id()) {
            continue;
        }
        if let Some(children) = children {
            for child in children.iter().filter(|c| instances.contains(*c)) {
                commands.entity(child).try_despawn();
            }
        }
        let Some(material) = mats.get(&mat.0) else {
            continue;
        };
        if !traceable(material) {
            continue;
        }
        let Some(source) = meshes.get(&mesh.0) else {
            continue;
        };
        let ray = if let Some(h) = cache.0.get(&mesh.0.id()) {
            h.clone()
        } else {
            let Some(rt) = rt_mesh(source) else {
                continue;
            };
            let h = meshes.add(rt);
            cache.0.insert(mesh.0.id(), h.clone());
            h
        };
        commands
            .entity(e)
            .insert(Prepared(mesh.0.id(), mat.0.id()))
            .with_children(|p| {
                p.spawn((
                    RayInstance(ray),
                    MeshMaterial3d(mat.0.clone()),
                    Transform::IDENTITY,
                    Visibility::Inherited,
                    PreviousGlobalTransform::default(),
                ));
            });
    }
}
fn previous(
    mut q: Query<
        (&GlobalTransform, &mut PreviousGlobalTransform),
        Or<(With<RayInstance>, With<Emitter>)>,
    >,
) {
    for (t, mut p) in &mut q {
        *p = PreviousGlobalTransform(t.affine());
    }
}
fn visibility(
    mut commands: Commands,
    status: Res<NativeStatus>,
    q: Query<(
        Entity,
        &RayInstance,
        &ChildOf,
        &InheritedVisibility,
        Has<RaytracingMesh3d>,
        &GlobalTransform,
    )>,
    hero: Query<&GlobalTransform, With<crate::components::hero::Hero>>,
    layers: Query<Option<&bevy::camera::visibility::RenderLayers>>,
    camera_layers: Query<Option<&bevy::camera::visibility::RenderLayers>, With<Camera3d>>,
    state: Res<State<crate::states::GameState>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    let center = hero.iter().next().map(|t| t.translation());
    let view = super::graphics::light_view(&cameras, center.unwrap_or_default().xy());
    let camera_mask = camera_layers
        .iter()
        .next()
        .flatten()
        .cloned()
        .unwrap_or_default();
    for (e, h, parent, v, active, t) in &q {
        let source_mask = layers
            .get(parent.parent())
            .ok()
            .flatten()
            .cloned()
            .unwrap_or_default();
        let visible = status.mode == 2
            && v.get()
            && camera_mask.intersects(&source_mask)
            && (*state.get() == crate::states::GameState::Rift
                || (t
                    .translation()
                    .xy()
                    .cmpge(view.min - Vec2::splat(800.))
                    .all()
                    && t.translation()
                        .xy()
                        .cmple(view.max + Vec2::splat(800.))
                        .all()));
        if visible && !active {
            commands.entity(e).insert(RaytracingMesh3d(h.0.clone()));
        }
        if !visible && active {
            commands.entity(e).remove::<RaytracingMesh3d>();
        }
    }
}
fn setup_emitters(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let sphere = Mesh::from(Sphere::new(EMITTER_RADIUS));
    commands.insert_resource(EmitMesh(meshes.add(rt_mesh(&sphere).unwrap())));
}
fn emitters(
    mut commands: Commands,
    status: Res<NativeStatus>,
    mesh: Res<EmitMesh>,
    lights: Query<
        (
            Entity,
            &PointLight,
            &Transform,
            Option<&bevy::camera::visibility::RenderLayers>,
        ),
        Without<Emitter>,
    >,
    cameras: Query<Option<&bevy::camera::visibility::RenderLayers>, With<Camera3d>>,
    mut instances: Query<
        (
            Entity,
            &Emitter,
            &mut Transform,
            &mut Visibility,
            &MeshMaterial3d<StandardMaterial>,
            Has<RaytracingMesh3d>,
            Option<&bevy::camera::visibility::RenderLayers>,
        ),
        Without<PointLight>,
    >,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let existing: std::collections::HashSet<_> =
        instances.iter().map(|(_, e, _, _, _, _, _)| e.0).collect();
    let camera_mask = cameras.iter().next().flatten().cloned().unwrap_or_default();
    for (e, light, t, layers) in &lights {
        if !existing.contains(&e) {
            commands.spawn((
                Emitter(e),
                layers.cloned().unwrap_or_default(),
                Mesh3d(mesh.0.clone()),
                MeshMaterial3d(mats.add(StandardMaterial {
                    emissive: LinearRgba::BLACK,
                    ..default()
                })),
                *t,
                Visibility::Inherited,
                PreviousGlobalTransform::default(),
            ));
        }
        let _ = light;
    }
    for (e, owner, mut t, mut visible, mat, active, current_mask) in &mut instances {
        let Ok((_, light, source, layers)) = lights.get(owner.0) else {
            commands.entity(e).despawn();
            continue;
        };
        // Write only on change: every write marks the transform/material dirty
        // and re-uploads it, which was 128+ material uploads per frame.
        if *t != *source {
            *t = *source;
        }
        let source_mask = layers.cloned().unwrap_or_default();
        if current_mask != Some(&source_mask) {
            commands.entity(e).insert(source_mask.clone());
        }
        let enabled =
            status.mode == 2 && light.intensity > 0. && camera_mask.intersects(&source_mask);
        let wanted = if enabled {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visible != wanted {
            *visible = wanted;
        }
        if enabled && !active {
            commands.entity(e).insert(RaytracingMesh3d(mesh.0.clone()));
        }
        if !enabled && active {
            commands.entity(e).remove::<RaytracingMesh3d>();
        }
        // PointLight intensity is lumens; StandardMaterial emission is cd/m².
        // A diffuse sphere emits flux = luminance * surface area * PI.
        // Preserve the actual light power when changing the emitter radius.
        let radiance =
            light.intensity.max(0.) / (4. * std::f32::consts::PI.powi(2) * EMITTER_RADIUS.powi(2));
        let emissive = light.color.to_linear() * radiance;
        if mats.get(&mat.0).is_some_and(|m| m.emissive != emissive) {
            if let Some(mut m) = mats.get_mut(&mat.0) {
                m.emissive = emissive;
            }
        }
    }
}

pub struct GpuSmokePlugin;
impl Plugin for GpuSmokePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
            .init_resource::<GpuProbe>()
            .add_systems(Update, gpu_probe);
    }
}
#[derive(Resource, Default)]
struct GpuProbe {
    fixture: bool,
    step: u8,
    since: f64,
}
fn gpu_probe(world: &mut World) {
    let elapsed = world.resource::<Time<Real>>().elapsed_secs_f64();
    if elapsed > 10. && !world.resource::<GpuProbe>().fixture {
        crate::smoke_test::liquid_gallery(world);
        super::graphics::smoke_torches(world);
        let mut cycle = world.resource_mut::<crate::resources::day_night_cycle::DayNightCycle>();
        cycle.current_time = cycle.cycle_duration * 0.75;
        world.resource_mut::<GpuProbe>().fixture = true;
    }
    let step = world.resource::<GpuProbe>().step;
    assert!(elapsed < 240., "GPU smoke timeout at step {step}");
    if elapsed < 25. || elapsed - world.resource::<GpuProbe>().since < 6. {
        return;
    }
    let status = world.resource::<NativeStatus>();
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    if step == 0
        && !gpu_timing(world, "render/dlss_super_resolution/elapsed_gpu").is_some_and(|v| v > 0.)
    {
        return;
    }
    if step == 2
        && !gpu_timing(world, "render/dlss_ray_reconstruction/elapsed_gpu").is_some_and(|v| v > 0.)
    {
        return;
    }
    match step {
        0 => {
            assert!(status.sr && status.rr);
            assert_eq!(status.mode, 1);
            world
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("falling-heads-gpu-dlss.png"));
        }
        1 => {
            world.resource_mut::<GraphicsSettings>().ray_tracing = true;
        }
        2 => {
            assert_eq!(status.mode, 2);
            verify_gpu_timings(world);
            assert!(world.query::<&RaytracingMesh3d>().iter(world).count() > 10);
            world
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("falling-heads-gpu-raytracing.png"));
        }
        3 => {
            world.resource_mut::<GraphicsSettings>().dlss = false;
        }
        4 => {
            assert_eq!(status.mode, 2);
            world
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("falling-heads-gpu-dlaa.png"));
        }
        5 => {
            world.resource_mut::<GraphicsSettings>().ray_tracing = false;
        }
        6 => {
            assert_eq!(status.mode, 0);
            assert_eq!(world.query::<&RaytracingMesh3d>().iter(world).count(), 0);
            world
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("falling-heads-gpu-raster.png"));
        }
        7 => {
            world.resource_mut::<GraphicsSettings>().dlss = true;
        }
        8 => {
            assert_eq!(status.mode, 1);
            world
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("falling-heads-gpu-dlss-restored.png"));
        }
        9 => {
            eprintln!(
                "GPU SMOKE PASSED: supported DLSS SR, Solari/RR, native RR, raster off and DLSS on transitions"
            );
            world.write_message(AppExit::Success);
        }
        _ => {}
    }
    let mut probe = world.resource_mut::<GpuProbe>();
    probe.step += 1;
    probe.since = elapsed;
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removing_a_raster_source_also_removes_its_rt_proxy() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<RayMeshes>()
            .add_message::<AssetEvent<Mesh>>()
            .add_systems(Update, attach);
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::new(64., 64., 64.));
        let mat = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let e = app
            .world_mut()
            .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(mat)))
            .id();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&RayInstance>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut().entity_mut(e).remove::<Mesh3d>();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&RayInstance>()
                .iter(app.world())
                .count(),
            0
        );
        assert!(app.world().get::<Prepared>(e).is_none());
        app.world_mut().entity_mut(e).insert(Mesh3d(mesh));
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&RayInstance>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut()
            .entity_mut(e)
            .remove::<MeshMaterial3d<StandardMaterial>>();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&RayInstance>()
                .iter(app.world())
                .count(),
            1,
            "Extended crack materials retain solid occlusion"
        );
        app.world_mut()
            .entity_mut(e)
            .insert(bevy::light::NotShadowCaster);
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&RayInstance>()
                .iter(app.world())
                .count(),
            0
        );
    }
    #[test]
    fn stable_renderer_does_not_reupload_materials_and_mode_switch_updates_only_lit_surfaces() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<StandardMaterial>()
            .init_resource::<NativeStatus>()
            .add_systems(Update, materials);
        let lit = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let unlit = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                unlit: true,
                ..default()
            });
        app.update();
        let mut cursor =
            bevy::ecs::message::MessageCursor::<AssetEvent<StandardMaterial>>::default();
        cursor
            .read(
                app.world()
                    .resource::<Messages<AssetEvent<StandardMaterial>>>(),
            )
            .for_each(drop);
        app.update();
        assert_eq!(
            cursor
                .read(
                    app.world()
                        .resource::<Messages<AssetEvent<StandardMaterial>>>()
                )
                .count(),
            0
        );
        app.world_mut().resource_mut::<NativeStatus>().mode = 2;
        app.update();
        let modified: Vec<_> = cursor
            .read(
                app.world()
                    .resource::<Messages<AssetEvent<StandardMaterial>>>(),
            )
            .filter_map(|event| match event {
                AssetEvent::Modified { id } => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(modified, vec![lit.id()]);
        let assets = app.world().resource::<Assets<StandardMaterial>>();
        assert_eq!(
            assets.get(&lit).unwrap().opaque_render_method,
            OpaqueRendererMethod::Deferred
        );
        assert_eq!(
            assets.get(&unlit).unwrap().opaque_render_method,
            OpaqueRendererMethod::Forward
        );
    }

    #[test]
    fn cutouts_write_primary_gbuffer_without_becoming_solid_ray_occluders() {
        let mask = StandardMaterial {
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        };
        assert!(primary_deferred(&mask));
        assert!(!traceable(&mask));
        assert!(!primary_deferred(&StandardMaterial {
            unlit: true,
            ..default()
        }));
        assert!(!primary_deferred(&StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            ..default()
        }));
    }
    #[test]
    fn compositor_has_independent_world_target_and_resizes() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .add_systems(Update, composite);
        let camera = app.world_mut().spawn(Camera3d::default()).id();
        let window = app
            .world_mut()
            .spawn(Window {
                resolution: bevy::window::WindowResolution::new(720, 1280),
                ..default()
            })
            .id();
        app.update();
        let world = app.world_mut();
        assert!(matches!(
            world.get::<bevy::camera::RenderTarget>(camera),
            Some(bevy::camera::RenderTarget::Image(_))
        ));
        assert_eq!(
            world
                .query_filtered::<&Sprite, With<WorldComposite>>()
                .single(world)
                .unwrap()
                .custom_size,
            Some(Vec2::new(720., 1280.))
        );
        world
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set(800., 600.);
        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<&Sprite, With<WorldComposite>>()
                .single(world)
                .unwrap()
                .custom_size,
            Some(Vec2::new(800., 600.))
        );
    }
    #[test]
    fn switching_renderer_cleans_temporal_components() {
        let mut app = App::new();
        app.insert_resource(GraphicsSettings::default())
            .init_resource::<NativeStatus>()
            .insert_resource(DefaultOpaqueRendererMethod::forward())
            .insert_resource(DlssSuperResolutionSupported)
            .insert_resource(DlssRayReconstructionSupported)
            .insert_resource(RayQuerySupported)
            .add_systems(Update, configure);
        let camera = app.world_mut().spawn(Camera3d::default()).id();
        app.update();
        assert!(app.world().get::<Dlss>(camera).is_some());
        assert_eq!(
            app.world()
                .get::<Dlss>(camera)
                .unwrap()
                .super_resolution_preset,
            13
        );
        app.world_mut()
            .resource_mut::<GraphicsSettings>()
            .dlss_model_45 = true;
        app.update();
        assert_eq!(
            app.world()
                .get::<Dlss>(camera)
                .unwrap()
                .super_resolution_preset,
            13
        );
        assert_eq!(
            app.world().get::<Dlss>(camera).unwrap().perf_quality_mode,
            DlssPerfQualityMode::Quality
        );
        app.world_mut()
            .resource_mut::<GraphicsSettings>()
            .dlss_45_performance = true;
        app.update();
        assert_eq!(
            app.world().get::<Dlss>(camera).unwrap().perf_quality_mode,
            DlssPerfQualityMode::Performance
        );
        app.world_mut()
            .resource_mut::<GraphicsSettings>()
            .ray_tracing = true;
        app.update();
        assert!(app.world().get::<Dlss>(camera).is_none());
        assert!(app.world().get::<SolariLighting>(camera).is_some());
        assert_eq!(
            app.world()
                .get::<Dlss<DlssRayReconstructionFeature>>(camera)
                .unwrap()
                .perf_quality_mode,
            DlssPerfQualityMode::Performance
        );
        {
            let mut s = app.world_mut().resource_mut::<GraphicsSettings>();
            s.ray_tracing = false;
            s.dlss = false;
        }
        app.update();
        assert!(app.world().get::<SolariLighting>(camera).is_none());
        assert!(app.world().get::<DeferredPrepass>(camera).is_none());
        assert!(app.world().get::<TemporalJitter>(camera).is_none());
        assert!(
            app.world()
                .get::<bevy::render::camera::MipBias>(camera)
                .is_none()
        );
    }
    #[test]
    fn ray_mesh_has_exact_layout_and_rejects_empty() {
        let m = rt_mesh(&Mesh::from(Cuboid::new(1., 1., 1.))).unwrap();
        assert!(m.enable_raytracing);
        assert!(matches!(m.indices(), Some(Indices::U32(_))));
        assert!(m.contains_attribute(Mesh::ATTRIBUTE_TANGENT));
        assert!(rt_mesh(&Mesh::new(PrimitiveTopology::TriangleList, default())).is_none());
    }
}

fn verify_gpu_timings(world: &World) {
    for path in [
        "render/solari_lighting/lighting/elapsed_gpu",
        "render/solari_lighting/world_cache/elapsed_gpu",
        "render/dlss_ray_reconstruction/elapsed_gpu",
    ] {
        let value = world
            .resource::<bevy::diagnostic::DiagnosticsStore>()
            .get(&bevy::diagnostic::DiagnosticPath::new(path))
            .and_then(|d| d.smoothed());
        assert!(
            value.is_some_and(|v| v > 0.),
            "GPU pass did not execute: {path}: {value:?}"
        );
        eprintln!("GPU PASS {path}: {value:?} ms");
    }
}

fn reset_camera_history(
    mut camera: Query<
        (
            &Transform,
            Option<&mut Dlss>,
            Option<&mut Dlss<DlssRayReconstructionFeature>>,
            Option<&mut SolariLighting>,
        ),
        With<Camera3d>,
    >,
    mut previous: Local<Option<Vec3>>,
) {
    for (t, sr, rr, solari) in &mut camera {
        if previous.is_some_and(|p| p.distance_squared(t.translation) > 256. * 256.) {
            if let Some(mut sr) = sr {
                sr.reset = true;
            }
            if let Some(mut rr) = rr {
                rr.reset = true;
            }
            if let Some(mut solari) = solari {
                solari.reset = true;
            }
        }
        *previous = Some(t.translation);
    }
}

fn gpu_timing(world: &World, path: &'static str) -> Option<f64> {
    world
        .resource::<bevy::diagnostic::DiagnosticsStore>()
        .get(&bevy::diagnostic::DiagnosticPath::new(path))
        .and_then(|d| d.smoothed())
}

#[derive(Component)]
struct WorldComposite;
#[derive(Default)]
struct Surface {
    image: Handle<Image>,
    size: UVec2,
    scale: f32,
}
fn composite(
    mut commands: Commands,
    windows: Query<&Window>,
    cameras: Query<Entity, With<Camera3d>>,
    mut images: ResMut<Assets<Image>>,
    mut surface: Local<Surface>,
    mut sprites: Query<&mut Sprite, With<WorldComposite>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    if window.physical_width() == 0 || window.physical_height() == 0 {
        return;
    }
    let size = UVec2::new(window.physical_width(), window.physical_height());
    let scale = window.scale_factor();
    if surface.size == size && surface.scale == scale {
        return;
    }
    let mut image = Image::new_target_texture(
        size.x,
        size.y,
        bevy::render::render_resource::TextureFormat::Bgra8UnormSrgb,
        None,
    );
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    if surface.size == UVec2::ZERO {
        surface.image = images.add(image);
        commands.spawn((
            WorldComposite,
            Sprite {
                image: surface.image.clone(),
                custom_size: Some(Vec2::new(window.width(), window.height())),
                ..default()
            },
            Transform::from_xyz(0., 0., -500.),
        ));
    } else {
        // A new handle invalidates camera and sprite caches together after resize.
        surface.image = images.add(image);
        for mut sprite in &mut sprites {
            sprite.image = surface.image.clone();
            sprite.custom_size = Some(Vec2::new(window.width(), window.height()));
        }
    }
    for camera in &cameras {
        commands
            .entity(camera)
            .insert(bevy::camera::RenderTarget::Image(
                bevy::camera::ImageRenderTarget {
                    handle: surface.image.clone(),
                    scale_factor: scale,
                },
            ));
    }
    surface.size = size;
    surface.scale = scale;
}
