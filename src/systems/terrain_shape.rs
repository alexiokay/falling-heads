//! Shared projectile/line-of-sight silhouette in world XY. Heads keep their existing
//! terrain response; pass-through decorations never become invisible square walls.
use super::biome_content::Tile;
use crate::components::block::BlockType;
use bevy::prelude::*;
#[path = "terrain_silhouettes.rs"]
mod masks;
pub(crate) fn hit(
    from: Vec2,
    to: Vec2,
    center: Vec2,
    kind: BlockType,
    tile: Option<&Tile>,
) -> Option<f32> {
    if kind == BlockType::Cobweb || super::plant_rules::soft(kind, tile) {
        return None;
    }
    let masks = match tile {
        Some(Tile::DripstoneUp) => Some(masks::POINTED_DRIPSTONE_UP_TIP),
        Some(Tile::DripstoneDown) => Some(masks::POINTED_DRIPSTONE_DOWN_TIP),
        Some(Tile::Crystal) => Some(masks::AMETHYST_CLUSTER),
        _ => None,
    };
    let half = if kind == BlockType::Cactus {
        Vec2::new(28., 32.)
    } else {
        Vec2::splat(32.)
    };
    let broad = super::projectile_system::segment_box(from, to, center, half)?;
    let Some(rows) = masks else {
        return Some(broad);
    };
    rows.iter()
        .filter_map(|r| {
            let min = Vec2::new(r[0], r[1]);
            let max = Vec2::new(r[2], r[3]);
            super::projectile_system::segment_box(
                from,
                to,
                center + (min + max) * 0.5,
                (max - min) * 0.5,
            )
        })
        .min_by(f32::total_cmp)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arrow_misses_transparent_dripstone_corners_but_hits_stone() {
        let from = Vec2::new(-40., -8.);
        let to = Vec2::new(-15., -8.);
        assert!(
            hit(
                from,
                to,
                Vec2::ZERO,
                BlockType::DeadBush,
                Some(&Tile::DripstoneDown)
            )
            .is_none()
        );
        assert!(
            hit(
                from,
                Vec2::new(40., -8.),
                Vec2::ZERO,
                BlockType::DeadBush,
                Some(&Tile::DripstoneDown)
            )
            .is_some()
        );
        assert!(hit(from, to, Vec2::ZERO, BlockType::Stone, None).is_some());
        assert!(hit(from, to, Vec2::ZERO, BlockType::DeadBush, Some(&Tile::Fern)).is_none());
    }
}
