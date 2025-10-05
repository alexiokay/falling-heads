use bevy::prelude::*;
use std::collections::HashMap;

/// Chunk-based spatial partitioning for efficient collision detection
#[derive(Resource, Default)]
pub struct SpatialHash {
    /// Map of chunk coordinates to entities in that chunk
    pub chunks: HashMap<(i32, i32), Vec<Entity>>,
    /// Size of each spatial chunk (in world units)
    pub chunk_size: f32,
}

impl SpatialHash {
    pub fn new(chunk_size: f32) -> Self {
        Self {
            chunks: HashMap::new(),
            chunk_size,
        }
    }

    /// Convert world position to chunk coordinates
    pub fn world_to_chunk(&self, x: f32, y: f32) -> (i32, i32) {
        (
            (x / self.chunk_size).floor() as i32,
            (y / self.chunk_size).floor() as i32,
        )
    }

    /// Clear all entries (call each frame before rebuilding)
    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    /// Insert an entity at a world position
    pub fn insert(&mut self, entity: Entity, x: f32, y: f32) {
        let chunk = self.world_to_chunk(x, y);
        self.chunks.entry(chunk).or_insert_with(Vec::new).push(entity);
    }

    /// Get all entities in nearby chunks (3x3 grid around position)
    pub fn query_nearby(&self, x: f32, y: f32) -> Vec<Entity> {
        let (cx, cy) = self.world_to_chunk(x, y);
        let mut results = Vec::new();

        // Check 3x3 grid of chunks
        for dx in -1..=1 {
            for dy in -1..=1 {
                let chunk = (cx + dx, cy + dy);
                if let Some(entities) = self.chunks.get(&chunk) {
                    results.extend_from_slice(entities);
                }
            }
        }

        results
    }

    /// Get entities in a specific chunk
    pub fn get_chunk(&self, chunk_x: i32, chunk_y: i32) -> Option<&Vec<Entity>> {
        self.chunks.get(&(chunk_x, chunk_y))
    }
}
