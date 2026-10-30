//! Rounded head-to-head contact in dense crowds; terrain bounce still uses its
//! existing square contact shape. A small nudge is reserved for sustained stalls.
use crate::components::{
    block::{Block, BlockType},
    hero::Hero,
    villager::Villager,
};
use avian3d::prelude::{Collider, LinearVelocity};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Default)]
struct Sample {
    anchor: Vec2,
    elapsed: f32,
    clear_time: f32,
    rounded: bool,
    cooldown: f32,
}
pub struct CrowdSlidePlugin;
fn crowded_collider() -> Collider {
    // Preserve square-looking corner contacts. The sphere experiment saved too
    // little whole-frame time to justify changing visible crowd packing.
    Collider::round_cuboid(48., 48., 48., 8.)
}
impl Plugin for CrowdSlidePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            slide
                .after(crate::plugins::physics_plugin::PhysicsSet::AI)
                .before(crate::plugins::physics_plugin::PhysicsSet::Collision)
                .run_if(in_state(crate::states::InGame)),
        );
    }
}
fn qualifies(neighbours: usize, seconds: f32, displacement: f32) -> bool {
    neighbours >= 3 && seconds >= 0.75 && displacement < 12.0
}
fn slide(
    time: Res<Time>,
    mut heads: Query<
        (Entity, &Transform, &mut LinearVelocity, &mut Collider),
        Or<(With<Hero>, With<Villager>)>,
    >,
    blocks: Query<(&Transform, &BlockType), With<Block>>,
    mut samples: Local<HashMap<Entity, Sample>>,
    terrain: Res<crate::resources::spatial_hash::SpatialHash>,
) {
    let positions: HashMap<_, _> = heads
        .iter()
        .map(|(e, t, _, _)| (e, t.translation.xy()))
        .collect();
    let mut bins = HashMap::<(i32, i32), Vec<(Entity, Vec2)>>::new();
    for (&e, &p) in &positions {
        let c = (p / 96.0).floor().as_ivec2();
        bins.entry((c.x, c.y)).or_default().push((e, p));
    }
    samples.retain(|e, _| positions.contains_key(e));
    for (entity, t, mut velocity, mut collider) in &mut heads {
        let p = t.translation.xy();
        let c = (p / 96.0).floor().as_ivec2();
        let mut count = 0;
        let mut away = 0.0;
        for y in -1..=1 {
            for x in -1..=1 {
                if let Some(near) = bins.get(&(c.x + x, c.y + y)) {
                    for &(other, q) in near {
                        if other != entity && p.distance_squared(q) < 96.0_f32.powi(2) {
                            count += 1;
                            away += p.x - q.x;
                        }
                    }
                }
            }
        }
        let state = samples.entry(entity).or_insert_with(|| Sample {
            anchor: p,
            ..default()
        });
        state.elapsed += time.delta_secs();
        state.cooldown = (state.cooldown - time.delta_secs()).max(0.0);
        if count >= 3 {
            state.clear_time = 0.0;
            if !state.rounded {
                *collider = crowded_collider();
                state.rounded = true;
            }
        } else {
            state.clear_time += time.delta_secs();
            if state.rounded && state.clear_time > 1.0 {
                *collider = Collider::cuboid(64.0, 64.0, 64.0);
                state.rounded = false;
            }
        }
        if qualifies(count, state.elapsed, p.distance(state.anchor)) && state.cooldown == 0.0 {
            let direction = if away.abs() > 0.1 {
                away.signum()
            } else if entity.to_bits() % 2 == 0 {
                1.0
            } else {
                -1.0
            };
            for sign in [direction, -direction] {
                let candidate = p + Vec2::X * sign * 12.0;
                let half = 32.0 * t.scale.x;
                let blocked = terrain
                    .nearby_iter(candidate.x, candidate.y)
                    .filter_map(|e| blocks.get(e).ok())
                    .any(|(b, kind)| {
                        *kind != BlockType::Cobweb
                            && (candidate - b.translation.xy())
                                .abs()
                                .cmplt(Vec2::splat(32.0 + half))
                                .all()
                    });
                if !blocked {
                    velocity.x = (velocity.x + sign * 45.0).clamp(-220.0, 220.0);
                    break;
                }
            }
            state.cooldown = 1.5;
        }
        if state.elapsed >= 0.75 {
            state.anchor = p;
            state.elapsed = 0.0;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assistance_requires_a_sustained_crowded_stall() {
        assert!(!qualifies(2, 2.0, 0.0));
        assert!(!qualifies(5, 0.2, 0.0));
        assert!(!qualifies(5, 1.0, 40.0));
        assert!(qualifies(5, 1.0, 3.0));
    }
}
