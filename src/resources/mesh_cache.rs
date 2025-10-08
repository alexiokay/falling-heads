use bevy::prelude::*;
use crate::resources::cube_mesh::{create_uv_cube, create_arrow_mesh, UVLayout};

#[derive(Resource)]
pub struct MeshCache {
    pub block_mesh: Handle<Mesh>,   // Shared Cuboid mesh for uniform blocks
    pub pumpkin_mesh: Handle<Mesh>, // Standard 4x1 atlas for pumpkins
    pub wood_mesh: Handle<Mesh>,    // Standard 4x1 atlas for wood/logs
    pub leaves_mesh: Handle<Mesh>,  // Standard 4x1 atlas for leaves
    pub melon_mesh: Handle<Mesh>,   // Standard 4x1 atlas for melons
    pub enemy_mesh: Handle<Mesh>,   // Shared Rectangle mesh for enemies
    pub resource_mesh: Handle<Mesh>,  // Shared Sphere mesh for resources
    pub arrow_mesh: Handle<Mesh>,   // Shared 3D arrow mesh (thin rectangular prism)
}

impl MeshCache {
    pub fn new(meshes: &mut ResMut<Assets<Mesh>>) -> Self {
        Self {
            block_mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            pumpkin_mesh: meshes.add(create_uv_cube(64.0, UVLayout::pumpkin())),
            wood_mesh: meshes.add(create_uv_cube(64.0, UVLayout::oak_log())),
            leaves_mesh: meshes.add(create_uv_cube(64.0, UVLayout::oak_leaves())),
            melon_mesh: meshes.add(create_uv_cube(64.0, UVLayout::melon())),
            enemy_mesh: meshes.add(Rectangle::new(64.0, 64.0)),
            resource_mesh: meshes.add(Sphere::new(0.15)),
            arrow_mesh: meshes.add(create_arrow_mesh()),
        }
    }
}
