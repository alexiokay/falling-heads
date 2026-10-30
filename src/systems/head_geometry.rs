//! Face geometry shared by effects/UI; root scaling is applied once by callers.
use bevy::{camera::primitives::MeshAabb, prelude::*};
#[derive(Clone, Copy)]
pub(crate) struct FaceBounds {
    pub center: Vec3,
    pub size: Vec3,
}
impl Default for FaceBounds {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            size: Vec3::new(64., 64., 0.),
        }
    }
}
pub(crate) fn bounds(meshes: &Assets<Mesh>, mesh: Option<&Mesh3d>) -> FaceBounds {
    mesh.and_then(|m| meshes.get(&m.0))
        .and_then(|m| m.get_aabb())
        .map(|b| FaceBounds {
            center: b.center.into(),
            size: Vec3::from(b.half_extents) * 2.,
        })
        .unwrap_or_default()
}
pub(crate) fn effect_scale(face: FaceBounds, t: &Transform) -> Vec2 {
    (face.size.xy() * t.scale.xy().abs() / 64.).max(Vec2::splat(0.05))
}
pub(crate) fn overlay_transform(face: FaceBounds, owner: &Transform) -> Transform {
    let scale = effect_scale(face, owner);
    Transform {
        translation: owner.transform_point(face.center + Vec3::Z * (face.size.z * 0.5))
            + Vec3::Z * 0.5,
        rotation: owner.rotation,
        scale: Vec3::new(scale.x, scale.y, 1.),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smaller_mesh_and_nonuniform_transform_use_actual_dimensions() {
        let mut assets = Assets::<Mesh>::default();
        let mesh = Mesh3d(assets.add(Rectangle::new(32., 20.)));
        let face = bounds(&assets, Some(&mesh));
        assert_eq!(face.size.xy(), Vec2::new(32., 20.));
        assert_eq!(
            effect_scale(face, &Transform::from_scale(Vec3::new(0.5, 2., 1.))),
            Vec2::new(0.25, 0.625)
        );
    }
}
