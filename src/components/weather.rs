use crate::components::block::BiomeType;
use bevy::prelude::*;

/// Weather event types - visual and gameplay effects
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeatherType {
    Clear,     // No weather
    Rain,      // Water droplets (Plains, Forest)
    Snow,      // Snowflakes (Mountains, Ice biomes)
    Sandstorm, // Sand particles (Desert)
    AshStorm,  // Ember/ash particles (Nether)
    VoidStorm, // Purple particles (End)
    // Special anomalies
    FireRain,    // Burning meteors (rare in any biome)
    BloodMoon,   // Red particles, more enemies (rare)
    AuroraStorm, // Beautiful multicolor particles (Mountains/End)
}

/// Weather category - normal or anomaly
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeatherCategory {
    Normal,  // Regular biome weather
    Anomaly, // Cross-biome rare event
}

/// Active weather in the world
#[derive(Resource)]
pub struct ActiveWeather {
    pub weather_type: WeatherType,
    pub category: WeatherCategory,
    pub intensity: f32,          // 0.0 = none, 1.0 = max
    pub duration_remaining: f32, // seconds until weather changes
    pub transition_timer: f32,   // for smooth fade in/out
}

impl Default for ActiveWeather {
    fn default() -> Self {
        Self {
            weather_type: WeatherType::Clear,
            category: WeatherCategory::Normal,
            intensity: 0.0,
            duration_remaining: 5.0, // Start weather change after 5 seconds instead of 60
            transition_timer: 0.0,
        }
    }
}

/// Weather configuration per biome
pub struct BiomeWeather {
    pub possible_weather: Vec<(WeatherType, f32)>, // (type, probability)
    pub min_duration: f32,
    pub max_duration: f32,
    pub anomaly_chance: f32, // Chance to roll for anomaly
}

impl BiomeType {
    pub fn weather_types(&self) -> BiomeWeather {
        match self {
            BiomeType::Plains => BiomeWeather {
                possible_weather: vec![(WeatherType::Clear, 0.7), (WeatherType::Rain, 0.3)],
                min_duration: 30.0,
                max_duration: 120.0,
                anomaly_chance: 0.10, // 10% chance to check for anomaly
            },
            BiomeType::Forest => BiomeWeather {
                possible_weather: vec![
                    (WeatherType::Clear, 0.5),
                    (WeatherType::Rain, 0.5), // More rain in forest
                ],
                min_duration: 20.0,
                max_duration: 90.0,
                anomaly_chance: 0.10,
            },
            BiomeType::Desert => BiomeWeather {
                possible_weather: vec![(WeatherType::Clear, 0.8), (WeatherType::Sandstorm, 0.2)],
                min_duration: 40.0,
                max_duration: 180.0,
                anomaly_chance: 0.08,
            },
            BiomeType::Mountains => BiomeWeather {
                possible_weather: vec![(WeatherType::Clear, 0.6), (WeatherType::Snow, 0.4)],
                min_duration: 25.0,
                max_duration: 100.0,
                anomaly_chance: 0.10,
            },
            BiomeType::Underground => BiomeWeather {
                possible_weather: vec![
                    (WeatherType::Clear, 1.0), // No weather underground
                ],
                min_duration: f32::MAX,
                max_duration: f32::MAX,
                anomaly_chance: 0.0, // No anomalies underground
            },
            BiomeType::Nether => BiomeWeather {
                possible_weather: vec![
                    (WeatherType::Clear, 0.7),
                    (WeatherType::AshStorm, 0.3), // Ash/ember storms
                ],
                min_duration: 20.0,
                max_duration: 60.0,
                anomaly_chance: 0.15, // 15% - most chaotic biome!
            },
            BiomeType::End => BiomeWeather {
                possible_weather: vec![
                    (WeatherType::Clear, 0.8),
                    (WeatherType::VoidStorm, 0.2), // Purple particle storms
                ],
                min_duration: 30.0,
                max_duration: 90.0,
                anomaly_chance: 0.12,
            },
            BiomeType::Iceland => BiomeWeather {
                possible_weather: vec![
                    (WeatherType::Clear, 0.3),
                    (WeatherType::Snow, 0.7), // Snow most of the time
                ],
                min_duration: 30.0,
                max_duration: 120.0,
                anomaly_chance: 0.10,
            },
        }
    }
}

impl ActiveWeather {
    pub fn is_snowing(&self) -> bool {
        self.weather_type == WeatherType::Snow
            && self.intensity >= 0.3
            && self.duration_remaining > 0.0
    }
}
