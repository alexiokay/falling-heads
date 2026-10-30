//! Hidden native comparison: dry, half-submerged and submerged heads of several sizes.
use crate::{
    components::{
        block::{Block, BlockType},
        chunk::ChunkMember,
        enemy::{Enemy, Health},
        hero::Hero,
        villager::Villager,
        weather::{ActiveWeather, WeatherType},
    },
    resources::{
        asset_handles::{EnemyTextures, GameTextures},
        graphics_settings::GraphicsSettings,
        material_cache::MaterialCache,
        mesh_cache::MeshCache,
    },
    systems::fluids::{FluidWorld, Liquid},
};
use avian3d::prelude::*;
use bevy::prelude::*;
#[derive(Resource, Default)]
struct Probe {
    step: u8,
    since: f32,
    anchor: Vec3,
    actors: Vec<Entity>,
}
fn capture(w: &mut World, name: &str) {
    w.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(format!(
            "target/visual-water-{name}.png"
        )));
}
pub fn run(w: &mut World) {
    if !w.contains_resource::<Probe>() {
        w.init_resource::<Probe>();
    }
    let now = w.resource::<Time<Real>>().elapsed_secs();
    let step = w.resource::<Probe>().step;
    let since = w.resource::<Probe>().since;
    assert!(now < 80., "Water test timeout at {step}");
    if step > 0 {
        let anchor = w.resource::<Probe>().anchor;
        let offset = if step == 2 || step == 5 { 160. } else { -160. };
        for (mut p, mut v) in w
            .query_filtered::<(&mut Position, &mut LinearVelocity), With<Hero>>()
            .iter_mut(w)
        {
            p.0 = anchor + Vec3::new(offset, 250., 30.);
            v.0 = Vec3::ZERO;
        }
        let remove: Vec<_> = w.query_filtered::<Entity, With<Enemy>>().iter(w).collect();
        for e in remove {
            w.despawn(e);
        }
        let mut weather = w.resource_mut::<ActiveWeather>();
        weather.weather_type = WeatherType::Clear;
        weather.intensity = 0.;
        weather.duration_remaining = 999.;
    }
    match step {
        0 if now > 8. => {
            let p = w
                .query_filtered::<&Transform, With<Hero>>()
                .single(w)
                .unwrap()
                .translation;
            let cell = (p.xy() / 64.).round().as_ivec2();
            let anchor = (cell.as_vec2() * 64.).extend(0.);
            w.resource_mut::<Probe>().anchor = anchor;
            w.query::<&mut Window>()
                .single_mut(w)
                .unwrap()
                .resolution
                .set(1360., 900.);
            let mut g = w.resource_mut::<GraphicsSettings>();
            g.ray_tracing = false;
            g.dlss = false;
            g.frame_generation = false;
            g.fog = false;
            let remove: Vec<_> = w
                .query::<(Entity, &Transform, Option<&Block>, Option<&Villager>)>()
                .iter(w)
                .filter(|(_, t, b, v)| {
                    (b.is_some() || v.is_some()) && t.translation.xy().distance(anchor.xy()) < 1100.
                })
                .map(|(e, _, _, _)| e)
                .collect();
            for e in remove {
                w.despawn(e);
            }
            w.insert_resource(FluidWorld::default());
            let mesh = w.resource::<MeshCache>().block_mesh.clone();
            let mat = w.resource::<MaterialCache>().materials[&BlockType::Stone].clone();
            for x in -7..=7 {
                for y in -4..=0 {
                    if y != -4 && x != -7 && x != 7 {
                        continue;
                    }
                    let c = cell + IVec2::new(x, y);
                    let chunk = c.div_euclid(IVec2::splat(16));
                    w.spawn((
                        Block {
                            health: 100,
                            max_health: 100,
                        },
                        BlockType::Stone,
                        ChunkMember {
                            chunk_x: chunk.x,
                            chunk_y: chunk.y,
                        },
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_translation((c.as_vec2() * 64.).extend(-10.))
                            .with_scale(Vec3::splat(64.)),
                    ));
                }
            }
            {
                let mut f = w.resource_mut::<FluidWorld>();
                for x in -6..=6 {
                    for y in -3..=0 {
                        f.deposit((cell.x + x, cell.y + y), Liquid::Water, 8);
                    }
                }
            }
            let images = [
                w.resource::<EnemyTextures>().skeleton.clone(),
                w.resource::<EnemyTextures>().skeleton2.clone(),
                w.resource::<EnemyTextures>().skeleton.clone(),
                w.resource::<GameTextures>().villager.clone(),
                w.resource::<GameTextures>().hero.clone(),
            ];
            let mut actors = Vec::new();
            for y in [160., 32., -128.] {
                for (i, image) in images.iter().enumerate() {
                    let size = [64., 64., 32., 64., 96.][i];
                    let mesh = w
                        .resource_mut::<Assets<Mesh>>()
                        .add(Rectangle::new(size, size));
                    let mat = w
                        .resource_mut::<Assets<StandardMaterial>>()
                        .add(StandardMaterial {
                            base_color_texture: Some(image.clone()),
                            unlit: true,
                            alpha_mode: AlphaMode::Mask(0.5),
                            ..default()
                        });
                    actors.push(
                        w.spawn((
                            Villager {
                                ai_timer: Timer::default(),
                                direction_change_timer: Timer::default(),
                                move_direction: 0.,
                            },
                            Health::new(3),
                            Mesh3d(mesh),
                            MeshMaterial3d(mat),
                            Transform::from_translation(
                                anchor + Vec3::new((i as f32 - 2.) * 144., y, 30.),
                            ),
                        ))
                        .id(),
                    );
                }
            }
            w.resource_mut::<Probe>().actors = actors;
        }
        1 if now > since + 4. => {
            capture(w, "raster-left");
        }
        2 if now > since + 4. => {
            capture(w, "raster-right");
            w.resource_mut::<GraphicsSettings>().dlss = true;
        }
        3 if now > since + 4. => {
            assert_eq!(
                w.resource::<crate::systems::native_gpu::NativeStatus>()
                    .mode,
                1
            );
            capture(w, "dlss");
            w.resource_mut::<GraphicsSettings>().ray_tracing = true;
        }
        4 if now > since + 5. => {
            assert_eq!(
                w.resource::<crate::systems::native_gpu::NativeStatus>()
                    .mode,
                2
            );
            capture(w, "rt-left");
        }
        5 if now > since + 4. => {
            capture(w, "rt-right");
        }
        6 if now > since + 2. => {
            assert_eq!(
                w.resource::<Probe>()
                    .actors
                    .iter()
                    .filter(|e| w.get_entity(**e).is_ok())
                    .count(),
                15
            );
            eprintln!(
                "[WATER TEST] completed: 15 dry/partial/submerged faces, three sizes, two camera positions, raster/DLSS/RT"
            );
            w.write_message(AppExit::Success);
        }
        _ => return,
    }
    let mut p = w.resource_mut::<Probe>();
    p.step += 1;
    p.since = now;
}
