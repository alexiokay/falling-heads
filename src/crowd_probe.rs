//! Reproducible visible crowd workload; uses the production villager spawn path.
use crate::{
    components::{
        block::{Block, BlockType},
        enemy::Enemy,
        hero::Hero,
        villager::Villager,
    },
    resources::{
        asset_handles::GameTextures, graphics_settings::GraphicsSettings,
        material_cache::MaterialCache, mesh_cache::MeshCache, settings::GameSettings,
    },
};
use avian3d::prelude::*;
use bevy::{
    ecs::system::SystemState,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
#[derive(Component)]
struct CrowdMember;
#[derive(Resource, Default)]
struct Probe {
    step: u8,
    since: f32,
    anchor: Vec3,
    frames: Vec<f32>,
    narrow: Vec<f32>,
    solver: Vec<f32>,
}
fn large_layout(n: usize) -> (usize, usize, i32, i32) {
    let columns = (n as f32 * 1.6).sqrt().ceil() as usize;
    let rows = n.div_ceil(columns);
    let half_x = ((columns as f32 * 60. * 0.5 + 64.) / 64.).ceil() as i32;
    let half_y = ((rows as f32 * 60. * 0.5 + 64.) / 64.).ceil() as i32;
    (columns, rows, half_x, half_y)
}
fn spawn(w: &mut World, n: usize, p: Vec3) {
    let old: Vec<_> = w
        .query_filtered::<Entity, With<Villager>>()
        .iter(w)
        .collect();
    for e in old {
        w.despawn(e);
    }
    let mut state = SystemState::<(
        Commands,
        Res<GameTextures>,
        ResMut<Assets<Mesh>>,
        ResMut<Assets<StandardMaterial>>,
        Res<AssetServer>,
    )>::new(w);
    let (mut commands, textures, mut meshes, mut mats, server) = state.get_mut(w).unwrap();
    for i in 0..n {
        let q = p + if n > 370 {
            let (cols, rows, _, _) = large_layout(n);
            Vec3::new(
                ((i % cols) as f32 - (cols - 1) as f32 * 0.5) * 60.,
                ((i / cols) as f32 - (rows - 1) as f32 * 0.5) * 60.,
                0.,
            )
        } else if n > 100 {
            Vec3::new(
                (i % 20) as f32 * 50. - 475.,
                (i / 20) as f32 * 44. - 330.,
                0.,
            )
        } else {
            Vec3::new(
                (i % 10) as f32 * 78. - 351.,
                (i / 10) as f32 * 74. - 270.,
                0.,
            )
        };
        let e = crate::systems::entity_spawn::spawn_villager(
            &mut commands,
            q,
            &textures,
            &mut meshes,
            &mut mats,
            if std::env::args().any(|a| a == "--crowd-named") {
                Some(format!("Viewer_{i:03}"))
            } else {
                None
            },
            &server,
        );
        commands.entity(e).insert(CrowdMember);
    }
    state.apply(w);
    // Identical movement inputs for each measured spawn; production randomness
    // must not make one collider case a different workload.
    for (i, mut v) in w
        .query_filtered::<&mut Villager, With<CrowdMember>>()
        .iter_mut(w)
        .enumerate()
    {
        v.move_direction = if i % 2 == 0 { 1. } else { -1. };
    }
}
pub fn run(w: &mut World) {
    if !w.contains_resource::<Probe>() {
        w.init_resource::<Probe>();
    }
    let dense = std::env::args().any(|a| a == "--crowd-dense");
    let count = if dense {
        std::env::args()
            .find_map(|a| {
                a.strip_prefix("--crowd-count=")
                    .and_then(|v| v.parse::<usize>().ok())
            })
            .unwrap_or(370)
            .clamp(1, 5000)
    } else {
        100
    };
    let (_, _, half_x, half_y) = large_layout(count);
    let now = w.resource::<Time<Real>>().elapsed_secs();
    let step = w.resource::<Probe>().step;
    let since = w.resource::<Probe>().since;
    assert!(now < 100., "crowd timeout");
    if let Some(mut events) =
        w.get_resource_mut::<crate::systems::random_events::RandomEventTimer>()
    {
        events.timer.reset();
    }
    {
        let mut weather = w.resource_mut::<crate::components::weather::ActiveWeather>();
        weather.weather_type = crate::components::weather::WeatherType::Clear;
        weather.intensity = 0.;
        weather.duration_remaining = 999.;
    }
    if step > 0 {
        let p = w.resource::<Probe>().anchor;
        for (mut pos, mut v) in w
            .query_filtered::<(&mut Position, &mut LinearVelocity), With<Hero>>()
            .iter_mut(w)
        {
            pos.0 = p;
            v.0 = Vec3::ZERO;
        }
        let old: Vec<_> = w
            .query_filtered::<Entity, Or<(With<Enemy>, (With<Villager>, Without<CrowdMember>))>>()
            .iter(w)
            .collect();
        for e in old {
            w.despawn(e);
        }
    }
    if matches!(step, 1 | 3 | 5 | 7) && now > since + 5. {
        let dt = w.resource::<Time<Real>>().delta_secs() * 1000.;
        w.resource_mut::<Probe>().frames.push(dt);
        if let Some(d) = w.get_resource::<avian3d::collision::CollisionDiagnostics>() {
            let ms = d.narrow_phase.as_secs_f32() * 1000.;
            w.resource_mut::<Probe>().narrow.push(ms);
        }
        if let Some(d) = w.get_resource::<avian3d::dynamics::solver::SolverDiagnostics>() {
            let ms = (d.prepare_constraints
                + d.solve_constraints
                + d.relax_velocities
                + d.apply_restitution)
                .as_secs_f32()
                * 1000.;
            w.resource_mut::<Probe>().solver.push(ms);
        }
    }
    match step {
        0 if now > 8. => {
            let full = std::env::args().any(|a| a == "--crowd-fullsize");
            w.query::<&mut Window>()
                .single_mut(w)
                .unwrap()
                .resolution
                .set(
                    if full { 2560. } else { 1360. },
                    if full { 1440. } else { 900. },
                );
            {
                let mut g = w.resource_mut::<GraphicsSettings>();
                g.ray_tracing = false;
                g.frame_generation = false;
                g.dlss = false;
                g.fog = false;
                g.set_quality_step(0);
            }
            {
                let mut s = w.resource_mut::<GameSettings>();
                s.fps_limit = 0;
                s.vsync_enabled = false;
                s.music_volume = 0.;
                s.effects_volume = 0.;
            }
            let p = w
                .query_filtered::<&Transform, With<Hero>>()
                .single(w)
                .unwrap()
                .translation;
            w.resource_mut::<Probe>().anchor = p;
            let extent = if count > 370 {
                half_x.max(half_y) as f32 * 64. + 128.
            } else {
                800.
            };
            let old: Vec<_> = w
                .query::<(Entity, &Transform, &Block)>()
                .iter(w)
                .filter(|(_, t, _)| (t.translation.xy() - p.xy()).abs().max_element() < extent)
                .map(|(e, _, _)| e)
                .collect();
            for e in old {
                w.despawn(e);
            }
            let mesh = w.resource::<MeshCache>().block_mesh.clone();
            let mat = w.resource::<MaterialCache>().materials[&BlockType::Stone].clone();
            let (left, right, bottom, top) = if count > 370 {
                (-half_x, half_x, -half_y, half_y)
            } else {
                (-10, 10, -6, 8)
            };
            if count > 370 {
                // Fit the larger physical arena without reducing actor size or simulation.
                // This is an explicitly zoomed-out scale test, not the normal game camera.
                let height = (half_y as f32 * 128. + 320.)
                    .max(half_x as f32 * 128. / (2560. / 1440.) + 320.);
                if let Projection::Perspective(ref mut projection) = *w
                    .query_filtered::<&mut Projection, With<Camera3d>>()
                    .single_mut(w)
                    .unwrap()
                {
                    projection.fov = 2. * (height / (2. * 1460.)).atan();
                }
                eprintln!(
                    "[CROWD CAMERA] count={count} zoomed_out=true arena={}x{} blocks",
                    right - left,
                    top - bottom
                );
            }
            for x in left..=right {
                for y in bottom..=top {
                    if y != bottom && y != top && x != left && x != right {
                        continue;
                    }
                    w.spawn((
                        Block {
                            health: 1000000,
                            max_health: 1000000,
                        },
                        BlockType::Stone,
                        crate::components::chunk::ChunkMember {
                            chunk_x: (p.x / 1024.).floor() as i32,
                            chunk_y: (p.y / 1024.).floor() as i32,
                        },
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_translation(
                            p + Vec3::new(x as f32 * 64., y as f32 * 64., -40.),
                        )
                        .with_scale(Vec3::splat(64.)),
                    ));
                }
            }
            spawn(w, 10, p);
        }
        1 | 3 | 5 | 7 if now > since + 13. => {
            let mut frames = std::mem::take(&mut w.resource_mut::<Probe>().frames);
            frames.sort_by(f32::total_cmp);
            let n = w.query::<&Villager>().iter(w).count();
            assert_eq!(n, if step == 1 || step == 7 { 10 } else { count });
            let points: Vec<_> = w
                .query_filtered::<&GlobalTransform, With<Villager>>()
                .iter(w)
                .map(|t| t.translation())
                .collect();
            let (camera, ct) = w
                .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>()
                .single(w)
                .unwrap();
            let size = camera.logical_viewport_size().unwrap();
            let visible = points
                .iter()
                .filter(|p| {
                    camera.world_to_viewport(ct, **p).is_ok_and(|q| {
                        q.cmpge(Vec2::splat(24.)).all() && q.cmple(size - Vec2::splat(24.)).all()
                    })
                })
                .count();
            assert_eq!(visible, n, "Benchmark requires the full crowd on screen");
            eprintln!(
                "[CROWD PROFILE] mode={step} villagers={n} visible={visible} frames={} median_ms={:.3} p95_ms={:.3} meshes={} materials={}",
                frames.len(),
                frames[frames.len() / 2],
                frames[frames.len() * 95 / 100],
                w.resource::<Assets<Mesh>>().len(),
                w.resource::<Assets<StandardMaterial>>().len()
            );
            let rows = w
                .query::<&crate::components::heart_ui::HeartUI>()
                .iter(w)
                .count();
            let labels = w
                .query::<&crate::components::username_label::UsernameLabel>()
                .iter(w)
                .count();
            eprintln!("[CROWD ENTITIES] mode={step} heart_rows={rows} username_labels={labels}");
            assert!(
                rows >= n && rows <= n + 1,
                "Crowd health rows should track owners without per-heart entities"
            );
            w.spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!(
                    "target/visual-crowd-{n}-mode{step}.png"
                )));
            for label in ["narrow", "solver"] {
                let mut values = if label == "narrow" {
                    std::mem::take(&mut w.resource_mut::<Probe>().narrow)
                } else {
                    std::mem::take(&mut w.resource_mut::<Probe>().solver)
                };
                values.sort_by(f32::total_cmp);
                if !values.is_empty() {
                    eprintln!(
                        "[CROWD CPU] mode={step} {label}_median_ms={:.3} p95_ms={:.3}",
                        values[values.len() / 2],
                        values[values.len() * 95 / 100]
                    );
                }
            }
            if !dense && step >= 5 {
                assert_eq!(
                    w.resource::<crate::systems::native_gpu::NativeStatus>()
                        .mode,
                    2
                );
            }
        }
        2 if now > since + 0.5 => {
            let p = w.resource::<Probe>().anchor;
            spawn(w, count, p);
        }
        4 if now > since + 0.5 => {
            if dense {
                let p = w.resource::<Probe>().anchor;
                spawn(w, count, p);
            } else {
                w.resource_mut::<GraphicsSettings>().ray_tracing = true;
            }
        }
        6 if now > since + 0.5 => {
            let p = w.resource::<Probe>().anchor;
            spawn(w, 10, p);
        }
        8 if now > since + 1. => {
            eprintln!("[CROWD TEST] completed");
            w.write_message(AppExit::Success);
        }
        _ => return,
    }
    let mut p = w.resource_mut::<Probe>();
    p.step += 1;
    p.since = now;
}
