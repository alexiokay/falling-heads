//! Pure, world-coordinate formations. Chunk boundaries never restart a feature.
use super::catalog::Tile;
use crate::components::block::{BiomeType as B, BlockType};
use crate::systems::world_shapes::{hash, is_open};

/// Underground geodes have a solid calcite rind, crystal lining and a hollow core.
/// A small throat opens the rind; adjoining tunnels or mining expose the chamber.
pub(crate) fn geode(x: i32, y: i32, b: B, seed: u32) -> Option<Option<Tile>> {
    if b != B::Underground {
        return None;
    }
    let rx = x.div_euclid(48);
    let ry = y.div_euclid(40);
    let h = hash(rx, ry, seed ^ 0xabc501);
    if !h.is_multiple_of(3) {
        return None;
    }
    let cx = rx * 48 + 16 + (h % 15) as i32;
    let cy = ry * 40 + 14 + ((h >> 8) % 11) as i32;
    let dx = x - cx;
    let dy = y - cy;
    let r = (dx as f32 / 8.).powi(2) + (dy as f32 / 6.).powi(2);
    if r > 1. {
        return None;
    }
    if dy.abs() <= 1 && dx >= 3 {
        return Some(None);
    }
    Some(if r > 0.76 {
        Some(Tile::Calcite)
    } else if r > 0.48 {
        Some(Tile::Amethyst)
    } else {
        None
    })
}
pub(crate) fn open(x: i32, y: i32, b: B, seed: u32) -> bool {
    geode(x, y, b, seed).map_or_else(|| is_open(x, y, b, seed), |g| g.is_none())
}
/// Connected geological patches; ores retain their original identity.
pub(crate) fn solid(x: i32, y: i32, b: B, seed: u32, base: BlockType) -> Option<Tile> {
    if let Some(g) = geode(x, y, b, seed) {
        return g;
    }
    if !matches!(
        base,
        BlockType::Dirt
            | BlockType::Stone
            | BlockType::Sand
            | BlockType::Sandstone
            | BlockType::Netherrack
            | BlockType::Snow
            | BlockType::Ice
            | BlockType::EndStone
            | BlockType::SoulSand
    ) {
        return None;
    }
    if b == B::Plains && feature(x, y + 1, b, seed) == Some(Tile::Wheat) {
        return Some(Tile::Farmland);
    }
    let patch = patch(x, y, seed);
    let surface = open(x, y + 1, b, seed);
    match b {
        B::Plains if patch < 2 && base == BlockType::Stone => Some(Tile::MossyCobble),
        B::Forest if patch < 3 => Some(if surface {
            Tile::Moss
        } else {
            Tile::RootedDirt
        }),
        B::Mountains if patch < 4 => Some(Tile::Tuff),
        B::Mountains if patch == 4 => Some(Tile::Calcite),
        B::Underground => Some(if surface && hash(x, y, seed ^ 19).is_multiple_of(37) {
            Tile::Infested
        } else {
            Tile::Deepslate
        }),
        B::Nether if surface && patch < 3 => Some(if patch == 0 {
            Tile::WarpedSoil
        } else {
            Tile::CrimsonSoil
        }),
        B::Nether if surface && patch == 3 => Some(Tile::Magma),
        B::Nether if patch < 7 => Some(Tile::Blackstone),
        B::Nether if patch < 10 => Some(Tile::Basalt),
        B::Iceland if patch < 3 => Some(Tile::BlueIce),
        B::Iceland if patch < 7 => Some(Tile::PackedIce),
        B::Iceland if surface && patch == 7 => Some(Tile::PowderSnow),
        _ => None,
    }
}
/// Supported floor/ceiling decorations, with enough open space to read the silhouette.
pub(crate) fn decoration(x: i32, y: i32, b: B, seed: u32) -> Option<Tile> {
    if !open(x, y, b, seed) {
        return None;
    }
    let h = hash(x, y, seed ^ 0xf17e);
    let floor = !open(x, y - 1, b, seed);
    let clear = open(x, y + 1, b, seed) && open(x - 1, y, b, seed) && open(x + 1, y, b, seed);
    if b == B::Underground {
        if floor && geode(x, y - 1, b, seed) == Some(Some(Tile::Amethyst)) && h.is_multiple_of(3) {
            return Some(Tile::Crystal);
        }
        if floor && clear && h.is_multiple_of(11) {
            return Some(Tile::DripstoneUp);
        }
        if !open(x, y + 1, b, seed) && open(x, y - 1, b, seed) && h.is_multiple_of(11) {
            return Some(Tile::DripstoneDown);
        }
    }
    if !floor || !clear {
        return None;
    }
    match b {
        B::Plains if h.is_multiple_of(4) => Some(Tile::Grass),
        B::Forest if h.is_multiple_of(17) => Some(Tile::Berries),
        B::Forest if h.is_multiple_of(7) => Some(Tile::Fern),
        B::Forest if h.is_multiple_of(23) => Some(Tile::Azalea),
        B::Nether if h.is_multiple_of(13) => Some(if h & 1 == 0 {
            Tile::WarpedFungus
        } else {
            Tile::CrimsonFungus
        }),
        _ => None,
    }
}
/// Sparse small grounded features, kept separate from large landmark reservations.
/// Entire footprints need support. No floating farms or ruins between layers.
pub(crate) fn feature(x: i32, y: i32, b: B, seed: u32) -> Option<Tile> {
    if matches!(b, B::Underground | B::Iceland) {
        return None;
    }
    let region = x.div_euclid(24);
    let cx = region * 24 + 8 + (hash(region, 0, seed ^ 319) % 8) as i32;
    let dx = x - cx;
    if dx.abs() > 4 {
        return None;
    }
    // Permit stepped terrain, but build short footings down to solid ground.
    // The anchor is chosen independently of the requesting cell/chunk.
    for floor_y in (y - 5)..=(y + 3) {
        if !hash(region, floor_y, seed ^ 741).is_multiple_of(5) {
            continue;
        }
        if !open(cx, floor_y, b, seed) {
            continue;
        }
        let heights: Option<Vec<i32>> = (-4..=4)
            .map(|i| {
                if !open(cx + i, floor_y, b, seed) || !open(cx + i, floor_y + 1, b, seed) {
                    return None;
                }
                (1..=3).find(|d| !open(cx + i, floor_y - d, b, seed))
            })
            .collect();
        let Some(heights) = heights else {
            continue;
        };
        if !heights.contains(&1) {
            continue;
        }
        let dy = y - floor_y;
        if dy < 0 && dy > -heights[(dx + 4) as usize] {
            return Some(match b {
                B::Plains => Tile::Farmland,
                B::Desert => Tile::Bone,
                B::Nether => Tile::Blackstone,
                B::End => Tile::EndBrick,
                _ => Tile::MossyCobble,
            });
        }
        let tile = match b {
            B::Plains if dy == 0 && dx.abs() >= 2 => Some(Tile::Wheat),
            B::Plains if dy == 0 && dx.abs() <= 1 => Some(Tile::Hay),
            B::Plains if dy == 1 && dx == 0 => Some(Tile::Hay),
            B::Forest if dy == 0 && dx.abs() <= 3 => Some(Tile::MossyBrick),
            B::Forest if (0..3).contains(&dy) && dx == -3 => Some(Tile::MossyCobble),
            B::Desert if dy == 0 => Some(Tile::Bone),
            B::Desert if (0..=2).contains(&dy) && dx % 2 == 0 => Some(Tile::Bone),
            B::Nether if (0..2 + (4 - dx.abs()) / 2).contains(&dy) && dx % 2 == 0 => {
                Some(Tile::Basalt)
            }
            B::End if dy == 0 => Some(Tile::EndBrick),
            B::End if (0..=4).contains(&dy) && dx.abs() == 3 => Some(Tile::PurpurPillar),
            B::Mountains if (0..3).contains(&dy) && dx == 0 => Some(Tile::Calcite),
            _ => None,
        };
        if tile.is_some() {
            return tile;
        }
    }
    None
}

/// Large landmarks retain their layouts but now use the local building palette.
pub(crate) fn structure(b: B, p: bevy::prelude::IVec2, kind: BlockType) -> Option<Tile> {
    match (b, kind) {
        (B::Plains, BlockType::Stone) => Some(Tile::MossyCobble),
        (B::Forest, BlockType::Stone) => Some(Tile::MossyBrick),
        (B::Forest, BlockType::Leaves) if p.y > 4 => Some(Tile::Azalea),
        (B::Mountains, BlockType::Stone) => Some(if p.x.abs() < 3 {
            Tile::Calcite
        } else {
            Tile::Tuff
        }),
        (B::Underground, BlockType::Stone) => Some(Tile::Deepslate),
        (B::Nether, BlockType::Obsidian | BlockType::Netherrack) => Some(if p.x.abs() > 3 {
            Tile::Basalt
        } else {
            Tile::Blackstone
        }),
        (B::End, BlockType::Purpur | BlockType::EndStone) => Some(if p.x.abs() == 5 {
            Tile::PurpurPillar
        } else {
            Tile::EndBrick
        }),
        (B::Iceland, BlockType::Ice) => Some(if p.y < 0 {
            Tile::BlueIce
        } else {
            Tile::PackedIce
        }),
        _ => None,
    }
}

// Interpolated seeded material field avoids checkerboard-shaped geological patches.
fn patch(x: i32, y: i32, seed: u32) -> u32 {
    let x = x as f32 / 11.;
    let y = y as f32 / 9.;
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let smooth = |v: f32| v * v * (3. - 2. * v);
    let u = smooth(x - x.floor());
    let v = smooth(y - y.floor());
    let n = |dx, dy| (hash(ix + dx, iy + dy, seed ^ 12309) % 1000) as f32 / 999.;
    let a = n(0, 0) * (1. - u) + n(1, 0) * u;
    let b = n(0, 1) * (1. - u) + n(1, 1) * u;
    ((a * (1. - v) + b * v) * 13.).min(12.) as u32
}
pub(crate) fn damp_grove(x: i32, y: i32, seed: u32) -> bool {
    patch(x, y, seed) < 3
}
