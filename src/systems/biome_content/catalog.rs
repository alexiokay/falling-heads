use crate::components::block::BlockType;
use bevy::prelude::*;

/// A named material variant; BlockType remains the shared simulation family.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Tile {
    Wheat,
    Farmland,
    Hay,
    Moss,
    RootedDirt,
    MossyBrick,
    MossyCobble,
    Grass,
    Fern,
    Berries,
    Azalea,
    RedSand,
    Terracotta,
    Bone,
    Tuff,
    Calcite,
    Deepslate,
    Infested,
    DripstoneUp,
    DripstoneDown,
    Amethyst,
    Crystal,
    Basalt,
    Blackstone,
    Magma,
    CrimsonFungus,
    WarpedFungus,
    CrimsonSoil,
    WarpedSoil,
    PackedIce,
    BlueIce,
    PowderSnow,
    EndBrick,
    PurpurPillar,
}
pub(crate) const TILES: &[Tile] = &[
    Tile::Wheat,
    Tile::Farmland,
    Tile::Hay,
    Tile::Moss,
    Tile::RootedDirt,
    Tile::MossyBrick,
    Tile::MossyCobble,
    Tile::Grass,
    Tile::Fern,
    Tile::Berries,
    Tile::Azalea,
    Tile::RedSand,
    Tile::Terracotta,
    Tile::Bone,
    Tile::Tuff,
    Tile::Calcite,
    Tile::Deepslate,
    Tile::Infested,
    Tile::DripstoneUp,
    Tile::DripstoneDown,
    Tile::Amethyst,
    Tile::Crystal,
    Tile::Basalt,
    Tile::Blackstone,
    Tile::Magma,
    Tile::CrimsonFungus,
    Tile::WarpedFungus,
    Tile::CrimsonSoil,
    Tile::WarpedSoil,
    Tile::PackedIce,
    Tile::BlueIce,
    Tile::PowderSnow,
    Tile::EndBrick,
    Tile::PurpurPillar,
];
impl Tile {
    pub fn texture(self) -> &'static str {
        match self {
            Self::Wheat => "wheat_stage7",
            Self::Farmland => "farmland_moist",
            Self::Hay => "hay_block_side",
            Self::Moss => "moss_block",
            Self::RootedDirt => "rooted_dirt",
            Self::MossyBrick => "mossy_stone_bricks",
            Self::MossyCobble => "mossy_cobblestone",
            Self::Grass => "short_grass",
            Self::Fern => "fern",
            Self::Berries => "sweet_berry_bush_stage3",
            Self::Azalea => "flowering_azalea_leaves",
            Self::RedSand => "red_sand",
            Self::Terracotta => "terracotta",
            Self::Bone => "bone_block_side",
            Self::Tuff => "tuff",
            Self::Calcite => "calcite",
            Self::Deepslate => "deepslate",
            Self::Infested => "cracked_deepslate_bricks",
            Self::DripstoneUp => "pointed_dripstone_up_tip",
            Self::DripstoneDown => "pointed_dripstone_down_tip",
            Self::Amethyst => "amethyst_block",
            Self::Crystal => "amethyst_cluster",
            Self::Basalt => "basalt_side",
            Self::Blackstone => "blackstone",
            Self::Magma => "magma_0",
            Self::CrimsonFungus => "crimson_fungus",
            Self::WarpedFungus => "warped_fungus",
            Self::CrimsonSoil => "crimson_nylium",
            Self::WarpedSoil => "warped_nylium",
            Self::PackedIce => "packed_ice",
            Self::BlueIce => "blue_ice",
            Self::PowderSnow => "powder_snow",
            Self::EndBrick => "end_stone_bricks",
            Self::PurpurPillar => "purpur_pillar_side",
        }
    }
    pub fn base(self) -> BlockType {
        match self {
            Self::Wheat => BlockType::DeadBush,
            Self::Farmland => BlockType::Dirt,
            Self::Hay => BlockType::Wood,
            Self::Moss => BlockType::Grass,
            Self::Azalea => BlockType::Leaves,
            Self::RootedDirt => BlockType::Dirt,
            Self::Grass | Self::Fern => BlockType::DeadBush,
            Self::Berries | Self::CrimsonFungus | Self::WarpedFungus => BlockType::Mushroom,
            Self::RedSand => BlockType::Sand,
            Self::Terracotta | Self::Bone => BlockType::Sandstone,
            Self::PackedIce | Self::BlueIce => BlockType::Ice,
            Self::PowderSnow => BlockType::Snow,
            Self::CrimsonSoil | Self::WarpedSoil => BlockType::Netherrack,
            Self::Magma | Self::Blackstone | Self::Basalt => BlockType::Netherrack,
            Self::EndBrick => BlockType::EndStone,
            Self::PurpurPillar => BlockType::Purpur,
            Self::DripstoneUp | Self::DripstoneDown | Self::Crystal => BlockType::DeadBush,
            _ => BlockType::Stone,
        }
    }
    pub fn decorative(self) -> bool {
        matches!(
            self,
            Self::Wheat
                | Self::Grass
                | Self::Fern
                | Self::Berries
                | Self::DripstoneUp
                | Self::DripstoneDown
                | Self::Crystal
                | Self::CrimsonFungus
                | Self::WarpedFungus
        )
    }
    pub fn combustible(self) -> bool {
        !matches!(
            self,
            Self::DripstoneUp | Self::DripstoneDown | Self::Crystal
        ) && self.base().combustible()
    }
}
