//! Separate-process checks for broad mist, cap-shaped snow and live audio gain.
use crate::{
    components::{
        block::{BiomeType, Block, BlockType},
        chunk::ChunkMember,
        enemy::Enemy,
        hero::Hero,
        snow_layer::SnowLayer,
        weather::{ActiveWeather, WeatherType},
    },
    resources::{
        graphics_settings::GraphicsSettings, material_cache::MaterialCache, mesh_cache::MeshCache,
        settings::GameSettings,
    },
    states::GameState,
    systems::{audio::effects, biome_mist, biome_teleport::BiomeTeleport},
};
use avian3d::prelude::*;
use bevy::{
    audio::Volume,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
#[derive(Resource, Default)]
struct Probe {
    step: u8,
    since: f32,
    anchor: Option<Vec3>,
    mushrooms: Vec<Entity>,
    audio: Option<Entity>,
    volume: f32,
}
fn capture(w: &mut World, name: &str) {
    w.spawn(Screenshot::primary_window())
        .observe(save_to_disk(format!("target/visual-polish-{name}.png")));
}
pub fn run(w: &mut World) {
    if !w.contains_resource::<Probe>() {
        w.init_resource::<Probe>();
    }
    let now = w.resource::<Time<Real>>().elapsed_secs();
    let step = w.resource::<Probe>().step;
    let since = w.resource::<Probe>().since;
    assert!(now < 100., "Polish probe timeout at {step}");
    if let Some(p) = w.resource::<Probe>().anchor {
        for (mut pos, mut v) in w
            .query_filtered::<(&mut Position, &mut LinearVelocity), With<Hero>>()
            .iter_mut(w)
        {
            pos.0 = p;
            v.0 = Vec3::ZERO;
        }
        let remove: Vec<_> = w.query_filtered::<Entity, With<Enemy>>().iter(w).collect();
        for e in remove {
            w.despawn(e);
        }
    }
    match step {
        0 if now > 7. => {
            w.query::<&mut Window>()
                .single_mut(w)
                .unwrap()
                .resolution
                .set(1360., 900.);
            let mut g = w.resource_mut::<GraphicsSettings>();
            g.ray_tracing = false;
            g.frame_generation = false;
            g.dlss = false;
            g.fog = true;
            let y = crate::utils::biome_utils::nearest_biome_y(0., BiomeType::Forest);
            w.resource_mut::<BiomeTeleport>().target = Some(Vec3::new(0., y, 90.));
            let mut weather = w.resource_mut::<ActiveWeather>();
            weather.weather_type = WeatherType::Clear;
            weather.intensity = 0.;
            weather.duration_remaining = 999.;
            w.resource_mut::<Probe>().volume = w.resource::<GameSettings>().effects_volume;
            w.resource_mut::<GameSettings>().effects_volume = 0.;
            let h = w
                .resource::<crate::systems::rocket_audio::RocketSounds>()
                .engine
                .clone();
            let e = w
                .spawn((
                    AudioPlayer::new(h),
                    PlaybackSettings::LOOP.with_volume(Volume::Linear(0.4)),
                ))
                .id();
            w.resource_mut::<Probe>().audio = Some(e);
        }
        1 if now > since + 7. => {
            effects::verify_live(w, 0.);
            w.resource_mut::<Probe>().anchor =
                Some(biome_mist::smoke_focus(w).expect("Natural forest bank"));
        }
        2 if now > since + 4. => {
            biome_mist::smoke_verify(w);
            capture(w, "forest-wide");
            w.resource_mut::<GameSettings>().effects_volume = 0.25;
            w.resource_mut::<NextState<GameState>>()
                .set(GameState::Settings);
        }
        3 if now > since + 1. => {
            effects::verify_live(w, 0.25);
            capture(w, "effects-settings");
            w.resource_mut::<GameSettings>().effects_volume = 0.8;
        }
        4 if now > since + 1. => {
            effects::verify_live(w, 0.8);
            w.resource_mut::<NextState<GameState>>()
                .set(GameState::Playing);
            w.resource_mut::<Probe>().anchor.as_mut().unwrap().x += 128.;
        }
        5 if now > since + 3. => {
            capture(w, "forest-moved");
            let p = w.resource::<Probe>().anchor.unwrap();
            let p = (p.xy() / 64.).round().extend(0.) * 64.;
            let old: Vec<_> = w
                .query::<(Entity, &Transform, &Block)>()
                .iter(w)
                .filter(|(_, t, _)| (t.translation.xy() - p.xy()).abs().max_element() < 450.)
                .map(|(e, _, _)| e)
                .collect();
            for e in old {
                w.despawn(e);
            }
            let block = w.resource::<MeshCache>().block_mesh.clone();
            let mushroom = w.resource::<MeshCache>().mushroom_mesh.clone();
            let ground = w.resource::<MaterialCache>().materials[&BlockType::Dirt].clone();
            let mat = w.resource::<MaterialCache>().materials[&BlockType::Mushroom].clone();
            let chunk = (p.xy() / 1024.).floor().as_ivec2();
            let member = ChunkMember {
                chunk_x: chunk.x,
                chunk_y: chunk.y,
            };
            for x in -6..=6 {
                w.spawn((
                    Block {
                        health: 100,
                        max_health: 100,
                    },
                    BlockType::Dirt,
                    ChunkMember {
                        chunk_x: chunk.x,
                        chunk_y: chunk.y,
                    },
                    Mesh3d(block.clone()),
                    MeshMaterial3d(ground.clone()),
                    Transform::from_translation(p + Vec3::new(x as f32 * 64., -192., -10.))
                        .with_scale(Vec3::splat(64.)),
                ));
            }
            let _ = member;
            for (x, stage) in [(-192., 1), (0., 2), (192., 4)] {
                let e = w
                    .spawn((
                        Block {
                            health: 100,
                            max_health: 100,
                        },
                        BlockType::Mushroom,
                        ChunkMember {
                            chunk_x: chunk.x,
                            chunk_y: chunk.y,
                        },
                        Mesh3d(mushroom.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_translation(p + Vec3::new(x, -128., -10.)),
                        SnowLayer { stage },
                    ))
                    .id();
                w.resource_mut::<Probe>().mushrooms.push(e);
            }
            let mut weather = w.resource_mut::<ActiveWeather>();
            weather.weather_type = WeatherType::Snow;
            weather.intensity = 0.7;
            weather.duration_remaining = 999.;
        }
        6 if now > since + 2. => {
            capture(w, "mushroom-snow");
        }
        7 if now > since + 3. => {
            capture(w, "mushroom-snow-thick");
            let list = w.resource::<Probe>().mushrooms.clone();
            for e in list {
                w.entity_mut(e).remove::<SnowLayer>();
            }
            let mut weather = w.resource_mut::<ActiveWeather>();
            weather.weather_type = WeatherType::Clear;
            weather.intensity = 0.;
        }
        8 if now > since + 2. => {
            capture(w, "mushroom-clear");
            w.resource_mut::<GameSettings>().effects_volume = 0.;
        }
        9 if now > since + 1. => {
            effects::verify_live(w, 0.);
            if let Some(e) = w.resource_mut::<Probe>().audio.take() {
                w.despawn(e);
            }
            w.resource_mut::<GameSettings>().effects_volume = w.resource::<Probe>().volume;
            let hero = w.query_filtered::<Entity, With<Hero>>().single(w).unwrap();
            crate::systems::buffs::buff_core::activate_growth(&mut w.commands(), hero);
            w.flush();
            w.entity_mut(hero)
                .insert(crate::components::buff::Buff::new(
                    crate::components::buff::BuffType::Burning,
                    2.,
                ));
            let old: Vec<_> = w
                .query_filtered::<Entity, With<crate::components::villager::Villager>>()
                .iter(w)
                .collect();
            for e in old {
                w.despawn(e);
            }
            let p = w.resource::<Probe>().anchor.unwrap();
            let p = (p.xy() / 64.).round().as_ivec2();
            let mesh = w.resource::<MeshCache>().block_mesh.clone();
            let material = w.resource::<MaterialCache>().materials[&BlockType::Stone].clone();
            for x in [-5, -1] {
                let c = p + IVec2::new(x, -2);
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
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation((c.as_vec2() * 64.).extend(-10.))
                        .with_scale(Vec3::splat(64.)),
                ));
            }
            let mushrooms = w.resource::<Probe>().mushrooms.clone();
            for e in mushrooms {
                if w.get::<Transform>(e)
                    .is_some_and(|t| (t.translation.x - (p.x - 3) as f32 * 64.).abs() < 1.)
                {
                    w.despawn(e);
                }
            }
            {
                let mut water = w.resource_mut::<crate::systems::fluids::FluidWorld>();
                for x in -4..=-2 {
                    water.deposit((p.x + x, p.y - 2), crate::systems::fluids::Liquid::Water, 8);
                }
            }
            let mut weather = w.resource_mut::<ActiveWeather>();
            weather.weather_type = WeatherType::Snow;
            weather.intensity = 1.;
            weather.duration_remaining = 999.;
        }
        10 if now > since + 2. => {
            let p = w.resource::<Probe>().anchor.unwrap();
            let p = (p.xy() / 64.).round().as_ivec2();
            let under: Vec<_> = w
                .query::<(&Transform, Option<&SnowLayer>, &Block)>()
                .iter(w)
                .filter(|(t, _, _)| {
                    let c = (t.translation.xy() / 64.).round().as_ivec2();
                    c.y == p.y - 3 && (p.x - 4..=p.x - 2).contains(&c.x)
                })
                .collect();
            assert_eq!(under.len(), 3);
            assert!(
                under.iter().all(|(_, snow, _)| snow.is_none()),
                "No snow cap beneath the pool"
            );
            assert!(
                w.query_filtered::<&crate::components::buff::GrowthPower, With<Hero>>()
                    .single(w)
                    .is_ok()
            );
            capture(w, "pool-water-grow");
        }
        11 if now > since + 15. => {
            let p = w.resource::<Probe>().anchor.unwrap();
            let p = (p.xy() / 64.).round().as_ivec2();
            let ice = w
                .query::<(&Transform, &BlockType)>()
                .iter(w)
                .filter(|(t, k)| {
                    let c = (t.translation.xy() / 64.).round().as_ivec2();
                    **k == BlockType::Ice && c.y == p.y - 2 && (p.x - 4..=p.x - 2).contains(&c.x)
                })
                .count();
            assert_eq!(
                ice, 3,
                "Settled pool freezes at the surface after sustained snow"
            );
            assert!(
                w.query_filtered::<&crate::components::buff::GrowthPower, With<Hero>>()
                    .single(w)
                    .is_ok()
            );
            capture(w, "pool-ice-grow");
        }
        12 if now > since + 1. => {
            eprintln!(
                "[POLISH TEST] completed: mist, snow caps, submerged cap removal, delayed surface ice, independent Grow and live audio"
            );
            w.write_message(AppExit::Success);
        }
        _ => return,
    }
    let mut p = w.resource_mut::<Probe>();
    p.step += 1;
    p.since = now;
}
