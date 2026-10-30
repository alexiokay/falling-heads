use bevy::prelude::*;
use std::collections::HashMap;

/// Chunk-based spatial partitioning for efficient collision detection
#[derive(Resource, Default)]
pub struct SpatialHash {
    /// Map of chunk coordinates to entities in that chunk
    pub chunks: HashMap<(i32, i32), Vec<Entity>>,
    /// Size of each spatial chunk (in world units)
    pub chunk_size: f32,
    /// Cell each entity is stored in, so moves and removals touch one cell only
    cell_of: HashMap<Entity, (i32, i32)>,
}

impl SpatialHash {
    pub fn new(chunk_size: f32) -> Self {
        Self {
            chunks: HashMap::new(),
            chunk_size,
            cell_of: HashMap::new(),
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
        self.cell_of.clear();
    }

    /// Insert an entity at a world position, or move it there if already stored
    pub fn insert(&mut self, entity: Entity, x: f32, y: f32) {
        let chunk = self.world_to_chunk(x, y);
        if let Some(old) = self.cell_of.insert(entity, chunk) {
            if old == chunk {
                return;
            }
            self.remove_from_cell(entity, old);
        }
        self.chunks.entry(chunk).or_default().push(entity);
    }

    /// Remove an entity (despawned or no longer a block)
    pub fn remove(&mut self, entity: Entity) {
        if let Some(cell) = self.cell_of.remove(&entity) {
            self.remove_from_cell(entity, cell);
        }
    }

    fn remove_from_cell(&mut self, entity: Entity, cell: (i32, i32)) {
        if let Some(entities) = self.chunks.get_mut(&cell) {
            entities.retain(|e| *e != entity);
            if entities.is_empty() {
                self.chunks.remove(&cell);
            }
        }
    }

    /// Get all entities in nearby chunks (3x3 grid around position)
    pub fn query_nearby(&self, x: f32, y: f32) -> Vec<Entity> {
        let (cx, cy) = self.world_to_chunk(x, y);
        let mut results = Vec::new();

        // Check 3x3 grid of chunks - use saturating math to prevent overflow
        for dx in -1..=1 {
            for dy in -1..=1 {
                let chunk_x = cx.saturating_add(dx);
                let chunk_y = cy.saturating_add(dy);
                let chunk = (chunk_x, chunk_y);
                if let Some(entities) = self.chunks.get(&chunk) {
                    results.extend_from_slice(entities);
                }
            }
        }

        results
    }

    /// Borrow candidates without allocating a temporary vector per actor/frame.
    pub fn nearby_iter(&self, x: f32, y: f32) -> impl Iterator<Item = Entity> + '_ {
        let (cx, cy) = self.world_to_chunk(x, y);
        (-1..=1).flat_map(move |dx| {
            (-1..=1).flat_map(move |dy| {
                self.chunks
                    .get(&(cx.saturating_add(dx), cy.saturating_add(dy)))
                    .into_iter()
                    .flatten()
                    .copied()
            })
        })
    }
    pub fn rect_iter(&self, min: Vec2, max: Vec2) -> impl Iterator<Item = Entity> + '_ {
        let (x0, y0) = self.world_to_chunk(min.x, min.y);
        let (x1, y1) = self.world_to_chunk(max.x, max.y);
        (x0..=x1).flat_map(move |x| {
            (y0..=y1).flat_map(move |y| self.chunks.get(&(x, y)).into_iter().flatten().copied())
        })
    }
}
