use crate::components::block::BiomeType;
use crate::components::weather::WeatherType;

/// Global anomaly configuration - works ANYWHERE
pub struct AnomalyConfig {
    pub weather_type: WeatherType,
    pub probability: f32, // Chance to trigger (0.01 = 1%)
    pub min_duration: f32,
    pub max_duration: f32,
    pub allowed_biomes: &'static [BiomeType], // Empty slice = all biomes
}

/// Global table of weather anomalies
pub const GLOBAL_ANOMALIES: &[AnomalyConfig] = &[
    // Snow in Nether - ICONIC anomaly!
    AnomalyConfig {
        weather_type: WeatherType::Snow,
        probability: 0.02, // 2% chance when anomaly check triggers
        min_duration: 15.0,
        max_duration: 45.0,
        allowed_biomes: &[BiomeType::Nether], // Only in Nether
    },
    // Fire rain in any biome - extremely rare
    AnomalyConfig {
        weather_type: WeatherType::FireRain,
        probability: 0.01, // 1% chance
        min_duration: 10.0,
        max_duration: 30.0,
        allowed_biomes: &[], // Empty = Can happen ANYWHERE
    },
    // Blood Moon - spawns more enemies
    AnomalyConfig {
        weather_type: WeatherType::BloodMoon,
        probability: 0.03, // 3% chance
        min_duration: 60.0,
        max_duration: 180.0,
        allowed_biomes: &[BiomeType::Plains, BiomeType::Forest, BiomeType::Desert],
    },
    // Aurora in Mountains/End
    AnomalyConfig {
        weather_type: WeatherType::AuroraStorm,
        probability: 0.05, // 5% chance
        min_duration: 30.0,
        max_duration: 120.0,
        allowed_biomes: &[BiomeType::Mountains, BiomeType::End],
    },
    // Sandstorm in Nether (ash + sand mix)
    AnomalyConfig {
        weather_type: WeatherType::Sandstorm,
        probability: 0.04, // 4% chance
        min_duration: 20.0,
        max_duration: 50.0,
        allowed_biomes: &[BiomeType::Nether],
    },
    // Rain in Desert - rare but possible
    AnomalyConfig {
        weather_type: WeatherType::Rain,
        probability: 0.03, // 3% chance
        min_duration: 15.0,
        max_duration: 40.0,
        allowed_biomes: &[BiomeType::Desert],
    },
    // Ash Storm in Mountains - volcanic ash
    AnomalyConfig {
        weather_type: WeatherType::AshStorm,
        probability: 0.02, // 2% chance
        min_duration: 20.0,
        max_duration: 60.0,
        allowed_biomes: &[BiomeType::Mountains],
    },
];

/// Weather system constants
pub mod constants {
    pub const WEATHER_TRANSITION_DURATION: f32 = 3.0; // Fade in/out time
    pub const PARTICLE_SPAWN_INTERVAL: f32 = 0.1; // Spawn every 0.1 seconds
}
