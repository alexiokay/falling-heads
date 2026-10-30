//! Small, deterministic 3D course. No main-world physics or entity IDs in history.
use bevy::prelude::*;
use std::collections::VecDeque;
pub(super) const STEP: f32 = 1. / 120.;
pub(super) const RADIUS: f32 = 26.;
pub(super) const COUNT: usize = 9;
const GRAVITY: f32 = 700.;
#[derive(Clone, Copy)]
pub(super) struct Pad {
    pub center: Vec3,
    pub half: Vec2,
    pub moving: bool,
}
pub(super) fn pad(i: usize, time: f32) -> Pad {
    let z = [0., -130., 80., 220., -40., -220., 40., 190., 0.][i];
    let y = [0., 30., 100., 30., 140., 70., 170., 90., 120.][i];
    let moving = i == 3 || i == 6;
    Pad {
        center: Vec3::new(
            i as f32 * 290.,
            y,
            z + if moving {
                (time * 1.2 + i as f32).sin() * 65.
            } else {
                0.
            },
        ),
        half: if i == 0 || i == COUNT - 1 {
            Vec2::splat(125.)
        } else {
            Vec2::new(102., 90.)
        },
        moving,
    }
}
pub(super) fn gem_position(i: usize, time: f32) -> Vec3 {
    if i < COUNT {
        return pad(i, time).center + Vec3::Y * (78. + (time * 2.).sin() * 7.);
    }
    let segment = (i - COUNT) * 2 + 1;
    pad(segment, time)
        .center
        .lerp(pad(segment + 1, time).center, 0.5)
        + Vec3::new(0., 265., if segment % 4 == 1 { 115. } else { -115. })
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Frame {
    pub p: Vec3,
    pub v: Vec3,
    pub time: f32,
    pub hold: f32,
    pub support: Option<usize>,
    pub checkpoint: usize,
    pub gems: u16,
    pub falls: u8,
    pub complete: bool,
}
pub(super) struct Course {
    pub frame: Frame,
    pub previous: Frame,
    pub history: VecDeque<Frame>,
    pub rewinding: bool,
    pub rewind_left: usize,
    pub used_rewind: bool,
}
impl Default for Course {
    fn default() -> Self {
        let f = Frame {
            p: pad(0, 0.).center + Vec3::Y * RADIUS,
            v: Vec3::ZERO,
            time: 0.,
            hold: 0.55,
            support: Some(0),
            checkpoint: 0,
            gems: 0,
            falls: 0,
            complete: false,
        };
        Self {
            frame: f,
            previous: f,
            history: VecDeque::with_capacity(601),
            rewinding: false,
            rewind_left: 0,
            used_rewind: false,
        }
    }
}
pub(super) fn launch(from: Vec3, target: Vec3, duration: f32) -> Vec3 {
    let delta = target - from;
    Vec3::new(
        delta.x / duration,
        delta.y / duration + 0.5 * GRAVITY * duration,
        delta.z / duration,
    )
}
pub(super) fn landing(from: Vec3, to: Vec3, i: usize, time: f32) -> Option<f32> {
    let a = pad(i, time - STEP);
    let b = pad(i, time);
    let d0 = from.y - RADIUS - a.center.y;
    let d1 = to.y - RADIUS - b.center.y;
    if d0 < -0.01 || d1 > 0. || d1 >= d0 {
        return None;
    }
    let u = (d0 / (d0 - d1)).clamp(0., 1.);
    let p = from.lerp(to, u);
    let c = a.center.lerp(b.center, u);
    ((p.x - c.x).abs() < b.half.x + RADIUS * 0.7 && (p.z - c.z).abs() < b.half.y + RADIUS * 0.7)
        .then_some(u)
}
impl Course {
    pub fn rewind(&mut self) -> bool {
        if self.used_rewind || self.frame.complete || self.history.len() < 60 {
            return false;
        }
        self.used_rewind = true;
        self.rewinding = true;
        self.rewind_left = self.history.len().min(600);
        true
    }
    pub fn tick(&mut self, steer: Vec2) {
        self.previous = self.frame;
        if self.rewinding {
            if let Some(f) = self.history.pop_back() {
                self.frame = f;
            }
            self.rewind_left = self.rewind_left.saturating_sub(1);
            if self.rewind_left == 0 || self.history.is_empty() {
                self.rewinding = false;
            }
            return;
        }
        if self.frame.complete {
            return;
        }
        if self.history.len() == 600 {
            self.history.pop_front();
        }
        self.history.push_back(self.frame);
        let f = &mut self.frame;
        f.time += STEP;
        if let Some(i) = f.support {
            f.p = pad(i, f.time).center + Vec3::Y * RADIUS;
            f.hold -= STEP;
            if f.hold <= 0. {
                if i == COUNT - 1 {
                    f.complete = true;
                    return;
                }
                let flight = 1.65;
                let target = pad(i + 1, f.time + flight).center + Vec3::Y * RADIUS;
                f.v = launch(f.p, target, flight);
                f.support = None;
            }
        } else {
            // Steering changes the landing, but default flight follows the route.
            f.v.x += steer.x * 220. * STEP;
            f.v.z += steer.y * 220. * STEP;
            f.v.x = f.v.x.clamp(-400., 400.);
            f.v.z = f.v.z.clamp(-360., 360.);
            let from = f.p;
            f.p += f.v * STEP - Vec3::Y * (0.5 * GRAVITY * STEP * STEP);
            f.v.y -= GRAVITY * STEP;
            for i in COUNT..COUNT + 4 {
                let gem = gem_position(i, f.time);
                let d = f.p - from;
                let u = ((gem - from).dot(d) / d.length_squared().max(0.0001)).clamp(0., 1.);
                if (from + d * u).distance(gem) < 48. {
                    f.gems |= 1 << i;
                }
            }
            if let Some((i, u)) = (0..COUNT)
                .filter_map(|i| landing(from, f.p, i, f.time).map(|u| (i, u)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
            {
                f.p = from.lerp(f.p, u);
                f.p.y = pad(i, f.time).center.y + RADIUS;
                f.support = Some(i);
                f.hold = 0.32;
                f.v = Vec3::ZERO;
                f.checkpoint = f.checkpoint.max(i);
                f.gems |= 1 << i;
            }
            if f.p.y < -550. || f.p.z.abs() > 1000. || f.p.x < -600. || f.p.x > 3000. {
                f.falls = f.falls.saturating_add(1);
                f.p = pad(f.checkpoint, f.time).center + Vec3::Y * RADIUS;
                f.v = Vec3::ZERO;
                f.support = Some(f.checkpoint);
                f.hold = 0.6;
            }
        }
    }
    pub fn pose(&self, alpha: f32) -> Vec3 {
        self.previous.p.lerp(self.frame.p, alpha.clamp(0., 1.))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_route_reaches_every_pad_in_three_dimensions() {
        let mut c = Course::default();
        for _ in 0..3600 {
            c.tick(Vec2::ZERO);
            if c.frame.complete {
                break;
            }
        }
        assert!(c.frame.complete, "{:?}", c.frame);
        assert_eq!(c.frame.falls, 0);
        assert_eq!(c.frame.gems, 0b111111110);
        assert!(c.frame.time > 15. && c.frame.time < 25.);
    }
    #[test]
    fn rewind_restores_course_time_motion_and_rewards_without_duplicate_claims() {
        let mut c = Course::default();
        for _ in 0..650 {
            c.tick(Vec2::ZERO);
        }
        let before = c.frame;
        assert!(c.rewind());
        assert!(!c.rewind());
        for _ in 0..600 {
            c.tick(Vec2::ZERO);
        }
        assert!(c.frame.time < before.time - 4.9);
        assert!(c.frame.gems < before.gems);
        for _ in 0..600 {
            c.tick(Vec2::ZERO);
        }
        assert!(c.frame.p.distance(before.p) < 0.02);
        assert_eq!(c.frame.gems, before.gems);
    }
    #[test]
    fn sweep_lands_at_high_speed_and_rejects_wrong_depth() {
        assert!(
            landing(
                Vec3::new(290., 200., -130.),
                Vec3::new(290., -100., -130.),
                1,
                1.
            )
            .is_some()
        );
        assert!(
            landing(
                Vec3::new(290., 200., 600.),
                Vec3::new(290., -100., 600.),
                1,
                1.
            )
            .is_none()
        );
    }
    #[test]
    fn falling_recovers_at_checkpoint_without_fake_completion() {
        let mut c = Course::default();
        c.frame.support = None;
        c.frame.p.y = -600.;
        c.tick(Vec2::ZERO);
        assert_eq!(c.frame.falls, 1);
        assert_eq!(c.frame.support, Some(0));
        assert!(!c.frame.complete);
    }
}
