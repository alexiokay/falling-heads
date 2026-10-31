//! Slime block pads (big bounce) and honey blocks (sticky sides, dead tops).
use super::contraptions::*;
use super::*;

pub(super) const SLIME_LAUNCH: f32 = 700.;
pub(super) const HONEY_SLIDE: f32 = -30.;
/// Exponential horizontal grip (per second) on a honey side and on its top.
const HONEY_SIDE_GRIP: f32 = 10.;
const HONEY_TOP_GRIP: f32 = 6.;
const CONTACT_SLOP: f32 = 6.;

#[derive(Component)]
pub(super) struct Pad {
    honey: bool,
    centre: Vec2,
    half: Vec2,
    cooldown: f32,
    /// Heads touching a honey side last step (slide sound once per stick).
    stuck: Vec<Entity>,
}
/// The new vertical speed for a head touching a slime block, if it launches.
/// Anything landing on (or resting on) the top is thrown up hard.
pub(super) fn slime_launch(side: Option<Side>, vy: f32) -> Option<f32> {
    (side == Some(Side::Top) && vy < 150.).then_some(SLIME_LAUNCH)
}
/// Honey: sides grab a head and let it slide down slowly while bleeding its
/// horizontal speed; the top absorbs bounces and slows walking.
pub(super) fn honey_velocity(side: Side, v: Vec2, dt: f32) -> Vec2 {
    match side {
        Side::Left | Side::Right => Vec2::new(
            v.x * (-HONEY_SIDE_GRIP * dt).exp(),
            if v.y < 120. { HONEY_SLIDE } else { v.y },
        ),
        Side::Top => Vec2::new(
            v.x * (-HONEY_TOP_GRIP * dt).exp(),
            if v.y > 0. { v.y * 0.05 } else { v.y },
        ),
        Side::Bottom => v,
    }
}
pub(super) fn spawn(
    commands: &mut Commands,
    p: Placement,
    anchor: (Anchor, ChunkMember),
    art: &GadgetArt,
) {
    let honey = p.gadget == Gadget::Honey;
    let height = if honey { p.len.max(1) } else { 1 } as f32 * 64.;
    let bottom = cell_centre(p.cell) - Vec2::Y * 32.;
    let centre = bottom + Vec2::Y * height / 2.;
    let size = Vec2::new(64., height);
    let (friction, restitution) = if honey {
        (
            Friction::new(1.).with_combine_rule(CoefficientCombine::Max),
            Restitution::new(0.).with_combine_rule(CoefficientCombine::Min),
        )
    } else {
        (
            Friction::new(0.2),
            Restitution::new(0.9).with_combine_rule(CoefficientCombine::Max),
        )
    };
    let root = commands
        .spawn((
            anchor,
            Pad {
                honey,
                centre,
                half: size / 2.,
                cooldown: 0.,
                stuck: Vec::new(),
            },
            Transform::from_translation(centre.extend(-10.)),
            Visibility::Inherited,
            solid(size.extend(64.), friction, restitution),
        ))
        .id();
    for i in 0..(height / 64.) as i32 {
        let y = i as f32 * 64. + 32. - height / 2.;
        if honey {
            // Translucent honey shell around a darker core, like Minecraft's.
            piece(
                commands,
                root,
                &art.cube,
                &art.honey,
                Transform::from_xyz(0., y, 0.).with_scale(Vec3::splat(56.)),
            );
            piece(
                commands,
                root,
                &art.cube,
                &art.honey,
                Transform::from_xyz(0., y, 0.).with_scale(Vec3::splat(64.)),
            );
        } else {
            piece(
                commands,
                root,
                &art.cube,
                &art.slime,
                Transform::from_xyz(0., y, 0.).with_scale(Vec3::splat(44.)),
            );
            piece(
                commands,
                root,
                &art.cube,
                &art.slime,
                Transform::from_xyz(0., y, 0.).with_scale(Vec3::splat(64.)),
            );
        }
    }
    if honey {
        piece(
            commands,
            root,
            &art.face,
            &art.honey_top,
            Transform::from_xyz(0., height / 2. + 0.3, 0.)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::new(64., 64., 1.)),
        );
    }
}
pub(super) fn drive_pads(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    art: Res<MachineArt>,
    mut pads: Query<&mut Pad>,
    mut heads: Query<(Entity, &Transform, &Position, &mut LinearVelocity), MachineHeads>,
) {
    let dt = time.delta_secs();
    if dt <= 0. {
        return;
    }
    let sounds = &art.gadgets.sounds;
    for mut pad in &mut pads {
        pad.cooldown = (pad.cooldown - dt).max(0.);
        let mut stuck = std::mem::take(&mut pad.stuck);
        let mut now_stuck = Vec::new();
        for (e, t, p, mut v) in &mut heads {
            let side = touching(p.0.xy(), head_half(t), pad.centre, pad.half, CONTACT_SLOP);
            if !pad.honey {
                if let Some(vy) = slime_launch(side, v.y) {
                    v.y = vy;
                    if pad.cooldown == 0. {
                        let i = (time.elapsed_secs() * 7.) as usize % 2;
                        play(&mut commands, &sounds.slime[i], pad.centre, 0.55, 1.);
                        pad.cooldown = 0.12;
                    }
                }
                continue;
            }
            let Some(side) = side else {
                continue;
            };
            let before = v.0.xy();
            let after = honey_velocity(side, before, dt);
            v.x = after.x;
            v.y = after.y;
            match side {
                Side::Left | Side::Right if after.y == HONEY_SLIDE => {
                    if !stuck.contains(&e) && pad.cooldown == 0. {
                        let i = (time.elapsed_secs() * 5.) as usize % 2;
                        play(&mut commands, &sounds.honey_slide[i], p.0.xy(), 0.5, 1.);
                        pad.cooldown = 0.6;
                    }
                    now_stuck.push(e);
                }
                Side::Top if before.y < -120. && pad.cooldown == 0. => {
                    play(&mut commands, &sounds.honey_step, p.0.xy(), 0.5, 1.);
                    pad.cooldown = 0.25;
                }
                _ => {}
            }
        }
        stuck.clear();
        stuck.extend(now_stuck);
        pad.stuck = stuck;
    }
}
