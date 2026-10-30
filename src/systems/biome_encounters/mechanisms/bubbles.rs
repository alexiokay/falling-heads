//! Bubble columns: a 1-wide water shaft over soul sand (lifts heads) or magma
//! (drags them down). A force volume with its own visuals, not fluid cells, so
//! the fluid simulation never sees or spreads this water.
use super::contraptions::*;
use super::*;

pub(super) const COLUMN_UP: f32 = 300.;
pub(super) const COLUMN_DOWN: f32 = -250.;
/// How quickly (per second) a head's vertical speed converges on the column's.
const COLUMN_RATE: f32 = 20.;

#[derive(Component)]
pub(super) struct Column {
    pub(super) up: bool,
    x: f32,
    /// Water bottom (top of the soul sand or magma) and water top.
    bottom: f32,
    top: f32,
    spawn: f32,
    ambient: f32,
    inside: Vec<Entity>,
}
#[derive(Component)]
pub(super) struct Bubble {
    speed: f32,
    age: f32,
    life: f32,
    phase: f32,
    x: f32,
}
/// Vertical speed after one step inside a column.
pub(super) fn column_velocity(vy: f32, up: bool, dt: f32) -> f32 {
    let target = if up { COLUMN_UP } else { COLUMN_DOWN };
    target + (vy - target) * (-COLUMN_RATE * dt).exp()
}
/// A head is inside when its centre is over the shaft and its box overlaps the water.
pub(super) fn in_column(p: Vec2, half: Vec2, x: f32, bottom: f32, top: f32) -> bool {
    (p.x - x).abs() < 32. && p.y - half.y < top && p.y + half.y > bottom
}
pub(super) fn spawn(
    commands: &mut Commands,
    p: Placement,
    anchor: (Anchor, ChunkMember),
    art: &GadgetArt,
) {
    let up = p.dir >= 0;
    let base = cell_centre(p.cell);
    let bottom = base.y + 32.;
    let top = bottom + 64. * p.len as f32;
    let root = commands
        .spawn((
            anchor,
            Column {
                up,
                x: base.x,
                bottom,
                top,
                spawn: 0.,
                ambient: 1.5,
                inside: Vec::new(),
            },
            Transform::from_translation(Vec3::new(base.x, bottom, 0.)),
            Visibility::Inherited,
        ))
        .id();
    // The soul sand or magma block is solid: heads stand on it.
    commands.spawn((
        ChildOf(root),
        Mesh3d(art.cube.clone()),
        MeshMaterial3d(if up {
            art.soul_sand.clone()
        } else {
            art.magma.clone()
        }),
        Transform::from_xyz(0., -32., -10.).with_scale(Vec3::splat(64.)),
        solid(Vec2::splat(64.), Friction::new(0.3), Restitution::new(0.2)),
    ));
    for i in 0..p.len {
        piece(
            commands,
            root,
            &art.face,
            &art.water,
            Transform::from_xyz(
                0.,
                32. + i as f32 * 64.,
                crate::systems::actor_depth::WATER_FRONT,
            )
            .with_scale(Vec3::new(64., 64., 1.)),
        );
        // A faint back plane gives the shaft depth in perspective.
        piece(
            commands,
            root,
            &art.face,
            &art.water,
            Transform::from_xyz(0., 32. + i as f32 * 64., -40.).with_scale(Vec3::new(64., 64., 1.)),
        );
    }
}
pub(super) fn push_columns(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    art: Res<MachineArt>,
    mut columns: Query<&mut Column>,
    mut heads: Query<(Entity, &Transform, &Position, &mut LinearVelocity), MachineHeads>,
) {
    let dt = time.delta_secs();
    if dt <= 0. {
        return;
    }
    let sounds = &art.gadgets.sounds;
    for mut c in &mut columns {
        let mut inside = std::mem::take(&mut c.inside);
        let before = inside.clone();
        inside.clear();
        for (e, t, p, mut v) in &mut heads {
            let pos = p.0.xy();
            if !in_column(pos, head_half(t), c.x, c.bottom, c.top) {
                continue;
            }
            v.y = column_velocity(v.y, c.up, dt);
            // Keep heads in the shaft without fighting their walking AI hard.
            v.x = v.x * (-4. * dt).exp() + (c.x - pos.x) * 6. * dt;
            if !before.contains(&e) {
                let i = usize::from(!c.up);
                play(&mut commands, &sounds.column_inside[i], pos, 0.45, 1.);
            }
            inside.push(e);
        }
        c.inside = inside;
        c.ambient -= dt;
        if c.ambient <= 0. {
            let i = usize::from(!c.up);
            let mid = Vec2::new(c.x, (c.bottom + c.top) / 2.);
            play(&mut commands, &sounds.column_ambient[i], mid, 0.22, 1.);
            play(&mut commands, &sounds.bubble_pop, mid, 0.18, 1.);
            c.ambient = 4.5 + (time.elapsed_secs() * 3.7).sin().abs() * 3.;
        }
    }
}
/// Rising (soul sand) or sinking (magma) bubbles inside each shaft.
pub(super) fn animate_bubbles(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<MachineArt>,
    mut columns: Query<(Entity, &mut Column)>,
    mut bubbles: Query<(Entity, &mut Bubble, &mut Transform)>,
) {
    use rand::RngExt;
    let dt = time.delta_secs();
    let mut rng = rand::rng();
    let z = crate::systems::actor_depth::WATER_FRONT + 1.5;
    for (root, mut c) in &mut columns {
        c.spawn -= dt;
        while c.spawn <= 0. {
            c.spawn += rng.random_range(0.08..0.2);
            let height = c.top - c.bottom;
            let speed = if c.up {
                rng.random_range(110.0..190.)
            } else {
                -rng.random_range(80.0..130.)
            };
            let x = rng.random_range(-22.0..22.);
            let y = if c.up { 4. } else { height - 4. };
            let size = rng.random_range(9.0..16.);
            commands.spawn((
                ChildOf(root),
                Bubble {
                    speed,
                    age: 0.,
                    life: (height - 8.) / speed.abs(),
                    phase: rng.random_range(0.0..6.3),
                    x,
                },
                Mesh3d(art.gadgets.face.clone()),
                MeshMaterial3d(art.gadgets.bubble.clone()),
                Transform::from_xyz(x, y, z).with_scale(Vec3::splat(size)),
            ));
        }
    }
    for (e, mut b, mut t) in &mut bubbles {
        b.age += dt;
        if b.age >= b.life {
            commands.entity(e).despawn();
            continue;
        }
        t.translation.y += b.speed * dt;
        t.translation.x = b.x + (b.age * 7. + b.phase).sin() * 4.;
    }
}
