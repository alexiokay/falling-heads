//! Exact nearest-target index rebuilt once per gameplay Update, before consumers.
//! Balanced k-d tree: O(H log H) build, usually O(log H) query; worst case O(H).
//! All existing friendly targets remain eligible. Ties resolve by entity ID.
use crate::components::{hero::Hero, villager::Villager};
use bevy::prelude::*;
#[derive(Clone, Copy, Debug)]
pub(crate) struct Target {
    pub entity: Entity,
    pub position: Vec3,
}
#[derive(Resource, Default)]
pub(crate) struct ActorTargets {
    points: Vec<Target>,
}
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TargetsReady;
pub(crate) struct TargetIndexPlugin;
impl Plugin for TargetIndexPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActorTargets>()
            .add_systems(Update, rebuild.in_set(TargetsReady));
    }
}
fn rebuild(
    mut targets: ResMut<ActorTargets>,
    // Dying actors and the hidden launcher passenger are not targets.
    actors: Query<
        (Entity, &Transform),
        (
            Or<(With<Hero>, With<Villager>)>,
            Without<crate::components::damage_visual::DeathAnimation>,
            Without<crate::systems::launcher::LauncherPassenger>,
            Without<crate::components::hero::Parked>,
        ),
    >,
) {
    targets.points.clear();
    targets
        .points
        .extend(actors.iter().filter_map(|(entity, t)| {
            t.translation.is_finite().then_some(Target {
                entity,
                position: t.translation,
            })
        }));
    partition(&mut targets.points, 0);
}
fn partition(points: &mut [Target], axis: usize) {
    if points.len() < 2 {
        return;
    }
    let mid = points.len() / 2;
    points.select_nth_unstable_by(mid, |a, b| {
        a.position[axis]
            .total_cmp(&b.position[axis])
            .then_with(|| a.entity.cmp(&b.entity))
    });
    let (left, right) = points.split_at_mut(mid);
    partition(left, (axis + 1) % 3);
    partition(&mut right[1..], (axis + 1) % 3);
}
impl ActorTargets {
    pub fn nearest(&self, point: Vec3, range: f32) -> Option<Target> {
        if !point.is_finite() || range < 0. {
            return None;
        }
        let mut best = None;
        let mut distance = range * range;
        search(&self.points, 0, point, &mut distance, &mut best);
        best
    }
}
fn search(
    points: &[Target],
    axis: usize,
    point: Vec3,
    distance: &mut f32,
    best: &mut Option<Target>,
) {
    if points.is_empty() {
        return;
    }
    let mid = points.len() / 2;
    let target = points[mid];
    let d = point.distance_squared(target.position);
    if d <= *distance && (d < *distance || best.is_none_or(|old| target.entity < old.entity)) {
        *distance = d;
        *best = Some(target);
    }
    let delta = point[axis] - target.position[axis];
    let left = &points[..mid];
    let right = &points[mid + 1..];
    let (near, far) = if delta < 0. {
        (left, right)
    } else {
        (right, left)
    };
    search(near, (axis + 1) % 3, point, distance, best);
    if delta * delta <= *distance {
        search(far, (axis + 1) % 3, point, distance, best);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_queries_match_brute_force_including_ties_and_range() {
        let mut world = World::new();
        let mut points = Vec::new();
        for i in 0..400 {
            points.push(Target {
                entity: world.spawn_empty().id(),
                position: Vec3::new(
                    ((i * 73) % 311) as f32 - 150.,
                    ((i * 31) % 239) as f32 - 100.,
                    (i % 4) as f32 * 8.,
                ),
            });
        }
        points.push(Target {
            entity: world.spawn_empty().id(),
            position: points[0].position,
        });
        let original = points.clone();
        partition(&mut points, 0);
        let index = ActorTargets { points };
        for i in 0..100 {
            let p = Vec3::new(i as f32 * 7. - 300., i as f32 * 3. - 160., 30.);
            for range in [0., 32., 100., f32::INFINITY] {
                let expected = original
                    .iter()
                    .filter(|t| p.distance_squared(t.position) <= range * range)
                    .min_by(|a, b| {
                        p.distance_squared(a.position)
                            .total_cmp(&p.distance_squared(b.position))
                            .then_with(|| a.entity.cmp(&b.entity))
                    })
                    .map(|t| t.entity);
                assert_eq!(index.nearest(p, range).map(|t| t.entity), expected);
            }
        }
    }
    #[test]
    fn rebuilding_removes_despawned_targets() {
        let mut app = App::new();
        app.add_plugins(TargetIndexPlugin);
        let e = app
            .world_mut()
            .spawn((
                Hero {
                    move_direction: 1.,
                    direction_change_timer: Timer::default(),
                },
                Transform::default(),
            ))
            .id();
        app.update();
        assert!(
            app.world()
                .resource::<ActorTargets>()
                .nearest(Vec3::ZERO, 1.)
                .is_some()
        );
        app.world_mut().despawn(e);
        app.update();
        assert!(
            app.world()
                .resource::<ActorTargets>()
                .nearest(Vec3::ZERO, 1.)
                .is_none()
        );
    }
}
