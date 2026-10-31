//! `--visual-smoke-test --ghost-floor-test`: hunts invisible floors. Fires
//! powers next to the heads for a minute and checks every body that rests:
//! it must stand on a block that is drawn. Sizes come from each mesh's
//! bounding box. Prints `[GHOST]` lines and fails on a body resting on air or
//! on an undrawn block.
use crate::components::block::{Block, BlockType};
use crate::components::enemy::{Enemy, Flying};
use crate::components::hero::Hero;
use crate::components::resource::Resource as Drop;
use crate::components::villager::Villager;
use crate::resources::command_queue::{PowerCommand, PowerCommandType as P};
use crate::systems::terrain_cracks::Cracked;
use avian3d::prelude::LinearVelocity;
use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use std::collections::HashMap;

const POWERS: [P; 6] = [
    P::BlackHole,
    P::IceAge,
    P::Bolt,
    P::Quake,
    P::BlackHole,
    P::Wind,
];
const RUN_FOR: f32 = 75.0;
/// Seconds a body must stay put before it counts as resting.
const REST: f32 = 1.5;

#[derive(Resource, Default)]
struct Probe {
    started: Option<f32>,
    next_power: f32,
    fired: usize,
    /// Where and since when each body has been still.
    still: HashMap<Entity, (Vec3, f32)>,
    reported: Vec<Entity>,
    problems: Vec<String>,
}

type Body<'a> = (
    Entity,
    &'a Transform,
    &'a LinearVelocity,
    Option<&'a Aabb>,
    Has<Hero>,
    Has<Villager>,
    Option<&'a Enemy>,
    Option<&'a crate::components::enemy::TerrainFootprint>,
    Option<&'a avian3d::prelude::RigidBody>,
    Option<&'a avian3d::prelude::GravityScale>,
    Has<crate::systems::slime_rift::RiftParked>,
    Option<&'a Children>,
    Holds,
);

/// States that hold a body still on purpose.
type Holds = (
    Has<crate::systems::slime_physics::SlimeContact>,
    Has<crate::systems::slime_physics::SlimeCoat>,
    Has<crate::components::snow_slide::SnowSlide>,
    Has<crate::systems::biome_encounters::Stunned>,
    Has<crate::components::frozen::FrozenStacks>,
    Has<crate::components::play_style::WallCling>,
    Has<avian3d::dynamics::rigid_body::sleeping::Sleeping>,
);

type BlockView<'a> = (
    &'a Transform,
    &'a BlockType,
    Option<&'a Aabb>,
    Option<&'a Visibility>,
    Option<&'a InheritedVisibility>,
    Has<Mesh3d>,
    Has<MeshMaterial3d<StandardMaterial>>,
    Has<MeshMaterial3d<Cracked>>,
);

/// World-space centre and half size from a mesh bounding box, if it has one.
fn bounds(t: &Transform, aabb: Option<&Aabb>, fallback: f32) -> (Vec2, Vec2) {
    let scale = t.scale.abs().truncate();
    match aabb {
        Some(a) => (
            t.translation.truncate() + Vec3::from(a.center).truncate() * scale,
            Vec3::from(a.half_extents).truncate() * scale,
        ),
        None => (t.translation.truncate(), Vec2::splat(fallback)),
    }
}

pub fn run(w: &mut World) {
    let now = w.resource::<Time<Real>>().elapsed_secs();
    if !w.contains_resource::<Probe>() {
        w.init_resource::<Probe>();
    }
    if w.resource::<Probe>().started.is_none() {
        let blocks = w.query_filtered::<(), With<Block>>().iter(w).count();
        if now < 8.0 || blocks < 100 {
            return;
        }
        let mut p = w.resource_mut::<Probe>();
        p.started = Some(now);
        p.next_power = now + 2.0;
        eprintln!("[GHOST] hunting invisible floors for {RUN_FOR}s");
    }
    let started = w.resource::<Probe>().started.unwrap();
    if now - started > RUN_FOR {
        let p = w.resource::<Probe>();
        eprintln!(
            "[GHOST] done: {} powers fired, {} problems",
            p.fired,
            p.problems.len()
        );
        let failed = !p.problems.is_empty();
        w.write_message(if failed {
            AppExit::error()
        } else {
            AppExit::Success
        });
        return;
    }
    if now >= w.resource::<Probe>().next_power {
        let mut p = w.resource_mut::<Probe>();
        let power = POWERS[p.fired % POWERS.len()];
        p.fired += 1;
        p.next_power = now + 6.0;
        eprintln!("[GHOST] firing {power:?}");
        if let Some(queue) = crate::plugins::external_api_plugin::get_command_queue() {
            queue
                .lock()
                .unwrap()
                .push_back(PowerCommand::new(power, "ghost probe".into()));
        }
    }
    let blocks: Vec<(Vec2, Vec2, BlockType, bool, String)> = w
        .query_filtered::<BlockView, With<Block>>()
        .iter(w)
        .map(|(t, kind, aabb, vis, inherited, mesh, standard, cracked)| {
            let (center, half) = bounds(t, aabb, 32.0);
            let drawn = vis.is_none_or(|v| *v != Visibility::Hidden)
                && inherited.is_none_or(|i| i.get())
                && mesh
                && (standard || cracked)
                && half.min_element() > 2.0;
            let why = format!(
                "visibility={vis:?} inherited={:?} mesh={mesh} material={} size={:.0}x{:.0}",
                inherited.map(|i| i.get()),
                standard || cracked,
                half.x * 2.0,
                half.y * 2.0
            );
            (center, half, *kind, drawn, why)
        })
        .collect();
    let mut bodies: Vec<(Entity, Vec2, Vec2, String)> = w
        .query_filtered::<Body, (Without<Flying>, Or<(With<Hero>, With<Villager>, With<Enemy>)>)>()
        .iter(w)
        .filter(|(_, _, v, ..)| v.0.y.abs() < 3.0)
        .map(|(e, t, _, aabb, hero, villager, enemy, footprint, body, gravity, parked, children, holds)| {
            let (center, half) = bounds(t, aabb, 30.0);
            let kind = if hero {
                "hero"
            } else if villager {
                "villager"
            } else {
                "enemy"
            };
            let solver_half = t.scale.truncate().abs() * footprint.map_or(Vec2::splat(32.0), |f| f.0);
            let kind = format!(
                "{kind}(type {:?}, body {:?}, gravity {:?}, parked {parked}, scale {:.2}, footprint {:?}, mesh half {:.0}x{:.0}, solver half {:.0}x{:.0}, children {}, holds slime-stick/slime-coat/snow-slide/stunned/frozen/cling/SLEEPING {:?})",
                enemy.map(|e| e.enemy_type),
                body,
                gravity.map(|g| g.0),
                t.scale.y,
                footprint.map(|f| f.0),
                half.x,
                half.y,
                solver_half.x,
                solver_half.y,
                children.map_or(0, |c| c.len()),
                holds
            );
            (e, center, half, kind)
        })
        .collect();
    bodies.extend(
        w.query_filtered::<(Entity, &Transform, &LinearVelocity, Option<&Aabb>), With<Drop>>()
            .iter(w)
            .filter(|(_, _, v, _)| v.0.y.abs() < 3.0)
            .map(|(e, t, _, aabb)| {
                let (center, half) = bounds(t, aabb, 16.0);
                (e, center, half, "drop".to_string())
            }),
    );
    // Only bodies over loaded terrain count: far ones wait for their chunk on purpose.
    let loaded: HashMap<Entity, bool> = {
        let chunks = w.resource::<crate::components::chunk::LoadedChunks>();
        bodies
            .iter()
            .map(|(e, c, h, _)| {
                let below = *c - Vec2::Y * (h.y + 32.0);
                let (x, y) =
                    crate::components::chunk::world_to_chunk_coords(below.x, below.y, 64.0);
                (*e, chunks.is_loaded(x, y))
            })
            .collect()
    };
    // Water the fluid simulation has at each body (it floats bodies even where none is drawn).
    let water: HashMap<Entity, (bool, bool)> = {
        let fluids = w.get_resource::<crate::systems::fluids::FluidWorld>();
        bodies
            .iter()
            .map(|(e, c, h, _)| {
                let at = fluids.is_some_and(|f| f.is_water_at(*c));
                let under = fluids.is_some_and(|f| f.covers_surface(*c - Vec2::Y * (h.y + 8.0)));
                (*e, (at, under))
            })
            .collect()
    };
    // Physics colliders that are not terrain blocks, with their bounding boxes.
    let colliders: Vec<(Entity, Vec2, Vec2, String)> = w
        .query_filtered::<(
            Entity,
            &avian3d::prelude::ColliderAabb,
            Option<&avian3d::prelude::RigidBody>,
            Option<&Name>,
            Has<avian3d::prelude::Sensor>,
            Has<Hero>,
            Has<Villager>,
            Option<&Enemy>,
        ), Without<Block>>()
        .iter(w)
        .map(|(e, aabb, body, name, sensor, hero, villager, enemy)| {
            let (min, max) = (aabb.min.truncate(), aabb.max.truncate());
            let what = format!(
                "body {body:?} name {:?} sensor {sensor} hero {hero} villager {villager} enemy {:?} size {:.0}x{:.0}",
                name.map(|n| n.as_str()),
                enemy.map(|e| e.enemy_type),
                max.x - min.x,
                max.y - min.y
            );
            (e, min, max, what)
        })
        .collect();
    let mut p = w.resource_mut::<Probe>();
    let moving: Vec<Entity> = p
        .still
        .keys()
        .filter(|e| !bodies.iter().any(|b| b.0 == **e))
        .copied()
        .collect();
    for e in moving {
        p.still.remove(&e);
    }
    for (entity, center, half, kind) in bodies {
        let at = center.extend(0.0);
        let (seen, since) = *p.still.entry(entity).or_insert((at, now));
        if (seen.y - at.y).abs() > 2.0 || (seen.x - at.x).abs() > 900.0 {
            p.still.insert(entity, (at, now));
            continue;
        }
        if now - since < REST || p.reported.contains(&entity) {
            continue;
        }
        if !loaded.get(&entity).copied().unwrap_or(false) {
            continue;
        }
        let feet = center.y - half.y;
        let under: Vec<_> = blocks
            .iter()
            .filter(|(b, bh, ..)| {
                (b.x - center.x).abs() < bh.x + half.x - 2.0 && (b.y + bh.y - feet).abs() < 12.0
            })
            .collect();
        let line = if under.is_empty() {
            let near: Vec<String> = blocks
                .iter()
                .filter(|(b, ..)| b.distance(center) < 110.0)
                .map(|(b, bh, k, d, _)| {
                    format!(
                        "{k:?}@({:.0},{:.0}) top {:.0}{}",
                        b.x,
                        b.y,
                        b.y + bh.y,
                        if *d { "" } else { " UNDRAWN" }
                    )
                })
                .collect();
            Some(format!(
                "{kind} {entity:?} rests in mid-air at ({:.0},{:.0}), feet {feet:.0}; water at/under {:?}; near: {}",
                center.x,
                center.y,
                water.get(&entity),
                near.join(", ")
            ))
        } else if under.iter().all(|u| !u.3) {
            let (b, _, k, _, why) = under[0];
            Some(format!(
                "{kind} {entity:?} at ({:.0},{:.0}) rests on undrawn {k:?}@({:.0},{:.0}) {why}",
                center.x, center.y, b.x, b.y
            ))
        } else {
            None
        };
        let line = line.map(|l| {
            if !l.contains("mid-air") {
                return l;
            }
            let feet = center.y - half.y;
            let under: Vec<String> = colliders
                .iter()
                .filter(|(e, min, max, _)| {
                    *e != entity
                        && min.x <= center.x + half.x
                        && max.x >= center.x - half.x
                        && max.y >= feet - 10.0
                        && min.y <= feet + 4.0
                })
                .map(|(e, min, max, what)| {
                    format!(
                        "{e:?} box ({:.0},{:.0})-({:.0},{:.0}) {what}",
                        min.x, min.y, max.x, max.y
                    )
                })
                .collect();
            format!("{l}; colliders under feet: {}", under.join(" | "))
        });
        if let Some(line) = line {
            eprintln!("[GHOST] {line}");
            p.reported.push(entity);
            p.problems.push(line);
        }
    }
}
