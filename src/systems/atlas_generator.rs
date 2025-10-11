use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageSampler, ImageSamplerDescriptor, ImageAddressMode, ImageFilterMode};
use crate::resources::asset_handles::{BlockTextures, BlockAtlases};
use crate::resources::material_cache::MaterialCache;
use crate::components::block::BlockType;

/// Generates texture atlases for directional blocks
/// Runs once after all block textures are loaded
pub fn generate_block_atlases(
    mut commands: Commands,
    block_textures: Res<BlockTextures>,
    mut images: ResMut<Assets<Image>>,
    mut atlases_generated: Local<bool>,
) {
    if *atlases_generated {
        return;
    }

    // Wait for all textures to load
    let pumpkin_top = images.get(&block_textures.pumpkin_top);
    let pumpkin_side = images.get(&block_textures.pumpkin_side);
    let pumpkin_front = images.get(&block_textures.pumpkin_front);
    let oak_log = images.get(&block_textures.oak_log);
    let oak_log_top = images.get(&block_textures.oak_log_top);
    let oak_leaves = images.get(&block_textures.oak_leaves);
    let tnt_top = images.get(&block_textures.tnt_top);
    let tnt_bottom = images.get(&block_textures.tnt_bottom);
    let tnt_side = images.get(&block_textures.tnt_side);
    let cactus_top = images.get(&block_textures.cactus_top);
    let cactus_bottom = images.get(&block_textures.cactus_bottom);
    let cactus_side = images.get(&block_textures.cactus_side);

    if pumpkin_top.is_none() || pumpkin_side.is_none() || pumpkin_front.is_none()
        || oak_log.is_none() || oak_log_top.is_none() || oak_leaves.is_none()
        || tnt_top.is_none() || tnt_bottom.is_none() || tnt_side.is_none()
        || cactus_top.is_none() || cactus_bottom.is_none() || cactus_side.is_none() {
        return; // Textures not loaded yet
    }

    info!("Generating block texture atlases...");

    // Create pumpkin atlas: [top | bottom | sides | front] in 4x1 layout
    let pumpkin_atlas = create_atlas_4x1(
        pumpkin_top.unwrap(),      // Top face
        pumpkin_top.unwrap(),      // Bottom face (same as top)
        pumpkin_side.unwrap(),     // Side faces
        pumpkin_front.unwrap(),    // Front face (carved)
    );

    // Create oak log atlas: [top | bottom(=top) | sides(bark) | front(=bark)] in 4x1 layout
    // Standard 4x1 atlas with texture reuse for missing textures
    let oak_log_top_img = oak_log_top.unwrap();
    let oak_log_bark = oak_log.unwrap();
    let oak_log_atlas = create_atlas_4x1(
        oak_log_top_img,           // Top
        oak_log_top_img,           // Bottom (reuse top)
        oak_log_bark,              // Sides (bark)
        oak_log_bark,              // Front (reuse bark)
    );

    // Create oak leaves atlas: [leaves | leaves | leaves | leaves] in 4x1 layout
    // Same texture on all faces for uniform appearance
    let oak_leaves_img = oak_leaves.unwrap();
    let oak_leaves_atlas = create_atlas_4x1(
        oak_leaves_img,            // Top
        oak_leaves_img,            // Bottom
        oak_leaves_img,            // Sides
        oak_leaves_img,            // Front
    );

    // Create TNT atlas: [tnt_top | tnt_bottom | tnt_side | tnt_side] in 4x1 layout
    let tnt_atlas = create_atlas_4x1(
        tnt_top.unwrap(),          // Top
        tnt_bottom.unwrap(),       // Bottom
        tnt_side.unwrap(),         // Sides
        tnt_side.unwrap(),         // Front (reuse side texture)
    );

    // Generate cactus atlas (4x1 layout: top | bottom | side | side)
    let cactus_atlas = create_atlas_4x1(
        cactus_top.unwrap(),
        cactus_bottom.unwrap(),
        cactus_side.unwrap(),
        cactus_side.unwrap()
    );

    // Add atlases to assets after creating them
    let pumpkin_atlas_handle = images.add(pumpkin_atlas);
    let oak_log_atlas_handle = images.add(oak_log_atlas);
    let oak_leaves_atlas_handle = images.add(oak_leaves_atlas);
    let tnt_atlas_handle = images.add(tnt_atlas);
    let cactus_atlas_handle = images.add(cactus_atlas);

    commands.insert_resource(BlockAtlases {
        pumpkin: pumpkin_atlas_handle.clone(),
        oak_log: oak_log_atlas_handle.clone(),
        oak_leaves: oak_leaves_atlas_handle.clone(),
        melon: oak_leaves_atlas_handle.clone(), // Placeholder (atlas generation moved to main.rs)
        tnt: tnt_atlas_handle.clone(),
        sandstone: tnt_atlas_handle, // Placeholder - uses tnt atlas for now
        cactus: cactus_atlas_handle,
    });

    *atlases_generated = true;
    info!("Block texture atlases generated!");
}

/// Updates MaterialCache with atlas-based materials for directional blocks
/// Runs after atlases are generated
pub fn update_directional_materials(
    atlases: Option<Res<BlockAtlases>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cache: ResMut<MaterialCache>,
    mut updated: Local<bool>,
) {
    if *updated {
        return;
    }

    let Some(atlases) = atlases else {
        return; // Atlases not generated yet
    };

    info!("Updating materials for directional blocks...");

    // Update Pumpkin material to use atlas
    if let Some(pumpkin_mat_handle) = cache.materials.get(&BlockType::Pumpkin) {
        if let Some(pumpkin_mat) = materials.get_mut(pumpkin_mat_handle) {
            pumpkin_mat.base_color_texture = Some(atlases.pumpkin.clone());
            pumpkin_mat.cull_mode = None;  // Disable backface culling
            pumpkin_mat.double_sided = true;  // Enable double-sided rendering
            info!("Updated Pumpkin material with atlas texture (double-sided)");
        }
    }

    // Update Wood (Oak Log) material to use atlas
    if let Some(wood_mat_handle) = cache.materials.get(&BlockType::Wood) {
        if let Some(wood_mat) = materials.get_mut(wood_mat_handle) {
            wood_mat.base_color_texture = Some(atlases.oak_log.clone());
            wood_mat.cull_mode = None;  // Disable backface culling
            wood_mat.double_sided = true;  // Enable double-sided rendering
            info!("Updated Wood material with atlas texture (double-sided)");
        }
    }

    // Update Leaves material to use atlas
    if let Some(leaves_mat_handle) = cache.materials.get(&BlockType::Leaves) {
        if let Some(leaves_mat) = materials.get_mut(leaves_mat_handle) {
            leaves_mat.base_color_texture = Some(atlases.oak_leaves.clone());
            leaves_mat.cull_mode = None;  // Disable backface culling
            leaves_mat.double_sided = true;  // Enable double-sided rendering
            info!("Updated Leaves material with atlas texture (double-sided)");
        }
    }

    *updated = true;
    info!("Directional block materials updated!");
}

/// Creates a horizontal 3x1 atlas texture: [left | middle | right]
fn create_horizontal_atlas_3x1(
    left: &Image,
    middle: &Image,
    right: &Image,
) -> Image {
    // Assume all textures are the same size
    let width = left.width();
    let height = left.height();

    // Create a 3x1 atlas (3 times wider)
    let atlas_width = width * 3;
    let atlas_height = height;

    let mut atlas_data = Vec::with_capacity((atlas_width * atlas_height * 4) as usize);

    // Get image data
    let left_data = left.data.as_ref().expect("Left image should have data");
    let middle_data = middle.data.as_ref().expect("Middle image should have data");
    let right_data = right.data.as_ref().expect("Right image should have data");

    // For each row, concatenate left, middle, right pixels
    for y in 0..height {
        for x in 0..width {
            let pixel_idx = ((y * width + x) * 4) as usize;
            // Left section
            atlas_data.extend_from_slice(&left_data[pixel_idx..pixel_idx + 4]);
        }
        for x in 0..width {
            let pixel_idx = ((y * width + x) * 4) as usize;
            // Middle section
            atlas_data.extend_from_slice(&middle_data[pixel_idx..pixel_idx + 4]);
        }
        for x in 0..width {
            let pixel_idx = ((y * width + x) * 4) as usize;
            // Right section
            atlas_data.extend_from_slice(&right_data[pixel_idx..pixel_idx + 4]);
        }
    }

    let mut atlas_image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        atlas_data,
        left.texture_descriptor.format,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    atlas_image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        ..default()
    });

    atlas_image
}

/// Creates a horizontal 4x1 atlas texture: [first | second | third | fourth]
pub fn create_atlas_4x1(
    first: &Image,
    second: &Image,
    third: &Image,
    fourth: &Image,
) -> Image {
    // Assume all textures are the same size
    let width = first.width();
    let height = first.height();

    // Create a 4x1 atlas (4 times wider)
    let atlas_width = width * 4;
    let atlas_height = height;

    let mut atlas_data = Vec::with_capacity((atlas_width * atlas_height * 4) as usize);

    // Get image data
    let first_data = first.data.as_ref().expect("First image should have data");
    let second_data = second.data.as_ref().expect("Second image should have data");
    let third_data = third.data.as_ref().expect("Third image should have data");
    let fourth_data = fourth.data.as_ref().expect("Fourth image should have data");

    // For each row, concatenate all four textures
    for y in 0..height {
        for x in 0..width {
            let pixel_idx = ((y * width + x) * 4) as usize;
            atlas_data.extend_from_slice(&first_data[pixel_idx..pixel_idx + 4]);
        }
        for x in 0..width {
            let pixel_idx = ((y * width + x) * 4) as usize;
            atlas_data.extend_from_slice(&second_data[pixel_idx..pixel_idx + 4]);
        }
        for x in 0..width {
            let pixel_idx = ((y * width + x) * 4) as usize;
            atlas_data.extend_from_slice(&third_data[pixel_idx..pixel_idx + 4]);
        }
        for x in 0..width {
            let pixel_idx = ((y * width + x) * 4) as usize;
            atlas_data.extend_from_slice(&fourth_data[pixel_idx..pixel_idx + 4]);
        }
    }

    let mut atlas_image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        atlas_data,
        first.texture_descriptor.format,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    atlas_image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        ..default()
    });

    atlas_image
}
