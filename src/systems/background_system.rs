use bevy::prelude::*;
use crate::components::background::{Background, BackgroundCache};
use crate::components::block::BiomeType;
use crate::components::hero::Hero;
use crate::resources::asset_handles::{GameTextures, BlockTextures};
use crate::utils::biome_utils::determine_biome_at_position;

/// Get material for a specific biome
fn get_biome_material(
    biome: BiomeType,
    block_textures: &BlockTextures,
    materials: &mut Assets<StandardMaterial>,
    bg_cache: &mut BackgroundCache,
) -> Handle<StandardMaterial> {
    // Return cached material if it exists
    if let Some(material) = bg_cache.materials.get(&biome) {
        return material.clone();
    }

    // Create new material for this biome
    // All backgrounds darkened to 0.4 for consistent visibility
    info!("Creating background material for biome: {:?}", biome);
    let (bg_color, bg_texture) = match biome {
        BiomeType::Nether => (
            Color::srgb(0.4, 0.4, 0.4),
            block_textures.nether.clone()
        ),
        BiomeType::End => (
            Color::srgb(0.4, 0.4, 0.4),
            block_textures.obsidian.clone()
        ),
        BiomeType::Underground => (
            Color::srgb(0.4, 0.4, 0.4),
            block_textures.stone.clone()  // Use stone for underground
        ),
        BiomeType::Desert => (
            Color::srgb(0.4, 0.4, 0.4),
            block_textures.sandstone.clone()
        ),
        BiomeType::Forest => (
            Color::srgb(0.4, 0.4, 0.4),
            block_textures.oak_log.clone()
        ),
        BiomeType::Plains => (
            Color::srgb(0.4, 0.4, 0.4),
            block_textures.dirt.clone()  // Use dirt for plains
        ),
        BiomeType::Mountains => (
            Color::srgb(0.4, 0.4, 0.4),
            block_textures.stone.clone()  // Use stone for mountains
        ),
        BiomeType::Iceland => (
            Color::srgb(0.4, 0.4, 0.4),
            block_textures.packed_ice.clone()  // Use packed ice for iceland
        ),
    };

    let material = materials.add(StandardMaterial {
        base_color: bg_color,
        base_color_texture: Some(bg_texture),
        unlit: true,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });

    bg_cache.materials.insert(biome, material.clone());
    material
}

/// Background system that creates tile-based backgrounds matching biomes
pub fn update_background(
    mut commands: Commands,
    hero_query: Query<&Transform, With<Hero>>,
    camera_query: Query<&Transform, (With<Camera3d>, Without<Hero>, Without<Background>)>,
    mut background_query: Query<(Entity, &mut Background, &mut Transform, &mut MeshMaterial3d<StandardMaterial>), (Without<Camera>, Without<Hero>)>,
    _textures: Res<GameTextures>,
    block_textures: Res<BlockTextures>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut bg_cache: ResMut<BackgroundCache>,
    windows: Query<&Window>,
) {
    let Ok(_hero_transform) = hero_query.single() else {
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

    // Background tiles are visual only (64x64 pixels)
    // Biomes are sampled based on camera's WORLD Y coordinate (blocks are 64*64 = 4096 world units apart)
    let bg_tile_size = 64.0;  // Visual size of each background tile

    // Calculate grid size to fill screen with extra padding
    let viewport_width: f32 = window_width * 6.0;
    let viewport_height: f32 = window_height * 6.0;
    let grid_width = (viewport_width / bg_tile_size).ceil() as i32;
    let grid_height = (viewport_height / bg_tile_size).ceil() as i32;

    // Check if we need to recreate the grid (size changed or no backgrounds exist)
    let needs_recreate = if let Some((_, background, _, _)) = background_query.iter().next() {
        background.grid_width != grid_width || background.grid_height != grid_height
    } else {
        true
    };

    if needs_recreate {
        // Despawn old background
        for (entity, _, _, _) in background_query.iter() {
            commands.entity(entity).despawn();
        }

        // PERFORMANCE FIX: Reuse single mesh for all backgrounds
        let mesh_handle = bg_cache.mesh.get_or_insert_with(|| {
            meshes.add(Rectangle::new(bg_tile_size, bg_tile_size))
        }).clone();

        let camera_x = camera_transform.translation.x;
        let camera_y = camera_transform.translation.y;

        // Spawn background grid centered on camera
        // Each tile determines its biome based on its WORLD Y position (where blocks exist)
        for x in -grid_width/2..grid_width/2 {
            for y in -grid_height/2..grid_height/2 {
                let tile_x = camera_x + (x as f32 * bg_tile_size);
                let tile_y = camera_y + (y as f32 * bg_tile_size);

                // Determine biome at THIS tile's world position (same calculation as blocks use)
                let tile_biome = determine_biome_at_position(tile_x, tile_y);
                let material = get_biome_material(tile_biome, &block_textures, &mut materials, &mut bg_cache);

                commands.spawn((
                    Background {
                        biome: tile_biome,
                        grid_width,
                        grid_height,
                    },
                    Mesh3d(mesh_handle.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(tile_x, tile_y, -10.0),
                ));
            }
        }

        info!("Background grid recreated: {}x{} tiles", grid_width, grid_height);
    } else {
        // PERFORMANCE FIX: Only update backgrounds when camera actually moves significantly
        let camera_x = camera_transform.translation.x;
        let camera_y = camera_transform.translation.y;

        // Snap camera position to tile grid
        let snapped_camera_x = (camera_x / bg_tile_size).round() * bg_tile_size;
        let snapped_camera_y = (camera_y / bg_tile_size).round() * bg_tile_size;

        // Check if camera has moved at least half a tile (32 units) since last update
        if let Some((_, _, bg_transform, _)) = background_query.iter().next() {
            let last_camera_x = bg_transform.translation.x;
            let last_camera_y = bg_transform.translation.y;

            let camera_moved_x = (snapped_camera_x - last_camera_x).abs();
            let camera_moved_y = (snapped_camera_y - last_camera_y).abs();

            // Only update if camera moved at least half a tile
            if camera_moved_x < bg_tile_size * 0.5 && camera_moved_y < bg_tile_size * 0.5 {
                return; // Camera hasn't moved enough, skip update
            }
        }

        // Get grid dimensions from the first background entity
        let stored_grid = if let Some((_, bg, _, _)) = background_query.iter().next() {
            (bg.grid_width, bg.grid_height)
        } else {
            return;
        };

        // Update each background tile position AND check if its biome changed based on its new world position
        let mut idx = 0;
        for (_, mut background, mut bg_transform, mut material) in background_query.iter_mut() {
            let x = (idx % stored_grid.0) as i32 - stored_grid.0/2;
            let y = (idx / stored_grid.0) as i32 - stored_grid.1/2;

            let new_x = snapped_camera_x + (x as f32 * bg_tile_size);
            let new_y = snapped_camera_y + (y as f32 * bg_tile_size);

            bg_transform.translation.x = new_x;
            bg_transform.translation.y = new_y;
            bg_transform.translation.z = -10.0;

            // Check if THIS tile's biome changed based on its new world position
            let tile_biome = determine_biome_at_position(new_x, new_y);
            if background.biome != tile_biome {
                background.biome = tile_biome;
                let new_material = get_biome_material(tile_biome, &block_textures, &mut materials, &mut bg_cache);
                *material = MeshMaterial3d(new_material);
            }

            idx += 1;
        }
    }
}
