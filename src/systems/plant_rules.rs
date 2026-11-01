//! Non-solid scenery retains interaction identity, but never supports other blocks.
use crate::{
    components::{
        block::{Block, BlockType},
        chunk::{ChunkMember, LoadedChunks},
    },
    events::block_events::BlockDamaged,
    resources::spatial_hash::SpatialHash,
    systems::biome_content::Tile,
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

pub(crate) fn soft(kind: BlockType, tile: Option<&Tile>) -> bool {
    tile.map_or(
        matches!(
            kind,
            BlockType::Mushroom | BlockType::DeadBush | BlockType::Vines
        ),
        |t| {
            matches!(
                t,
                Tile::Wheat
                    | Tile::Grass
                    | Tile::Fern
                    | Tile::Berries
                    | Tile::CrimsonFungus
                    | Tile::WarpedFungus
            )
        },
    )
}
/// Bodies and dropped items move through cobwebs, plants and decorations.
pub(crate) fn passable(kind: BlockType, tile: Option<&Tile>) -> bool {
    kind == BlockType::Cobweb || soft(kind, tile) || tile.is_some_and(|t| t.decorative())
}
pub(crate) fn support_offset(kind: BlockType, tile: Option<&Tile>) -> Option<IVec2> {
    if tile == Some(&Tile::DripstoneDown) {
        Some(IVec2::Y)
    } else if soft(kind, tile) && kind != BlockType::Vines
        || matches!(tile, Some(Tile::Crystal | Tile::DripstoneUp))
    {
        Some(IVec2::NEG_Y)
    } else {
        None
    }
}
fn solid(kind: BlockType, tile: Option<&Tile>) -> bool {
    !passable(kind, tile)
}
#[derive(Default)]
pub(crate) struct Supports {
    cached: HashMap<Entity, Entity>,
    pending: HashSet<Entity>,
    next: f32,
}
#[allow(clippy::type_complexity)]
pub(crate) fn maintain(
    time: Res<Time>,
    spatial: Res<SpatialHash>,
    loaded: Res<LoadedChunks>,
    blocks: Query<(
        Entity,
        &Transform,
        &BlockType,
        &Block,
        Option<&Tile>,
        &ChunkMember,
    )>,
    mut state: Local<Supports>,
    mut damage: MessageWriter<BlockDamaged>,
) {
    if time.elapsed_secs() < state.next {
        return;
    }
    state.next = time.elapsed_secs() + 0.1;
    state.cached.retain(|e, _| blocks.contains(*e));
    state.pending.retain(|e| blocks.contains(*e));
    for (e, t, kind, block, tile, _) in &blocks {
        let Some(offset) = support_offset(*kind, tile) else {
            continue;
        };
        if block.health == 0 || state.pending.contains(&e) {
            continue;
        }
        if state
            .cached
            .get(&e)
            .and_then(|s| blocks.get(*s).ok())
            .is_some_and(|(_, _, k, b, tile, _)| b.health > 0 && solid(*k, tile))
        {
            continue;
        }
        let expected = t.translation.xy() + offset.as_vec2() * 64.;
        // Streaming a neighbouring chunk is not the same as destroying support.
        let cell = (expected / 64.).round().as_ivec2();
        if !loaded.is_loaded(cell.x.div_euclid(16), cell.y.div_euclid(16)) {
            continue;
        }
        let support = spatial.nearby_iter(expected.x, expected.y).find(|s| {
            blocks.get(*s).is_ok_and(|(_, bt, k, b, tile, _)| {
                b.health > 0
                    && solid(*k, tile)
                    && bt.translation.xy().distance_squared(expected) < 4.
            })
        });
        if let Some(support) = support {
            state.cached.insert(e, support);
        } else {
            state.pending.insert(e);
            // Normal destruction keeps loot, chunk bookkeeping and visuals consistent.
            damage.write(BlockDamaged {
                entity: e,
                damage: block.health,
                remaining_health: 0,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plants_are_not_solid_and_ceiling_growth_uses_support_above() {
        assert!(soft(BlockType::DeadBush, Some(&Tile::Fern)));
        assert!(soft(BlockType::Mushroom, None));
        assert!(!soft(BlockType::DeadBush, Some(&Tile::Crystal)));
        assert!(!solid(BlockType::DeadBush, Some(&Tile::Fern)));
        assert_eq!(
            support_offset(BlockType::DeadBush, Some(&Tile::DripstoneDown)),
            Some(IVec2::Y)
        );
        assert_eq!(
            support_offset(BlockType::DeadBush, Some(&Tile::Wheat)),
            Some(IVec2::NEG_Y)
        );
        assert_eq!(support_offset(BlockType::Wood, None), None);
    }
    #[test]
    fn removed_support_removes_plant_once_but_unloaded_neighbour_waits() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<LoadedChunks>()
            .insert_resource(SpatialHash::new(256.))
            .add_message::<BlockDamaged>()
            .add_message::<crate::events::block_events::HeadHitBlock>()
            .add_systems(Update, maintain);
        let ground = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0., 0., 0.),
                BlockType::Dirt,
                Block {
                    health: 5,
                    max_health: 5,
                },
                ChunkMember {
                    chunk_x: 0,
                    chunk_y: 0,
                },
            ))
            .id();
        let plant = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0., 64., 0.),
                BlockType::DeadBush,
                Tile::Fern,
                Block {
                    health: 2,
                    max_health: 2,
                },
                ChunkMember {
                    chunk_x: 0,
                    chunk_y: 0,
                },
            ))
            .id();
        app.world_mut().resource_mut::<LoadedChunks>().add(0, 0);
        app.world_mut()
            .resource_mut::<SpatialHash>()
            .insert(ground, 0., 0.);
        app.update();
        assert!(app.world().resource::<Messages<BlockDamaged>>().is_empty());
        app.world_mut().despawn(ground);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.11));
        app.update();
        let events: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<BlockDamaged>>()
            .drain()
            .collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity, plant);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.11));
        app.update();
        assert!(app.world().resource::<Messages<BlockDamaged>>().is_empty());
    }
}
