use bevy::prelude::*;

#[derive(Component)]
pub struct Block {
    pub health: u32,
    pub max_health: u32,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    // Terrain blocks
    Stone,
    Dirt,
    Grass,
    Sand,
    Sandstone,
    Snow,

    // Ore blocks
    Coal,
    Iron,
    Gold,
    Diamond,
    Emerald,
    Redstone,

    // Decorative/Natural blocks
    Wood,
    Leaves,
    Pumpkin,
    Melon,
    Cactus,
    DeadBush,
    Mushroom,
    Vines,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BiomeType {
    Plains,
    Forest,
    Desert,
    Mountains,
    Underground,
}

impl BiomeType {
    /// Returns the primary terrain block for this biome
    pub fn terrain_block(&self) -> BlockType {
        match self {
            BiomeType::Plains => BlockType::Grass,
            BiomeType::Forest => BlockType::Dirt,
            BiomeType::Desert => BlockType::Sand,
            BiomeType::Mountains => BlockType::Stone,
            BiomeType::Underground => BlockType::Stone,
        }
    }

    /// Returns decorative blocks that can spawn in this biome
    pub fn decorative_blocks(&self) -> &[BlockType] {
        match self {
            BiomeType::Plains => &[BlockType::Pumpkin, BlockType::Melon],
            BiomeType::Forest => &[BlockType::Wood, BlockType::Leaves],
            BiomeType::Desert => &[BlockType::Cactus, BlockType::DeadBush],
            BiomeType::Mountains => &[BlockType::Snow],
            BiomeType::Underground => &[BlockType::Mushroom, BlockType::Vines],
        }
    }
}
