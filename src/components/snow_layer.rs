use bevy::prelude::*;

/// Snow accumulation on block surfaces - 4 stages
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct SnowLayer {
    pub stage: u8,  // 0 = none, 1 = light dusting, 2 = partial, 3 = mostly covered, 4 = fully covered
}

impl SnowLayer {
    pub fn new() -> Self {
        Self { stage: 0 }
    }

    /// Add snow (called when snowflake hits block)
    pub fn add_snow(&mut self) -> bool {
        if self.stage < 4 {
            self.stage += 1;
            true  // Changed
        } else {
            false  // Already max
        }
    }

    /// Remove snow (melting)
    pub fn melt(&mut self) -> bool {
        if self.stage > 0 {
            self.stage -= 1;
            true  // Changed
        } else {
            false  // Already clear
        }
    }

    /// Check if block has any snow
    pub fn has_snow(&self) -> bool {
        self.stage > 0
    }

    /// Get alpha for visual overlay (0.0 = invisible, 1.0 = opaque)
    pub fn get_alpha(&self) -> f32 {
        match self.stage {
            0 => 0.0,
            1 => 0.25,  // Light dusting
            2 => 0.50,  // Partial
            3 => 0.75,  // Mostly covered
            4 => 1.0,   // Fully covered
            _ => 0.0,
        }
    }
}

impl Default for SnowLayer {
    fn default() -> Self {
        Self::new()
    }
}
