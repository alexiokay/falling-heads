//! NVIDIA Streamline Vulkan Frame Generation with matching render-frame snapshots.
use ash::vk::Handle;
use bevy::{prelude::*, render::renderer::RenderDevice};
use std::{ffi::c_void, os::windows::ffi::OsStrExt};
#[repr(C)]
#[derive(Default, Debug)]
struct NativeStatus {
    result: i32,
    supported: u32,
    maximum: u32,
    presented: u32,
    status: u32,
    minimum: u32,
    rendered: u32,
}
unsafe extern "C" {
    fn mh_fg_init(plugins: *const u16, logs: *const u16) -> *const c_void;
    fn mh_fg_probe(physical: *mut c_void, out: *mut NativeStatus) -> i32;
    fn mh_fg_shutdown();
    fn mh_fg_before_present();
    fn mh_fg_after_present(result: i32);
    fn mh_fg_begin(frame: u32, limit_us: u32) -> i32;
    fn mh_fg_simulation_end(frame: u32);
    fn mh_fg_render_begin(frame: u32);
    fn mh_fg_render_end(frame: u32, inputs: *const NativeInputs) -> i32;
    fn mh_fg_stats(out: *mut NativeStatus);
    fn mh_fg_suspend();
}
#[derive(Resource, Default)]
pub struct FrameGenerationStatus {
    pub available: bool,
    pub active: bool,
    pub rendered_fps: f32,
    pub presented_fps: f32,
    pub detail: String,
}
#[derive(Resource)]
struct Backend;
pub fn prepare(app: &mut App) {
    app.init_resource::<FrameGenerationStatus>();
    let executable = std::env::current_exe().ok();
    let installed = executable
        .as_ref()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf());
    let plugins = installed
        .filter(|p| p.join("sl.interposer.dll").is_file())
        .or_else(|| {
            cfg!(debug_assertions).then(|| {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("vendor/Streamline-2.14.16/bin/x64")
            })
        });
    let Some(plugins) = plugins.filter(|p| p.join("sl.interposer.dll").is_file()) else {
        return;
    };
    let logs = std::env::current_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("logs/streamline");
    if std::fs::create_dir_all(&logs).is_err() {
        return;
    }
    let wide = |p: &std::path::Path| {
        p.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let plugins = wide(&plugins);
    let logs = wide(&logs);
    // SAFETY: both UTF-16 buffers are NUL-terminated and live through this synchronous call.
    let entry = unsafe { mh_fg_init(plugins.as_ptr(), logs.as_ptr()) };
    if entry.is_null() {
        return;
    }
    // SAFETY: initialization verifies the module and returns vkGetInstanceProcAddr;
    // it remains loaded until process exit, outliving every Vulkan function pointer.
    unsafe {
        wgpu_hal::vulkan::install_interposition(wgpu_hal::vulkan::InterpositionHooks {
            get_instance_proc_addr: std::mem::transmute(entry),
            unorm_swapchain: true,
            before_device_destroy: mh_fg_shutdown,
            before_present: mh_fg_before_present,
            after_present: mh_fg_after_present,
        })
        .expect("Streamline installation must precede Vulkan initialization");
    }
    // SAFETY: the process-lived C callback is idempotent and registered before device creation.
    unsafe {
        dlss_wgpu::set_external_ngx_shutdown(mh_fg_shutdown);
    }
    app.insert_resource(Backend);
}

use bevy::{
    camera::{MainPassResolutionOverride, NormalizedRenderTarget},
    core_pipeline::prepass::{
        DepthPrepass, MotionVectorPrepass, PreviousViewData, ViewPrepassTextures,
    },
    render::{
        RenderApp, RenderStartup,
        camera::{ExtractedCamera, TemporalJitter},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        renderer::{RenderGraph, RenderGraphSystems, RenderQueue},
        texture::GpuImage,
        view::ExtractedView,
    },
};
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct NativeTexture {
    image: u64,
    view: u64,
    width: u32,
    height: u32,
    format: u32,
    usage: u32,
    layout: u32,
}
#[repr(C)]
#[derive(Default)]
struct NativeInputs {
    color: NativeTexture,
    depth: NativeTexture,
    motion: NativeTexture,
    projection: [f32; 16],
    inverse_projection: [f32; 16],
    clip_to_previous: [f32; 16],
    previous_to_clip: [f32; 16],
    position: [f32; 3],
    up: [f32; 3],
    right: [f32; 3],
    forward: [f32; 3],
    jitter: [f32; 2],
    near: f32,
    far: f32,
    fov: f32,
    aspect: f32,
    reset: u32,
    orthographic: u32,
}
// Keep these checks paired with native/streamline.h; no SDK-owned structs
// or Rust references cross the C ABI. This bridge is Windows x64 only.
const _: () = {
    assert!(std::mem::size_of::<NativeStatus>() == 28);
    assert!(std::mem::size_of::<NativeTexture>() == 40);
    assert!(std::mem::size_of::<NativeInputs>() == 456);
    assert!(std::mem::offset_of!(NativeInputs, projection) == 120);
    assert!(std::mem::offset_of!(NativeInputs, reset) == 448);
};
#[derive(Resource, Default, Clone, ExtractResource)]
#[extract_app(RenderApp)]
struct Frame {
    index: u32,
    enabled: bool,
}
#[derive(Resource, Default)]
struct Copies {
    data: Option<CopyTextures>,
    pipeline: Option<wgpu::ComputePipeline>,
    was_enabled: bool,
    skip_frames: u8,
}
struct CopyTextures {
    size: UVec2,
    render_size: UVec2,
    color: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    motion: wgpu::Texture,
    motion_view: wgpu::TextureView,
}
pub fn install(app: &mut App) {
    app.add_systems(Update, update_status);
    if !app.world().contains_resource::<Backend>() {
        return;
    }
    if std::env::args().any(|a| a == "--fg-smoke-test") {
        app.add_systems(Update, smoke);
        app.world_mut()
            .resource_mut::<crate::resources::graphics_settings::GraphicsSettings>()
            .frame_generation = true;
    }
    if std::env::args().any(|a| a == "--background-motion-test") {
        app.add_systems(
            PostUpdate,
            super::background_system::motion_probe
                .before(bevy::transform::TransformSystems::Propagate),
        );
    }
    app.init_resource::<Frame>()
        .add_plugins(ExtractResourcePlugin::<Frame>::default())
        .add_systems(First, simulation_begin)
        .add_systems(Last, simulation_end)
        .add_systems(PostUpdate, prepasses);
    let render = app.sub_app_mut(RenderApp);
    render
        .init_resource::<Copies>()
        .add_systems(RenderStartup, probe)
        .add_systems(
            bevy::render::Render,
            preflight
                .after(bevy::render::RenderSystems::ExtractCommands)
                .before(bevy::render::view::window::create_surfaces)
                .before(bevy::render::view::window::prepare_windows),
        )
        .add_systems(RenderGraph, render_begin.in_set(RenderGraphSystems::Begin))
        .add_systems(RenderGraph, render_end.in_set(RenderGraphSystems::Finish));
}
fn probe(device: Res<RenderDevice>) {
    let device =
        unsafe { device.wgpu_device().as_hal::<wgpu_hal::api::Vulkan>() }.expect("Vulkan renderer");
    let mut result = NativeStatus::default();
    unsafe {
        mh_fg_probe(
            device.raw_physical_device().as_raw() as *mut c_void,
            &mut result,
        );
    }
    eprintln!("[FG PROBE] {result:?}");
}
fn simulation_begin(
    mut frame: ResMut<Frame>,
    settings: Res<crate::resources::settings::GameSettings>,
) {
    frame.index = frame.index.wrapping_add(1);
    let limit = if settings.fps_limit == 0 || settings.vsync_enabled {
        0
    } else {
        1_000_000 / settings.fps_limit as u32
    };
    unsafe {
        mh_fg_begin(frame.index, limit);
    }
}
fn simulation_end(
    mut frame: ResMut<Frame>,
    state: Res<State<crate::states::GameState>>,
    teleport: Res<super::biome_teleport::BiomeTeleport>,
    settings: Res<crate::resources::settings::GameSettings>,
    graphics: Res<crate::resources::graphics_settings::GraphicsSettings>,
    status: Res<FrameGenerationStatus>,
    windows: Query<&Window>,
    closing: MessageReader<bevy::window::WindowCloseRequested>,
    exits: MessageReader<AppExit>,
) {
    let window_ok = windows
        .single()
        .is_ok_and(|w| w.physical_width() >= 100 && w.physical_height() >= 100);
    frame.enabled = graphics.frame_generation
        && status.available
        && window_ok
        && closing.is_empty()
        && exits.is_empty()
        && *state.get() == crate::states::GameState::Playing
        && teleport.phase == super::biome_teleport::TeleportPhase::Idle
        && !settings.vsync_enabled;
    unsafe {
        mh_fg_simulation_end(frame.index);
    }
}
fn prepasses(
    mut commands: Commands,
    settings: Res<crate::resources::graphics_settings::GraphicsSettings>,
    status: Res<FrameGenerationStatus>,
    cameras: Query<(Entity, Has<DepthPrepass>, Has<MotionVectorPrepass>), With<Camera3d>>,
) {
    if !settings.frame_generation || !status.available {
        return;
    }
    for (e, depth, motion) in &cameras {
        if !depth || !motion {
            commands
                .entity(e)
                .insert((DepthPrepass, MotionVectorPrepass));
        }
    }
}
fn render_begin(frame: Res<Frame>) {
    unsafe {
        mh_fg_render_begin(frame.index);
    }
}
fn native_texture(
    texture: &wgpu::Texture,
    view: &wgpu::TextureView,
    format: ash::vk::Format,
    usage: ash::vk::ImageUsageFlags,
    layout: ash::vk::ImageLayout,
) -> NativeTexture {
    unsafe {
        // SAFETY: owners retain the textures/views through present; guards only expose borrowed Vulkan handles.
        // Drop each raw guard before acquiring another: wgpu has a non-recursive device snatch lock.
        let image = {
            texture
                .as_hal::<wgpu_hal::api::Vulkan>()
                .unwrap()
                .raw_handle()
                .as_raw()
        };
        let view = {
            view.as_hal::<wgpu_hal::api::Vulkan>()
                .unwrap()
                .raw_handle()
                .as_raw()
        };
        NativeTexture {
            image,
            view,
            width: texture.width(),
            height: texture.height(),
            format: format.as_raw() as u32,
            usage: usage.as_raw(),
            layout: layout.as_raw() as u32,
        }
    }
}
// Streamline normalizes the motion field over its tagged extent, not the allocation.
fn fg_render_extent(allocation: UVec2, resolution_override: Option<UVec2>) -> UVec2 {
    resolution_override
        .unwrap_or(allocation)
        .min(allocation)
        .max(UVec2::ONE)
}

fn fg_camera_cut(
    projection: Mat4,
    world: Mat4,
    previous_projection: Mat4,
    previous_world: Mat4,
) -> bool {
    // Match the game's SR/RR translation-cut threshold; also reset on abrupt
    // direction/FOV changes. Ordinary camera following retains temporal history.
    world.w_axis.truncate().distance_squared(previous_world.w_axis.truncate()) > 256. * 256.
        || world.z_axis.truncate().normalize().dot(previous_world.z_axis.truncate().normalize()) < 0.5
        // Ignore temporal jitter in previous clip_from_view (z-axis xy).
        || (projection.x_axis.x - previous_projection.x_axis.x).abs() > 0.001
        || (projection.y_axis.y - previous_projection.y_axis.y).abs() > 0.001
}

fn render_end(
    frame: Res<Frame>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<GpuImage>>,
    cameras: Query<(
        &ExtractedCamera,
        &ExtractedView,
        &ViewPrepassTextures,
        Option<&PreviousViewData>,
        Option<&TemporalJitter>,
        Option<&MainPassResolutionOverride>,
    )>,
    mut copies: ResMut<Copies>,
) {
    let mut inputs = None;
    if frame.enabled && copies.skip_frames == 0 {
        if let Some((camera, view, prepass, previous, jitter, resolution_override)) = cameras
            .iter()
            .find(|(c, _, _, _, _, _)| matches!(c.target, Some(NormalizedRenderTarget::Image(_))))
        {
            if let Some(NormalizedRenderTarget::Image(target)) = &camera.target {
                if let (Some(color), Some(depth), Some(motion)) = (
                    images.get(&target.handle),
                    prepass.depth_only_view(),
                    prepass.motion_vectors_view(),
                ) {
                    let size = UVec2::new(color.texture.width(), color.texture.height());
                    // Bevy keeps prepass allocations at OUTPUT resolution. DLSS only writes
                    // the top-left render-resolution region (also used by its own node).
                    // Copy/tag that region, otherwise FG stretches cleared/stale pixels
                    // across the scene and displaces the actual depth/motion data.
                    let render_size = fg_render_extent(
                        UVec2::new(prepass.size.width, prepass.size.height),
                        resolution_override.map(|r| r.0),
                    );
                    let d = device.wgpu_device();
                    if copies.pipeline.is_none() {
                        let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
                            label: Some("FG depth and motion snapshot"),
                            source: wgpu::ShaderSource::Wgsl(
                                include_str!("../../native/fg_copy.wgsl").into(),
                            ),
                        });
                        copies.pipeline =
                            Some(d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                                label: Some("FG snapshot"),
                                layout: None,
                                module: &module,
                                entry_point: Some("main"),
                                compilation_options: default(),
                                cache: None,
                            }));
                    }
                    let resized = copies
                        .data
                        .as_ref()
                        .is_none_or(|c| c.size != size || c.render_size != render_size);
                    if resized {
                        if std::env::args().any(|a| a == "--fg-smoke-test") {
                            eprintln!(
                                "[FG INPUT] output={}x{} allocation={}x{} active={}x{}",
                                size.x,
                                size.y,
                                prepass.size.width,
                                prepass.size.height,
                                render_size.x,
                                render_size.y
                            );
                        }
                        // The previous present blocks its client queue until FG has consumed these resources.
                        let make = |name: &'static str, size: UVec2, format, usage| {
                            d.create_texture(&wgpu::TextureDescriptor {
                                label: Some(name),
                                size: wgpu::Extent3d {
                                    width: size.x,
                                    height: size.y,
                                    depth_or_array_layers: 1,
                                },
                                mip_level_count: 1,
                                sample_count: 1,
                                dimension: wgpu::TextureDimension::D2,
                                format,
                                usage,
                                view_formats: &[],
                            })
                        };
                        let color = make(
                            "FG HUD-free color",
                            size,
                            wgpu::TextureFormat::Bgra8Unorm,
                            wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                        );
                        let depth = make(
                            "FG depth",
                            render_size,
                            wgpu::TextureFormat::R32Float,
                            wgpu::TextureUsages::STORAGE_BINDING
                                | wgpu::TextureUsages::TEXTURE_BINDING,
                        );
                        let motion = make(
                            "FG motion",
                            render_size,
                            wgpu::TextureFormat::Rg16Float,
                            wgpu::TextureUsages::STORAGE_BINDING
                                | wgpu::TextureUsages::TEXTURE_BINDING,
                        );
                        copies.data = Some(CopyTextures {
                            size,
                            render_size,
                            color_view: color.create_view(&default()),
                            depth_view: depth.create_view(&default()),
                            motion_view: motion.create_view(&default()),
                            color,
                            depth,
                            motion,
                        });
                    }
                    let c = copies.data.as_ref().unwrap();
                    let pipeline = copies.pipeline.as_ref().unwrap();
                    let bind = d.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("FG snapshot resources"),
                        layout: &pipeline.get_bind_group_layout(0),
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(depth),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(motion),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::TextureView(&c.depth_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: wgpu::BindingResource::TextureView(&c.motion_view),
                            },
                        ],
                    });
                    let mut encoder = d.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("FG input snapshot"),
                    });
                    encoder.copy_texture_to_texture(
                        color.texture.as_image_copy(),
                        c.color.as_image_copy(),
                        c.color.size(),
                    );
                    {
                        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("FG snapshot"),
                            ..default()
                        });
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(0, &bind, &[]);
                        pass.dispatch_workgroups(
                            render_size.x.div_ceil(8),
                            render_size.y.div_ceil(8),
                            1,
                        );
                    }
                    queue.submit([encoder.finish()]);
                    let projection = view.clip_from_view;
                    let world = view.world_from_view.to_matrix();
                    let clip = projection * world.inverse();
                    let to_previous = previous.map_or(Mat4::IDENTITY, |p| {
                        p.unjittered_clip_from_world * clip.inverse()
                    });
                    // Streamline stores matrix rows; Glam stores columns (match NVIDIA Vulkan sample).
                    let orthographic = projection.w_axis.w == 1.;
                    let near = if orthographic {
                        (projection.w_axis.z - 1.) / projection.z_axis.z
                    } else {
                        projection.w_axis.z
                    };
                    let far = if orthographic {
                        projection.w_axis.z / projection.z_axis.z
                    } else {
                        10000.
                    };
                    use ash::vk::{Format as F, ImageLayout as L, ImageUsageFlags as U};
                    inputs = Some(NativeInputs {
                        color: native_texture(
                            &c.color,
                            &c.color_view,
                            F::B8G8R8A8_UNORM,
                            U::TRANSFER_DST | U::SAMPLED,
                            L::TRANSFER_DST_OPTIMAL,
                        ),
                        depth: native_texture(
                            &c.depth,
                            &c.depth_view,
                            F::R32_SFLOAT,
                            U::STORAGE | U::SAMPLED,
                            L::GENERAL,
                        ),
                        motion: native_texture(
                            &c.motion,
                            &c.motion_view,
                            F::R16G16_SFLOAT,
                            U::STORAGE | U::SAMPLED,
                            L::GENERAL,
                        ),
                        projection: projection.transpose().to_cols_array(),
                        inverse_projection: projection.inverse().transpose().to_cols_array(),
                        clip_to_previous: to_previous.transpose().to_cols_array(),
                        previous_to_clip: to_previous.inverse().transpose().to_cols_array(),
                        position: world.w_axis.truncate().to_array(),
                        up: world.y_axis.truncate().normalize().to_array(),
                        right: world.x_axis.truncate().normalize().to_array(),
                        forward: (-world.z_axis.truncate().normalize()).to_array(),
                        jitter: jitter.map_or([0., 0.], |j| (-j.offset).to_array()),
                        near,
                        far,
                        fov: if orthographic {
                            std::f32::consts::FRAC_PI_2
                        } else {
                            2. * (1. / projection.y_axis.y).atan()
                        },
                        aspect: size.x as f32 / size.y as f32,
                        reset: u32::from(
                            resized
                                || !copies.was_enabled
                                || previous.is_none_or(|p| {
                                    fg_camera_cut(
                                        projection,
                                        world,
                                        p.clip_from_view,
                                        p.view_from_world.inverse(),
                                    )
                                }),
                        ),
                        orthographic: u32::from(orthographic),
                    });
                }
            }
        }
    }
    copies.skip_frames = copies.skip_frames.saturating_sub(1);
    copies.was_enabled = inputs.is_some();
    // SAFETY: C copies the stack input synchronously; Copies retains image/view owners
    // through present. RenderGraph Finish runs after the snapshot submission on the same queue.
    unsafe {
        mh_fg_render_end(frame.index, inputs.as_ref().map_or(std::ptr::null(), |i| i));
    }
}

fn preflight(
    windows: Query<&bevy::render::view::ExtractedWindow>,
    mut previous: Local<UVec2>,
    mut copies: ResMut<Copies>,
) {
    if let Ok(w) = windows.single() {
        let size = UVec2::new(w.physical_width, w.physical_height);
        if size != *previous {
            unsafe {
                mh_fg_suspend();
            }
            copies.was_enabled = false;
            copies.skip_frames = 2;
            *previous = size;
        }
    }
}
fn update_status(
    time: Res<Time<Real>>,
    settings: Res<crate::resources::graphics_settings::GraphicsSettings>,
    general: Res<crate::resources::settings::GameSettings>,
    mut status: ResMut<FrameGenerationStatus>,
    mut previous: Local<(f64, u32, u32)>,
    state: Res<State<crate::states::GameState>>,
) {
    let mut native = NativeStatus::default();
    unsafe {
        mh_fg_stats(&mut native);
    }
    status.available = native.supported != 0;
    let elapsed = time.elapsed_secs_f64() - previous.0;
    if elapsed >= 0.5 {
        status.rendered_fps = native.rendered.saturating_sub(previous.1) as f32 / elapsed as f32;
        status.presented_fps = native.presented.saturating_sub(previous.2) as f32 / elapsed as f32;
        *previous = (time.elapsed_secs_f64(), native.rendered, native.presented);
    }
    status.active = settings.frame_generation && status.presented_fps > status.rendered_fps * 1.25;
    status.detail = if !status.available {
        "FG unavailable on this GPU/runtime".into()
    } else if !settings.frame_generation {
        "Frame generation: OFF".into()
    } else if general.vsync_enabled {
        "FG suspended: turn VSync off".into()
    } else if native.result != 0 || native.status != 0 {
        format!("FG error: {} / {}", native.result, native.status)
    } else if *state.get() != crate::states::GameState::Playing {
        "FG 2x: suspended in menus".into()
    } else if status.active {
        format!(
            "FG 2x: {:.0} rendered / {:.0} presented FPS",
            status.rendered_fps, status.presented_fps
        )
    } else {
        "FG 2x ready (requires focused game)".into()
    };
}
#[derive(Resource, Default)]
struct Probe {
    setup: bool,
    step: u32,
    generated: u32,
    captures: u32,
}
fn smoke(world: &mut World) {
    let elapsed = world.resource::<Time<Real>>().elapsed_secs();
    if !world.contains_resource::<Probe>() {
        world.init_resource::<Probe>();
    }
    if elapsed > 8. && !world.resource::<Probe>().setup {
        crate::smoke_test::liquid_gallery(world);
        super::graphics::smoke_torches(world);
        world.resource_mut::<Probe>().setup = true;
    }
    for (index, (at, name)) in [(11., "sr"), (21., "resized"), (30., "rr"), (36., "raster")]
        .iter()
        .enumerate()
    {
        let mask = 1 << index;
        if elapsed > *at && world.resource::<Probe>().captures & mask == 0 {
            world
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(format!(
                    "target/fg-{name}-frame.png"
                )));
            world.resource_mut::<Probe>().captures |= mask;
            eprintln!(
                "[FG SMOKE] capture {name}: mode={}",
                world.resource::<super::native_gpu::NativeStatus>().mode
            );
        }
    }
    let step = world.resource::<Probe>().step;
    let mut stats = NativeStatus::default();
    unsafe {
        mh_fg_stats(&mut stats);
    }
    world.resource_mut::<Probe>().generated = stats.presented.saturating_sub(stats.rendered);
    use crate::{resources::graphics_settings::GraphicsSettings, states::GameState};
    match step {
        0 if elapsed > 12. => {
            world
                .resource_mut::<NextState<GameState>>()
                .set(GameState::Paused);
        }
        1 if elapsed > 15. => {
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(800., 1100.);
        }
        2 if elapsed > 18. => {
            world
                .resource_mut::<NextState<GameState>>()
                .set(GameState::Playing);
        }
        3 if elapsed > 23. => {
            world.resource_mut::<GraphicsSettings>().ray_tracing = true;
        }
        4 if elapsed > 31. => {
            world.resource_mut::<GraphicsSettings>().ray_tracing = false;
            world.resource_mut::<GraphicsSettings>().dlss = false;
        }
        5 if elapsed > 37. => {
            world.resource_mut::<GraphicsSettings>().frame_generation = false;
        }
        6 if elapsed > 40. => {
            eprintln!(
                "[FG SMOKE] {stats:?}; generated={}",
                world.resource::<Probe>().generated
            );
            // Switching back to Codex legitimately suspends generation; validate actual extra
            // presents accumulated while the test owned focus, independent of render frame rate.
            assert_eq!(stats.status, 0, "FG runtime status failure");
            assert_eq!(stats.result, 0, "FG API failure");
            if std::env::args().any(|a| a == "--fg-visible-test") {
                assert!(
                    world.resource::<Probe>().generated > 30,
                    "Expected actual generated frames while focused"
                );
            }
            world.write_message(AppExit::Success);
        }
        _ => return,
    }
    eprintln!("[FG SMOKE] transition {step} at {elapsed:.1}s {stats:?}");
    world.resource_mut::<Probe>().step += 1;
}

#[cfg(test)]
mod temporal_input_tests {
    use super::*;

    #[test]
    fn dlss_motion_field_covers_the_scene_without_unused_allocation_padding() {
        // Bevy's 1920x1080 prepass contains only 1280x720 valid DLSS pixels.
        let allocated = UVec2::new(1920, 1080);
        let active = UVec2::new(1280, 720);
        let tagged = fg_render_extent(allocated, Some(active));
        assert_eq!(tagged, active);
        // A feature at 75% of the rendered image must still be at 75% for FG.
        let pixel = active.as_vec2() * 0.75;
        assert!((pixel / tagged.as_vec2()).abs_diff_eq(Vec2::splat(0.75), 1e-6));
        assert_eq!(fg_render_extent(allocated, None), allocated);
        assert_eq!(
            fg_render_extent(UVec2::new(800, 1100), Some(UVec2::new(533, 733))),
            UVec2::new(533, 733)
        );
    }

    #[test]
    fn camera_cuts_reset_but_normal_follow_and_jitter_preserve_history() {
        let projection = Mat4::perspective_infinite_reverse_rh(0.8, 1.5, 0.1);
        let old = Mat4::from_translation(Vec3::new(30., 80., 1500.));
        let follow = Mat4::from_translation(Vec3::new(33., 84., 1500.));
        assert!(!fg_camera_cut(projection, follow, projection, old));
        let mut jittered = projection;
        jittered.z_axis.x += 0.002;
        jittered.z_axis.y -= 0.002;
        assert!(!fg_camera_cut(projection, follow, jittered, old));
        assert!(fg_camera_cut(
            projection,
            Mat4::from_translation(Vec3::new(600., 80., 1500.)),
            projection,
            old
        ));
        assert!(fg_camera_cut(
            projection,
            old * Mat4::from_rotation_y(1.6),
            projection,
            old
        ));
        let zoom = Mat4::perspective_infinite_reverse_rh(1.0, 1.5, 0.1);
        assert!(fg_camera_cut(zoom, old, projection, old));
    }
}
