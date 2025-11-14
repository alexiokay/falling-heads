use bevy::prelude::*;

/// Tracks the "center of action" for dynamic camera focusing
/// Calculates a weighted position based on hero, combat, and other gameplay elements
#[derive(Resource)]
pub struct ActionCenter {
    /// The calculated center point where action is happening (smoothed)
    pub center: Vec3,
    /// The raw calculated center (before smoothing)
    pub raw_center: Vec3,
    /// Combat intensity (0.0 = no combat, 1.0 = intense combat)
    pub combat_intensity: f32,
    /// Last time combat was detected
    pub last_combat_time: f32,
}

impl Default for ActionCenter {
    fn default() -> Self {
        Self {
            center: Vec3::new(0.0, 130000.0, 10.0),  // Start at Forest biome spawn
            raw_center: Vec3::new(0.0, 130000.0, 10.0),
            combat_intensity: 0.0,
            last_combat_time: 0.0,
        }
    }
}

/// Weights for calculating action center
pub struct ActionWeights {
    pub hero: f32,
    pub combat: f32,
    pub villagers: f32,
    pub projectiles: f32,
}

impl Default for ActionWeights {
    fn default() -> Self {
        Self {
            hero: 0.8,         // Hero is primary focus (80%) - much higher to reduce camera drift
            combat: 0.1,       // Active combat areas (10%) - reduced
            villagers: 0.05,   // Nearby villagers (5%) - reduced
            projectiles: 0.05, // Projectiles in flight (5%)
        }
    }
}

impl ActionWeights {
    /// Get combat-adjusted weights (shifts focus to combat when intensity is high)
    pub fn adjust_for_combat(&self, combat_intensity: f32) -> Self {
        let combat_shift = combat_intensity * 0.1; // Up to 10% shift during intense combat (reduced from 20%)
        Self {
            hero: (self.hero - combat_shift).max(0.7),  // Hero weight stays minimum 70% (up from 30%)
            combat: (self.combat + combat_shift).min(0.2), // Combat weight max 20% (down from 50%)
            villagers: self.villagers,
            projectiles: self.projectiles,
        }
    }
}
