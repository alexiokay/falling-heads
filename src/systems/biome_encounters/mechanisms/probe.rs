//! Opt-in native gate, piston and contraption fixture; never runs in normal play.
use super::*;
#[derive(Resource, Default)]
struct Probe {
    step: u8,
    since: f32,
    gate: Option<Entity>,
    piston: Option<Entity>,
    anchor: Vec3,
}
fn capture(world: &mut World, name: &str) {
    world
        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(format!(
            "target/visual-mechanism-{name}.png"
        )));
}
fn place(world: &mut World, pos: Vec3, dynamic: bool) {
    let hero = world
        .query_filtered::<Entity, With<Hero>>()
        .single(world)
        .unwrap();
    world.entity_mut(hero).insert((
        Position(pos),
        Transform::from_translation(pos),
        LinearVelocity::ZERO,
        GravityScale(0.),
        if dynamic {
            RigidBody::Dynamic
        } else {
            RigidBody::Static
        },
    ));
}
/// True once the gate is shut. Any creature within reach opens a gate, so mobs
/// that wandered or spawned near the fixture are cleared while it settles.
fn settled_shut(world: &mut World, gate: Entity, waited: f32) -> bool {
    let (amount, home) = {
        let m = world.get::<Machine>(gate).unwrap();
        (m.amount, m.home)
    };
    if amount < 0.01 {
        return true;
    }
    let near: Vec<_> = world
        .query_filtered::<(Entity, &Transform), (Or<(With<Enemy>, With<Villager>)>, Without<Hero>)>(
        )
        .iter(world)
        .filter(|(_, t)| t.translation.xy().distance(home.xy()) < 500.)
        .map(|(e, _)| e)
        .collect();
    for e in near {
        world.despawn(e);
    }
    assert!(waited < 10., "gate never shut (amount {amount})");
    false
}
pub(crate) fn run(world: &mut World) {
    world.init_resource::<Probe>();
    let now = world.resource::<Time<Real>>().elapsed_secs();
    let p = world.resource::<Probe>();
    let step = p.step;
    let since = p.since;
    let gate = p.gate;
    let piston = p.piston;
    let anchor = p.anchor;
    assert!(now < 90., "Mechanism probe timed out at {step}");
    match step {
        0 if now > 8. => {
            let anchor = world
                .query_filtered::<&Transform, With<Hero>>()
                .single(world)
                .unwrap()
                .translation;
            let origin = (anchor.xy() / 64.).round().as_ivec2();
            let anchor = (origin.as_vec2() * 64.).extend(30.);
            let old: Vec<_> = world
                .query::<(
                    Entity,
                    &Transform,
                    Option<&Block>,
                    Option<&Enemy>,
                    Option<&Villager>,
                    Option<&Machine>,
                )>()
                .iter(world)
                .filter(|(_, t, b, e, v, m)| {
                    (b.is_some() || e.is_some() || v.is_some() || m.is_some())
                        && t.translation.xy().distance(anchor.xy()) < 1200.
                })
                .map(|(e, ..)| e)
                .collect();
            for e in old {
                world.despawn(e);
            }
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(1360., 900.);
            let mut graphics =
                world.resource_mut::<crate::resources::graphics_settings::GraphicsSettings>();
            graphics.ray_tracing = false;
            graphics.dlss = false;
            graphics.frame_generation = false;
            let site = Site {
                origin,
                biome: BiomeType::Mountains,
                seed: 4,
            };
            world.resource_scope(|w, meshes: Mut<MeshCache>| {
                w.resource_scope(|w, mats: Mut<MaterialCache>| {
                    super::super::structures::spawn(&mut w.commands(), site, &meshes, &mats);
                });
            });
            world.flush();
            // Suppress normal encounter instantiation: fixture owns exactly these machines.
            let pending: Vec<_> = world
                .query_filtered::<Entity, With<PendingLandmark>>()
                .iter(world)
                .collect();
            for e in pending {
                world.entity_mut(e).remove::<PendingLandmark>();
            }
            let art = world.resource::<MachineArt>().clone();
            spawn_selected(&mut world.commands(), site, &art, true);
            world.flush();
            let mut gate = None;
            let mut piston = None;
            for (e, m) in world.query::<(Entity, &Machine)>().iter(world) {
                if m.kind == Kind::Gate {
                    gate = Some(e);
                } else {
                    piston = Some(e);
                }
            }
            assert!(
                gate.is_some() && piston.is_some(),
                "Fixture needs both machine types"
            );
            let home = world.get::<Machine>(gate.unwrap()).unwrap().home;
            let mut p = world.resource_mut::<Probe>();
            p.gate = gate;
            p.piston = piston;
            p.anchor = anchor;
            place(world, home + Vec3::new(400., 0., 40.), false);
        }
        1 if now > since + 2. => {
            if !settled_shut(world, gate.unwrap(), now - since) {
                return;
            }
            capture(world, "closed");
            let home = world.get::<Machine>(gate.unwrap()).unwrap().home;
            place(world, home + Vec3::new(180., 0., 40.), false);
        }
        2 if now > since + 1. => {
            assert!(world.get::<Machine>(gate.unwrap()).unwrap().amount > 0.99);
            capture(world, "open");
            let home = world.get::<Machine>(gate.unwrap()).unwrap().home;
            place(world, home + Vec3::Z * 40., false);
        }
        3 if now > since + 3. => {
            assert!(world.get::<Machine>(gate.unwrap()).unwrap().amount > 0.99);
            capture(world, "occupied-safe");
            let home = world.get::<Machine>(gate.unwrap()).unwrap().home;
            place(world, home + Vec3::new(400., 0., 40.), false);
        }
        4 if now > since + 4. => {
            if !settled_shut(world, gate.unwrap(), now - since) {
                return;
            }
            capture(world, "closed-again");
            let home = world.get::<Machine>(piston.unwrap()).unwrap().home;
            place(world, home + Vec3::new(0., 65., 40.), true);
        }
        5 if now > since + 0.43 => {
            assert!(world.get::<Machine>(piston.unwrap()).unwrap().fired);
            capture(world, "slime-launch");
        }
        6 if now > since + 1. => {
            let m = world.get::<Machine>(piston.unwrap()).unwrap();
            assert!(m.amount < 0.01);
        }
        // Escape: drop the hero onto the piston under normal gravity and steer
        // away. A head must never stay pinned on or beside a piston.
        7 if now > since + 2.4 => {
            let home = world.get::<Machine>(piston.unwrap()).unwrap().home;
            place(world, home + Vec3::new(0., 60., 30.), true);
            let hero = world
                .query_filtered::<Entity, With<Hero>>()
                .single(world)
                .unwrap();
            world.entity_mut(hero).insert(GravityScale(1.5));
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(if anchor.x > home.x {
                KeyCode::KeyD
            } else {
                KeyCode::KeyA
            });
        }
        8 => {
            let home = world.get::<Machine>(piston.unwrap()).unwrap().home;
            let hero = world
                .query_filtered::<&Transform, With<Hero>>()
                .single(world)
                .unwrap()
                .translation;
            if ((now - since) * 4.).fract() < 0.02 {
                eprintln!(
                    "[MECHANISM] escape t={:.2} hero=({:.0},{:.0}) piston=({:.0},{:.0})",
                    now - since,
                    hero.x,
                    hero.y,
                    home.x,
                    home.y
                );
            }
            if (hero.x - home.x).abs() < 160. {
                assert!(
                    now < since + 5.,
                    "Hero stuck at the piston: hero {hero:?}, piston {home:?}"
                );
                return;
            }
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyD);
            keys.release(KeyCode::KeyA);
            eprintln!("[MECHANISM] escaped the piston after {:.2}s", now - since);
        }
        9 => {
            if !super::contraption_probe::run(world, anchor) {
                return;
            }
            // Every machine collider stays block-sized. A collider sized in world
            // units on an entity already scaled to block size becomes a 64x larger
            // invisible box that holds heads and items in mid-air.
            let oversized: Vec<String> = world
                .query_filtered::<(Entity, &avian3d::prelude::ColliderAabb), Without<Block>>()
                .iter(world)
                .filter(|(_, aabb)| {
                    let size = aabb.max - aabb.min;
                    size.x > 8. * 64. || size.y > 8. * 64.
                })
                .map(|(e, aabb)| format!("{e:?} {:?}", aabb.max - aabb.min))
                .collect();
            assert!(oversized.is_empty(), "Oversized colliders: {oversized:?}");
            eprintln!(
                "[MECHANISM TEST] completed: closed/open gate, occupied safety, clear closure, slime extension/rebound/retraction, contraptions (slime pad, honey, dispenser, bubble columns, note blocks, bell stun, TNT tripwire)"
            );
            world.write_message(AppExit::Success);
        }
        _ => return,
    }
    let mut p = world.resource_mut::<Probe>();
    p.step += 1;
    p.since = now;
}
