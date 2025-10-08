use bevy::prelude::*;
use bevy::gltf::GltfAssetLabel;
use crate::resources::asset_handles::*;

pub struct AssetLoaderPlugin;

impl Plugin for AssetLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_all_assets);
        info!("AssetLoaderPlugin loaded - centralizing asset loading");
    }
}

/// Centralized asset loading system - keeps main.rs clean
fn load_all_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    info!("Loading all game assets...");

    // Load game textures (UI, entities)
    let textures = GameTextures {
        hero: asset_server.load("textures/entities/hero.jpg"),
        tnt: asset_server.load("textures/effects/tnt.png"),
        nether_bg: asset_server.load("textures/blocks/nether/nether.png"),
        villager: asset_server.load("textures/entities/villager.png"),
    };
    commands.insert_resource(textures);

    // Load block textures
    let block_textures = BlockTextures {
        ancient_debris_top: asset_server.load("textures/blocks/nether/ancient_debris_top.png"),
        ancient_debris_side: asset_server.load("textures/blocks/nether/ancient_debris_side.png"),
        netherrack: asset_server.load("textures/blocks/nether/netherrack.png"),
        nether: asset_server.load("textures/blocks/nether/nether.png"),
        obsidian: asset_server.load("textures/blocks/nether/obsidian.png"),
        sandstone: asset_server.load("textures/blocks/desert/sandstone.png"),
        oak_log: asset_server.load("textures/blocks/forest/oak_log/oak_log.png"),
        oak_log_top: asset_server.load("textures/blocks/forest/oak_log/oak_log_top.png"),
        oak_leaves: asset_server.load("textures/blocks/forest/oak_leaves.png"),
        pumpkin_front: asset_server.load("textures/blocks/forest/pumpkin/pumpkin_front.png"),
        pumpkin_side: asset_server.load("textures/blocks/forest/pumpkin/pumpkin_side.png"),
        pumpkin_top: asset_server.load("textures/blocks/forest/pumpkin/pumpkin_top.png"),
        melon_side: asset_server.load("textures/blocks/forest/melon/melon_side.png"),
        melon_top: asset_server.load("textures/blocks/forest/melon/melon_top.png"),
        dirt: asset_server.load("textures/blocks/general/dirt.png"),
        stone: asset_server.load("textures/blocks/general/stone.png"),
        coal_ore: asset_server.load("textures/blocks/general/coal_ore.png"),
        diamond_ore: asset_server.load("textures/blocks/general/diamond_ore.png"),
        diamond_block: asset_server.load("textures/blocks/general/diamond_block.png"),
        emerald_ore: asset_server.load("textures/blocks/general/emerald_ore.png"),
        gold_block: asset_server.load("textures/blocks/general/gold_block.png"),
        snow: asset_server.load("textures/blocks/iceland/snow.png"),
        ice: asset_server.load("textures/blocks/iceland/ice.png"),
        packed_ice: asset_server.load("textures/blocks/iceland/packed_ice.png"),
    };
    commands.insert_resource(block_textures);

    // Load resource textures (drops)
    let resource_textures = ResourceTextures {
        coal: asset_server.load("textures/resources/coal.png"),
        diamond: asset_server.load("textures/resources/diamond.png"),
        emerald: asset_server.load("textures/resources/emerald.png"),
        gold_ingot: asset_server.load("textures/resources/gold_ingot.png"),
        iron_ingot: asset_server.load("textures/resources/iron_ingot.png"),
        redstone: asset_server.load("textures/resources/redstone.png"),
        netherite_ingot: asset_server.load("textures/resources/netherite_scrap.png"),
    };
    commands.insert_resource(resource_textures);

    // Load enemy textures
    let enemy_textures = EnemyTextures {
        skeleton: asset_server.load("textures/entities/enemy/skeleton.png"),
        skeleton2: asset_server.load("textures/entities/enemy/skeleton2.png"),
        yeti: asset_server.load("textures/entities/enemy/yeti.png"),
        penguin: asset_server.load("textures/entities/enemy/penguin.png"),
    };
    commands.insert_resource(enemy_textures);

    // Load projectile textures
    let projectile_textures = ProjectileTextures {
        arrow: asset_server.load("textures/projectiles/arrow.png"),
        snowball: asset_server.load("textures/projectiles/snowball.png"),
    };
    commands.insert_resource(projectile_textures);

    // Load projectile 3D models
    let projectile_models = crate::resources::asset_handles::ProjectileModels {
        arrow: asset_server.load(
            GltfAssetLabel::Scene(0).from_asset("models/projectiles/arrow2.glb")
        ),  // Load GLB model - Scene 0
    };
    commands.insert_resource(projectile_models);

    // Load heart textures (UI)
    let heart_textures = HeartTextures {
        container: asset_server.load("textures/effects/heart/container.png"),
        half: asset_server.load("textures/effects/heart/half.png"),
        full: asset_server.load("textures/effects/heart/full.png"),
    };
    commands.insert_resource(heart_textures);

    // Load damage overlay textures
    let damage_textures = DamageStageTextures {
        stages: [
            asset_server.load("textures/effects/destroy/destroy_stage_0.png"),
            asset_server.load("textures/effects/destroy/destroy_stage_1.png"),
            asset_server.load("textures/effects/destroy/destroy_stage_2.png"),
            asset_server.load("textures/effects/destroy/destroy_stage_3.png"),
            asset_server.load("textures/effects/destroy/destroy_stage_4.png"),
            asset_server.load("textures/effects/destroy/destroy_stage_5.png"),
            asset_server.load("textures/effects/destroy/destroy_stage_6.png"),
            asset_server.load("textures/effects/destroy/destroy_stage_7.png"),
            asset_server.load("textures/effects/destroy/destroy_stage_8.png"),
        ],
    };
    commands.insert_resource(damage_textures);

    info!("✅ All assets loaded successfully!");
}
