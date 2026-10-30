use crate::components::block::BiomeType;
use bevy::prelude::*;

/// Represents a chunk of the world (16x16 blocks)
#[derive(Component)]
pub struct Chunk {
    pub chunk_x: i32, // Chunk coordinate in X axis
    pub chunk_y: i32, // Chunk coordinate in Y axis
    pub biome: BiomeType,
}

/// Marker component for blocks that belong to a specific chunk
#[derive(Component, Clone, Copy)]
pub struct ChunkMember {
    pub chunk_x: i32,
    pub chunk_y: i32,
}

impl ChunkMember {
    /// Chunk containing a world position, so the entity unloads with that chunk.
    pub fn at(position: Vec3) -> Self {
        let size = CHUNK_SIZE_F32 * crate::utils::constants::BLOCK_SIZE;
        Self {
            chunk_x: (position.x / size).floor() as i32,
            chunk_y: (position.y / size).floor() as i32,
        }
    }
}

/// Resource to track which chunks are currently loaded
#[derive(Resource, Default)]
pub struct LoadedChunks {
    pub chunks: std::collections::HashSet<(i32, i32)>,
}

impl LoadedChunks {
    pub fn is_loaded(&self, chunk_x: i32, chunk_y: i32) -> bool {
        self.chunks.contains(&(chunk_x, chunk_y))
    }

    pub fn add(&mut self, chunk_x: i32, chunk_y: i32) {
        self.chunks.insert((chunk_x, chunk_y));
    }

    pub fn remove(&mut self, chunk_x: i32, chunk_y: i32) {
        self.chunks.remove(&(chunk_x, chunk_y));
    }
}

pub const CHUNK_SIZE: i32 = 16;
pub const CHUNK_SIZE_F32: f32 = 16.0;

/// Convert world position to chunk coordinates
pub fn world_to_chunk_coords(world_x: f32, world_y: f32, block_size: f32) -> (i32, i32) {
    let block_x = (world_x / block_size).floor() as i32;
    let block_y = (world_y / block_size).floor() as i32;

    (
        block_x.div_euclid(CHUNK_SIZE),
        block_y.div_euclid(CHUNK_SIZE),
    )
}
