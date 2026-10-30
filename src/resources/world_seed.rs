use bevy::prelude::*;

/// World generation seed - generated randomly at game start for unique worlds
#[derive(Resource)]
pub struct WorldSeed {
    pub seed: u32,
}

impl Default for WorldSeed {
    fn default() -> Self {
        use rand::RngExt;
        Self {
            seed: rand::rng().random_range(0..u32::MAX),
        }
    }
}

impl WorldSeed {
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    /// Get seed for Perlin noise generator
    pub fn perlin_seed(&self) -> u32 {
        self.seed
    }

    /// Get seed for Simplex noise generator
    pub fn simplex_seed(&self) -> u32 {
        self.seed.wrapping_add(12345)
    }
}
