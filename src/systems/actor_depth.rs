//! Shared depth policy. Physics owns root Position; presentation must never feed
//! interpolated XY back into it. Root collider Z remains part of the existing
//! contact contract until a complete render-child migration is undertaken.
use crate::components::{enemy::Enemy, hero::Hero, villager::Villager};
use bevy::prelude::*;
// Ordered depth bands prevent coplanar scene/effect surfaces.
pub(crate) const BACKGROUND: f32 = -100.;
// Recessed masonry has real thickness: front -44, back -84. Leave air
// behind terrain's rear face (-42) and ahead of the wallpaper (-100).
pub(crate) const STRUCTURE_THICKNESS: f32 = 40.;
pub(crate) const STRUCTURE_BACKING: f32 = -64.;
pub(crate) const MIST: f32 = -5.;
pub(crate) const ICE_OVERLAY: f32 = 1.;
pub(crate) const FIRE_OVERLAY: f32 = 3.;
pub(crate) const TERRAIN_CENTER: f32 = -10.;
pub(crate) const WATER_FRONT: f32 = 24.;
pub(crate) const HEAD_FACE: f32 = 30.;
const FLYING_FACE: f32 = 55.;
const HEAD_BAND: f32 = 8.;
pub struct ActorDepthPlugin;
impl Plugin for ActorDepthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedPostUpdate,
            actor_depth.before(avian3d::prelude::PhysicsSystems::Prepare),
        );
    }
}
// All characters share a foreground plane. Stable, separated depths remove
// coplanar head/death flashes while retaining the same 2D gameplay coordinates.
fn actor_depth(
    mut actors: Query<
        (
            Entity,
            &mut Transform,
            Option<&Enemy>,
            Option<&mut avian3d::prelude::Position>,
        ),
        (
            Or<(With<Hero>, With<Villager>, With<Enemy>)>,
            Without<super::dragon_flyover::EndDragon>,
        ),
    >,
    mut order: Local<Vec<Entity>>,
) {
    order.clear();
    order.extend(actors.iter().map(|(e, _, _, _)| e));
    order.sort_unstable();
    let count = order.len();
    for (rank, entity) in order.iter().copied().enumerate() {
        if let Ok((_, mut transform, enemy, position)) = actors.get_mut(entity) {
            let flying =
                enemy.is_some_and(|e| e.enemy_type == crate::components::enemy::EnemyType::Ghast);
            let z = actor_layer(flying, rank, count);
            // Write authoritative physics depth, never the interpolated render
            // transform: marking that transform changed feeds its older XY back
            // into physics and slows movement above the fixed simulation rate.
            if let Some(mut position) = position {
                if position.z != z {
                    position.z = z;
                }
            } else if transform.translation.z != z {
                transform.translation.z = z;
            }
        }
    }
}

// Keep ordinary faces in a bounded foreground band, including large crowds.
// Depth offsets must not grow without bound with actor count.
fn actor_layer(flying: bool, rank: usize, count: usize) -> f32 {
    (if flying { FLYING_FACE } else { HEAD_FACE }) + HEAD_BAND * rank as f32 / count.max(1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crowded_actor_depth_stays_in_the_same_water_layer() {
        for count in [1, 100, 1000, 10000] {
            for flying in [false, true] {
                let mut previous = 0.;
                for rank in 0..count {
                    let z = actor_layer(flying, rank, count);
                    assert!(z > previous && z < 64.);
                    previous = z;
                }
            }
        }
    }
}
#[cfg(test)]
mod motion_regression {
    use super::*;
    use avian3d::prelude::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    #[test]
    fn visual_depth_preserves_real_time_motion_at_different_frame_rates() {
        for fps in [30_u32, 60, 90, 120, 144] {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, TransformPlugin, AssetPlugin::default()))
                .init_asset::<Mesh>()
                .add_plugins(
                    PhysicsPlugins::default()
                        .with_length_unit(64.)
                        .build()
                        .disable::<PhysicsInterpolationPlugin>()
                        .add(PhysicsInterpolationPlugin::interpolate_all()),
                )
                .insert_resource(Gravity(Vec3::ZERO))
                .insert_resource(SubstepCount(1))
                .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                    1. / fps as f64,
                )))
                .add_systems(FixedPostUpdate, actor_depth.before(PhysicsSystems::Prepare));
            let actor = app
                .world_mut()
                .spawn((
                    Hero {
                        move_direction: 1.,
                        direction_change_timer: Timer::from_seconds(1., TimerMode::Repeating),
                    },
                    RigidBody::Dynamic,
                    Collider::sphere(1.),
                    Mass(1.),
                    Transform::from_xyz(0., 0., 30.),
                    LinearVelocity(Vec3::X * 300.),
                ))
                .id();
            app.finish();
            app.cleanup();
            app.update();
            for _ in 0..fps * 2 {
                app.update();
            }
            let physical = app.world().get::<Position>(actor).unwrap().x;
            let rendered = app.world().get::<Transform>(actor).unwrap().translation.x;
            eprintln!(
                "MOTION {fps} FPS: expected 600 units; physics={physical}, rendered={rendered}"
            );
            assert!(
                (physical - 600.).abs() < 15.,
                "{fps} FPS lost simulation distance: {physical}"
            );
            assert!(
                (rendered - 600.).abs() < 20.,
                "{fps} FPS lost visual distance: {rendered}"
            );
        }
    }
}
