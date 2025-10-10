use bevy::prelude::*;
use rand::Rng;
use crate::components::weather::{WeatherType, WeatherCategory, ActiveWeather, BiomeWeather};
use crate::components::weather_particle::WeatherParticle;
use crate::components::hero::Hero;
use crate::components::block::BiomeType;
use crate::components::bundles::SpriteBundle3d;
use crate::resources::weather_config::{GLOBAL_ANOMALIES, AnomalyConfig, constants};
use crate::states::GameState;

pub struct WeatherSystemPlugin;

impl Plugin for WeatherSystemPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ActiveWeather>()
            .init_resource::<ParticleSpawnTimer>()
            .add_systems(Update, (
                update_weather,
                spawn_weather_particles,
                update_weather_particles,
                apply_weather_gameplay_effects,
            ).run_if(in_state(GameState::Playing)));

        info!("WeatherSystemPlugin loaded");
    }
}

/// Timer for particle spawning
#[derive(Resource)]
struct ParticleSpawnTimer(Timer);

impl Default for ParticleSpawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(constants::PARTICLE_SPAWN_INTERVAL, TimerMode::Repeating))
    }
}

/// Get biome from Y position (matches world generation logic)
fn get_biome_at_y(y: f32) -> BiomeType {
    let chunk_height = 64.0 * 2.0; // 2 chunks per biome (matches world_generation.rs)
    let biome_zone = (y / chunk_height) as i32 / 2;
    let biome_seed = (biome_zone * 179) % 100;

    match biome_seed.abs() % 8 {  // 8 biomes including Iceland
        0 => BiomeType::Plains,
        1 => BiomeType::Forest,
        2 => BiomeType::Desert,
        3 => BiomeType::Mountains,
        4 => BiomeType::Underground,
        5 => BiomeType::Nether,
        6 => BiomeType::End,
        _ => BiomeType::Iceland,
    }
}

/// Select random weather from weighted probabilities
fn select_random_weather(options: &[(WeatherType, f32)]) -> WeatherType {
    let total: f32 = options.iter().map(|(_, prob)| prob).sum();
    let mut rng = rand::thread_rng();
    let mut roll = rng.gen_range(0.0..1.0) * total;

    for (weather_type, probability) in options {
        roll -= probability;
        if roll <= 0.0 {
            return *weather_type;
        }
    }

    // Fallback
    options[0].0
}

/// Try to select a random anomaly for the current biome
fn select_random_anomaly(current_biome: BiomeType) -> Option<&'static AnomalyConfig> {
    let mut rng = rand::thread_rng();

    // Filter anomalies valid for this biome
    let valid_anomalies: Vec<&AnomalyConfig> = GLOBAL_ANOMALIES
        .iter()
        .filter(|anomaly| {
            // If allowed_biomes is empty, works everywhere
            if anomaly.allowed_biomes.is_empty() {
                true  // No restrictions
            } else {
                anomaly.allowed_biomes.contains(&current_biome)
            }
        })
        .collect();

    if valid_anomalies.is_empty() {
        return None;
    }

    // Roll for each anomaly's individual probability
    for anomaly in valid_anomalies {
        if rng.gen_range(0.0..1.0) < anomaly.probability {
            return Some(anomaly);
        }
    }

    None
}

/// Main weather update system - changes weather based on biome and time
fn update_weather(
    mut weather: ResMut<ActiveWeather>,
    time: Res<Time>,
    hero_query: Query<&Transform, With<Hero>>,
) {
    // Decrease duration
    weather.duration_remaining -= time.delta_secs();

    // Update transition timer for smooth fade
    if weather.transition_timer > 0.0 {
        weather.transition_timer -= time.delta_secs();

        // Lerp intensity during transition
        let target_intensity = if weather.weather_type == WeatherType::Clear { 0.0 } else { 1.0 };
        let lerp_factor = time.delta_secs() / constants::WEATHER_TRANSITION_DURATION.max(0.01);
        weather.intensity = weather.intensity + (target_intensity - weather.intensity) * lerp_factor.min(1.0);
    }

    // Check if weather should change
    if weather.duration_remaining <= 0.0 {
        let Ok(hero_transform) = hero_query.single() else {
            return;
        };

        let current_biome = get_biome_at_y(hero_transform.translation.y);
        let biome_weather = current_biome.weather_types();

        // Roll for anomaly
        let mut rng = rand::thread_rng();
        let is_anomaly_check = rng.gen_range(0.0..1.0) < biome_weather.anomaly_chance;

        if is_anomaly_check {
            // Try to spawn an anomaly
            if let Some(anomaly) = select_random_anomaly(current_biome) {
                info!("🌪️ ANOMALY TRIGGERED: {:?} in {:?}!", anomaly.weather_type, current_biome);

                weather.weather_type = anomaly.weather_type;
                weather.category = WeatherCategory::Anomaly;
                weather.duration_remaining = rng.gen_range(anomaly.min_duration..=anomaly.max_duration);
                weather.transition_timer = constants::WEATHER_TRANSITION_DURATION;
                weather.intensity = 0.0; // Start from 0, will fade in
                return;
            }
        }

        // Normal weather selection (no anomaly triggered)
        let new_weather = select_random_weather(&biome_weather.possible_weather);

        weather.weather_type = new_weather;
        weather.category = WeatherCategory::Normal;
        weather.duration_remaining = rng.gen_range(biome_weather.min_duration..=biome_weather.max_duration);
        weather.transition_timer = constants::WEATHER_TRANSITION_DURATION;
        weather.intensity = 0.0; // Start from 0, will fade in

        info!("🌦️ Weather changed: {:?} ({:?}) for {:.0}s in {:?}",
            weather.weather_type, weather.category, weather.duration_remaining, current_biome);
    }
}

/// Spawn weather particles based on active weather
fn spawn_weather_particles(
    mut commands: Commands,
    mut spawn_timer: ResMut<ParticleSpawnTimer>,
    weather: Res<ActiveWeather>,
    hero_query: Query<&Transform, With<Hero>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    windows: Query<&Window>,
) {
    // Skip if no weather or intensity too low
    if weather.weather_type == WeatherType::Clear || weather.intensity < 0.1 {
        return;
    }

    spawn_timer.0.tick(time.delta());

    if !spawn_timer.0.just_finished() {
        return;
    }

    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let camera_pos = hero_transform.translation;

    // Get actual window dimensions
    let (screen_width, screen_height) = if let Some(window) = windows.iter().next() {
        (window.width(), window.height())
    } else {
        return;
    };

    // Number of particles to spawn (scaled by intensity)
    let base_count = match weather.weather_type {
        WeatherType::Snow | WeatherType::AuroraStorm => 1.5, // Slower particles = fewer spawns
        _ => 3.0,
    };
    let particle_count = (weather.intensity * base_count) as usize;

    let mut rng = rand::thread_rng();

    for i in 0..particle_count {
        // Get texture, velocity, and lifetime based on weather type
        let (texture_path, velocity, particle_color, lifetime) = match weather.weather_type {
            WeatherType::Rain => (
                "textures/effects/rain.png",
                Vec2::new(rng.gen_range(-10.0..10.0), -800.0), // Fast falling with slight drift
                Color::srgba(1.0, 1.0, 1.0, weather.intensity),
                3.0, // Fast rain needs less time
            ),
            WeatherType::Snow => (
                "textures/effects/snow.png",
                Vec2::new(rng.gen_range(-60.0..60.0), rng.gen_range(-180.0..-120.0)), // Slower gentle drift
                Color::srgba(1.0, 1.0, 1.0, weather.intensity),
                12.0, // Reduced lifetime to prevent accumulation
            ),
            WeatherType::Sandstorm => (
                "textures/effects/rain.png", // Reuse rain texture, will tint brown
                Vec2::new(300.0, -100.0), // Horizontal
                Color::srgba(0.76, 0.7, 0.5, weather.intensity),
                10.0, // Moderate speed
            ),
            WeatherType::AshStorm => (
                "textures/effects/snow.png", // Reuse snow texture, will tint red/orange
                Vec2::new(rng.gen_range(-50.0..50.0), rng.gen_range(-100.0..100.0)), // Erratic
                Color::srgba(1.0, 0.3, 0.1, 1.0),
                12.0, // Erratic movement needs time
            ),
            WeatherType::VoidStorm => (
                "textures/effects/snow.png", // Reuse snow texture, will tint purple
                Vec2::new(rng.gen_range(-100.0..100.0), -150.0),
                Color::srgba(0.7, 0.0, 1.0, 1.0),
                8.0, // Moderate speed
            ),
            WeatherType::FireRain => (
                "textures/effects/rain.png", // Will tint red/orange
                Vec2::new(0.0, -1000.0), // Very fast
                Color::srgba(1.5, 0.5, 0.0, weather.intensity), // Bright orange
                2.0, // Very fast, short lifetime
            ),
            WeatherType::BloodMoon => (
                "textures/effects/snow.png", // Small red particles
                Vec2::new(rng.gen_range(-50.0..50.0), -100.0),
                Color::srgba(1.0, 0.0, 0.0, 1.0),
                12.0, // Slow drift
            ),
            WeatherType::AuroraStorm => (
                "textures/effects/snow.png", // Multicolor particles
                Vec2::new(rng.gen_range(-30.0..30.0), rng.gen_range(-50.0..50.0)),
                Color::srgba(
                    rng.gen_range(0.5..1.5),
                    rng.gen_range(0.5..1.5),
                    rng.gen_range(0.5..1.5),
                    1.0,
                ),
                15.0, // Very slow, long lifetime
            ),
            WeatherType::Clear => return, // No particles
        };

        // Anomaly visual indicator - pinkish glow
        let final_color = if weather.category == WeatherCategory::Anomaly {
            Color::srgba(
                particle_color.to_srgba().red * 1.2,
                particle_color.to_srgba().green * 0.9,
                particle_color.to_srgba().blue * 1.2,
                particle_color.alpha(),
            )
        } else {
            particle_color
        };

        // Fully random distribution across entire screen width
        let spawn_margin = screen_width * 0.3; // 30% margin on each side for edge coverage
        let x = camera_pos.x + rng.gen_range(-(screen_width / 2.0 + spawn_margin)..(screen_width / 2.0 + spawn_margin));
        let y = camera_pos.y + screen_height / 2.0 + rng.gen_range(100.0..500.0); // Higher spawn point
        let z = rng.gen_range(80.0..120.0); // Slight depth variation
        let spawn_pos = Vec3::new(x, y, z);

        // Create 3D sprite for weather particle with slight size variation
        let size_variation = rng.gen_range(0.9..1.3);
        let particle_mesh = meshes.add(Rectangle::new(192.0 * size_variation, 768.0 * size_variation)); // 1.5x bigger
        let particle_material = materials.add(StandardMaterial {
            base_color: final_color,
            base_color_texture: Some(asset_server.load(texture_path)),
            unlit: true,  // Weather particles glow
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        // Use camera-following particles for continuous weather effects
        let particle = WeatherParticle::new_camera_following(velocity, lifetime);

        commands.spawn((
            particle,
            SpriteBundle3d::new(particle_mesh, particle_material, spawn_pos),
        ));
    }
}

/// Update weather particle positions and lifetime - camera-relative movement
fn update_weather_particles(
    mut commands: Commands,
    mut particle_query: Query<(Entity, &mut Transform, &mut WeatherParticle)>,
    time: Res<Time>,
    hero_query: Query<&Transform, (With<Hero>, Without<WeatherParticle>)>,
    windows: Query<&Window>,
) {
    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let camera_pos = hero_transform.translation;
    let screen_height = if let Some(window) = windows.iter().next() {
        window.height()
    } else {
        return;
    };

    for (entity, mut transform, mut particle) in particle_query.iter_mut() {
        // Move particle relative to its spawn offset from camera
        transform.translation.x += particle.velocity.x * time.delta_secs();
        transform.translation.y += particle.velocity.y * time.delta_secs();

        // Update camera-relative offset (follow camera on X axis for smooth scrolling)
        // Particles maintain their X velocity but follow camera vertically for continuous effect
        if particle.follows_camera {
            // Only follow Y movement, particles drift on their own X axis
            let camera_relative_y = transform.translation.y - camera_pos.y;

            // If particle drifted too far off screen, respawn at top
            if camera_relative_y < -screen_height / 2.0 - 200.0 {
                transform.translation.y = camera_pos.y + screen_height / 2.0 + 200.0;
            }
        }

        // Update lifetime
        particle.lifetime.tick(time.delta());

        // Despawn if off-screen (horizontally) or expired
        let distance_x = (transform.translation.x - camera_pos.x).abs();
        let distance_below_camera = camera_pos.y - transform.translation.y;

        if particle.lifetime.is_finished()
            || distance_x > 2000.0  // Far off screen horizontally
            || distance_below_camera > screen_height / 2.0 + 500.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Apply gameplay effects based on active weather
fn apply_weather_gameplay_effects(
    weather: Res<ActiveWeather>,
    mut hero_query: Query<&mut crate::components::physics::Velocity, With<Hero>>,
) {
    if weather.intensity < 0.5 {
        return; // Only apply effects when weather is strong enough
    }

    let Ok(mut hero_velocity) = hero_query.single_mut() else {
        return;
    };

    match weather.weather_type {
        WeatherType::Sandstorm => {
            // Push hero horizontally (wind)
            hero_velocity.x += 50.0 * weather.intensity;
        }
        WeatherType::Snow => {
            // Slow movement slightly
            hero_velocity.x *= 1.0 - (0.1 * weather.intensity);
        }
        WeatherType::AshStorm => {
            // Random turbulence
            let mut rng = rand::thread_rng();
            hero_velocity.x += rng.gen_range(-20.0..20.0) * weather.intensity;
        }
        _ => {}
    }
}
