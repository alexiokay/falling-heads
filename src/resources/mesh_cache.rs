use crate::resources::cube_mesh::{UVLayout, create_arrow_mesh, create_uv_cube};
use bevy::prelude::*;

#[derive(Resource)]
pub struct MeshCache {
    pub dead_bush_mesh: Handle<Mesh>,
    pub mushroom_mesh: Handle<Mesh>,
    pub block_mesh: Handle<Mesh>, // Shared Cuboid mesh for uniform blocks
    pub pumpkin_mesh: Handle<Mesh>, // Standard 4x1 atlas for pumpkins
    pub wood_mesh: Handle<Mesh>,  // Standard 4x1 atlas for wood/logs
    pub leaves_mesh: Handle<Mesh>, // Standard 4x1 atlas for leaves
    pub melon_mesh: Handle<Mesh>, // Standard 4x1 atlas for melons
    pub enemy_mesh: Handle<Mesh>, // Shared Rectangle mesh for enemies
    pub resource_mesh: Handle<Mesh>, // Shared Sphere mesh for resources
    pub arrow_mesh: Handle<Mesh>, // Shared 3D arrow mesh (thin rectangular prism)
    pub tnt_mesh: Handle<Mesh>,   // Shared UV-mapped cube for TNT (same UV system as blocks)
    pub cactus_mesh: Handle<Mesh>, // Shared UV-mapped cube for Cactus (4x1 atlas)
    pub sandstone_mesh: Handle<Mesh>, // Shared UV-mapped cube for Sandstone (4x1 atlas)
}

impl MeshCache {
    pub fn new(meshes: &mut ResMut<Assets<Mesh>>) -> Self {
        Self {
            dead_bush_mesh: meshes.add(dead_bush_mesh()),
            mushroom_mesh: meshes.add(mushroom_mesh()),
            block_mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            pumpkin_mesh: meshes.add(create_uv_cube(64.0, UVLayout::pumpkin())),
            wood_mesh: meshes.add(create_uv_cube(64.0, UVLayout::oak_log())),
            leaves_mesh: meshes.add(create_uv_cube(64.0, UVLayout::oak_leaves())),
            melon_mesh: meshes.add(create_uv_cube(64.0, UVLayout::melon())),
            enemy_mesh: meshes.add(Rectangle::new(64.0, 64.0)),
            resource_mesh: meshes.add(Sphere::new(0.15)),
            arrow_mesh: meshes.add(create_arrow_mesh()),
            tnt_mesh: meshes.add(create_uv_cube(64.0, UVLayout::tnt())), // Directional TNT with 4x1 atlas
            cactus_mesh: meshes.add(crate::resources::cube_mesh::create_cactus_mesh()), // Directional Cactus with 4x1 atlas
            sandstone_mesh: meshes.add(create_uv_cube(64.0, UVLayout::sandstone())), // Directional Sandstone with 4x1 atlas
        }
    }
}

/// A low stem and stepped cap sit on the supporting tile instead of filling a cube.
fn mushroom_mesh() -> Mesh {
    let parts = [
        (
            Vec3::new(0., -22., 20.),
            Vec3::new(8., 20., 8.),
            [0.78, 0.72, 0.55, 1.],
        ),
        (
            Vec3::new(0., -10., 20.),
            Vec3::new(32., 8., 26.),
            [0.65, 0.08, 0.045, 1.],
        ),
        (
            Vec3::new(0., -4., 20.),
            Vec3::new(22., 5., 18.),
            [0.82, 0.15, 0.07, 1.],
        ),
        (
            Vec3::new(-6., -4., 30.),
            Vec3::new(4., 4., 2.),
            [0.92, 0.83, 0.63, 1.],
        ),
        (
            Vec3::new(7., -10., 34.),
            Vec3::new(5., 4., 2.),
            [0.92, 0.83, 0.63, 1.],
        ),
    ];
    let mut result: Option<Mesh> = None;
    for (position, size, color) in parts {
        let mut part = Mesh::from(Cuboid::from_size(size))
            .transformed_by(Transform::from_translation(position));
        part.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![color; part.count_vertices()]);
        if let Some(mesh) = result.as_mut() {
            mesh.merge(&part).unwrap();
        } else {
            result = Some(part);
        }
    }
    result.unwrap()
}

/// Shared dry branching silhouette; no placeholder solid cube.
fn dead_bush_mesh() -> Mesh {
    let branches = [
        ((0., -30.), (1., 16.), 5.),
        ((0., -8.), (-21., 8.), 4.),
        ((1., 2.), (22., 17.), 4.),
        ((-12., 1.), (-17., 23.), 3.),
        ((-21., 8.), (-28., 10.), 3.),
        ((12., 10.), (15., 28.), 3.),
        ((1., 12.), (-5., 28.), 3.),
        ((22., 17.), (29., 24.), 2.5),
    ];
    let mut result: Option<Mesh> = None;
    for ((ax, ay), (bx, by), width) in branches {
        let a = Vec2::new(ax, ay);
        let b = Vec2::new(bx, by);
        let delta = b - a;
        let branch = Mesh::from(Cuboid::new(delta.length(), width, 5.)).transformed_by(
            Transform::from_translation(((a + b) * 0.5).extend(22.))
                .with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x))),
        );
        if let Some(mesh) = result.as_mut() {
            mesh.merge(&branch).unwrap();
        } else {
            result = Some(branch);
        }
    }
    result.unwrap()
}
