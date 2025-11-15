use crate::components::block::BiomeType;
use crate::components::chunk::world_to_chunk_coords;

/// Centralized biome determination logic
/// This SINGLE SOURCE OF TRUTH is used by both world generation and background rendering
/// to ensure biomes are always calculated consistently.
///
/// To change how biomes are distributed, modify this function ONLY.
/// Both blocks and backgrounds will automatically use the new logic.
pub fn determine_biome_at_position(x: f32, y: f32) -> BiomeType {
    // Convert world coordinates to chunk coordinates
    // TOTAL_BLOCK_SIZE * 64.0 = 1.0 * 64.0 = 64.0
    let (_, chunk_y) = world_to_chunk_coords(x, y, 64.0);

    // Biomes change every 10 chunks vertically (~640 blocks) - increased from 4 for longer biomes
    // All chunks at same Y level have same biome (consistent horizontal layers)
    let biome_zone = chunk_y / 10;
    let biome_seed = (biome_zone * 179) % 100;

    // Randomly pick biome based on this vertical zone (8 biomes)
    match biome_seed.abs() % 8 {
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

/// Determine biome from chunk coordinates (for chunk generation)
pub fn determine_biome_at_chunk(_chunk_x: i32, chunk_y: i32) -> BiomeType {
    // Same logic as position-based, but using chunk coordinates directly
    let biome_zone = chunk_y / 10;  // Match the 10 chunks from determine_biome_at_position
    let biome_seed = (biome_zone * 179) % 100;

    match biome_seed.abs() % 8 {
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
