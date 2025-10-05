use bevy::prelude::*;

#[derive(Component)]
pub struct Block {
    pub health: u32,
    pub max_health: u32,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
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

    // Nether blocks
    Netherrack,
    Netherite,
    AncientDebris,  // Has top/side textures
    SoulSand,
    Glowstone,

    // End blocks
    EndStone,
    Obsidian,
    ChorusFruit,
    Purpur,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BiomeType {
    Plains,
    Forest,
    Desert,
    Mountains,
    Underground,
    Nether,
    End,
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
            BiomeType::Nether => BlockType::Netherrack,
            BiomeType::End => BlockType::EndStone,
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
            BiomeType::Nether => &[BlockType::SoulSand, BlockType::Glowstone],
            BiomeType::End => &[BlockType::ChorusFruit, BlockType::Purpur],
        }
    }

    /// Returns ore blocks for this biome with custom ores per biome
    pub fn ore_blocks(&self) -> &[BlockType] {
        match self {
            BiomeType::Nether => &[BlockType::AncientDebris, BlockType::Netherite, BlockType::Obsidian, BlockType::Gold],
            BiomeType::End => &[BlockType::Obsidian, BlockType::Diamond, BlockType::Emerald],
            _ => &[BlockType::Coal, BlockType::Iron, BlockType::Gold, BlockType::Diamond, BlockType::Emerald, BlockType::Redstone],
        }
    }

    /// Returns cave generation intensity (LOWER = smaller caves)
    pub fn cave_intensity(&self) -> f32 {
        match self {
            BiomeType::Nether => 0.15,       // Small caves
            BiomeType::End => 0.12,          // Tiny caves
            BiomeType::Underground => 0.2,   // Small caves
            BiomeType::Mountains => 0.18,    // Small caves
            _ => 0.15,                       // Small caves by default
        }
    }

    /// Returns cave threshold (LOWER threshold = more frequent caves)
    pub fn cave_threshold(&self) -> f32 {
        match self {
            BiomeType::Nether => 1.0,       // Frequent small caves
            BiomeType::End => 1.3,          // Less frequent
            BiomeType::Underground => 0.9,  // Very frequent
            _ => 1.1,                       // Moderate frequency
        }
    }

    /// Returns enemy types that spawn in this biome
    pub fn enemy_types(&self) -> &[crate::components::enemy::EnemyType] {
        use crate::components::enemy::EnemyType;
        match self {
            BiomeType::Nether => &[EnemyType::Skeleton], // Only Skeleton (has texture)
            BiomeType::End => &[], // No enemies (Enderman has no texture yet)
            BiomeType::Plains | BiomeType::Forest | BiomeType::Desert | BiomeType::Mountains => {
                &[EnemyType::Skeleton] // Only Skeleton (Zombie has no texture yet)
            },
            BiomeType::Underground => &[EnemyType::Skeleton], // Only Skeleton
        }
    }

    /// Returns spawn rate for enemies (HIGHER = more enemies)
    pub fn enemy_spawn_rate(&self) -> f32 {
        match self {
            BiomeType::Nether => 0.80,      // VERY HIGH spawn rate (was 0.40)
            BiomeType::End => 0.50,          // Higher spawn rate
            BiomeType::Underground => 0.70,  // VERY HIGH spawn rate
            _ => 0.50,                      // Higher by default
        }
    }
}
