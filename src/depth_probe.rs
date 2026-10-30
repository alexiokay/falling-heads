//! Opt-in depth-only A/B measurement using the current mesh geometry.
//! Archived measurements belong to their recorded executable hash.
//! Never runs during ordinary play.
use crate::{
    components::{
        background::Background,
        block::{BiomeType, Block},
        enemy::Enemy,
        hero::Hero,
        villager::Villager,
    },
    resources::{graphics_settings::GraphicsSettings, settings::GameSettings},
    systems::{biome_encounters::StructureBackdrop, biome_teleport::BiomeTeleport},
};
use bevy::{
    diagnostic::DiagnosticsStore,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use std::collections::BTreeMap;

const ORDER: [bool; 8] = [false, true, true, false, true, false, false, true];
#[derive(Resource, Default)]
struct Probe {
    setup: u8,
    phase: usize,
    since: f64,
    warmup: f64,
    shot: bool,
    frames: Vec<f64>,
    gpu: BTreeMap<String, Vec<f64>>,
    last: BTreeMap<String, bevy::platform::time::Instant>,
    blocks: Vec<(u64, [u32; 3])>,
    camera: Option<Transform>,
    results: Vec<serde_json::Value>,
}
fn terrain(w: &mut World) -> Vec<(u64, [u32; 3])> {
    let mut result: Vec<_> = w
        .query_filtered::<(Entity, &Transform), With<Block>>()
        .iter(w)
        .map(|(e, t)| (e.to_bits(), t.translation.to_array().map(f32::to_bits)))
        .collect();
    result.sort_unstable();
    result
}
fn apply_depth(w: &mut World, after: bool) {
    for (mut t, b, s) in w
        .query::<(
            &mut Transform,
            Option<&Background>,
            Option<&StructureBackdrop>,
        )>()
        .iter_mut(w)
    {
        if b.is_some() {
            t.translation.z = if after {
                crate::systems::actor_depth::BACKGROUND
            } else {
                -10.
            };
        }
        if s.is_some() {
            t.translation.z = if after {
                crate::systems::actor_depth::STRUCTURE_BACKING
            } else {
                -5.
            };
        }
    }
}
fn begin(w: &mut World, now: f64) {
    let phase = w.resource::<Probe>().phase;
    apply_depth(w, ORDER[phase % 8]);
    let mut p = w.resource_mut::<Probe>();
    p.since = now;
    p.warmup = if phase.is_multiple_of(8) { 25. } else { 5. };
    p.shot = false;
    p.frames.clear();
    p.gpu.clear();
    p.last.clear();
    eprintln!(
        "[DEPTH] phase={phase} after={} warmup={}",
        ORDER[phase % 8],
        p.warmup
    );
}
fn median(v: &[f64]) -> f64 {
    let mut a = v.to_vec();
    a.sort_by(f64::total_cmp);
    a[a.len() / 2]
}
pub(crate) fn run(w: &mut World) {
    w.init_resource::<Probe>();
    let now = w.resource::<Time<Real>>().elapsed_secs_f64();
    assert!(now < 420., "Depth benchmark timeout");
    let setup = w.resource::<Probe>().setup;
    if setup == 0 && now > 8. {
        w.query::<&mut Window>()
            .single_mut(w)
            .unwrap()
            .resolution
            .set(1920., 1080.);
        {
            let mut g = w.resource_mut::<GraphicsSettings>();
            *g = GraphicsSettings::default();
            g.dlss = false;
            g.ray_tracing = false;
            g.frame_generation = false;
            g.fog = false;
            g.weather_particles = false;
        }
        let mut settings = w.resource_mut::<GameSettings>();
        settings.fps_limit = 0;
        settings.vsync_enabled = false;
        let y = crate::utils::biome_utils::nearest_biome_y(0., BiomeType::Underground);
        w.resource_mut::<BiomeTeleport>().target = Some(Vec3::new(0., y, 30.));
        let mut weather = w.resource_mut::<crate::components::weather::ActiveWeather>();
        weather.weather_type = crate::components::weather::WeatherType::Clear;
        weather.intensity = 0.;
        weather.duration_remaining = 9999.;
        w.resource_mut::<Probe>().setup = 1;
    } else if setup == 1 && now > 18. {
        let hero = w.query_filtered::<Entity, With<Hero>>().single(w).unwrap();
        w.entity_mut(hero).insert((
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::LinearVelocity::ZERO,
        ));
        let old: Vec<_> = w
            .query_filtered::<Entity, Or<(With<Enemy>, With<Villager>)>>()
            .iter(w)
            .collect();
        for e in old {
            w.despawn(e);
        }
        w.resource_mut::<Probe>().setup = 2;
    } else if setup == 2 && now > 28. {
        w.resource_mut::<Time<Virtual>>().pause();
        w.resource_mut::<Probe>().setup = 3;
    } else if setup == 3 && now > 35. {
        let blocks = terrain(w);
        let camera = *w
            .query_filtered::<&Transform, With<Camera3d>>()
            .single(w)
            .unwrap();
        let bg = w
            .query_filtered::<Entity, With<Background>>()
            .iter(w)
            .count();
        let backings = w
            .query_filtered::<Entity, With<StructureBackdrop>>()
            .iter(w)
            .count();
        eprintln!(
            "[DEPTH] scene blocks={} wallpaper={bg} backings={backings} camera={camera:?}",
            blocks.len()
        );
        let mut p = w.resource_mut::<Probe>();
        p.blocks = blocks;
        p.camera = Some(camera);
        p.setup = 4;
        begin(w, now);
    }
    if w.resource::<Probe>().setup != 4 {
        return;
    }
    let p = w.resource::<Probe>();
    let phase = p.phase;
    let elapsed = now - p.since;
    let warmup = p.warmup;
    if !p.shot && elapsed > 2. {
        if phase % 8 < 2 {
            w.spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!(
                    "target/depth-{}-{}.png",
                    if phase < 8 { "raster" } else { "rt" },
                    if ORDER[phase % 8] { "after" } else { "before" }
                )));
        }
        w.resource_mut::<Probe>().shot = true;
    }
    if elapsed <= warmup {
        return;
    }
    assert_eq!(
        w.resource::<crate::systems::native_gpu::NativeStatus>()
            .mode,
        if phase < 8 { 0 } else { 2 },
        "Expected renderer is active"
    );
    let dt = w.resource::<Time<Real>>().delta_secs_f64() * 1000.;
    let samples: Vec<_> = w
        .resource::<DiagnosticsStore>()
        .iter()
        .filter(|d| d.path().as_str().ends_with("/elapsed_gpu"))
        .filter_map(|d| {
            d.measurement()
                .map(|m| (d.path().as_str().to_owned(), m.time, m.value))
        })
        .collect();
    {
        let mut p = w.resource_mut::<Probe>();
        p.frames.push(dt);
        for (path, time, value) in samples {
            // Async GPU readback may not publish every main-world frame.
            // Do not count the same stale diagnostic repeatedly.
            if time.elapsed().as_secs_f64() < 1. && p.last.get(&path) != Some(&time) {
                p.last.insert(path.clone(), time);
                p.gpu.entry(path).or_default().push(value);
            }
        }
    }
    if elapsed < warmup + 8. {
        return;
    }
    assert_eq!(
        terrain(w),
        w.resource::<Probe>().blocks,
        "Terrain must be identical throughout A/B"
    );
    let camera = *w
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(w)
        .unwrap();
    assert_eq!(
        camera,
        w.resource::<Probe>().camera.unwrap(),
        "Camera must stay fixed"
    );
    let mut p = w.resource_mut::<Probe>();
    assert!(
        p.frames.len() > 100 && !p.gpu.is_empty(),
        "Need frame and GPU samples"
    );
    eprintln!(
        "[DEPTH] phase={phase} after={} frames={} median_ms={:.4} gpu_passes={}",
        ORDER[phase % 8],
        p.frames.len(),
        median(&p.frames),
        p.gpu.len()
    );
    let result = serde_json::json!({"phase":phase,"after":ORDER[phase%8],"mode":if phase<8 {"raster"}else{"rt"},"frame_ms":p.frames,"gpu_ms":p.gpu});
    p.results.push(result);
    std::fs::write(
        "target/depth-benchmark.json",
        serde_json::to_vec(&p.results).unwrap(),
    )
    .unwrap();
    p.phase += 1;
    if p.phase == 16 {
        eprintln!("[DEPTH] completed: unchanged terrain and camera, 16 measured phases");
        w.write_message(AppExit::Success);
        return;
    }
    if p.phase == 8 {
        let mut g = w.resource_mut::<GraphicsSettings>();
        g.ray_tracing = true;
        g.dlss = true;
        g.set_quality_step(0);
    }
    begin(w, now);
}
