use bevy::prelude::*;
use crate::components::block::BlockType;

#[derive(Message)]
pub struct BlockDestroyed {
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
