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
