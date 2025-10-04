use bevy::prelude::*;
use crate::components::background::Background;
use crate::components::block::BiomeType;
use crate::components::hero::Hero;
use crate::resources::asset_handles::{GameTextures, BlockTextures};

/// Determine biome based on Y position
fn get_biome_at_y(y: f32) -> BiomeType {
    if y > 50.0 * 64.0 {
        BiomeType::End
    } else if y > 0.0 {
        BiomeType::Plains  // Surface biomes
    } else if y > -100.0 * 64.0 {
        BiomeType::Underground
    } else {
        BiomeType::Nether  // Deep underground
    }
}

/// Get biome transition Y positions
fn get_biome_transitions() -> Vec<(f32, BiomeType)> {
    vec![
        (50.0 * 64.0, BiomeType::End),
        (0.0, BiomeType::Plains),
        (-100.0 * 64.0, BiomeType::Underground),
        (-200.0 * 64.0, BiomeType::Nether),
    ]
}

/// Simple background that follows camera and fills screen based on biome
pub fn update_background(
    mut commands: Commands,
    hero_query: Query<&Transform, With<Hero>>,
    camera_query: Query<&Transform, (With<Camera>, Without<Hero>, Without<Background>)>,
    mut background_query: Query<(Entity, &Background, &mut Transform), (Without<Camera>, Without<Hero>)>,
    _textures: Res<GameTextures>,
    block_textures: Res<BlockTextures>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    windows: Query<&Window>,
) {
    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let (window_width, window_height) = if let Some(window) = windows.iter().next() {
        (window.width(), window.height())
    } else {
        return;
    };

    let current_biome = get_biome_at_y(hero_transform.translation.y);
    let block_size = 64.0;

    // Calculate grid size to fill screen with extra padding
    let viewport_width: f32 = window_width * 6.0;
    let viewport_height: f32 = window_height * 6.0;
    let grid_width = (viewport_width / block_size).ceil() as i32;
    let grid_height = (viewport_height / block_size).ceil() as i32;
    let total_blocks_needed = (grid_width * grid_height) as usize;

    // Only recreate if background doesn't exist, biome changed, or grid size changed
    let needs_recreate = if let Some((_, background, _)) = background_query.iter().next() {
        background.biome != current_biome || background.grid_width != grid_width || background.grid_height != grid_height
    } else {
        true
    };

    if needs_recreate {
        // Despawn old background
        for (entity, _, _) in background_query.iter() {
            commands.entity(entity).despawn();
        }

        // Get biome colors and texture
        let (bg_color, bg_texture) = match current_biome {
            BiomeType::Nether => (
                Color::srgb(0.4, 0.4, 0.4),
                block_textures.netherrack.clone()
            ),
            BiomeType::End => (
                Color::srgb(0.2, 0.2, 0.2),
                block_textures.obsidian.clone()
            ),
            BiomeType::Underground => (
                Color::srgb(0.3, 0.3, 0.3),
                block_textures.netherrack.clone()
            ),
            BiomeType::Plains | BiomeType::Forest | BiomeType::Desert | BiomeType::Mountains => (
                Color::srgb(0.6, 0.7, 0.9),
                block_textures.netherrack.clone()
            ),
        };

        let material = materials.add(StandardMaterial {
            base_color: bg_color,
            base_color_texture: Some(bg_texture),
            unlit: true,
            alpha_mode: AlphaMode::Opaque,
            ..default()
        });

        let mesh_handle = meshes.add(Rectangle::new(block_size, block_size));

        let camera_x = camera_transform.translation.x;
        let camera_y = camera_transform.translation.y;

        // Spawn background grid centered on camera
        for x in -grid_width/2..grid_width/2 {
            for y in -grid_height/2..grid_height/2 {
                commands.spawn((
                    Background {
                        biome: current_biome,
                        grid_width,
                        grid_height,
                    },
                    Mesh3d(mesh_handle.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(
                        camera_x + (x as f32 * block_size),
                        camera_y + (y as f32 * block_size),
                        -10.0
                    ),
                ));
            }
        }

        info!("Background recreated: biome {:?}", current_biome);
    } else {
        // Update all background blocks to follow camera - SNAPPED TO GRID
        let camera_x = camera_transform.translation.x;
        let camera_y = camera_transform.translation.y;

        // Snap camera position to block grid
        let snapped_camera_x = (camera_x / block_size).round() * block_size;
        let snapped_camera_y = (camera_y / block_size).round() * block_size;

        // Get grid dimensions from the first background entity
        let stored_grid = if let Some((_, bg, _)) = background_query.iter().next() {
            (bg.grid_width, bg.grid_height)
        } else {
            return;
        };

        let mut idx = 0;
        for (_, _, mut bg_transform) in background_query.iter_mut() {
            let x = (idx % stored_grid.0) as i32 - stored_grid.0/2;
            let y = (idx / stored_grid.0) as i32 - stored_grid.1/2;

            bg_transform.translation.x = snapped_camera_x + (x as f32 * block_size);
            bg_transform.translation.y = snapped_camera_y + (y as f32 * block_size);
            bg_transform.translation.z = -10.0;

            idx += 1;
        }
    }
}
