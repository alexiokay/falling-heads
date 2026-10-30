//! Opt-in contraption bench for `--mechanism-test`, run after the gate and
//! piston checks: every contraption on one flat floor, screenshots of each and
//! a runtime check of its rule. Never runs in normal play.
use super::bubbles::Column;
use super::contraptions::*;
use super::music::{Bell, NoteBlock, Stunned};
use super::traps::{Dispenser, TntTrap};
use super::*;

#[derive(Resource, Default)]
struct Bench {
    step: u8,
    since: f32,
    base: IVec2,
    dummies: Vec<Entity>,
}
/// Bench cells, relative to the floor row's origin.
const SLIME: IVec2 = IVec2::new(-16, 1);
const HONEY: IVec2 = IVec2::new(-13, 1);
const DISPENSER: IVec2 = IVec2::new(-10, 1);
const COLUMN_UP: IVec2 = IVec2::new(-5, 1);
const COLUMN_DOWN: IVec2 = IVec2::new(-3, 1);
const NOTES: IVec2 = IVec2::new(0, 1);
const BELL: IVec2 = IVec2::new(6, 3);
const WIRE: IVec2 = IVec2::new(11, 1);
fn capture(world: &mut World, name: &str) {
    world
        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(format!(
            "target/visual-mechanism-{name}.png"
        )));
}
fn hero(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, With<Hero>>()
        .single(world)
        .unwrap()
}
fn place(world: &mut World, pos: Vec2, dynamic: bool) {
    let pos = pos.extend(40.);
    let hero = hero(world);
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
/// An invisible, physics-free head: contraption rules see it, nothing moves it.
fn dummy(world: &mut World, pos: Vec2, enemy: bool) -> Entity {
    let mut e = world.spawn((
        Transform::from_translation(pos.extend(30.)),
        Position(pos.extend(30.)),
        LinearVelocity::ZERO,
    ));
    if enemy {
        e.insert(Enemy::new(crate::components::enemy::EnemyType::Zombie));
    } else {
        e.insert(Health::new(3));
    }
    let id = e.id();
    world.resource_mut::<Bench>().dummies.push(id);
    id
}
/// Bench components only: streamed landmarks nearby may own contraptions too.
fn bench<T: Component>(world: &mut World, base: IVec2) -> Vec<Entity> {
    let centre = cell_centre(base);
    world
        .query_filtered::<(Entity, &GlobalTransform), With<T>>()
        .iter(world)
        .filter(|(_, t)| t.translation().xy().distance(centre) < 1300.)
        .map(|(e, _)| e)
        .collect()
}
fn at(base: IVec2, cell: IVec2) -> Vec2 {
    cell_centre(base + cell)
}
fn build(world: &mut World, anchor: Vec3) -> IVec2 {
    let old: Vec<_> = world
        .query::<(
            Entity,
            &Transform,
            Has<Block>,
            Has<Enemy>,
            Has<Villager>,
            Has<Machine>,
            Has<Anchor>,
        )>()
        .iter(world)
        .filter(|(_, t, b, e, v, m, a)| {
            (*b || *e || *v || *m || *a) && t.translation.xy().distance(anchor.xy()) < 1400.
        })
        .map(|(e, ..)| e)
        .collect();
    for e in old {
        if let Ok(e) = world.get_entity_mut(e) {
            e.despawn();
        }
    }
    let base = (anchor.xy() / 64.).round().as_ivec2() - IVec2::Y * 2;
    let mut cells: Vec<(IVec2, BlockType)> = (-18..=15)
        .map(|x| (IVec2::new(x, 0), BlockType::Stone))
        .collect();
    // Under the note row: bass drum, snare, bass, chime.
    for (i, kind) in [
        BlockType::Stone,
        BlockType::Sand,
        BlockType::Wood,
        BlockType::Ice,
    ]
    .into_iter()
    .enumerate()
    {
        cells.retain(|(c, _)| *c != NOTES + IVec2::new(i as i32, -1));
        cells.push((NOTES + IVec2::new(i as i32, -1), kind));
    }
    // Bell beam and the TNT corridor's walls.
    cells.extend((5..=7).map(|x| (IVec2::new(x, 4), BlockType::Stone)));
    cells.extend([
        (WIRE - IVec2::X, BlockType::Stone),
        (WIRE + IVec2::X * 3, BlockType::Stone),
    ]);
    world.resource_scope(|world, meshes: Mut<MeshCache>| {
        world.resource_scope(|world, mats: Mut<MaterialCache>| {
            for (cell, kind) in cells {
                let cell = base + cell;
                let chunk = cell.div_euclid(IVec2::splat(16));
                let (mesh, scale) = if kind == BlockType::Wood {
                    (meshes.wood_mesh.clone(), 1.)
                } else {
                    (meshes.block_mesh.clone(), 64.)
                };
                world.spawn((
                    Block {
                        health: kind.starting_health(),
                        max_health: kind.starting_health(),
                    },
                    kind,
                    ChunkMember {
                        chunk_x: chunk.x,
                        chunk_y: chunk.y,
                    },
                    Mesh3d(mesh),
                    MeshMaterial3d(mats.materials[&kind].clone()),
                    Transform::from_translation((cell.as_vec2() * 64.).extend(-10.))
                        .with_scale(Vec3::splat(scale)),
                ));
            }
        });
    });
    let mut notes = Placement::new(Gadget::Notes, NOTES, 4, 0);
    notes.under[..4].copy_from_slice(&[
        BlockType::Stone,
        BlockType::Sand,
        BlockType::Wood,
        BlockType::Ice,
    ]);
    let art = world.resource::<MachineArt>().gadgets.clone();
    for mut p in [
        Placement::new(Gadget::SlimePad, SLIME, 1, 0),
        Placement::new(Gadget::Honey, HONEY, 2, 0),
        Placement::new(Gadget::Dispenser, DISPENSER, 2, 1),
        Placement::new(Gadget::Bubbles, COLUMN_UP, 4, 1),
        Placement::new(Gadget::Bubbles, COLUMN_DOWN, 4, -1),
        notes,
        Placement::new(Gadget::Bell, BELL, 1, 0),
        Placement::new(Gadget::Tripwire, WIRE, 3, 0),
    ] {
        p.cell += base;
        spawn_placement(&mut world.commands(), p, BiomeType::Plains, base, &art);
    }
    world.flush();
    base
}
/// Drives the bench; returns true once every check has passed.
pub(super) fn run(world: &mut World, anchor: Vec3) -> bool {
    world.init_resource::<Bench>();
    let now = world.resource::<Time<Real>>().elapsed_secs();
    let b = world.resource::<Bench>();
    let (step, since, base) = (b.step, b.since, b.base);
    let wait = |s: f32| now > since + s;
    let floor_top = |cell: IVec2| at(base, cell).y - 32.;
    // Each action first parks the hero (static) where it will act, so the
    // following camera settles before the capture.
    let slime = Vec2::new(at(base, SLIME).x, floor_top(SLIME) + 64. + 33.);
    let honey = at(base, HONEY + IVec2::Y) - Vec2::X * 65.;
    let column = at(base, COLUMN_UP + IVec2::Y);
    let bell = Vec2::new(at(base, BELL).x, at(base, BELL).y + 32. - 54. - 2. - 32.);
    let note = at(base, NOTES + IVec2::new(2, 0)) + Vec2::Y * 65.;
    match step {
        0 => {
            let base = build(world, anchor);
            world.resource_mut::<Bench>().base = base;
            place(world, at(base, IVec2::new(-9, 6)), false);
        }
        1 if wait(1.5) => {
            let kinds: std::collections::HashSet<_> = world
                .query::<&Anchor>()
                .iter(world)
                .map(|a| a.gadget)
                .collect();
            assert_eq!(kinds.len(), 7, "every contraption kind spawned: {kinds:?}");
            capture(world, "bench-left");
            place(world, at(base, IVec2::new(4, 6)), false);
        }
        2 if wait(1.2) => {
            capture(world, "bench-right");
            place(world, slime, false);
        }
        3 if wait(1.) => place(world, slime, true),
        4 if wait(0.15) => {
            let h = hero(world);
            let v = world.get::<LinearVelocity>(h).unwrap().y;
            assert!(v > 600., "slime launch {v}");
            capture(world, "gadget-slime");
            place(world, honey, false);
        }
        5 if wait(1.) => {
            // The rule on a physics-free head stuck to the honey's right side
            // (the hero rests against its left side for the picture).
            let side = at(base, HONEY + IVec2::Y) + Vec2::X * 65.;
            let d = dummy(world, side, false);
            world.get_mut::<LinearVelocity>(d).unwrap().0 = Vec3::new(-200., -400., 0.);
        }
        6 if wait(0.5) => {
            let d = world.resource::<Bench>().dummies[0];
            let v = world.get::<LinearVelocity>(d).unwrap().0;
            assert!(
                v.y == super::pads::HONEY_SLIDE && v.x.abs() < 5.,
                "honey stick and slide {v}"
            );
            capture(world, "gadget-honey");
            for e in std::mem::take(&mut world.resource_mut::<Bench>().dummies) {
                world.despawn(e);
            }
            // Dummies on the dispenser wire, in both columns and on a note block.
            dummy(world, at(base, DISPENSER + IVec2::X * 2) + Vec2::Y, false);
            dummy(world, at(base, COLUMN_DOWN + IVec2::Y * 2), false);
            dummy(world, at(base, COLUMN_UP + IVec2::Y * 2), false);
            dummy(world, at(base, NOTES + IVec2::Y) + Vec2::Y, false);
        }
        7 if wait(0.08) => {
            let d = bench::<Dispenser>(world, base);
            let cooldown = world.get::<Dispenser>(d[0]).unwrap().cooldown;
            assert!(cooldown > 2., "dispenser fired ({cooldown})");
            let arrows = world
                .query_filtered::<(), With<crate::components::projectile::Arrow>>()
                .iter(world)
                .count();
            assert!(arrows > 0, "dispenser arrow in flight");
            capture(world, "gadget-dispenser");
            place(world, column, false);
        }
        8 if wait(1.) => place(world, column, true),
        9 if wait(0.5) => {
            let d = world.resource::<Bench>().dummies.clone();
            let down = world.get::<LinearVelocity>(d[1]).unwrap().y;
            let up = world.get::<LinearVelocity>(d[2]).unwrap().y;
            assert!(up > 250. && down < -200., "columns up {up} down {down}");
            let h = hero(world);
            let rising = world.get::<LinearVelocity>(h).unwrap().y;
            assert!(rising > 200., "hero rides the column up ({rising})");
            let columns = bench::<Column>(world, base);
            let up = columns
                .iter()
                .filter(|e| world.get::<Column>(**e).unwrap().up);
            assert_eq!(up.count(), 1);
            let played: Vec<u8> = bench::<NoteBlock>(world, base)
                .iter()
                .map(|e| world.get::<NoteBlock>(*e).unwrap().n)
                .collect();
            assert!(
                played.contains(&7),
                "first note block stepped up: {played:?}"
            );
            capture(world, "gadget-bubbles");
            for e in std::mem::take(&mut world.resource_mut::<Bench>().dummies) {
                world.despawn(e);
            }
            dummy(world, at(base, BELL + IVec2::new(4, -2)), true);
            place(world, bell - Vec2::Y * 30., false);
        }
        10 if wait(1.) => place(world, bell, false),
        11 if wait(0.15) => {
            let bells = bench::<Bell>(world, base);
            let rings = world.get::<Bell>(bells[0]).unwrap().rings;
            assert!(rings >= 1, "bell rang");
            let foe = world.resource::<Bench>().dummies[0];
            assert!(
                world.get::<Stunned>(foe).is_some(),
                "bell stunned the enemy"
            );
            capture(world, "gadget-bell");
            place(world, note + Vec2::Y * 40., false);
        }
        12 if wait(1.) => place(world, note, false),
        13 if wait(0.2) => {
            capture(world, "gadget-notes");
            for e in std::mem::take(&mut world.resource_mut::<Bench>().dummies) {
                world.despawn(e);
            }
            // Out of the blast radius, the corridor still in view.
            place(world, at(base, IVec2::new(4, 7)), false);
        }
        14 if wait(1.) => {
            dummy(world, at(base, WIRE + IVec2::X) + Vec2::Y, false);
        }
        15 if wait(0.5) => {
            let trap = bench::<TntTrap>(world, base);
            let fuse = world.get::<TntTrap>(trap[0]).unwrap().fuse;
            assert!(fuse.is_some(), "tripwire lit the TNT");
            capture(world, "gadget-tnt-fuse");
        }
        16 if wait(0.8) => {
            assert!(bench::<TntTrap>(world, base).is_empty(), "TNT exploded");
            capture(world, "gadget-tnt-explosion");
            for e in std::mem::take(&mut world.resource_mut::<Bench>().dummies) {
                world.despawn(e);
            }
        }
        17 if wait(1.) => return true,
        _ => return false,
    }
    let mut b = world.resource_mut::<Bench>();
    b.step += 1;
    b.since = now;
    false
}
