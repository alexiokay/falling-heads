//! One terrain-contact path for every question-box and resource spawn source.
//! Sweep from the last verified position, and validate against all neighbouring
//! cells before publishing a position. This also handles terrain streaming in.
use crate::{
    components::{
        block::{Block, BlockType},
        chunk::LoadedChunks,
        lootbox::RandomBox,
        resource::Resource as DroppedResource,
        snow_layer::SnowLayer,
    },
    utils::constants::{BLOCK_SIZE, RESOURCE_SIZE},
};
use avian3d::prelude::{LinearVelocity, Restitution};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component, Default)]
struct DropContact {
    clear: Option<Vec3>,
}

#[derive(Default)]
struct Terrain {
    cells: HashMap<(i32, i32), f32>,
}
impl Terrain {
    fn free(&self, p: Vec2, half: f32) -> bool {
        let lo = ((p - Vec2::splat(half + BLOCK_SIZE)) / BLOCK_SIZE)
            .floor()
            .as_ivec2();
        let hi = ((p + Vec2::splat(half + BLOCK_SIZE)) / BLOCK_SIZE)
            .ceil()
            .as_ivec2();
        for y in lo.y..=hi.y {
            for x in lo.x..=hi.x {
                let Some(snow) = self.cells.get(&(x, y)) else {
                    continue;
                };
                let center = Vec2::new(x as f32 * BLOCK_SIZE, y as f32 * BLOCK_SIZE + snow * 0.5);
                let extent = Vec2::new(32.0, 32.0 + snow * 0.5) + Vec2::splat(half);
                if (p - center).abs().cmplt(extent - Vec2::splat(0.001)).all() {
                    return false;
                }
            }
        }
        true
    }

    fn nearest_free(&self, p: Vec2, half: f32, loaded: impl Fn(Vec2) -> bool) -> Option<Vec2> {
        if loaded(p) && self.free(p, half) {
            return Some(p);
        }
        let origin = (p / BLOCK_SIZE).round();
        // Bounded search: never fall back to an unverified position inside terrain.
        for ring in 1_i32..=16 {
            let mut best: Option<Vec2> = None;
            for y in -ring..=ring {
                for x in -ring..=ring {
                    if x.abs().max(y.abs()) != ring {
                        continue;
                    }
                    let candidate = (origin + Vec2::new(x as f32, y as f32)) * BLOCK_SIZE;
                    if loaded(candidate)
                        && self.free(candidate, half)
                        && best
                            .is_none_or(|b| candidate.distance_squared(p) < b.distance_squared(p))
                    {
                        best = Some(candidate);
                    }
                }
            }
            if best.is_some() {
                return best;
            }
        }
        None
    }

    fn sweep(&self, from: Vec2, to: Vec2, half: f32) -> (Vec2, BVec2) {
        let delta = to - from;
        let steps = (delta.abs().max_element() / 8.0).ceil().max(1.0) as usize;
        // A teleport is handled by nearest_free, never an unbounded sweep.
        if steps > 256 {
            return (from, BVec2::TRUE);
        }
        let step = delta / steps as f32;
        let mut p = from;
        let mut blocked = BVec2::FALSE;
        for _ in 0..steps {
            for axis in 0..2 {
                if (axis == 0 && blocked.x) || (axis == 1 && blocked.y) {
                    continue;
                }
                let mut next = p;
                next[axis] += step[axis];
                if self.free(next, half) {
                    p = next;
                } else {
                    // Bisect to the contact face rather than stopping a full substep away.
                    let (mut low, mut high) = (0.0, 1.0);
                    for _ in 0..12 {
                        let mid = (low + high) * 0.5;
                        if self.free(p.lerp(next, mid), half) {
                            low = mid;
                        } else {
                            high = mid;
                        }
                    }
                    p = p.lerp(next, low);
                    if axis == 0 {
                        blocked.x = true;
                    } else {
                        blocked.y = true;
                    }
                }
            }
        }
        (p, blocked)
    }
}

pub struct DropTerrainPlugin;
impl Plugin for DropTerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            resolve
                .before(TransformSystems::Propagate)
                .run_if(in_state(crate::states::InGame)),
        );
    }
}

fn resolve(
    mut commands: Commands,
    blocks: Query<
        (
            &Transform,
            &BlockType,
            Option<&SnowLayer>,
            Option<&crate::systems::biome_content::Tile>,
        ),
        With<Block>,
    >,
    mut drops: Query<
        (
            Entity,
            &mut Transform,
            &mut LinearVelocity,
            &mut Visibility,
            Option<&Restitution>,
            Option<&RandomBox>,
            Option<&mut DropContact>,
        ),
        (Or<(With<RandomBox>, With<DroppedResource>)>, Without<Block>),
    >,
    loaded: Res<LoadedChunks>,
    changed: Query<
        (),
        (
            With<Block>,
            Or<(
                Added<Block>,
                Changed<Transform>,
                Changed<BlockType>,
                Changed<SnowLayer>,
            )>,
        ),
    >,
    mut removed_blocks: RemovedComponents<Block>,
    mut removed_snow: RemovedComponents<SnowLayer>,
    mut terrain: Local<Terrain>,
) {
    let removed = removed_blocks.read().count() + removed_snow.read().count() > 0;
    if removed {
        terrain.cells.clear();
    }
    if drops.is_empty() {
        terrain.cells.clear();
        return;
    }
    if terrain.cells.is_empty() || !changed.is_empty() {
        terrain.cells.clear();
        for (t, kind, snow, tile) in &blocks {
            // Same rule as heads: items fall through plants, vines and decorations.
            if crate::systems::plant_rules::passable(*kind, tile) {
                continue;
            }
            let cell = (t.translation.xy() / BLOCK_SIZE).round().as_ivec2();
            terrain
                .cells
                .insert((cell.x, cell.y), snow.map_or(0.0, SnowLayer::thickness));
        }
    }
    let is_loaded = |p: Vec2| {
        let (x, y) = crate::components::chunk::world_to_chunk_coords(p.x, p.y, BLOCK_SIZE);
        loaded.is_loaded(x, y)
    };
    for (entity, mut transform, mut velocity, mut visibility, restitution, box_marker, contact) in
        &mut drops
    {
        let half = if box_marker.is_some() {
            24.0
        } else {
            RESOURCE_SIZE * 0.5
        } * transform.scale.x.abs();
        let proposed = transform.translation;
        let previous = contact
            .as_ref()
            .and_then(|c| c.clear)
            .filter(|p| terrain.free(p.xy(), half) && is_loaded(p.xy()));
        let result = if let Some(previous) = previous {
            let (p, hit) = terrain.sweep(previous.xy(), proposed.xy(), half);
            let bounce = if box_marker.is_some() {
                restitution.map_or(1.0, |r| r.coefficient)
            } else {
                0.0
            };
            // Interpolated frames can repeat a contact: reflect only motion into it.
            if hit.x && velocity.x * (proposed.x - previous.x) > 0.0 {
                velocity.x = -velocity.x * if box_marker.is_some() { bounce } else { 0.3 };
                if box_marker.is_some() {
                    velocity.x = velocity.x.clamp(-550.0, 550.0);
                }
            }
            if hit.y && velocity.y * (proposed.y - previous.y) > 0.0 {
                let falling = velocity.y < 0.0;
                velocity.y = -velocity.y * bounce;
                if box_marker.is_some() {
                    velocity.y = if falling {
                        velocity.y.clamp(400.0, 550.0)
                    } else {
                        0.0
                    };
                }
                if box_marker.is_none() {
                    velocity.x *= 0.8;
                }
            }
            if is_loaded(p) {
                Some(p)
            } else {
                Some(previous.xy())
            }
        } else {
            terrain.nearest_free(proposed.xy(), half, is_loaded)
        };
        let clear = if let Some(p) = result {
            transform.translation = Vec3::new(p.x, p.y, proposed.z);
            *visibility = Visibility::Inherited;
            Some(transform.translation)
        } else {
            // Wait for a loaded free cell; never display an invalid fallback spawn.
            *visibility = Visibility::Hidden;
            velocity.0 = Vec3::ZERO;
            None
        };
        if let Some(mut contact) = contact {
            contact.clear = clear;
        } else {
            commands.entity(entity).insert(DropContact { clear });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_render_contact_does_not_reverse_a_box_already_bouncing_away() {
        let mut app = App::new();
        let mut loaded = LoadedChunks::default();
        loaded.add(0, 0);
        app.insert_resource(loaded).add_systems(Update, resolve);
        app.world_mut().spawn((
            Block {
                health: 3,
                max_health: 3,
            },
            BlockType::Stone,
            Transform::IDENTITY,
        ));
        let id = app
            .world_mut()
            .spawn((
                RandomBox,
                Transform::from_xyz(0.0, 100.0, 10.0),
                LinearVelocity(Vec3::new(0.0, -500.0, 0.0)),
                Visibility::Inherited,
                Restitution::new(1.0),
            ))
            .id();
        app.update();
        app.world_mut()
            .get_mut::<Transform>(id)
            .unwrap()
            .translation
            .y = 0.0;
        app.update();
        assert_eq!(app.world().get::<LinearVelocity>(id).unwrap().y, 500.0);
        assert!((app.world().get::<Transform>(id).unwrap().translation.y - 56.0).abs() < 0.01);
        app.world_mut()
            .get_mut::<Transform>(id)
            .unwrap()
            .translation
            .y = 0.0;
        app.update();
        assert_eq!(app.world().get::<LinearVelocity>(id).unwrap().y, 500.0);
    }
    #[test]
    fn real_system_validates_spawns_and_terrain_streamed_over_existing_drops() {
        let mut app = App::new();
        let mut loaded = LoadedChunks::default();
        for y in -2..=2 {
            for x in -2..=2 {
                loaded.add(x, y);
            }
        }
        app.insert_resource(loaded).add_systems(Update, resolve);
        app.world_mut().spawn((
            Block {
                health: 3,
                max_health: 3,
            },
            BlockType::Stone,
            Transform::IDENTITY,
        ));
        let box_id = app
            .world_mut()
            .spawn((
                RandomBox,
                Transform::IDENTITY,
                LinearVelocity::ZERO,
                Visibility::Inherited,
                Restitution::new(1.0),
            ))
            .id();
        let resource = app
            .world_mut()
            .spawn((
                DroppedResource {
                    resource_type: crate::components::resource::ResourceType::Gold,
                },
                Transform::IDENTITY,
                LinearVelocity::ZERO,
                Visibility::Inherited,
            ))
            .id();
        app.update();
        for id in [box_id, resource] {
            assert_ne!(
                app.world().get::<Transform>(id).unwrap().translation,
                Vec3::ZERO
            );
            assert!(app.world().get::<DropContact>(id).unwrap().clear.is_some());
        }
        let old = app.world().get::<Transform>(box_id).unwrap().translation;
        app.world_mut().spawn((
            Block {
                health: 3,
                max_health: 3,
            },
            BlockType::Leaves,
            Transform::from_translation(old),
        ));
        app.update();
        assert_ne!(
            app.world().get::<Transform>(box_id).unwrap().translation,
            old
        );
        app.world_mut()
            .resource_mut::<LoadedChunks>()
            .chunks
            .clear();
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(box_id).unwrap(),
            Visibility::Hidden
        );
    }
    #[test]
    fn embedded_box_and_resources_find_verified_space_in_dense_terrain() {
        let mut terrain = Terrain::default();
        for y in -4..=4 {
            for x in -4..=4 {
                terrain.cells.insert((x, y), 0.0);
            }
        }
        terrain.cells.remove(&(2, 1));
        for half in [16.0, 24.0] {
            let p = terrain.nearest_free(Vec2::ZERO, half, |_| true).unwrap();
            assert_eq!(p, Vec2::new(128.0, 64.0));
            assert!(terrain.free(p, half));
            assert!(terrain.nearest_free(Vec2::ZERO, half, |_| false).is_none());
        }
    }
    #[test]
    fn fast_drops_cannot_tunnel_through_walls_or_snow() {
        let mut terrain = Terrain::default();
        terrain.cells.insert((0, 0), 8.0);
        for half in [16.0, 24.0] {
            let (p, hit) = terrain.sweep(Vec2::new(-200.0, 0.0), Vec2::new(200.0, 0.0), half);
            assert!(hit.x && terrain.free(p, half));
            assert!((p.x + 32.0 + half).abs() < 0.01);
            let (p, hit) = terrain.sweep(Vec2::new(0.0, 200.0), Vec2::new(0.0, -200.0), half);
            assert!(hit.y && terrain.free(p, half));
            assert!((p.y - 40.0 - half).abs() < 0.01);
        }
    }
}
