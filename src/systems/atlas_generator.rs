use bevy::asset::{LoadState, RenderAssetUsages};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Creates a horizontal 4x1 atlas texture: [first | second | third | fourth].
///
/// Every face is resampled (nearest) to the first face's size and converted to
/// RGBA8, so a texture with another size or pixel format cannot index out of
/// bounds; it is just stretched into its cell.
pub fn create_atlas_4x1(first: &Image, second: &Image, third: &Image, fourth: &Image) -> Image {
    let width = first.width().max(1);
    let height = first.height().max(1);

    let mut atlas_image = Image::new_fill(
        Extent3d {
            width: width * 4,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    for (cell, face) in [first, second, third, fourth].into_iter().enumerate() {
        let (face_width, face_height) = (face.width().max(1), face.height().max(1));
        for y in 0..height {
            for x in 0..width {
                let color = face
                    .get_color_at(x * face_width / width, y * face_height / height)
                    .unwrap_or(Color::srgb(1.0, 0.0, 1.0));
                let _ = atlas_image.set_color_at(cell as u32 * width + x, y, color);
            }
        }
    }

    atlas_image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        ..default()
    });

    atlas_image
}

/// Replaces textures that failed to load with a magenta/black checker so the
/// world still builds, and logs which file is broken (once per handle).
/// Returns true when every texture is available.
pub fn textures_ready_or_placeholder(
    handles: &[&Handle<Image>],
    images: &mut Assets<Image>,
    asset_server: &AssetServer,
) -> bool {
    let mut ready = true;
    for handle in handles {
        if images.contains(*handle) {
            continue;
        }
        if let Some(LoadState::Failed(error)) = asset_server.get_load_state(*handle) {
            let path = handle
                .path()
                .map_or_else(|| "<unnamed>".to_string(), ToString::to_string);
            error!("Block texture {path} failed to load ({error}); using placeholder");
            let _ = images.insert(*handle, placeholder_texture());
        } else {
            ready = false;
        }
    }
    ready
}

/// 16x16 magenta/black checker, the usual "missing texture" look.
pub fn placeholder_texture() -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    for y in 0..16 {
        for x in 0..16 {
            if (x / 8 + y / 8) % 2 == 0 {
                let _ = image.set_color_at(x, y, Color::srgb(1.0, 0.0, 1.0));
            }
        }
    }
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mismatched_faces_are_resampled_instead_of_panicking() {
        let big = placeholder_texture();
        let small = Image::new_fill(
            Extent3d {
                width: 4,
                height: 2,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 255, 255, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD,
        );
        let atlas = create_atlas_4x1(&big, &small, &big, &small);
        assert_eq!((atlas.width(), atlas.height()), (64, 16));
        assert_eq!(
            atlas.get_color_at(16 + 15, 15).unwrap().to_srgba(),
            Srgba::WHITE
        );
    }
}
