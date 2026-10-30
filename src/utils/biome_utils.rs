use crate::components::block::BiomeType;
use crate::components::chunk::world_to_chunk_coords;

/// Centralized biome determination logic
/// This SINGLE SOURCE OF TRUTH is used by both world generation and background rendering
/// to ensure biomes are always calculated consistently.
///
/// To change how biomes are distributed, modify this function ONLY.
/// Both blocks and backgrounds will automatically use the new logic.
pub fn determine_biome_at_position(x: f32, y: f32) -> BiomeType {
    let (cx, cy) = world_to_chunk_coords(x, y, crate::utils::constants::BLOCK_SIZE);
    determine_biome_at_chunk(cx, cy)
}

/// Determine biome from chunk coordinates (for chunk generation)
pub fn determine_biome_at_chunk(_chunk_x: i32, chunk_y: i32) -> BiomeType {
    // A repeating seeded sequence with room for forests to develop. Euclidean
    // wrapping keeps negative coordinates identical to positive band boundaries.
    let mut position = chunk_y.rem_euclid(1130);
    for zone in 0..100 {
        let biome = zone_biome(zone);
        let width = if biome == BiomeType::Forest { 20 } else { 10 };
        if position < width {
            return biome;
        }
        position -= width;
    }
    unreachable!("The 100-zone cycle contains thirteen forests")
}
fn zone_biome(zone: i32) -> BiomeType {
    match ((zone * 179) % 100) % 8 {
        0 => BiomeType::Plains,
        1 => BiomeType::Forest,
        2 => BiomeType::Desert,
        3 => BiomeType::Mountains,
        4 => BiomeType::Underground,
        5 => BiomeType::Nether,
        6 => BiomeType::End,
        _ => BiomeType::Iceland,
    }
}

/// Find the nearest matching zone interior using the generator's actual mapping.
pub fn nearest_biome_y(current_y: f32, biome: BiomeType) -> f32 {
    use crate::{components::chunk::CHUNK_SIZE, utils::constants::BLOCK_SIZE};
    let chunk_world = CHUNK_SIZE as f32 * BLOCK_SIZE;
    let current_chunk = (current_y / chunk_world).floor() as i32;
    (-1200..=1200)
        .map(|offset| current_chunk + offset)
        .filter(|&cy| {
            determine_biome_at_chunk(0, cy) == biome && determine_biome_at_chunk(0, cy - 1) != biome
        })
        .map(|start| {
            let height = (1..=100)
                .find(|dy| determine_biome_at_chunk(0, start + dy) != biome)
                .unwrap();
            // Enter in the upper third: falling leaves most of the biome ahead.
            (start as f32 + height as f32 * 0.70) * chunk_world
        })
        .min_by(|a, b| (a - current_y).abs().total_cmp(&(b - current_y).abs()))
        .expect("Every biome occurs in the bounded 100-zone cycle")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forests_have_twenty_chunks_and_entry_leaves_room_to_descend() {
        for y in [-500000., -130000., 0., 130000., 500000.] {
            let target = nearest_biome_y(y, BiomeType::Forest);
            let cy = (target / 1024.).floor() as i32;
            let mut lo = cy;
            let mut hi = cy;
            while determine_biome_at_chunk(0, lo - 1) == BiomeType::Forest {
                lo -= 1;
            }
            while determine_biome_at_chunk(0, hi + 1) == BiomeType::Forest {
                hi += 1;
            }
            assert_eq!(hi - lo + 1, 20);
            assert!(
                cy - lo >= 12,
                "Forest entry is too close to the downward exit"
            );
        }
    }
    #[test]
    fn every_teleport_target_matches_generator_at_positive_and_negative_heights() {
        for y in [-130000.0, -4096.0, 0.0, 130000.0] {
            for biome in [
                BiomeType::Plains,
                BiomeType::Forest,
                BiomeType::Desert,
                BiomeType::Mountains,
                BiomeType::Underground,
                BiomeType::Nether,
                BiomeType::End,
                BiomeType::Iceland,
            ] {
                let target = nearest_biome_y(y, biome);
                assert_eq!(determine_biome_at_position(0.0, target), biome);
                assert_eq!(determine_biome_at_position(0.0, target - 1024.0), biome);
                assert_eq!(determine_biome_at_position(0.0, target + 1024.0), biome);
            }
        }
    }
}
