use bevy::prelude::*;

#[derive(Resource)]
pub struct MeshCache {
    pub block_mesh: Handle<Mesh>,  // Shared Cuboid mesh for all blocks
    pub enemy_mesh: Handle<Mesh>,  // Shared Rectangle mesh for enemies
    pub resource_mesh: Handle<Mesh>,  // Shared Sphere mesh for resources
}

impl MeshCache {
    pub fn new(meshes: &mut ResMut<Assets<Mesh>>) -> Self {
        Self {
            block_mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),  // Unit cube for blocks
            enemy_mesh: meshes.add(Rectangle::new(64.0, 64.0)),  // Bigger rectangle for enemies (same as hero)
            resource_mesh: meshes.add(Sphere::new(0.15)),  // Small sphere for resources
        }
    }
}
