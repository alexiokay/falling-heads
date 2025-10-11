use bevy::prelude::*;
use crate::components::weather::{WeatherType, ActiveWeather};
use crate::components::weather_particle::WeatherParticle;
use crate::components::block::Block;
use crate::components::snow_layer::SnowLayer;
use crate::components::bundles::SpriteBundle3d;
use crate::resources::asset_handles::BlockTextures;

pub struct SnowAccumulationPlugin;

impl Plugin for SnowAccumulationPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                detect_snow_particle_collisions,
                update_snow_overlays,
                melt_snow_in_clear_weather,
            ));

        info!("SnowAccumulationPlugin loaded");
    }
}

/// Marker component for snow overlay visual
#[derive(Component)]
struct SnowOverlay;

/// Detect when snow particles hit blocks and add snow layers
fn detect_snow_particle_collisions(
    mut commands: Commands,
    weather: Res<ActiveWeather>,
    particle_query: Query<&Transform, With<WeatherParticle>>,
    mut block_query: Query<(Entity, &Transform, Option<&mut SnowLayer>), (With<Block>, Without<WeatherParticle>)>,
    // Check for blocks above to ensure snow only accumulates on exposed surfaces
    block_check_query: Query<&Transform, (With<Block>, Without<WeatherParticle>)>,
) {
    // Only accumulate during snow weather
    if weather.weather_type != WeatherType::Snow || weather.intensity < 0.5 {
        return;
    }

    const BLOCK_SIZE: f32 = 128.0;  // Your actual block size
    const COLLISION_THRESHOLD: f32 = BLOCK_SIZE / 2.0 + 50.0;  // Detect near top of block
    const CHECK_DISTANCE: f32 = BLOCK_SIZE + 10.0;  // Distance to check above for air

    for particle_transform in particle_query.iter() {
        let particle_pos = particle_transform.translation;

        for (block_entity, block_transform, snow_layer_opt) in block_query.iter_mut() {
            let block_pos = block_transform.translation;

            // Check if particle is near the top of this block
            let dx = (particle_pos.x - block_pos.x).abs();
            let dy = particle_pos.y - block_pos.y;

            // Particle must be:
            // 1. Horizontally aligned with block (within block width)
            // 2. Just above the top surface of block
            if dx < BLOCK_SIZE / 2.0 && dy > 0.0 && dy < COLLISION_THRESHOLD {
                // Check if there's a block directly above (if so, skip - no air exposure)
                let mut has_block_above = false;
                for check_transform in block_check_query.iter() {
                    let check_pos = check_transform.translation;
                    let check_dx = (check_pos.x - block_pos.x).abs();
                    let check_dy = check_pos.y - block_pos.y;

                    // Check if there's a block directly above this one
                    if check_dx < BLOCK_SIZE / 2.0
                        && check_dy > CHECK_DISTANCE * 0.8
                        && check_dy < CHECK_DISTANCE * 1.2 {
                        has_block_above = true;
                        break;
                    }
                }

                // Only add snow if there's air above
                if !has_block_above {
                    if let Some(mut snow_layer) = snow_layer_opt {
                        snow_layer.add_snow();
                    } else {
                        // Add SnowLayer component to block
                        let mut new_layer = SnowLayer::new();
                        new_layer.add_snow();
                        commands.entity(block_entity).insert(new_layer);
                    }
                }
            }
        }
    }
}

/// Component linking snow overlay to its parent block
#[derive(Component)]
struct SnowOverlayParent(Entity);

/// Update visual snow overlays based on snow layer stage
fn update_snow_overlays(
    mut commands: Commands,
    block_query: Query<(Entity, &Transform, &SnowLayer), (With<Block>, Changed<SnowLayer>)>,
    overlay_query: Query<(Entity, &SnowOverlayParent), With<SnowOverlay>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    _block_textures: Res<BlockTextures>,
) {
    for (block_entity, block_transform, snow_layer) in block_query.iter() {
        // Remove existing overlay if any
        for (overlay_entity, parent_ref) in overlay_query.iter() {
            if parent_ref.0 == block_entity {
                commands.entity(overlay_entity).despawn();
            }
        }

        // Add new overlay if snow exists
        if snow_layer.has_snow() {
            let alpha = snow_layer.get_alpha();

            // Create snow overlay mesh (slightly larger than block to be visible on top)
            let overlay_size = 130.0;
            let overlay_mesh = meshes.add(Rectangle::new(overlay_size, overlay_size));

            // Load snow texture
            let snow_texture = asset_server.load("textures/blocks/snow.png");

            let overlay_material = materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 1.0, 1.0, alpha),
                base_color_texture: Some(snow_texture),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            });

            // Position overlay slightly above block surface
            let overlay_pos = Vec3::new(
                block_transform.translation.x,
                block_transform.translation.y,
                block_transform.translation.z - 1.0,  // Slightly in front
            );

            // Spawn overlay with parent reference
            commands.spawn((
                SnowOverlay,
                SnowOverlayParent(block_entity),
                SpriteBundle3d::new(overlay_mesh, overlay_material, overlay_pos),
            ));
        }
    }
}

/// Gradually melt snow during non-snowy weather
fn melt_snow_in_clear_weather(
    mut commands: Commands,
    weather: Res<ActiveWeather>,
    time: Res<Time>,
    mut block_query: Query<(Entity, &mut SnowLayer), With<Block>>,
    mut melt_timer: Local<f32>,
) {
    // Skip if actively snowing
    if weather.weather_type == WeatherType::Snow && weather.intensity > 0.3 {
        *melt_timer = 0.0;
        return;
    }

    // Melt snow every 5 seconds during clear/non-snow weather
    *melt_timer += time.delta_secs();

    if *melt_timer >= 5.0 {
        *melt_timer = 0.0;

        for (block_entity, mut snow_layer) in block_query.iter_mut() {
            let was_changed = snow_layer.melt();

            // Remove component if fully melted
            if !snow_layer.has_snow() && was_changed {
                commands.entity(block_entity).remove::<SnowLayer>();
            }
        }
    }
}
