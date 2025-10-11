use bevy::prelude::*;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::asset::RenderAssetUsages;

/// UV layout for directional block textures
/// STANDARD 4x1 horizontal atlas: [top | bottom | sides | front]
/// Each section is 1/4 of the texture width (0.25)
#[derive(Clone, Copy, Debug)]
pub enum UVLayout {
    /// All faces use the same texture (uniform blocks like netherrack)
    Uniform,
    /// Directional blocks with standard 4x1 atlas: [top | bottom | sides | front]
    /// Missing textures will automatically reuse available ones
    Directional {
        /// UV region for top face
        top: (f32, f32, f32, f32),
        /// UV region for bottom face
        bottom: (f32, f32, f32, f32),
        /// UV region for side faces (left, right)
        sides: (f32, f32, f32, f32),
        /// UV region for front face (visible from camera)
        front: (f32, f32, f32, f32),
    },
}

impl Default for UVLayout {
    fn default() -> Self {
        UVLayout::Uniform
    }
}

impl UVLayout {
    /// Standard 4x1 atlas layout: [top | bottom | sides | front]
    /// Use this for all directional blocks (pumpkins, logs, etc.)
    pub fn standard_4x1(top: usize, bottom: usize, sides: usize, front: usize) -> Self {
        // Convert texture indices to UV regions (0.0-0.25, 0.25-0.5, 0.5-0.75, 0.75-1.0)
        let regions = [
            (0.0, 0.0, 0.25, 1.0),    // Slot 0
            (0.25, 0.0, 0.5, 1.0),    // Slot 1
            (0.5, 0.0, 0.75, 1.0),    // Slot 2
            (0.75, 0.0, 1.0, 1.0),    // Slot 3
        ];

        UVLayout::Directional {
            top: regions[top.min(3)],
            bottom: regions[bottom.min(3)],
            sides: regions[sides.min(3)],
            front: regions[front.min(3)],
        }
    }

    /// Pumpkin: [pumpkin_top | pumpkin_top | pumpkin_side | pumpkin_front]
    /// Atlas slots: [0:top, 1:bottom(=top), 2:sides, 3:front(carved)]
    pub fn pumpkin() -> Self {
        Self::standard_4x1(0, 0, 2, 3)  // top reused for bottom
    }

    /// Oak Log: [log_top | log_top | log | log]
    /// Atlas slots: [0:top, 1:bottom(=top), 2:sides(bark), 3:front(=bark)]
    /// If front texture missing, it uses sides texture
    pub fn oak_log() -> Self {
        Self::standard_4x1(0, 0, 2, 2)  // top reused for bottom, sides reused for front
    }

    /// Oak Leaves: [leaves | leaves | leaves | leaves]
    /// Atlas slots: All use same leaves texture (slot 0)
    /// Uniform leaves texture on all sides
    pub fn oak_leaves() -> Self {
        Self::standard_4x1(0, 0, 0, 0)  // Same texture on all faces
    }

    /// Melon: [melon_top | melon_top | melon_side | melon_side]
    /// Atlas slots: [0:top, 1:bottom(=top), 2:sides, 3:front(=sides)]
    pub fn melon() -> Self {
        Self::standard_4x1(0, 0, 2, 2)  // top reused for bottom, sides reused for front
    }

    /// TNT: [tnt_top | tnt_bottom | tnt_side | tnt_side]
    /// Atlas slots: [0:top, 1:bottom, 2:sides, 3:front(=sides)]
    pub fn tnt() -> Self {
        Self::standard_4x1(0, 1, 2, 2)  // top, bottom, sides for all 4 sides
    }

    /// Cactus: [cactus_top | cactus_bottom | cactus_side | cactus_side]
    /// Atlas slots: [0:top, 1:bottom, 2:sides, 3:front(=sides)]
    pub fn cactus() -> Self {
        Self::standard_4x1(0, 1, 2, 2)  // top, bottom, sides for all 4 sides
    }

    /// Sandstone: [sandstone_top | sandstone_bottom | sandstone | sandstone]
    /// Atlas slots: [0:top, 1:bottom, 2:sides, 3:front(=sides)]
    pub fn sandstone() -> Self {
        Self::standard_4x1(0, 1, 2, 2)  // top, bottom, sides for all 4 sides
    }
}

/// Helper function to generate UV coordinates for top/bottom faces
/// Pattern: bottom-left, bottom-right, top-right, top-left
#[inline]
fn uv_top_bottom(region: (f32, f32, f32, f32)) -> [[f32; 2]; 4] {
    [
        [region.0, region.3], // bottom-left
        [region.2, region.3], // bottom-right
        [region.2, region.1], // top-right
        [region.0, region.1], // top-left
    ]
}

/// Helper function to generate UV coordinates for front face
/// Pattern: left-bottom, left-top, right-top, right-bottom
#[inline]
fn uv_front(region: (f32, f32, f32, f32)) -> [[f32; 2]; 4] {
    [
        [region.0, region.3], // left-bottom
        [region.0, region.1], // left-top
        [region.2, region.1], // right-top
        [region.2, region.3], // right-bottom
    ]
}

/// Helper function to generate UV coordinates for right side face
/// Pattern: left-bottom, left-top, right-top, right-bottom (corrected for winding order)
#[inline]
fn uv_right(region: (f32, f32, f32, f32)) -> [[f32; 2]; 4] {
    [
        [region.0, region.3], // left-bottom
        [region.0, region.1], // left-top
        [region.2, region.1], // right-top
        [region.2, region.3], // right-bottom
    ]
}

/// Helper function to generate UV coordinates for left side face
/// Pattern: left-bottom, left-top, right-top, right-bottom (corrected for winding order)
#[inline]
fn uv_left(region: (f32, f32, f32, f32)) -> [[f32; 2]; 4] {
    [
        [region.0, region.3], // left-bottom
        [region.0, region.1], // left-top
        [region.2, region.1], // right-top
        [region.2, region.3], // right-bottom
    ]
}

/// Creates arrow mesh - simple rectangle (your texture is a 2D sprite)
/// For true 3D, you would need a different texture with all sides unwrapped
pub fn create_arrow_mesh() -> Mesh {
    // Your texture is 16x16 pixels showing a side view arrow
    // Use it as-is on a rectangle - rotation system handles pointing direction
    Rectangle::new(90.0, 48.0).into()
}

/// Creates a custom cube mesh with configurable UV mapping
/// Size is the edge length of the cube (default: 1.0)
pub fn create_uv_cube(size: f32, uv_layout: UVLayout) -> Mesh {
    let half = size / 2.0;

    // Optimized cube with 20 vertices (4 per face, 5 faces total)
    // Back face (-Z) removed as it's never visible from camera at Z=+1500
    // Only render 5 faces - skip back face (-Z) as it's never visible from camera
    let positions = vec![
        // Top face (+Y) - Fixed winding order
        [-half, half, half],
        [half, half, half],
        [half, half, -half],
        [-half, half, -half],

        // Bottom face (-Y) - Fixed winding order
        [-half, -half, -half],
        [half, -half, -half],
        [half, -half, half],
        [-half, -half, half],

        // Front face (+Z) - VISIBLE from camera (fixed winding order)
        [half, -half, half],
        [half, half, half],
        [-half, half, half],
        [-half, -half, half],

        // Right face (+X) - Fixed winding order for correct backface culling
        [half, -half, -half],
        [half, half, -half],
        [half, half, half],
        [half, -half, half],

        // Left face (-X) - Fixed winding order for correct backface culling
        [-half, -half, half],
        [-half, half, half],
        [-half, half, -half],
        [-half, -half, -half],
    ];

    // Normals for each face (5 faces, no back face)
    let normals = vec![
        // Top
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        // Bottom
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        // Front
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        // Right
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        // Left
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
    ];

    // UV coordinates based on layout
    // Note: In Bevy, UV coordinates have (0,0) at top-left, (1,1) at bottom-right
    // Each face has 4 vertices in order: bottom-left, bottom-right, top-right, top-left
    // Build UV coordinates using helper functions (single source of truth)
    let full_texture = (0.0, 0.0, 1.0, 1.0);
    let uvs = match uv_layout {
        UVLayout::Uniform => {
            // All faces use full texture (0,0) to (1,1) - 5 faces, no back
            let mut uv_data = Vec::with_capacity(20);
            uv_data.extend_from_slice(&uv_top_bottom(full_texture));      // Top
            uv_data.extend_from_slice(&uv_top_bottom(full_texture));      // Bottom
            uv_data.extend_from_slice(&uv_front(full_texture));           // Front
            uv_data.extend_from_slice(&uv_right(full_texture));           // Right
            uv_data.extend_from_slice(&uv_left(full_texture));            // Left
            uv_data
        }
        UVLayout::Directional { top, bottom, sides, front } => {
            // Standard 4x1 atlas for ALL directional blocks
            // Missing textures handled by reusing available slots
            let mut uv_data = Vec::with_capacity(20);
            uv_data.extend_from_slice(&uv_top_bottom(top));          // Top
            uv_data.extend_from_slice(&uv_top_bottom(bottom));       // Bottom
            uv_data.extend_from_slice(&uv_front(front));             // Front (visible from camera)
            uv_data.extend_from_slice(&uv_right(sides));             // Right
            uv_data.extend_from_slice(&uv_left(sides));              // Left
            uv_data
        }
    };

    // Indices for 5 faces (no back face), 2 triangles per face
    let indices = vec![
        0, 1, 2, 2, 3, 0,       // Top
        4, 5, 6, 6, 7, 4,       // Bottom
        8, 9, 10, 10, 11, 8,    // Front (visible)
        12, 13, 14, 14, 15, 12, // Right
        16, 17, 18, 18, 19, 16, // Left
    ];

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}
