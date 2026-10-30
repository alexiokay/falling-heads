use bevy::prelude::*;

#[derive(Component)]
pub struct Block {
    pub health: u32,
    pub max_health: u32,
}

/// Marker component for cobweb blocks
#[derive(Component)]
pub struct CobwebBlock;

/// Trait for biome behavior - defines terrain generation and spawning rules
pub trait BiomeBehavior {
    /// Get the primary terrain block for this biome
    fn terrain_block(&self) -> BlockType;

    /// Get decorative blocks that can spawn
    fn decorative_blocks(&self) -> &[BlockType];

    /// Get ore blocks for this biome
    fn ore_blocks(&self) -> &[BlockType];

    /// Get enemy types that spawn in this biome
    fn enemy_types(&self) -> &[crate::components::enemy::EnemyType];

    /// Get enemy spawn rate (higher = more enemies)
    fn enemy_spawn_rate(&self) -> f32;
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockType {
    // Terrain blocks
    Stone,
    Dirt,
    Grass,
    Sand,
    Sandstone,
    Snow,
    Ice,

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
    Cobweb, // Slows down entities that touch it

    // Nether blocks
    Netherrack,
    Netherite,
    AncientDebris, // Has top/side textures
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
    Iceland, // Snow/ice biome
}

impl BlockType {
    /// Full, rigid surfaces can carry fixtures. Plants and porous foliage cannot.
    pub fn supports_fixture(self) -> bool {
        !matches!(
            self,
            Self::Mushroom
                | Self::DeadBush
                | Self::Vines
                | Self::Cobweb
                | Self::Cactus
                | Self::Leaves
                | Self::ChorusFruit
        )
    }
}

impl BiomeBehavior for BiomeType {
    fn terrain_block(&self) -> BlockType {
        match self {
            BiomeType::Plains => BlockType::Dirt, // Rolling dirt shelves receive grass on exposed tops.
            BiomeType::Forest => BlockType::Dirt,
            BiomeType::Desert => BlockType::Sand,
            BiomeType::Mountains => BlockType::Stone,
            BiomeType::Underground => BlockType::Stone,
            BiomeType::Nether => BlockType::Netherrack,
            BiomeType::End => BlockType::EndStone,
            BiomeType::Iceland => BlockType::Snow, // Snow terrain
        }
    }

    fn decorative_blocks(&self) -> &[BlockType] {
        match self {
            BiomeType::Plains => &[BlockType::Pumpkin, BlockType::Melon],
            BiomeType::Forest => &[], // Trees are connected assemblies on open ground.
            BiomeType::Desert => &[BlockType::Cactus, BlockType::DeadBush],
            BiomeType::Mountains => &[BlockType::Snow],
            BiomeType::Underground => &[BlockType::Mushroom, BlockType::Cobweb],
            BiomeType::Nether => &[BlockType::SoulSand, BlockType::Glowstone],
            BiomeType::End => &[],
            BiomeType::Iceland => &[], // Pure ice/snow (no decorative blocks)
        }
    }

    fn ore_blocks(&self) -> &[BlockType] {
        match self {
            BiomeType::Nether => &[
                BlockType::AncientDebris,
                BlockType::Netherite,
                BlockType::Obsidian,
                BlockType::Gold,
            ],
            BiomeType::End => &[BlockType::Obsidian, BlockType::Diamond, BlockType::Emerald],
            BiomeType::Iceland => &[BlockType::Diamond, BlockType::Emerald, BlockType::Iron], // Rare frozen gems
            _ => &[
                BlockType::Coal,
                BlockType::Iron,
                BlockType::Gold,
                BlockType::Diamond,
                BlockType::Emerald,
                BlockType::Redstone,
            ],
        }
    }

    fn enemy_types(&self) -> &[crate::components::enemy::EnemyType] {
        use crate::components::enemy::EnemyType;
        match self {
            BiomeType::Plains => &[
                EnemyType::Skeleton,
                EnemyType::Slime,
                EnemyType::Slime,
                EnemyType::Creeper,
            ],
            BiomeType::Forest => &[EnemyType::Skeleton, EnemyType::Skeleton, EnemyType::Bogged],
            BiomeType::Desert => &[EnemyType::Husk, EnemyType::Husk, EnemyType::Skeleton],
            BiomeType::Mountains => &[EnemyType::Skeleton, EnemyType::Breeze],
            BiomeType::Underground => &[EnemyType::Skeleton, EnemyType::Spider, EnemyType::Bat],
            BiomeType::Nether => &[EnemyType::Blaze, EnemyType::MagmaCube, EnemyType::Skeleton],
            BiomeType::End => &[EnemyType::Enderman, EnemyType::Shulker],
            BiomeType::Iceland => &[EnemyType::Yeti, EnemyType::Stray],
        }
    }

    fn enemy_spawn_rate(&self) -> f32 {
        match self {
            BiomeType::Nether => 0.80,      // VERY HIGH spawn rate (was 0.40)
            BiomeType::End => 0.50,         // Higher spawn rate
            BiomeType::Underground => 0.70, // VERY HIGH spawn rate
            BiomeType::Iceland => 0.40,     // Moderate spawn rate
            _ => 0.50,                      // Higher by default
        }
    }
}

/// Material families own heat behavior, independently of the effect that hits them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialFamily {
    Timber,
    Foliage,
    Frozen,
    Mineral,
}
impl BlockType {
    pub fn family(self) -> MaterialFamily {
        match self {
            Self::Wood => MaterialFamily::Timber,
            Self::Leaves
            | Self::Grass
            | Self::Vines
            | Self::DeadBush
            | Self::Cobweb
            | Self::Mushroom
            | Self::Cactus
            | Self::Melon
            | Self::Pumpkin => MaterialFamily::Foliage,
            Self::Snow | Self::Ice => MaterialFamily::Frozen,
            _ => MaterialFamily::Mineral,
        }
    }
    pub fn combustible(self) -> bool {
        matches!(
            self.family(),
            MaterialFamily::Timber | MaterialFamily::Foliage
        )
    }
}
/// Material resistance gives biomes distinct digging pace without retuning head bounce.
impl BlockType {
    pub fn starting_health(self) -> u32 {
        match self {
            Self::Leaves | Self::Grass | Self::Vines | Self::DeadBush | Self::Mushroom => 1,
            Self::Sand | Self::Snow | Self::Ice | Self::SoulSand | Self::Netherrack => 2,
            Self::Obsidian | Self::AncientDebris => 5,
            Self::EndStone | Self::Purpur => 4,
            _ => 3,
        }
    }
}

#[cfg(test)]
mod material_tests {
    use super::*;

    #[test]
    fn foliage_is_faster_to_break_than_stone() {
        assert_eq!(BlockType::Leaves.starting_health(), 1);
        assert!(BlockType::Sand.starting_health() < BlockType::Stone.starting_health());
        assert!(BlockType::Obsidian.starting_health() > BlockType::Stone.starting_health());
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;
    #[test]
    fn fixtures_require_rigid_full_faces() {
        for k in [
            BlockType::Mushroom,
            BlockType::DeadBush,
            BlockType::Cobweb,
            BlockType::Leaves,
            BlockType::ChorusFruit,
            BlockType::Cactus,
            BlockType::Vines,
        ] {
            assert!(!k.supports_fixture());
        }
        for k in [
            BlockType::Stone,
            BlockType::Dirt,
            BlockType::Wood,
            BlockType::Sandstone,
            BlockType::Purpur,
        ] {
            assert!(k.supports_fixture());
        }
    }
}
