//! Tripwire traps: a dispenser that shoots along its wire, and a wire across a
//! corridor that primes a hidden TNT block.
use super::contraptions::*;
use super::*;

pub(super) const DISPENSER_COOLDOWN: f32 = 2.5;
pub(super) const TNT_TRAP_FUSE: f32 = 1.;
const ARROW_SHOT: f32 = 1100.;
const SNOWBALL_SHOT: f32 = 650.;
/// Wire height above the floor surface: every walking head crosses it.
const WIRE_HEIGHT: f32 = 12.;

#[derive(Component)]
pub(super) struct Dispenser {
    pub(super) facing: f32,
    wire: (f32, f32, f32),
    muzzle: Vec2,
    snow: bool,
    pub(super) cooldown: f32,
}
#[derive(Component)]
pub(super) struct TntTrap {
    wire: (f32, f32, f32),
    tnt: Entity,
    pub(super) fuse: Option<f32>,
}
#[derive(Component)]
pub(super) struct HiddenTnt {
    home: Vec3,
    lit: Option<f32>,
}
/// Whether a head box overlaps the horizontal wire `x0..x1` at height `y`.
pub(super) fn wire_crossed(p: Vec2, half: Vec2, (x0, x1, y): (f32, f32, f32)) -> bool {
    p.x + half.x > x0 && p.x - half.x < x1 && p.y - half.y < y && p.y + half.y > y
}
/// One dispenser step: returns the new cooldown and whether it fires now.
pub(super) fn dispenser_step(cooldown: f32, tripped: bool, dt: f32) -> (f32, bool) {
    let cooldown = (cooldown - dt).max(0.);
    if tripped && cooldown == 0. {
        (DISPENSER_COOLDOWN, true)
    } else {
        (cooldown, false)
    }
}
/// Horizontal launch velocity along the dispenser's facing.
pub(super) fn shot_velocity(facing: f32, snow: bool) -> Vec3 {
    Vec3::X * facing.signum() * if snow { SNOWBALL_SHOT } else { ARROW_SHOT }
}
/// One TNT trap step: an unlit trap lights when tripped, then explodes once
/// its fuse has burned for `TNT_TRAP_FUSE` seconds.
pub(super) fn tnt_step(fuse: Option<f32>, tripped: bool, dt: f32) -> (Option<f32>, bool) {
    match fuse {
        None if tripped => (Some(0.), false),
        None => (None, false),
        Some(t) => {
            let t = t + dt;
            (Some(t), t >= TNT_TRAP_FUSE)
        }
    }
}
fn wire_visuals(
    commands: &mut Commands,
    root: Entity,
    art: &GadgetArt,
    from: f32,
    cells: i32,
    y: f32,
    hooks: [Option<f32>; 2],
) {
    for i in 0..cells {
        let x = from + 32. + i as f32 * 64.;
        piece(
            commands,
            root,
            &art.face,
            &art.wire,
            Transform::from_xyz(x, y, 26.).with_scale(Vec3::new(64., 9., 1.)),
        );
        // A taut core so the thin, sparse string still reads from afar.
        piece(
            commands,
            root,
            &art.face,
            &art.string,
            Transform::from_xyz(x, y, 25.9).with_scale(Vec3::new(64., 2.5, 1.)),
        );
    }
    // Hooks point their stick into the wall they hang on.
    for (x, angle) in [(from, hooks[0]), (from + cells as f32 * 64., hooks[1])] {
        if let Some(angle) = angle {
            piece(
                commands,
                root,
                &art.face,
                &art.hook,
                Transform::from_xyz(x, y + 4., 27.)
                    .with_rotation(Quat::from_rotation_z(angle))
                    .with_scale(Vec3::splat(48.)),
            );
        }
    }
}
pub(super) fn spawn(
    commands: &mut Commands,
    p: Placement,
    biome: BiomeType,
    anchor: (Anchor, ChunkMember),
    art: &GadgetArt,
) {
    use std::f32::consts::FRAC_PI_2;
    let floor_top = cell_centre(p.cell).y - 32.;
    let wire_y = floor_top + WIRE_HEIGHT;
    if p.gadget == Gadget::Dispenser {
        let facing = if p.dir < 0 { -1. } else { 1. };
        let centre = cell_centre(p.cell);
        // Wire covers the cells in front of the dispenser.
        let near = centre.x + facing * 32.;
        let far = near + facing * 64. * p.len as f32;
        let (x0, x1) = (near.min(far), near.max(far));
        let root = commands
            .spawn((
                anchor,
                Dispenser {
                    facing,
                    wire: (x0, x1, wire_y),
                    muzzle: centre + Vec2::X * facing * 52.,
                    snow: matches!(biome, BiomeType::Iceland | BiomeType::Mountains),
                    cooldown: 0.,
                },
                Transform::from_translation(centre.extend(-10.)),
                Visibility::Inherited,
                solid(Vec2::splat(64.), Friction::new(0.1), Restitution::new(0.3)),
            ))
            .id();
        cube_with_top(
            commands,
            root,
            art,
            &art.furnace,
            &art.furnace_top,
            Vec3::ZERO,
            Vec3::splat(64.),
        );
        // The mouth faces the viewer (side view), mirrored toward the wire.
        piece(
            commands,
            root,
            &art.face,
            &art.dispenser,
            Transform::from_xyz(0., 0., 32.3).with_scale(Vec3::new(64. * facing, 64., 1.)),
        );
        let (from, hooks) = if facing > 0. {
            (near - centre.x, [Some(-FRAC_PI_2), Some(0.)])
        } else {
            (far - centre.x, [Some(0.), Some(FRAC_PI_2)])
        };
        wire_visuals(commands, root, art, from, p.len, wire_y - centre.y, hooks);
        return;
    }
    // Corridor tripwire: walls at both ends, TNT hidden behind the floor.
    let start = cell_centre(p.cell).x - 32.;
    let end = start + 64. * p.len as f32;
    let mid = IVec2::new(p.cell.x + (p.len - 1) / 2, p.cell.y - 1);
    // Hidden inside the floor block under the wire's middle.
    let tnt_home = cell_centre(mid).extend(-10.);
    let origin = Vec3::new(start, floor_top, 0.);
    let root = commands
        .spawn((
            anchor,
            Transform::from_translation(origin),
            Visibility::Inherited,
        ))
        .id();
    let tnt = commands
        .spawn((
            ChildOf(root),
            HiddenTnt {
                home: tnt_home,
                lit: None,
            },
            Mesh3d(art.cube.clone()),
            MeshMaterial3d(art.tnt.clone()),
            Transform::from_translation(tnt_home - origin).with_scale(Vec3::splat(56.)),
        ))
        .with_child((
            Mesh3d(art.face.clone()),
            MeshMaterial3d(art.tnt_top.clone()),
            Transform::from_xyz(0., 0.505, 0.).with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        ))
        .id();
    commands.entity(root).insert(TntTrap {
        wire: (start, end, wire_y),
        tnt,
        fuse: None,
    });
    wire_visuals(
        commands,
        root,
        art,
        0.,
        p.len,
        WIRE_HEIGHT,
        [Some(-FRAC_PI_2), Some(FRAC_PI_2)],
    );
}
type Targets<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static Position,
        Has<crate::components::enemy::Enemy>,
    ),
    MachineHeads,
>;
#[allow(clippy::too_many_arguments)] // Bevy system parameters
pub(super) fn drive_traps(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    art: Res<MachineArt>,
    mut dispensers: Query<&mut Dispenser>,
    mut traps: Query<(Entity, &mut TntTrap)>,
    mut tnts: Query<&mut HiddenTnt>,
    heads: Targets,
    models: Option<Res<crate::resources::asset_handles::ProjectileModels>>,
) {
    let dt = time.delta_secs();
    if dt <= 0. {
        return;
    }
    let sounds = &art.gadgets.sounds;
    for mut d in &mut dispensers {
        let target = heads
            .iter()
            .find(|(t, p, _)| wire_crossed(p.0.xy(), head_half(t), d.wire));
        let (cooldown, fire) = dispenser_step(d.cooldown, target.is_some(), dt);
        d.cooldown = cooldown;
        let Some((_, _, enemy)) = target.filter(|_| fire) else {
            continue;
        };
        use crate::components::projectile::Team;
        // Friendly fire toward enemies, hostile toward heroes and villagers.
        let team = if enemy { Team::Friendly } else { Team::Enemy };
        let velocity = shot_velocity(d.facing, d.snow);
        let at = d.muzzle.extend(0.);
        play(&mut commands, &sounds.click, d.muzzle, 0.6, 1.);
        if d.snow {
            commands.spawn((
                crate::components::projectile::Snowball {
                    team,
                    ..crate::components::projectile::Snowball::new()
                },
                LinearVelocity(velocity),
                RigidBody::Dynamic,
                Mass(0.1),
                Collider::sphere(12.),
                Sensor,
                GravityScale(0.6),
                Restitution::new(0.),
                Friction::new(0.1),
                LockedAxes::new()
                    .lock_rotation_x()
                    .lock_rotation_y()
                    .lock_rotation_z()
                    .lock_translation_z(),
                crate::components::bundles::SpriteBundle3d::new(
                    art.gadgets.snowball_mesh.clone(),
                    art.gadgets.snowball.clone(),
                    at,
                ),
            ));
        } else {
            let Some(models) = models.as_deref() else {
                continue;
            };
            play(&mut commands, &sounds.bow, d.muzzle, 0.5, 1.);
            commands.spawn((
                crate::components::projectile::Arrow {
                    team,
                    ..crate::components::projectile::Arrow::new(1)
                },
                crate::components::bundles::avian_physics::arrow(velocity),
                bevy::world_serialization::WorldAssetRoot(models.arrow.clone()),
                Transform::from_translation(at).with_scale(Vec3::splat(15.)),
                Visibility::Visible,
            ));
        }
    }
    for (e, mut trap) in &mut traps {
        let tripped = heads
            .iter()
            .any(|(t, p, _)| wire_crossed(p.0.xy(), head_half(t), trap.wire));
        let was_lit = trap.fuse.is_some();
        let (fuse, explode) = tnt_step(trap.fuse, tripped, dt);
        trap.fuse = fuse;
        let Ok(mut tnt) = tnts.get_mut(trap.tnt) else {
            continue;
        };
        if !was_lit && fuse.is_some() {
            tnt.lit = Some(0.);
            let at = tnt.home.xy();
            play(&mut commands, &sounds.click, at, 0.6, 1.);
            play(&mut commands, &sounds.fuse, at, 0.8, 1.);
        }
        if explode {
            // The existing TNT explosion: blocks, knockback, damage, sound.
            commands.spawn((
                crate::components::explosive::Explosion {
                    radius: crate::utils::constants::TNT_EXPLOSION_RADIUS,
                },
                Transform::from_translation(tnt.home.xy().extend(0.) + Vec3::Y * 64.),
            ));
            commands.entity(e).despawn();
        }
    }
}
/// A lit TNT rises out of its floor block into the corridor and flashes.
pub(super) fn animate_tnt(
    time: Res<Time>,
    art: Res<MachineArt>,
    mut tnts: Query<(
        &mut HiddenTnt,
        &ChildOf,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    roots: Query<&Transform, Without<HiddenTnt>>,
) {
    for (mut tnt, parent, mut t, mut mat) in &mut tnts {
        let Some(age) = tnt.lit.as_mut() else {
            continue;
        };
        *age += time.delta_secs();
        let age = *age;
        let Ok(root) = roots.get(parent.parent()) else {
            continue;
        };
        let rise = (age / 0.25).min(1.);
        let goal = tnt.home + Vec3::Y * 64. * rise;
        t.translation = goal - root.translation;
        let flash = (age * 8.).fract() > 0.5;
        let want = if flash {
            &art.gadgets.tnt_lit
        } else {
            &art.gadgets.tnt
        };
        if mat.0 != *want {
            mat.0 = want.clone();
        }
    }
}
