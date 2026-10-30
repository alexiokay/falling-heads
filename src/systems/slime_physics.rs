//! Brief contact adhesion followed by a bounded spring; never a permanent weld.
use bevy::prelude::*;
#[derive(Component)]
pub(crate) struct SlimeCoat {
    pub until: f32,
}
#[derive(Component, Clone, Copy)]
pub(crate) struct SlimeContact {
    pub support: Entity,
    pub release_at: f32,
    pub rebound: f32,
    pub horizontal: f32,
}
impl SlimeContact {
    pub fn new(support: Entity, now: f32, velocity: Vec2) -> Self {
        Self {
            support,
            release_at: now + 0.22,
            rebound: (velocity.y.abs() * 1.08).clamp(430., 590.),
            horizontal: velocity.x * 0.45,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adhesion_has_a_short_deadline_and_bounded_release() {
        for speed in [20., 400., 2000.] {
            let c = SlimeContact::new(Entity::PLACEHOLDER, 10., Vec2::new(200., -speed));
            assert!((c.release_at - 10.22).abs() < 0.001);
            assert!((430. ..=590.).contains(&c.rebound));
            assert_eq!(c.horizontal, 90.);
        }
    }
}
