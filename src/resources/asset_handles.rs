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
pub struct EnemyModels {
    pub dragon: Handle<Scene>,  // GLB/GLTF dragon model
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
    pub sand: Handle<Image>,
    pub sandstone: Handle<Image>,
    pub sandstone_top: Handle<Image>,
    pub sandstone_bottom: Handle<Image>,
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
    pub gold_ore: Handle<Image>,
    pub iron_ore: Handle<Image>,
    pub redstone_ore: Handle<Image>,
    pub glowstone: Handle<Image>,
    pub end_stone: Handle<Image>,
    pub cactus_top: Handle<Image>,
    pub cactus_side: Handle<Image>,
    pub cactus_bottom: Handle<Image>,
    // Iceland biome textures
    pub snow: Handle<Image>,
    pub ice: Handle<Image>,
    pub packed_ice: Handle<Image>,
    // TNT textures for atlas
    pub tnt_top: Handle<Image>,
    pub tnt_bottom: Handle<Image>,
    pub tnt_side: Handle<Image>,
    // Missing Nether blocks
    pub soul_sand: Handle<Image>,
    pub netherite_block: Handle<Image>,
    pub crimson_nylium: Handle<Image>,
    pub crimson_nylium_side: Handle<Image>,
    pub nether_quartz_ore: Handle<Image>,
}

/// Block texture atlases for directional blocks (4x1 layout: top|bottom|sides|front)
#[derive(Resource, Default)]
pub struct BlockAtlases {
    pub pumpkin: Handle<Image>,     // Pumpkin atlas: pumpkin_top | pumpkin_top | pumpkin_side | carved_pumpkin
    pub oak_log: Handle<Image>,     // Oak log atlas: oak_log_top | oak_log_top | oak_log | oak_log
    pub oak_leaves: Handle<Image>,  // Oak leaves atlas: oak_leaves | oak_leaves | oak_leaves | oak_leaves
    pub melon: Handle<Image>,       // Melon atlas: melon_top | melon_top | melon_side | melon_side
    pub tnt: Handle<Image>,         // TNT atlas: tnt_top | tnt_bottom | tnt_side | tnt_side
    pub sandstone: Handle<Image>,   // Sandstone atlas: sandstone_top | sandstone_bottom | sandstone | sandstone
    pub cactus: Handle<Image>,      // Cactus atlas: cactus_top | cactus_bottom | cactus_side | cactus_side
}

/// Explosion animation frames (16 frames total)
#[derive(Resource)]
pub struct ExplosionTextures {
    pub frames: Vec<Handle<Image>>,
}

/// Effect textures (fire, etc.)
#[derive(Resource, Default)]
pub struct EffectTextures {
    pub fire: Handle<Image>,
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

#[derive(Resource)]
pub struct DragonAnimation {
    pub fly: Handle<AnimationClip>,  // Built-in fly animation from GLB
}

/// Hero sound effects
#[derive(Resource, Default)]
pub struct HeroSounds {
    pub hit1: Handle<AudioSource>,
    pub hit2: Handle<AudioSource>,
    pub hit3: Handle<AudioSource>,
    pub fallbig: Handle<AudioSource>,
    pub fallsmall: Handle<AudioSource>,
}

/// Skeleton sound effects
#[derive(Resource, Default)]
pub struct SkeletonSounds {
    pub hurt1: Handle<AudioSource>,
    pub hurt2: Handle<AudioSource>,
    pub hurt3: Handle<AudioSource>,
    pub hurt4: Handle<AudioSource>,
    pub death: Handle<AudioSource>,
}

/// Villager sound effects
#[derive(Resource, Default)]
pub struct VillagerSounds {
    pub hit1: Handle<AudioSource>,
    pub hit2: Handle<AudioSource>,
    pub hit3: Handle<AudioSource>,
    pub hit4: Handle<AudioSource>,
    pub death: Handle<AudioSource>,
}

/// Bow sound effects
#[derive(Resource, Default)]
pub struct BowSounds {
    pub shoot: Handle<AudioSource>,
}

/// Block breaking sound effects
#[derive(Resource, Default)]
pub struct BlockSounds {
    pub stone_break1: Handle<AudioSource>,
    pub stone_break2: Handle<AudioSource>,
    pub stone_break3: Handle<AudioSource>,
    pub wood_break: Handle<AudioSource>,
    pub leaves_break1: Handle<AudioSource>,
    pub leaves_break2: Handle<AudioSource>,
    pub leaves_break3: Handle<AudioSource>,
    pub leaves_break4: Handle<AudioSource>,
    pub leaves_break5: Handle<AudioSource>,
    pub sand_break1: Handle<AudioSource>,
    pub sand_break2: Handle<AudioSource>,
    pub sand_break3: Handle<AudioSource>,
    pub sand_break4: Handle<AudioSource>,
    pub snow_break1: Handle<AudioSource>,
    pub snow_break2: Handle<AudioSource>,
    pub snow_break3: Handle<AudioSource>,
    pub cactus_break1: Handle<AudioSource>,
    pub cactus_break2: Handle<AudioSource>,
    pub cactus_break3: Handle<AudioSource>,
    pub cactus_break4: Handle<AudioSource>,
    pub cactus_break5: Handle<AudioSource>,
    pub pumpkin_carve1: Handle<AudioSource>,
    pub pumpkin_carve2: Handle<AudioSource>,
}
