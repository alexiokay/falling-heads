use crate::{components::hero::Hero, states::GameState};
use bevy::prelude::*;
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum TeleportPhase {
    #[default]
    Idle,
    Charging,
    Revealing,
    Failed,
}
#[derive(Resource, Default)]
pub struct BiomeTeleport {
    pub target: Option<Vec3>,
    pub phase: TeleportPhase,
    pub elapsed: f32,
    pub progress: f32,
    requested: Option<Vec3>,
}
pub const CHARGE_SECONDS: f32 = 0.55;
pub const REVEAL_SECONDS: f32 = 0.40;
pub struct BiomeTeleportPlugin;
impl Plugin for BiomeTeleportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BiomeTeleport>()
            .add_plugins(crate::systems::teleport_vfx::TeleportVfxPlugin)
            .add_systems(
                PostUpdate,
                finish_biome_teleport
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
/// Commit only after the destination is loaded, selecting real empty cells (including decorations).
pub(crate) fn finish_biome_teleport(
    time: Res<Time>,
    fluids: Option<Res<crate::systems::fluids::FluidWorld>>,
    mut teleport: ResMut<BiomeTeleport>,
    loaded: Res<crate::components::chunk::LoadedChunks>,
    mut hero: Query<
        (
            &mut Transform,
            &mut avian3d::prelude::LinearVelocity,
            Option<&mut avian3d::prelude::Position>,
        ),
        (With<Hero>, Without<crate::components::block::Block>),
    >,
    blocks: Query<(Entity, &Transform), (With<crate::components::block::Block>, Without<Hero>)>,
    mut cameras: Query<
        &mut Transform,
        (
            With<Camera3d>,
            Without<Hero>,
            Without<crate::components::block::Block>,
        ),
    >,
    mut center: ResMut<crate::resources::action_center::ActionCenter>,
    mut spatial: ResMut<crate::resources::spatial_hash::SpatialHash>,
) {
    use crate::{
        components::chunk::world_to_chunk_coords,
        utils::{biome_utils::determine_biome_at_position, constants::BLOCK_SIZE},
    };
    let Some(target) = teleport.target else {
        if teleport.phase != TeleportPhase::Idle {
            teleport.elapsed += time.delta_secs();
            let duration = if teleport.phase == TeleportPhase::Failed {
                2.0
            } else {
                REVEAL_SECONDS
            };
            if teleport.elapsed >= duration {
                teleport.phase = TeleportPhase::Idle;
                teleport.requested = None;
            }
        }
        return;
    };
    if teleport.requested != Some(target) || teleport.phase != TeleportPhase::Charging {
        teleport.requested = Some(target);
        teleport.phase = TeleportPhase::Charging;
        teleport.elapsed = 0.0;
        return; // Render at least one charge frame before advancing, even after a hitch.
    }
    teleport.elapsed += time.delta_secs().min(0.1);
    if teleport.elapsed > 5.0 {
        teleport.target = None;
        teleport.phase = TeleportPhase::Failed;
        teleport.elapsed = 0.0;
        warn!("Teleport preparation timed out; keeping the player at the source");
        return;
    }
    let (cx, cy) = world_to_chunk_coords(target.x, target.y, BLOCK_SIZE);
    let ready = (-1..=1)
        .flat_map(|dx| (-1..=1).map(move |dy| (cx + dx, cy + dy)))
        .filter(|&(x, y)| loaded.is_loaded(x, y))
        .count();
    teleport.progress = ready as f32 / 9.0;
    if ready < 9 || teleport.elapsed < CHARGE_SECONDS {
        return;
    }
    let occupied: std::collections::HashSet<_> = blocks
        .iter()
        .map(|(_, t)| {
            (
                (t.translation.x / BLOCK_SIZE).round() as i32,
                (t.translation.y / BLOCK_SIZE).round() as i32,
            )
        })
        .collect();
    let bx = (target.x / BLOCK_SIZE).round() as i32;
    let by = (target.y / BLOCK_SIZE).round() as i32;
    let biome = determine_biome_at_position(target.x, target.y);
    let mut candidates: Vec<_> = (-48..=48)
        .flat_map(|dx| (-48..=48).map(move |dy| (dx * dx + dy * dy, bx + dx, by + dy)))
        .collect();
    candidates.sort_unstable();
    let destination = [1, 0].into_iter().find_map(|clearance| {
        candidates.iter().copied().find_map(|(_, x, y)| {
            let p = Vec3::new(x as f32 * BLOCK_SIZE, y as f32 * BLOCK_SIZE, target.z);
            (determine_biome_at_position(p.x, p.y) == biome
                && (-clearance..=clearance).all(|dx| {
                    (-clearance..=clearance).all(|dy| {
                        let (cx, cy) = ((x + dx).div_euclid(16), (y + dy).div_euclid(16));
                        loaded.is_loaded(cx, cy)
                            && !occupied.contains(&(x + dx, y + dy))
                            && !fluids
                                .as_ref()
                                .is_some_and(|f| f.has_liquid_at(x + dx, y + dy))
                    })
                }))
            .then_some(p)
        })
    });
    if destination.is_none() && teleport.elapsed < 4.5 {
        return;
    }
    teleport.target = None;
    let Some(destination) = destination else {
        teleport.phase = TeleportPhase::Failed;
        teleport.elapsed = 0.0;
        warn!("No clear teleport destination in requested biome; player left in place");
        return;
    };
    let Ok((mut transform, mut velocity, position)) = hero.single_mut() else {
        return;
    };
    teleport.phase = TeleportPhase::Revealing;
    teleport.elapsed = 0.0;
    // Avian 0.8 treats Update/PostUpdate Transform edits as teleports and resets easing.
    transform.translation = destination;
    if let Some(mut position) = position {
        position.0 = destination;
    }
    velocity.0 = Vec3::ZERO;
    center.center = destination;
    center.raw_center = destination;
    center.combat_intensity = 0.0;
    for mut camera in &mut cameras {
        *camera = Transform::from_xyz(destination.x + 30.0, destination.y + 80.0, 1500.0)
            .looking_at(Vec3::new(destination.x, destination.y, 0.0), Vec3::Y);
    }
    spatial.clear();
    for (entity, transform) in &blocks {
        spatial.insert(entity, transform.translation.x, transform.translation.y);
    }
    info!("Teleported to {:?} at {:?}", biome, destination);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        components::{block::Block, chunk::LoadedChunks},
        resources::{action_center::ActionCenter, spatial_hash::SpatialHash},
        utils::{biome_utils::nearest_biome_y, constants::BLOCK_SIZE},
    };
    #[test]
    fn teleport_waits_for_chunks_then_moves_player_camera_and_collision_index_together() {
        let target = Vec3::new(
            0.,
            nearest_biome_y(130000., crate::components::block::BiomeType::Desert),
            10.,
        );
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(BiomeTeleport {
                target: Some(target),
                ..default()
            })
            .init_resource::<LoadedChunks>()
            .init_resource::<ActionCenter>()
            .insert_resource(SpatialHash::new(256.))
            .add_systems(Update, finish_biome_teleport);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.3));
        let player = app
            .world_mut()
            .spawn((
                Hero {
                    move_direction: 1.,
                    direction_change_timer: Timer::from_seconds(1., TimerMode::Repeating),
                },
                Transform::from_xyz(0., 130000., 10.),
                avian3d::prelude::LinearVelocity(Vec3::splat(100.)),
            ))
            .id();
        let camera = app
            .world_mut()
            .spawn((Camera3d::default(), Transform::IDENTITY))
            .id();
        let solid = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                Transform::from_translation(target),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Transform>(player).unwrap().translation.y,
            130000.
        );
        assert!(app.world().resource::<BiomeTeleport>().target.is_some());
        let (cx, cy) =
            crate::components::chunk::world_to_chunk_coords(target.x, target.y, BLOCK_SIZE);
        for dx in -1..=1 {
            for dy in -1..=1 {
                app.world_mut()
                    .resource_mut::<LoadedChunks>()
                    .add(cx + dx, cy + dy);
            }
        }
        for _ in 0..8 {
            app.update();
        }
        let landed = app.world().get::<Transform>(player).unwrap().translation;
        assert!(landed.distance(target) >= BLOCK_SIZE);
        assert_eq!(
            app.world()
                .get::<avian3d::prelude::LinearVelocity>(player)
                .unwrap()
                .0,
            Vec3::ZERO
        );
        assert_eq!(app.world().resource::<ActionCenter>().center, landed);
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            Vec3::new(landed.x + 30., landed.y + 80., 1500.)
        );
        assert!(
            app.world()
                .resource::<SpatialHash>()
                .query_nearby(target.x, target.y)
                .contains(&solid)
        );
        assert!(app.world().resource::<BiomeTeleport>().target.is_none());
        // Clicking the same destination during reveal must charge again.
        {
            let mut travel = app.world_mut().resource_mut::<BiomeTeleport>();
            travel.target = Some(target);
            travel.phase = TeleportPhase::Revealing;
        }
        app.update();
        assert!(app.world().resource::<BiomeTeleport>().phase == TeleportPhase::Charging);
        assert_eq!(app.world().resource::<BiomeTeleport>().elapsed, 0.);
        assert_eq!(
            app.world().get::<Transform>(player).unwrap().translation,
            landed
        );
    }
}
