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
