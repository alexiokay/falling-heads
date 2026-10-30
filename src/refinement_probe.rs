//! Opt-in hidden renderer check for fog persistence, actual Fire Rain UI and spiders.
use crate::{
    components::{
        block::{BiomeType, Block, BlockType},
        buff::Buffable,
        bundles::avian_physics,
        dev_panel::WeatherControlButton,
        enemy::{Enemy, EnemyType, Health},
        hero::Hero,
        weather::{ActiveWeather, WeatherType},
    },
    resources::{
        graphics_settings::GraphicsSettings, material_cache::MaterialCache, mesh_cache::MeshCache,
    },
    systems::{biome_mist, biome_teleport::BiomeTeleport},
};
use avian3d::prelude::*;
use bevy::{
    picking::hover::PickingInteraction,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
#[derive(Resource, Default)]
struct Probe {
    step: u8,
    since: f32,
    anchor: Option<Vec3>,
    spider: Option<Entity>,
}
fn capture(world: &mut World, name: &str) {
    world
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(format!("target/visual-refinement-{name}.png")));
}
pub fn run(world: &mut World) {
    if !world.contains_resource::<Probe>() {
        world.init_resource::<Probe>();
    }
    let time = world.resource::<Time<Real>>().elapsed_secs();
    let step = world.resource::<Probe>().step;
    let since = world.resource::<Probe>().since;
    assert!(time < 100., "Refinement probe timeout stage {step}");
    if let Some(spider) = world.resource::<Probe>().spider {
        let others:Vec<_>=world.query_filtered::<Entity,Or<(With<Enemy>,With<crate::components::villager::Villager>)>>().iter(world).filter(|e|*e!=spider).collect();
        for e in others {
            world.despawn(e);
        }
        if let Some(mut hp) = world.get_mut::<Health>(spider) {
            hp.current = hp.max;
        }
    }
    if let Some(p) = world.resource::<Probe>().anchor {
        for (mut t, mut pos, mut v) in world
            .query_filtered::<(&mut Transform, &mut Position, &mut LinearVelocity), With<Hero>>()
            .iter_mut(world)
        {
            t.translation = p;
            pos.0 = p;
            v.0 = Vec3::ZERO;
        }
    }
    match step {
        0 if time > 6. => {
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(1360., 900.);
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.frame_generation = false;
            g.ray_tracing = false;
            g.dlss = false;
            g.fog = true;
            let y = crate::utils::biome_utils::nearest_biome_y(0., BiomeType::Forest);
            world.resource_mut::<BiomeTeleport>().target = Some(Vec3::new(0., y, 90.));
            let mut w = world.resource_mut::<ActiveWeather>();
            w.weather_type = WeatherType::Clear;
            w.intensity = 0.;
            w.duration_remaining = 999.;
        }
        1 if time > since + 7. => {
            world.resource_mut::<Probe>().anchor =
                Some(biome_mist::smoke_focus(world).expect("Natural forest mist bank"));
        }
        2 if time > since + 5. => {
            biome_mist::smoke_verify(world);
            capture(world, "forest-mist");
        }
        3 if time > since + 2. => {
            world.resource_mut::<Probe>().anchor.as_mut().unwrap().x += 64.;
        }
        4 if time > since + 3. => {
            biome_mist::smoke_verify(world);
            capture(world, "forest-mist-moved");
            let button = world
                .query::<(Entity, &WeatherControlButton)>()
                .iter(world)
                .find(|(_, b)| b.weather_type == WeatherType::FireRain)
                .unwrap()
                .0;
            world.entity_mut(button).insert(PickingInteraction::Pressed);
        }
        5 if time > since + 5. => {
            assert_eq!(
                world.resource::<ActiveWeather>().weather_type,
                WeatherType::FireRain
            );
            let count = world
                .query::<&crate::components::weather_particle::WeatherParticle>()
                .iter(world)
                .count();
            assert!(count > 20);
            eprintln!("[REFINEMENT TEST] Fire Rain active, {count} live particles");
            capture(world, "fire-rain");
        }
        6 if time > since + 2. => {
            world.resource_mut::<Probe>().anchor = None;
            let mut w = world.resource_mut::<ActiveWeather>();
            w.weather_type = WeatherType::Clear;
            w.intensity = 0.;
            w.duration_remaining = 999.;
            let y = crate::utils::biome_utils::nearest_biome_y(0., BiomeType::Underground);
            world.resource_mut::<BiomeTeleport>().target = Some(Vec3::new(0., y, 90.));
        }
        7 if time > since + 7. => {
            let count = world
                .query::<&crate::systems::spiders::Spider>()
                .iter(world)
                .count();
            eprintln!("[REFINEMENT TEST] Natural underground spiders: {count}");
            let p = world
                .query_filtered::<&Transform, With<Hero>>()
                .single(world)
                .unwrap()
                .translation;
            let p = (p.xy() / 64.).round().extend(0.) * 64. + Vec3::Z * 90.;
            world.resource_mut::<Probe>().anchor = Some(p);
            let old: Vec<_> = world
                .query::<(Entity, &Transform, Option<&Block>, Option<&Enemy>)>()
                .iter(world)
                .filter(|(_, t, b, e)| {
                    (b.is_some() || e.is_some()) && t.translation.xy().distance(p.xy()) < 550.
                })
                .map(|(e, _, _, _)| e)
                .collect();
            for e in old {
                world.despawn(e);
            }
            let mesh = world.resource::<MeshCache>().block_mesh.clone();
            let mat = world.resource::<MaterialCache>().materials[&BlockType::Stone].clone();
            let (chunk_x, chunk_y) = crate::components::chunk::world_to_chunk_coords(p.x, p.y, 64.);
            for x in -6..=6 {
                world.spawn((
                    Block {
                        health: 100,
                        max_health: 100,
                    },
                    BlockType::Stone,
                    crate::components::chunk::ChunkMember { chunk_x, chunk_y },
                    Transform::from_xyz(p.x + x as f32 * 64., p.y - 64., -10.)
                        .with_scale(Vec3::splat(64.)),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(mat.clone()),
                ));
            }
            let spider = world
                .spawn((
                    Enemy::new(EnemyType::Spider),
                    Health::new(4),
                    Buffable,
                    avian_physics::enemy_with_velocity(Vec3::ZERO),
                    Transform::from_translation(p + Vec3::new(180., 0., -80.)),
                    Visibility::Inherited,
                ))
                .id();
            world.resource_mut::<Probe>().spider = Some(spider);
        }
        8 if time > since + 1.5 => {
            assert!(
                world
                    .query::<&crate::systems::spiders::Spider>()
                    .iter(world)
                    .count()
                    > 0
            );
            capture(world, "spider-windup");
        }
        9 if time > since + 1.8 => {
            capture(world, "spider-pounce-web");
        }
        10 if time > since + 3. => {
            eprintln!(
                "[REFINEMENT TEST] completed: natural mist, motion persistence, Fire Rain button, spider model and attack"
            );
            world.write_message(AppExit::Success);
        }
        _ => return,
    }
    let mut p = world.resource_mut::<Probe>();
    p.step += 1;
    p.since = time;
}
