use crate::components::block::BiomeType;
use crate::components::bundles::SpriteBundle3d;
use crate::components::hero::Hero;
use crate::components::weather::{ActiveWeather, WeatherCategory, WeatherType};
use crate::components::weather_particle::WeatherParticle;
use crate::resources::weather_config::{AnomalyConfig, GLOBAL_ANOMALIES, constants};
use bevy::prelude::*;
use rand::RngExt;

pub struct WeatherSystemPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct WeatherConditions;

impl Plugin for WeatherSystemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveWeather>()
            .init_resource::<ParticleSpawnTimer>()
            .init_resource::<WeatherVisuals>()
            .add_systems(
                Update,
                update_weather
                    .in_set(WeatherConditions)
                    .run_if(in_state(crate::states::InGame)),
            )
            .add_systems(
                Update,
                (spawn_weather_particles, update_weather_particles)
                    .chain()
                    .after(WeatherConditions)
                    .run_if(in_state(crate::states::InGame)),
            );

        info!("WeatherSystemPlugin loaded");
    }
}

#[derive(Resource, Default)]
struct WeatherVisuals {
    mesh: Option<Handle<Mesh>>,
    // Reuse identical appearances; changing weather releases the old cache handle
    // while live particles retain their original colour and alpha.
    last_material: Option<(&'static str, Color, Handle<StandardMaterial>)>,
}

/// Timer for particle spawning
#[derive(Resource)]
struct ParticleSpawnTimer(Timer);

impl Default for ParticleSpawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            constants::PARTICLE_SPAWN_INTERVAL,
            TimerMode::Repeating,
        ))
    }
}

/// Select random weather from weighted probabilities
fn select_random_weather(options: &[(WeatherType, f32)]) -> WeatherType {
    // CRITICAL FIX: Handle empty options to prevent panic
    if options.is_empty() {
        return WeatherType::Clear; // Default fallback
    }

    let total: f32 = options.iter().map(|(_, prob)| prob).sum();

    // Handle zero total probability
    if total <= 0.0 {
        return options[0].0;
    }

    let mut rng = rand::rng();
    let mut roll = rng.random_range(0.0..1.0) * total;

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
    let mut rng = rand::rng();

    // Filter anomalies valid for this biome
    let valid_anomalies: Vec<&AnomalyConfig> = GLOBAL_ANOMALIES
        .iter()
        .filter(|anomaly| {
            // If allowed_biomes is empty, works everywhere
            if anomaly.allowed_biomes.is_empty() {
                true // No restrictions
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
        if rng.random_range(0.0..1.0) < anomaly.probability {
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
    modes: Option<Res<crate::systems::game_modes::GameModes>>,
) {
    // Weather turned off in Settings > Game modes: clear skies, no new weather.
    if !crate::systems::game_modes::is_on(
        modes.as_deref(),
        crate::systems::game_modes::GameMode::Weather,
    ) {
        if weather.weather_type != WeatherType::Clear || weather.intensity != 0.0 {
            weather.weather_type = WeatherType::Clear;
            weather.intensity = 0.0;
        }
        return;
    }
    // Decrease duration
    weather.duration_remaining -= time.delta_secs();

    // Update transition timer for smooth fade
    if weather.transition_timer > 0.0 {
        weather.transition_timer -= time.delta_secs();

        // Lerp intensity during transition
        let target_intensity = if weather.weather_type == WeatherType::Clear {
            0.0
        } else {
            1.0
        };
        let lerp_factor = time.delta_secs() / constants::WEATHER_TRANSITION_DURATION.max(0.01);
        weather.intensity =
            weather.intensity + (target_intensity - weather.intensity) * lerp_factor.min(1.0);
    }

    // Check if weather should change
    if weather.duration_remaining <= 0.0 {
        let Ok(hero_transform) = hero_query.single() else {
            return;
        };

        let current_biome = crate::utils::biome_utils::determine_biome_at_position(
            hero_transform.translation.x,
            hero_transform.translation.y,
        );
        let biome_weather = current_biome.weather_types();

        // Roll for anomaly
        let mut rng = rand::rng();
        let is_anomaly_check = rng.random_range(0.0..1.0) < biome_weather.anomaly_chance;

        if is_anomaly_check {
            // Try to spawn an anomaly
            if let Some(anomaly) = select_random_anomaly(current_biome) {
                info!(
                    "🌪️ ANOMALY TRIGGERED: {:?} in {:?}!",
                    anomaly.weather_type, current_biome
                );

                weather.weather_type = anomaly.weather_type;
                weather.category = WeatherCategory::Anomaly;
                weather.duration_remaining =
                    rng.random_range(anomaly.min_duration..=anomaly.max_duration);
                weather.transition_timer = constants::WEATHER_TRANSITION_DURATION;
                weather.intensity = 0.0; // Start from 0, will fade in
                return;
            }
        }

        // Normal weather selection (no anomaly triggered)
        let new_weather = select_random_weather(&biome_weather.possible_weather);

        weather.weather_type = new_weather;
        weather.category = WeatherCategory::Normal;
        weather.duration_remaining =
            rng.random_range(biome_weather.min_duration..=biome_weather.max_duration);
        weather.transition_timer = constants::WEATHER_TRANSITION_DURATION;
        weather.intensity = 0.0; // Start from 0, will fade in

        info!(
            "🌦️ Weather changed: {:?} ({:?}) for {:.0}s in {:?}",
            weather.weather_type, weather.category, weather.duration_remaining, current_biome
        );
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
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut emission: Local<f32>,
    mut visuals: ResMut<WeatherVisuals>,
    particles: Query<(), With<WeatherParticle>>,
) {
    // Skip if no weather or intensity too low
    if weather.weather_type == WeatherType::Clear
        || weather.intensity < 0.1
        || weather.duration_remaining <= 0.0
    {
        *emission = 0.0;
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

    let view = crate::systems::graphics::light_view(&cameras, camera_pos.xy());
    let screen_width = view.width();
    let screen_height = view.height();
    let camera_pos = view.center();

    // Number of particles to spawn (scaled by intensity)
    let base_count = match weather.weather_type {
        WeatherType::FireRain => 18.,
        WeatherType::Snow | WeatherType::AuroraStorm => 1.5, // Slower particles = fewer spawns
        _ => 3.0,
    };
    let particle_count = emission_count(&mut emission, weather.intensity * base_count)
        .min(600usize.saturating_sub(particles.iter().count()));

    let mut rng = rand::rng();

    for _i in 0..particle_count {
        // Get texture, velocity, and lifetime based on weather type
        let (texture_path, velocity, particle_color, mut lifetime): (&str, Vec2, Color, f32) =
            match weather.weather_type {
                WeatherType::Rain => (
                    "textures/effects/rain.png",
                    Vec2::new(rng.random_range(-10.0..10.0), -800.0), // Fast falling with slight drift
                    Color::srgba(1.0, 1.0, 1.0, weather.intensity),
                    3.0, // Fast rain needs less time
                ),
                WeatherType::Snow => (
                    "textures/effects/snow.png",
                    Vec2::new(
                        rng.random_range(-60.0..60.0),
                        rng.random_range(-120.0..-80.0),
                    ), // Slower gentle drift
                    Color::srgba(1.0, 1.0, 1.0, weather.intensity),
                    12.0, // Reduced lifetime to prevent accumulation
                ),
                WeatherType::Sandstorm => (
                    "textures/effects/rain.png", // Reuse rain texture, will tint brown
                    Vec2::new(300.0, -100.0),    // Horizontal
                    Color::srgba(0.76, 0.7, 0.5, weather.intensity),
                    10.0, // Moderate speed
                ),
                WeatherType::AshStorm => (
                    "textures/effects/snow.png", // Reuse snow texture, will tint red/orange
                    Vec2::new(
                        rng.random_range(-50.0..50.0),
                        rng.random_range(-100.0..100.0),
                    ), // Erratic
                    Color::srgba(1.0, 0.3, 0.1, 1.0),
                    12.0, // Erratic movement needs time
                ),
                WeatherType::VoidStorm => (
                    "textures/effects/snow.png", // Reuse snow texture, will tint purple
                    Vec2::new(rng.random_range(-100.0..100.0), -150.0),
                    Color::srgba(0.7, 0.0, 1.0, 1.0),
                    8.0, // Moderate speed
                ),
                WeatherType::FireRain => (
                    "textures/effects/rain.png",                      // Will tint red/orange
                    Vec2::new(0.0, -1000.0),                          // Very fast
                    Color::srgba(1.0, 0.45, 0.06, weather.intensity), // Bright orange
                    2.0,                                              // Very fast, short lifetime
                ),
                WeatherType::BloodMoon => (
                    "textures/effects/snow.png", // Small red particles
                    Vec2::new(rng.random_range(-50.0..50.0), -100.0),
                    Color::srgba(1.0, 0.0, 0.0, 1.0),
                    12.0, // Slow drift
                ),
                WeatherType::AuroraStorm => (
                    "textures/effects/snow.png", // Multicolor particles
                    Vec2::new(rng.random_range(-30.0..30.0), rng.random_range(-50.0..50.0)),
                    Color::srgba(
                        rng.random_range(0.5..1.5),
                        rng.random_range(0.5..1.5),
                        rng.random_range(0.5..1.5),
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
        let x = camera_pos.x
            + rng.random_range(
                -(screen_width / 2.0 + spawn_margin)..(screen_width / 2.0 + spawn_margin),
            );
        let y = camera_pos.y + screen_height / 2.0 + rng.random_range(100.0..500.0); // Higher spawn point
        let z = rng.random_range(80.0..120.0); // Slight depth variation
        let spawn_pos = Vec3::new(x, y, z);
        lifetime = lifetime.max((screen_height + 800.) / velocity.y.abs().max(80.));

        // Create 3D sprite for weather particle with slight size variation
        let size_variation = rng.random_range(0.9..1.3);
        let particle_mesh = visuals
            .mesh
            .get_or_insert_with(|| meshes.add(Rectangle::new(192., 768.)))
            .clone();
        let particle_material = match &visuals.last_material {
            Some((path, color, handle)) if *path == texture_path && *color == final_color => {
                handle.clone()
            }
            _ => {
                let handle = materials.add(StandardMaterial {
                    base_color: final_color,
                    base_color_texture: if weather.weather_type == WeatherType::FireRain {
                        None
                    } else {
                        Some(asset_server.load(texture_path))
                    },
                    emissive: if weather.weather_type == WeatherType::FireRain {
                        LinearRgba::rgb(2., 0.3, 0.01)
                    } else {
                        LinearRgba::BLACK
                    },
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                });
                visuals.last_material = Some((texture_path, final_color, handle.clone()));
                handle
            }
        };

        // Use camera-following particles for continuous weather effects
        let particle = WeatherParticle::new_camera_following(velocity, lifetime);

        commands.spawn((
            particle,
            SpriteBundle3d::with_scale(
                particle_mesh,
                particle_material,
                spawn_pos,
                if weather.weather_type == WeatherType::FireRain {
                    Vec3::new(0.022 * size_variation, 0.055 * size_variation, 1.)
                } else {
                    Vec3::new(size_variation, size_variation, 1.)
                },
            ),
            bevy::light::NotShadowCaster,
        ));
    }
}

/// Update weather particle positions and lifetime - camera-relative movement
fn update_weather_particles(
    mut commands: Commands,
    mut particle_query: Query<(Entity, &mut Transform, &mut WeatherParticle)>,
    time: Res<Time>,
    hero_query: Query<&Transform, (With<Hero>, Without<WeatherParticle>)>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let camera_pos = hero_transform.translation;
    let view = crate::systems::graphics::light_view(&cameras, camera_pos.xy());
    let screen_height = view.height();
    let camera_pos = view.center();

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
            || distance_x > view.width()*0.8+200.  // Far off screen horizontally
            || distance_below_camera > screen_height / 2.0 + 500.0
        {
            commands.entity(entity).despawn();
        }
    }
}

fn emission_count(carry: &mut f32, amount: f32) -> usize {
    *carry += amount.max(0.0);
    let count = carry.floor() as usize;
    *carry -= count as f32;
    count
}

#[cfg(test)]
mod emission_tests {
    use super::*;
    #[test]
    fn light_snow_still_emits_visible_particles() {
        let mut carry = 0.;
        let count: usize = (0..10).map(|_| emission_count(&mut carry, 0.3 * 1.5)).sum();
        assert_eq!(count, 4);
        assert!(carry > 0.4 && carry < 0.6);
    }
}
