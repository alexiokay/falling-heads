//! Note blocks (a pitch that climbs per hit, instrument from the block below)
//! and hanging bells that stun nearby enemies.
use super::contraptions::*;
use super::*;

/// Bells stun enemies within six blocks for two seconds.
pub(super) const BELL_RANGE: f32 = 6. * 64.;
pub(super) const STUN_SECONDS: f32 = 2.;
const CONTACT_SLOP: f32 = 4.;
/// A short scale so a row of note blocks plays a tiny tune when walked.
const ROW_START: [u8; 6] = [6, 8, 10, 11, 13, 15];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Instrument {
    Harp,
    BassDrum,
    Snare,
    Bass,
    Chime,
}
/// Minecraft's rule: the block under a note block picks its instrument.
pub(super) fn instrument(under: BlockType) -> Instrument {
    use BlockType::*;
    match under {
        Stone | Coal | Iron | Gold | Diamond | Emerald | Redstone | Sandstone | Netherrack
        | Netherite | AncientDebris | EndStone | Obsidian | Purpur => Instrument::BassDrum,
        Sand | SoulSand => Instrument::Snare,
        Wood => Instrument::Bass,
        Ice | Glowstone => Instrument::Chime,
        _ => Instrument::Harp,
    }
}
/// Playback speed for pitch `n` (0..=24): two octaves, F#3..F#5 as in
/// Minecraft, where n = 12 is the sample's own pitch.
pub(super) fn note_speed(n: u8) -> f32 {
    2f32.powf((n.min(24) as f32 - 12.) / 12.)
}
/// Each hit steps one semitone up, wrapping after 24.
pub(super) fn next_note(n: u8) -> u8 {
    (n + 1) % 25
}
/// Whether an enemy at `p` hears a bell at `bell` (and is stunned).
pub(super) fn bell_stuns(bell: Vec2, p: Vec2) -> bool {
    bell.distance_squared(p) <= BELL_RANGE * BELL_RANGE
}
/// Remaining stun after `dt`, or `None` once it has worn off.
pub(super) fn stun_left(remaining: f32, dt: f32) -> Option<f32> {
    let left = remaining - dt;
    (left > 0.).then_some(left)
}
/// Enemy AI (movement and attacks) skips stunned enemies; `hold_stunned`
/// also pins their velocity so foes with their own AI stand still too.
#[derive(Component)]
pub(crate) struct Stunned {
    pub(crate) remaining: f32,
}
#[derive(Component)]
pub(super) struct NoteBlock {
    pub(super) n: u8,
    pub(super) instrument: Instrument,
    centre: Vec2,
    touching: Vec<Entity>,
}
#[derive(Component)]
pub(super) struct NoteParticle {
    age: f32,
}
#[derive(Component)]
pub(super) struct Bell {
    centre: Vec2,
    half: Vec2,
    cooldown: f32,
    pub(super) rings: u32,
    swing: f32,
    dir: f32,
    touching: Vec<Entity>,
}
#[derive(Component)]
pub(super) struct BellPivot;
pub(super) fn spawn(
    commands: &mut Commands,
    p: Placement,
    anchor: (Anchor, ChunkMember),
    art: &GadgetArt,
) {
    if p.gadget == Gadget::Notes {
        // The row's first block owns the anchor; the others follow it.
        let root = commands
            .spawn((
                anchor,
                Transform::from_translation(cell_centre(p.cell).extend(0.)),
                Visibility::Inherited,
            ))
            .id();
        for i in 0..p.len.clamp(1, 6) {
            let centre = cell_centre(p.cell + IVec2::X * i);
            let block = commands
                .spawn((
                    ChildOf(root),
                    NoteBlock {
                        n: ROW_START[i as usize],
                        instrument: instrument(p.under[i as usize]),
                        centre,
                        touching: Vec::new(),
                    },
                    Transform::from_xyz(i as f32 * 64., 0., -10.),
                    Visibility::Inherited,
                    solid(Vec2::splat(64.), Friction::new(0.1), Restitution::new(0.3)),
                ))
                .id();
            cube_with_top(
                commands,
                block,
                art,
                &art.note_block,
                &art.note_block,
                Vec3::ZERO,
                Vec3::splat(64.),
            );
        }
        return;
    }
    // Bell: hangs from the ceiling cell above `p.cell`.
    let pivot = cell_centre(p.cell) + Vec2::Y * 32.;
    let half = Vec2::new(21., 23.);
    let centre = pivot - Vec2::new(0., 8. + half.y);
    let root = commands
        .spawn((
            anchor,
            Bell {
                centre,
                half,
                cooldown: 0.,
                rings: 0,
                swing: 10.,
                dir: 1.,
                touching: Vec::new(),
            },
            Transform::from_translation(pivot.extend(-10.)),
            Visibility::Inherited,
        ))
        .id();
    // Static collider at rest: the swing is purely visual.
    commands.spawn((
        ChildOf(root),
        Transform::from_translation((centre - pivot).extend(0.)),
        solid(half * 2., Friction::new(0.1), Restitution::new(0.5)),
    ));
    // Ceiling bracket and the swinging bell (Minecraft proportions x1.25).
    piece(
        commands,
        root,
        &art.cube,
        &art.iron,
        Transform::from_xyz(0., -2., 0.).with_scale(Vec3::new(40., 4., 12.)),
    );
    let swing = commands
        .spawn((
            ChildOf(root),
            BellPivot,
            Transform::default(),
            Visibility::Inherited,
        ))
        .id();
    piece(
        commands,
        swing,
        &art.cube,
        &art.iron,
        Transform::from_xyz(0., -5., 0.).with_scale(Vec3::new(6., 10., 6.)),
    );
    piece(
        commands,
        swing,
        &art.cube,
        &art.bell_body,
        Transform::from_xyz(0., -10. - 17.5, 0.).with_scale(Vec3::new(30., 35., 30.)),
    );
    piece(
        commands,
        swing,
        &art.cube,
        &art.bell_lip,
        Transform::from_xyz(0., -10. - 35. - 5., 0.).with_scale(Vec3::new(40., 10., 40.)),
    );
}
type Heads<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        &'static Position,
        &'static LinearVelocity,
    ),
    MachineHeads,
>;
/// New contacts this step: heads touching now that were not touching before.
fn contacts(heads: &Heads, centre: Vec2, half: Vec2, before: &[Entity]) -> (Vec<Entity>, bool) {
    let mut now = Vec::new();
    let mut fresh = false;
    for (e, t, p, _) in heads {
        if touching(p.0.xy(), head_half(t), centre, half, CONTACT_SLOP).is_some() {
            fresh |= !before.contains(&e);
            now.push(e);
        }
    }
    (now, fresh)
}
#[allow(clippy::too_many_arguments)] // Bevy system parameters
pub(super) fn drive_music(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    art: Res<MachineArt>,
    mut notes: Query<&mut NoteBlock>,
    mut bells: Query<&mut Bell>,
    heads: Heads,
    enemies: Query<(Entity, &Position, Option<&Stunned>), With<Enemy>>,
) {
    let dt = time.delta_secs();
    if dt <= 0. {
        return;
    }
    let art = &art.gadgets;
    for mut note in &mut notes {
        let (now, fresh) = contacts(&heads, note.centre, Vec2::splat(32.), &note.touching);
        note.touching = now;
        if !fresh {
            continue;
        }
        let n = note.n;
        play(
            &mut commands,
            &art.sounds.instruments[note.instrument as usize],
            note.centre,
            0.6,
            note_speed(n),
        );
        commands.spawn((
            NoteParticle { age: 0. },
            Mesh3d(art.face.clone()),
            MeshMaterial3d(art.notes[n as usize].clone()),
            Transform::from_translation(note.centre.extend(40.) + Vec3::Y * 44.)
                .with_scale(Vec3::splat(26.)),
        ));
        note.n = next_note(n);
    }
    for mut bell in &mut bells {
        bell.cooldown = (bell.cooldown - dt).max(0.);
        let (now, fresh) = contacts(&heads, bell.centre, bell.half, &bell.touching);
        let hit_from = now
            .iter()
            .filter_map(|e| heads.get(*e).ok())
            .map(|(_, _, p, _)| p.0.x - bell.centre.x)
            .next();
        bell.touching = now;
        if !fresh || bell.cooldown > 0. {
            continue;
        }
        bell.cooldown = 0.7;
        bell.rings += 1;
        bell.swing = 0.;
        bell.dir = if hit_from.unwrap_or(1.) > 0. { -1. } else { 1. };
        let i = bell.rings as usize % 2;
        play(&mut commands, &art.sounds.bell[i], bell.centre, 0.8, 1.);
        if bell.rings.is_multiple_of(3) {
            play(
                &mut commands,
                &art.sounds.bell_resonate,
                bell.centre,
                0.35,
                1.,
            );
        }
        for (e, p, stunned) in &enemies {
            if bell_stuns(bell.centre, p.0.xy())
                && stunned.is_none_or(|s| s.remaining < STUN_SECONDS)
            {
                commands.entity(e).try_insert(Stunned {
                    remaining: STUN_SECONDS,
                });
            }
        }
    }
}
/// Stunned enemies stand still (flying ones hover) until the stun wears off.
pub(super) fn hold_stunned(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut stunned: Query<(
        Entity,
        &mut Stunned,
        &mut LinearVelocity,
        Has<crate::components::enemy::Flying>,
    )>,
) {
    let dt = time.delta_secs();
    for (e, mut s, mut v, flying) in &mut stunned {
        match stun_left(s.remaining, dt) {
            Some(left) => s.remaining = left,
            None => {
                commands.entity(e).remove::<Stunned>();
                continue;
            }
        }
        v.x = 0.;
        if flying {
            v.y = 0.;
        }
    }
}
pub(super) fn animate_music(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut NoteParticle, &mut Transform), Without<BellPivot>>,
    mut bells: Query<&mut Bell>,
    mut pivots: Query<(&ChildOf, &mut Transform), With<BellPivot>>,
) {
    let dt = time.delta_secs();
    for (e, mut n, mut t) in &mut particles {
        n.age += dt;
        if n.age > 0.9 {
            commands.entity(e).despawn();
            continue;
        }
        t.translation.y += 60. * dt;
        let s = 26. * (1. - ((n.age - 0.6) / 0.3).max(0.));
        t.scale = Vec3::splat(s.max(0.1));
    }
    for (parent, mut t) in &mut pivots {
        let Ok(mut bell) = bells.get_mut(parent.parent()) else {
            continue;
        };
        if bell.swing > 4. {
            continue;
        }
        bell.swing += dt;
        let a = bell.dir * 0.5 * (-1.6 * bell.swing).exp() * (bell.swing * 11.).sin();
        t.rotation = Quat::from_rotation_z(a);
    }
}
