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
}

#[derive(Resource, Default)]
pub struct ProjectileTextures {
    pub arrow: Handle<Image>,
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
    pub obsidian: Handle<Image>,
    pub gold_block: Handle<Image>,
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
