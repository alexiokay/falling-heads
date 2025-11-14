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
    time: Res<Time>,
    hero_query: Query<&Transform, With<crate::components::hero::Hero>>,
    mut block_query: Query<(Entity, &Transform, Option<&mut SnowLayer>), With<Block>>,
    block_check_query: Query<&Transform, With<Block>>,
    mut accumulation_timer: Local<f32>,
) {
    // Only accumulate during snow weather
    if weather.weather_type != WeatherType::Snow || weather.intensity < 0.3 {
        return;
    }

    const BLOCK_SIZE: f32 = 128.0;
    const CHECK_DISTANCE: f32 = BLOCK_SIZE + 10.0;
    const ACCUMULATION_INTERVAL: f32 = 3.0;  // Add snow every 3 seconds
    const MAX_BLOCKS_PER_FRAME: usize = 100;  // Check more blocks per frame
    const SCREEN_RANGE: f32 = 2000.0;  // Larger range to cover visible area

    // Update accumulation timer
    *accumulation_timer += time.delta_secs();

    // Only try to accumulate every interval to prevent too-fast growth
    if *accumulation_timer < ACCUMULATION_INTERVAL {
        return;
    }

    *accumulation_timer = 0.0;

    // Get hero position for camera-relative checks
    let Ok(hero_transform) = hero_query.single() else {
        return;
    };
    let camera_pos = hero_transform.translation;

    // Collect nearby blocks first, then filter by exposure
    let mut checked = 0;
    for (block_entity, block_transform, snow_layer_opt) in block_query.iter_mut() {
        let block_pos = block_transform.translation;

        // Skip blocks too far from camera
        let distance = (block_pos - camera_pos).length();
        if distance > SCREEN_RANGE {
            continue;
        }

        // Check if there's a block DIRECTLY above (one block up)
        // Only the immediately adjacent block should block snow
        let mut has_block_above = false;
        for check_transform in block_check_query.iter() {
            let check_pos = check_transform.translation;

            let dx = (check_pos.x - block_pos.x).abs();
            let dy = check_pos.y - block_pos.y;

            // Check if there's a block directly above (within one block height)
            // Blocks are 128 units apart, so check for blocks 128 units above (±20 units tolerance)
            if dx < BLOCK_SIZE * 0.6  // Same column (wider tolerance for alignment)
                && dy > BLOCK_SIZE - 20.0  // Around 108-148 units above
                && dy < BLOCK_SIZE + 20.0 {
                has_block_above = true;
                break;
            }
        }

        // Only add snow if there's air above (exposed to sky)
        if !has_block_above {
            if let Some(mut snow_layer) = snow_layer_opt {
                snow_layer.add_snow();
            } else {
                // Add SnowLayer component to block
                let mut new_layer = SnowLayer::new();
                new_layer.add_snow();
                commands.entity(block_entity).insert(new_layer);
            }

            checked += 1;
            if checked >= MAX_BLOCKS_PER_FRAME {
                break;  // Stop after checking enough blocks
            }
        }
    }
}

/// Component linking snow overlay to its parent block
#[derive(Component)]
struct SnowOverlayParent(Entity);

/// Update visual snow overlays as child entities (like damage overlays)
fn update_snow_overlays(
    mut commands: Commands,
    block_query: Query<(Entity, &Transform, &SnowLayer, Option<&Children>), (With<Block>, Changed<SnowLayer>)>,
    overlay_query: Query<&SnowOverlay>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let mut spawned_count = 0;
    let mut blocks_with_snow = 0;

    for (block_entity, block_transform, snow_layer, children) in block_query.iter() {
        if snow_layer.has_snow() {
            blocks_with_snow += 1;
        }

        // Check if this block already has a snow overlay child
        let has_overlay = if let Some(children) = children {
            children.iter().any(|child| overlay_query.get(child).is_ok())
        } else {
            false
        };

        if snow_layer.has_snow() && !has_overlay {
            // Load snow texture (white/ice texture)
            let snow_texture = asset_server.load("textures/blocks/iceland/snow.png");

            // Determine overlay scale based on parent block's scale (same as damage overlay logic)
            let overlay_scale = if block_transform.scale.x > 1.5 {
                1.0  // Parent is already scaled (stone/coal)
            } else {
                64.0  // Parent not scaled (melon/wood)
            };

            // Z offset - render in front of block
            let z_offset = if overlay_scale > 1.5 {
                32.51  // For scaled overlays
            } else {
                0.51   // For unscaled overlays
            };

            info!("❄️ Creating snow overlay: stage={}, parent_scale={:.1}, overlay_scale={:.1}, z_offset={:.2}",
                snow_layer.stage, block_transform.scale.x, overlay_scale, z_offset);

            // Create overlay mesh (1x1 rectangle, will be scaled)
            let overlay_mesh = meshes.add(Rectangle::new(1.0, 1.0));

            // Create snow material with alpha blending
            let alpha = 0.7 + (snow_layer.stage as f32 / 4.0) * 0.3;
            let snow_material = materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 1.0, 1.0, alpha),
                base_color_texture: Some(snow_texture),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            });

            // Spawn overlay as child entity
            let overlay = commands.spawn((
                SnowOverlay,
                Mesh3d(overlay_mesh),
                MeshMaterial3d(snow_material),
                Transform::from_translation(Vec3::new(0.0, 0.0, z_offset))  // POSITIVE z_offset (in front of block!)
                    .with_scale(Vec3::splat(overlay_scale)),
                Visibility::Visible,
            )).id();

            // Add as child to block
            commands.entity(block_entity).add_child(overlay);

            spawned_count += 1;
        }
    }

    if blocks_with_snow > 0 {
        info!("❄️ Snow update: {} blocks with snow, {} new overlays spawned", blocks_with_snow, spawned_count);
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
