use bevy::prelude::*;

#[derive(Component)]
pub struct Block {
    pub health: u32,
    pub max_health: u32,
}

/// Trait for biome behavior - defines terrain generation and spawning rules
pub trait BiomeBehavior {
    /// Get the primary terrain block for this biome
    fn terrain_block(&self) -> BlockType;

    /// Get decorative blocks that can spawn
    fn decorative_blocks(&self) -> &[BlockType];

    /// Get ore blocks for this biome
    fn ore_blocks(&self) -> &[BlockType];

    /// Get cave generation intensity (lower = smaller caves)
    fn cave_intensity(&self) -> f32;

    /// Get cave threshold (lower = more frequent caves)
    fn cave_threshold(&self) -> f32;

    /// Get enemy types that spawn in this biome
    fn enemy_types(&self) -> &[crate::components::enemy::EnemyType];

    /// Get enemy spawn rate (higher = more enemies)
    fn enemy_spawn_rate(&self) -> f32;
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum BiomeType {
    Plains,
    Forest,
    Desert,
    Mountains,
    Underground,
    Nether,
    End,
    Iceland,  // Snow/ice biome
}

impl BiomeBehavior for BiomeType {
    fn terrain_block(&self) -> BlockType {
        match self {
            BiomeType::Plains => BlockType::Leaves,  // Plains uses leaves (grass-like appearance)
            BiomeType::Forest => BlockType::Melon,  // Changed from Leaves to Melon (leaves texture has alpha issues)
            BiomeType::Desert => BlockType::Sand,
            BiomeType::Mountains => BlockType::Stone,
            BiomeType::Underground => BlockType::Stone,
            BiomeType::Nether => BlockType::Netherrack,
            BiomeType::End => BlockType::EndStone,
            BiomeType::Iceland => BlockType::Snow,  // Snow terrain
        }
    }

    fn decorative_blocks(&self) -> &[BlockType] {
        match self {
            BiomeType::Plains => &[BlockType::Pumpkin, BlockType::Melon],
            BiomeType::Forest => &[BlockType::Wood, BlockType::Pumpkin],
            BiomeType::Desert => &[BlockType::Cactus, BlockType::DeadBush],
            BiomeType::Mountains => &[BlockType::Snow],
            BiomeType::Underground => &[BlockType::Mushroom],
            BiomeType::Nether => &[BlockType::SoulSand, BlockType::Glowstone],
            BiomeType::End => &[],
            BiomeType::Iceland => &[],  // Pure ice/snow (no decorative blocks)
        }
    }

    fn ore_blocks(&self) -> &[BlockType] {
        match self {
            BiomeType::Nether => &[BlockType::AncientDebris, BlockType::Netherite, BlockType::Obsidian, BlockType::Gold],
            BiomeType::End => &[BlockType::Obsidian, BlockType::Diamond, BlockType::Emerald],
            BiomeType::Iceland => &[BlockType::Diamond, BlockType::Emerald, BlockType::Iron],  // Rare frozen gems
            _ => &[BlockType::Coal, BlockType::Iron, BlockType::Gold, BlockType::Diamond, BlockType::Emerald, BlockType::Redstone],
        }
    }

    fn cave_intensity(&self) -> f32 {
        match self {
            BiomeType::Nether => 0.15,       // Small caves
            BiomeType::End => 0.12,          // Tiny caves
            BiomeType::Underground => 0.2,   // Small caves
            BiomeType::Mountains => 0.18,    // Small caves
            BiomeType::Iceland => 0.10,      // Very small ice caves
            _ => 0.15,                       // Small caves by default
        }
    }

    fn cave_threshold(&self) -> f32 {
        match self {
            BiomeType::Nether => 1.0,       // Frequent small caves
            BiomeType::End => 1.3,          // Less frequent
            BiomeType::Underground => 0.9,  // Very frequent
            BiomeType::Iceland => 1.5,      // Rare ice caves
            _ => 1.1,                       // Moderate frequency
        }
    }

    fn enemy_types(&self) -> &[crate::components::enemy::EnemyType] {
        use crate::components::enemy::EnemyType;
        match self {
            BiomeType::Nether => &[EnemyType::Skeleton], // Only Skeleton (has texture)
            BiomeType::End => &[], // No enemies (Enderman has no texture yet)
            BiomeType::Iceland => &[EnemyType::Yeti], // Yeti throws snowballs!
            BiomeType::Plains | BiomeType::Forest | BiomeType::Desert | BiomeType::Mountains => {
                &[EnemyType::Skeleton] // Only Skeleton (Zombie has no texture yet)
            },
            BiomeType::Underground => &[EnemyType::Skeleton], // Only Skeleton
        }
    }

    fn enemy_spawn_rate(&self) -> f32 {
        match self {
            BiomeType::Nether => 0.80,      // VERY HIGH spawn rate (was 0.40)
            BiomeType::End => 0.50,          // Higher spawn rate
            BiomeType::Underground => 0.70,  // VERY HIGH spawn rate
            BiomeType::Iceland => 0.40,      // Moderate spawn rate
            _ => 0.50,                      // Higher by default
        }
    }
}
