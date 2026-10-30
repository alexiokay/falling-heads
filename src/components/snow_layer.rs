use bevy::prelude::*;

/// Snow accumulation on block surfaces - 4 stages
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct SnowLayer {
    pub stage: u8, // 0 = none, 1 = light dusting, 2 = partial, 3 = mostly covered, 4 = fully covered
}

impl SnowLayer {
    pub fn new() -> Self {
        Self { stage: 0 }
    }

    /// Add snow (called when snowflake hits block)
    pub fn add_snow(&mut self) -> bool {
        if self.stage < 4 {
            self.stage += 1;
            true // Changed
        } else {
            false // Already max
        }
    }

    /// Remove snow (melting)
    pub fn melt(&mut self) -> bool {
        if self.stage > 0 {
            self.stage -= 1;
            true // Changed
        } else {
            false // Already clear
        }
    }

    /// Check if block has any snow
    pub fn has_snow(&self) -> bool {
        self.stage > 0
    }

    /// Thickness in world units; the block itself is 64 units high.
    pub fn thickness(&self) -> f32 {
        self.stage.min(4) as f32 * 2.0
    }
}

impl Default for SnowLayer {
    fn default() -> Self {
        Self::new()
    }
}
