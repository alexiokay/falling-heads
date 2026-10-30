//! Continuous XY terrain contacts. Rectangles are expanded by the actor's half-size,
//! then its centre is swept to the earliest face (including simultaneous corner hits).
use bevy::prelude::*;

const SKIN: f32 = 1.0;
const EPS: f32 = 0.00001;

#[derive(Clone, Copy)]
pub(super) struct Bounds {
    pub min: Vec2,
    pub max: Vec2,
}
impl Bounds {
    pub(super) fn contains(self, p: Vec2) -> bool {
        p.cmpgt(self.min).all() && p.cmplt(self.max).all()
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Contact {
    pub index: usize,
    pub normal: Vec2,
}

fn sweep(start: Vec2, delta: Vec2, b: Bounds) -> Option<(f32, Vec2)> {
    let mut enter = f32::NEG_INFINITY;
    let mut leave = f32::INFINITY;
    let mut normal = Vec2::ZERO;
    for axis in 0..2 {
        if delta[axis].abs() < EPS {
            // Sliding along an exact edge is not penetration.
            if start[axis] <= b.min[axis] || start[axis] >= b.max[axis] {
                return None;
            }
            continue;
        }
        let a = (b.min[axis] - start[axis]) / delta[axis];
        let z = (b.max[axis] - start[axis]) / delta[axis];
        let near = a.min(z);
        leave = leave.min(a.max(z));
        let mut n = Vec2::ZERO;
        n[axis] = -delta[axis].signum();
        if near > enter + EPS {
            enter = near;
            normal = n;
        } else if (near - enter).abs() <= EPS {
            normal += n;
        }
    }
    if enter >= -EPS && enter <= 1.0 && leave > enter.max(0.) {
        Some((enter.max(0.), normal))
    } else {
        None
    }
}

/// Recover spawn/growth overlaps against the union of blocks, not one block at a
/// time. This avoids pushing out of one tile and straight into its neighbour.
fn recover(p: Vec2, bounds: &[Bounds]) -> Option<(Vec2, Vec2)> {
    if !bounds.iter().any(|b| b.contains(p)) {
        return None;
    }
    let mut best = Vec2::splat(f32::INFINITY);
    let mut best_normal = Vec2::ZERO;
    for axis in 0..2 {
        for sign in [-1., 1.] {
            let mut q = p;
            // Each pass crosses at least one boundary; termination is bounded.
            for _ in 0..=bounds.len() {
                let mut next = q[axis];
                for b in bounds {
                    if b.contains(q) {
                        next = if sign < 0. {
                            next.min(b.min[axis] - SKIN)
                        } else {
                            next.max(b.max[axis] + SKIN)
                        };
                    }
                }
                if next == q[axis] {
                    break;
                }
                q[axis] = next;
            }
            if q.distance_squared(p) < best.distance_squared(p) {
                best = q;
                best_normal = Vec2::ZERO;
                best_normal[axis] = sign;
            }
        }
    }
    Some((best, best_normal))
}

pub(super) fn resolve_into(
    start: Vec2,
    end: Vec2,
    bounds: &[Bounds],
    contacts: &mut Vec<Contact>,
) -> Vec2 {
    let mut p = start;
    contacts.clear();
    let mut remaining = end - start;
    if let Some((outside, normal)) = recover(start, bounds) {
        for (index, b) in bounds.iter().enumerate() {
            if b.contains(start) {
                contacts.push(Contact { index, normal });
            }
        }
        p = outside;
        // Recovery itself must not add movement through another surface.
        for axis in 0..2 {
            if remaining[axis] * normal[axis] < 0. {
                remaining[axis] = 0.;
            }
        }
    }
    for _ in 0..4 {
        if remaining.length_squared() < EPS * EPS {
            break;
        }
        let earliest = bounds
            .iter()
            .filter_map(|b| sweep(p, remaining, *b))
            .map(|(t, _)| t)
            .min_by(f32::total_cmp);
        let Some(t) = earliest else {
            p += remaining;
            break;
        };
        let mut normal = Vec2::ZERO;
        for (index, b) in bounds.iter().enumerate() {
            if let Some((hit, n)) = sweep(p, remaining, *b) {
                if (hit - t).abs() <= EPS {
                    contacts.push(Contact { index, normal: n });
                    normal += n;
                }
            }
        }
        p += remaining * t;
        remaining *= 1. - t;
        for axis in 0..2 {
            if normal[axis] != 0. {
                p[axis] += normal[axis].signum() * SKIN;
                remaining[axis] = 0.;
            }
        }
    }
    p
}

#[cfg(test)]
pub(super) fn resolve(start: Vec2, end: Vec2, bounds: &[Bounds]) -> (Vec2, Vec<Contact>) {
    let mut contacts = Vec::new();
    let position = resolve_into(start, end, bounds, &mut contacts);
    (position, contacts)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn tile(x: f32, y: f32, half: f32) -> Bounds {
        Bounds {
            min: Vec2::new(x, y) - Vec2::splat(32. + half),
            max: Vec2::new(x, y) + Vec2::splat(32. + half),
        }
    }
    #[test]
    fn fast_motion_stops_at_entry_face_even_after_crossing_entire_tile() {
        for end in [10., 63., 160., 700.] {
            let (p, hits) = resolve(
                Vec2::new(-100., 0.),
                Vec2::new(end, 0.),
                &[tile(0., 0., 32.)],
            );
            assert_eq!(p, Vec2::new(-65., 0.));
            assert_eq!(hits.len(), 1);
        }
        let (p, _) = resolve(
            Vec2::new(0., 200.),
            Vec2::new(0., -700.),
            &[tile(0., 0., 32.)],
        );
        assert_eq!(p, Vec2::new(0., 65.));
    }
    #[test]
    fn diagonal_corner_and_adjacent_tiles_resolve_all_faces() {
        let blocks = [tile(0., 0., 32.), tile(64., 0., 32.)];
        let (p, hits) = resolve(Vec2::new(32., 200.), Vec2::new(32., -100.), &blocks);
        assert_eq!(p.y, 65.);
        assert_eq!(hits.len(), 2);
        let (p, _) = resolve(Vec2::splat(-150.), Vec2::splat(100.), &[tile(0., 0., 32.)]);
        assert_eq!(p, Vec2::splat(-65.));
    }
    #[test]
    fn wall_slide_hits_floor_but_never_damages_tiles_behind_wall() {
        let blocks = [
            tile(0., 64., 32.),
            tile(0., 0., 32.),
            tile(-64., -64., 32.),
            tile(64., 64., 32.),
        ];
        let (p, hits) = resolve(Vec2::new(-100., 100.), Vec2::new(200., -100.), &blocks);
        assert!(p.x <= -65. && p.y >= 1.);
        assert!(!hits.iter().any(|h| h.index == 3));
        assert!(!blocks.iter().any(|b| b.contains(p)));
    }
    #[test]
    fn growth_overlap_escapes_whole_wall_and_outgoing_motion_does_not_hit() {
        let blocks = [tile(0., 0., 40.), tile(64., 0., 40.), tile(128., 0., 40.)];
        let (p, _) = resolve(Vec2::new(32., 50.), Vec2::new(32., 45.), &blocks);
        assert_eq!(p.y, 73.);
        assert!(!blocks.iter().any(|b| b.contains(p)));
        let (p, hits) = resolve(
            Vec2::new(-64., 0.),
            Vec2::new(-100., 0.),
            &[tile(0., 0., 32.)],
        );
        assert_eq!(p.x, -100.);
        assert!(hits.is_empty());
    }
}
