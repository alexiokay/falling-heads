//! Pure seeded terrain geometry, shared by the game and windowless preview.
use crate::components::block::BiomeType;
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use std::cell::RefCell;
thread_local! {static FIELD:RefCell<(u32,Perlin)>=RefCell::new((0,Perlin::new(0)));}
pub(crate) fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(73856093) ^ (y as u32).wrapping_mul(19349663) ^ seed;
    h = (h ^ (h >> 16)).wrapping_mul(0x7feb352d);
    h ^ (h >> 15)
}
fn center(cell: IVec2, seed: u32, forest: bool) -> (Vec2, Vec2) {
    let h = hash(cell.x, cell.y, seed);
    let step = if forest {
        Vec2::new(58., 38.)
    } else {
        Vec2::new(36., 30.)
    };
    let jitter = Vec2::new((h % 17) as f32 - 8., ((h >> 8) % 13) as f32 - 6.);
    let radius = if forest {
        Vec2::new(20. + ((h >> 16) % 10) as f32, 10. + ((h >> 24) % 6) as f32)
    } else {
        Vec2::new(6. + ((h >> 16) % 8) as f32, 4. + ((h >> 24) % 6) as f32)
    };
    (cell.as_vec2() * step + step * 0.5 + jitter, radius)
}
fn segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.length_squared().max(0.01)).clamp(0., 1.);
    p.distance(a + d * t)
}
fn network(p: Vec2, seed: u32, forest: bool, n: &Perlin) -> bool {
    let step = if forest {
        Vec2::new(58., 38.)
    } else {
        Vec2::new(36., 30.)
    };
    let cell = (p / step).floor().as_ivec2();
    for dx in -1..=1 {
        for dy in -1..=1 {
            let c = cell + IVec2::new(dx, dy);
            let (a, r) = center(c, seed, forest);
            let q = (p - a) / r;
            let room = if forest {
                // Broad clearings have rolling footings and arched ceilings, not tiled floors.
                let floor = a.y - r.y + (n.get([p.x as f64 * 0.045, c.y as f64, 61.]) * 1.6) as f32;
                q.x.abs() < 1. && p.y > floor && p.y < a.y + r.y * (1. - q.x * q.x).max(0.).sqrt()
            } else {
                q.length_squared() < 1.
            };
            if room {
                return true;
            }
            for offset in [
                IVec2::X,
                IVec2::new(
                    if hash(c.x, c.y, seed ^ 71) % 2 == 0 {
                        0
                    } else {
                        1
                    },
                    -1,
                ),
            ] {
                let (b, _) = center(c + offset, seed, forest);
                let h = hash(c.x, c.y, seed ^ 93);
                let bend = Vec2::new(((h >> 5) % 11) as f32 - 5., ((h >> 13) % 13) as f32 - 6.);
                let midpoint = (a + b) * 0.5 + bend;
                let radius = if forest {
                    if offset == IVec2::X { 3.5 } else { 2. }
                } else {
                    2. + (h % 3) as f32 * 0.5
                };
                if segment(p, a, midpoint).min(segment(p, midpoint, b)) < radius {
                    return true;
                }
            }
        }
    }
    false
}
pub(crate) fn is_open(x: i32, y: i32, b: BiomeType, seed: u32) -> bool {
    FIELD.with(|field| {
        let mut field = field.borrow_mut();
        if field.0 != seed {
            *field = (seed, Perlin::new(seed));
        }
        let n = &field.1;
        let x = x as f64;
        let y = y as f64;
        let warp = n.get([x * 0.009, y * 0.009, 5.]) * 11.;
        let broad = n.get([(x + warp) * 0.026, y * 0.043, 17.]);
        let detail = n.get([x * 0.064, (y + warp) * 0.052, 31.]);
        match b {
            BiomeType::Plains => broad + detail * 0.16 > 0.015,
            BiomeType::Forest => network(Vec2::new(x as f32, y as f32), seed, true, n),
            BiomeType::Underground => network(Vec2::new(x as f32, y as f32), seed, false, n),
            BiomeType::Desert => {
                n.get([(x + warp) * 0.018, y * 0.043, 49.]) + detail * 0.10 > -0.05
            }
            BiomeType::Mountains => {
                n.get([(x + y * 0.36) * 0.034, (y - x * 0.15) * 0.043, 67.])
                    .abs()
                    < 0.17
                    || broad > 0.28
            }
            BiomeType::Nether => {
                n.get([(x + warp) * 0.026, (y + warp) * 0.029, 81.]) > -0.09 || detail.abs() < 0.07
            }
            BiomeType::End => broad + detail * 0.23 < 0.18,
            BiomeType::Iceland => {
                n.get([(x + y * 0.6) * 0.045, y * 0.025, 101.]).abs() < 0.14 || broad > 0.24
            }
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn biome_geometry_is_seeded_diverse_and_not_sixteen_tile_shelves() {
        for b in [
            BiomeType::Plains,
            BiomeType::Forest,
            BiomeType::Underground,
            BiomeType::Desert,
            BiomeType::Mountains,
            BiomeType::Nether,
            BiomeType::End,
            BiomeType::Iceland,
        ] {
            let mut air = 0;
            let mut shifted = 0;
            for x in 900..1028 {
                for y in -128..0 {
                    let v = is_open(x, y, b, 921);
                    air += v as usize;
                    shifted += (v != is_open(x, y + 16, b, 921)) as usize;
                    assert_eq!(v, is_open(x, y, b, 921));
                }
            }
            assert!(air > 2200 && air < 14500, "Air/solid balance {b:?}: {air}");
            assert!(shifted > 1800, "Repeated shelves in {b:?}");
        }
    }
}
