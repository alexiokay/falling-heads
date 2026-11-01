use crate::components::block::BlockType;
use bevy::prelude::*;

#[derive(Message)]
pub struct BlockDestroyed {
    pub(crate) tile: Option<crate::systems::biome_content::Tile>,
    pub entity: Entity,
    pub block_type: BlockType,
    pub position: Vec3,
}

#[derive(Message)]
pub struct BlockDamaged {
    pub entity: Entity,
    pub damage: u32,
    pub remaining_health: u32,
}

/// A friendly head (hero, villager or split twin) hit a block. Combos credit
/// the head when that block is destroyed.
#[derive(Message, Clone, Copy, Debug)]
pub struct HeadHitBlock {
    pub head: Entity,
    pub block: Entity,
}

/// An extra resource drop as if a block of `block_type` broke at `position`
/// (combo rewards). Only ore types drop something.
#[derive(Message, Clone, Copy, Debug)]
pub struct BonusDrop {
    pub block_type: BlockType,
    pub position: Vec3,
}
