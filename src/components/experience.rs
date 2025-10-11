use bevy::prelude::*;

/// Experience component attached to the hero
#[derive(Component, Debug)]
pub struct Experience {
    pub current_xp: u32,
    pub level: u32,
    pub xp_to_next_level: u32,
}

impl Default for Experience {
    fn default() -> Self {
        Self {
            current_xp: 0,
            level: 1,
            xp_to_next_level: Self::calculate_xp_for_level(2),
        }
    }
}

impl Experience {
    /// Calculate XP required to reach a specific level
    /// Uses exponential scaling: level^2 * 50
    pub fn calculate_xp_for_level(level: u32) -> u32 {
        level * level * 50
    }

    /// Add XP and check for level up
    /// Returns true if leveled up
    pub fn add_xp(&mut self, amount: u32) -> bool {
        self.current_xp += amount;

        // Check for level up
        if self.current_xp >= self.xp_to_next_level {
            self.level += 1;
            self.current_xp -= self.xp_to_next_level;
            self.xp_to_next_level = Self::calculate_xp_for_level(self.level + 1);
            true
        } else {
            false
        }
    }

    /// Get progress towards next level (0.0 to 1.0)
    pub fn get_progress(&self) -> f32 {
        self.current_xp as f32 / self.xp_to_next_level as f32
    }
}

/// Marker component for the XP bar UI
#[derive(Component)]
pub struct XpBar;

/// Marker component for the XP bar progress fill
#[derive(Component)]
pub struct XpBarProgress;

/// Marker component for the level text
#[derive(Component)]
pub struct LevelText;
