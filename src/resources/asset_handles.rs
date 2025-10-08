use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct GameTextures {
    pub hero: Handle<Image>,
    pub tnt: Handle<Image>,
    pub nether_bg: Handle<Image>,
    pub villager: Handle<Image>,
}

#[derive(Resource, Default)]
pub struct EnemyTextures {
    pub skeleton: Handle<Image>,
    pub skeleton2: Handle<Image>,
    pub yeti: Handle<Image>,
    pub penguin: Handle<Image>,
}

#[derive(Resource, Default)]
pub struct ProjectileTextures {
    pub arrow: Handle<Image>,
    pub snowball: Handle<Image>,
}

#[derive(Resource, Default)]
pub struct ProjectileModels {
    pub arrow: Handle<Scene>,  // GLB/GLTF arrow model
}

#[derive(Resource, Default)]
pub struct ResourceTextures {
    pub coal: Handle<Image>,
    pub diamond: Handle<Image>,
    pub emerald: Handle<Image>,
    pub gold_ingot: Handle<Image>,
    pub iron_ingot: Handle<Image>,
    pub redstone: Handle<Image>,
    pub netherite_ingot: Handle<Image>,
}

#[derive(Resource, Default)]
pub struct BlockTextures {
    pub ancient_debris_top: Handle<Image>,
    pub ancient_debris_side: Handle<Image>,
    pub netherrack: Handle<Image>,
    pub nether: Handle<Image>,
    pub obsidian: Handle<Image>,
    pub sandstone: Handle<Image>,
    pub oak_log: Handle<Image>,
    pub oak_log_top: Handle<Image>,
    pub oak_leaves: Handle<Image>,
    pub pumpkin_front: Handle<Image>,
    pub pumpkin_side: Handle<Image>,
    pub pumpkin_top: Handle<Image>,
    pub melon_side: Handle<Image>,
    pub melon_top: Handle<Image>,
    pub dirt: Handle<Image>,
    pub stone: Handle<Image>,
    pub coal_ore: Handle<Image>,
    pub diamond_ore: Handle<Image>,
    pub diamond_block: Handle<Image>,
    pub emerald_ore: Handle<Image>,
    pub gold_block: Handle<Image>,
    // Iceland biome textures
    pub snow: Handle<Image>,
    pub ice: Handle<Image>,
    pub packed_ice: Handle<Image>,
}

/// Block texture atlases for directional blocks (4x1 layout: top|bottom|sides|front)
#[derive(Resource, Default)]
pub struct BlockAtlases {
    pub pumpkin: Handle<Image>,     // Pumpkin atlas: pumpkin_top | pumpkin_top | pumpkin_side | carved_pumpkin
    pub oak_log: Handle<Image>,     // Oak log atlas: oak_log_top | oak_log_top | oak_log | oak_log
    pub oak_leaves: Handle<Image>,  // Oak leaves atlas: oak_leaves | oak_leaves | oak_leaves | oak_leaves
    pub melon: Handle<Image>,       // Melon atlas: melon_top | melon_top | melon_side | melon_side
}

#[derive(Resource, Default)]
pub struct HeartTextures {
    pub container: Handle<Image>,
    pub half: Handle<Image>,
    pub full: Handle<Image>,
}

#[derive(Resource, Default)]
pub struct DamageStageTextures {
    pub stages: [Handle<Image>; 9],  // destroy_stage_0 through destroy_stage_8
}

/// Cache for damage overlay materials and mesh (prevents 7k+ material/mesh leak!)
#[derive(Resource, Default)]
pub struct DamageOverlayCache {
    pub materials: Vec<Handle<StandardMaterial>>,  // One material per damage stage (0-8)
    pub mesh: Option<Handle<Mesh>>,  // Single unit rectangle mesh shared by all overlays
}
