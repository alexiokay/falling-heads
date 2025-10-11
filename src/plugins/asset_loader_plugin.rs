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
        tnt: asset_server.load("textures/effects/tnt/tnt_side.png"),
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
        sand: asset_server.load("textures/blocks/desert/sand.png"),
        sandstone: asset_server.load("textures/blocks/desert/sandstone/sandstone.png"),
        sandstone_top: asset_server.load("textures/blocks/desert/sandstone/sandstone_top.png"),
        sandstone_bottom: asset_server.load("textures/blocks/desert/sandstone/sandstone_bottom.png"),
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
        gold_ore: asset_server.load("textures/blocks/general/gold_ore.png"),
        iron_ore: asset_server.load("textures/blocks/general/iron_ore.png"),
        redstone_ore: asset_server.load("textures/blocks/general/redstone_ore.png"),
        glowstone: asset_server.load("textures/blocks/nether/glowstone.png"),
        end_stone: asset_server.load("textures/blocks/end/end_stone.png"),
        cactus_top: asset_server.load("textures/blocks/cactus/cactus_top.png"),
        cactus_side: asset_server.load("textures/blocks/cactus/cactus_side.png"),
        cactus_bottom: asset_server.load("textures/blocks/cactus/cactus_bottom.png"),
        snow: asset_server.load("textures/blocks/iceland/snow.png"),
        ice: asset_server.load("textures/blocks/iceland/ice.png"),
        packed_ice: asset_server.load("textures/blocks/iceland/packed_ice.png"),
        tnt_top: asset_server.load("textures/effects/tnt/tnt_top.png"),
        tnt_bottom: asset_server.load("textures/effects/tnt/tnt_bottom.png"),
        tnt_side: asset_server.load("textures/effects/tnt/tnt_side.png"),
        soul_sand: asset_server.load("textures/blocks/nether/soul_sand.png"),
        netherite_block: asset_server.load("textures/blocks/nether/netherite_block.png"),
        crimson_nylium: asset_server.load("textures/blocks/nether/crimson_nylium.png"),
        crimson_nylium_side: asset_server.load("textures/blocks/nether/crimson_nylium_side.png"),
        nether_quartz_ore: asset_server.load("textures/blocks/nether/nether_quartz_ore.png"),
    };
    commands.insert_resource(block_textures);

    // Load block break sounds (individual handles match existing struct)
    let block_sounds = BlockSounds {
        stone_break1: asset_server.load("audio/sfx/blocks/stone/break1.ogg"),
        stone_break2: asset_server.load("audio/sfx/blocks/stone/break2.ogg"),
        stone_break3: asset_server.load("audio/sfx/blocks/stone/break3.ogg"),
        wood_break: asset_server.load("audio/sfx/blocks/wood/break1.ogg"),
        sand_break1: asset_server.load("audio/sfx/blocks/sand/break1.ogg"),
        sand_break2: asset_server.load("audio/sfx/blocks/sand/break2.ogg"),
        sand_break3: asset_server.load("audio/sfx/blocks/sand/break3.ogg"),
        sand_break4: asset_server.load("audio/sfx/blocks/sand/break4.ogg"),
        snow_break1: asset_server.load("audio/sfx/blocks/snow/break1.ogg"),
        snow_break2: asset_server.load("audio/sfx/blocks/snow/break2.ogg"),
        snow_break3: asset_server.load("audio/sfx/blocks/snow/break3.ogg"),
        leaves_break1: asset_server.load("audio/sfx/blocks/leaves/break1.ogg"),
        leaves_break2: asset_server.load("audio/sfx/blocks/leaves/break2.ogg"),
        leaves_break3: asset_server.load("audio/sfx/blocks/leaves/break3.ogg"),
        leaves_break4: asset_server.load("audio/sfx/blocks/leaves/break4.ogg"),
        leaves_break5: asset_server.load("audio/sfx/blocks/leaves/break5.ogg"),
        cactus_break1: asset_server.load("audio/sfx/blocks/cactus/break1.ogg"),
        cactus_break2: asset_server.load("audio/sfx/blocks/cactus/break2.ogg"),
        cactus_break3: asset_server.load("audio/sfx/blocks/cactus/break3.ogg"),
        cactus_break4: asset_server.load("audio/sfx/blocks/cactus/break4.ogg"),
        cactus_break5: asset_server.load("audio/sfx/blocks/cactus/break5.ogg"),
        pumpkin_carve1: asset_server.load("audio/sfx/blocks/pumpkin/carve1.ogg"),
        pumpkin_carve2: asset_server.load("audio/sfx/blocks/pumpkin/carve2.ogg"),
    };
    commands.insert_resource(block_sounds);

    // Load resource textures (drops)
    let resource_textures = ResourceTextures {
        coal: asset_server.load("textures/resources/coal.png"),
        diamond: asset_server.load("textures/resources/diamond.png"),
        emerald: asset_server.load("textures/resources/emerald.png"),
        gold_ingot: asset_server.load("textures/resources/gold_ingot.png"),
        iron_ingot: asset_server.load("textures/resources/iron_ingot.png"),
        redstone: asset_server.load("textures/resources/redstone.png"),
        netherite_ingot: asset_server.load("textures/resources/netherite_ingot.png"),
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

    // Load enemy 3D models
    let enemy_models = crate::resources::asset_handles::EnemyModels {
        dragon: asset_server.load(
            GltfAssetLabel::Scene(0).from_asset("models/minecraft_dragon_-_free_-_black_and_white.glb")
        ),  // Load dragon GLB model with built-in animation - Scene 0
    };
    commands.insert_resource(enemy_models);

    // Load dragon animation clip
    let dragon_animation = asset_server.load(
        GltfAssetLabel::Animation(0).from_asset("models/minecraft_dragon_-_free_-_black_and_white.glb")
    );
    commands.insert_resource(crate::resources::asset_handles::DragonAnimation { fly: dragon_animation });

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

    // Load explosion animation frames (16 frames)
    let explosion_frames: Vec<Handle<Image>> = (0..16)
        .map(|i| asset_server.load(format!("textures/effects/explosion/explosion_{}.png", i)))
        .collect();

    commands.insert_resource(crate::resources::asset_handles::ExplosionTextures {
        frames: explosion_frames,
    });

    // Load effect textures
    let effect_textures = crate::resources::asset_handles::EffectTextures {
        fire: asset_server.load("textures/effects/buffs/campfire_fire.png"),
    };
    commands.insert_resource(effect_textures);

    // Load hero sound effects
    let hero_sounds = crate::resources::asset_handles::HeroSounds {
        hit1: asset_server.load("audio/sfx/mobs/hero/hit1.ogg"),
        hit2: asset_server.load("audio/sfx/mobs/hero/hit2.ogg"),
        hit3: asset_server.load("audio/sfx/mobs/hero/hit3.ogg"),
        fallbig: asset_server.load("audio/sfx/mobs/hero/fallbig.ogg"),
        fallsmall: asset_server.load("audio/sfx/mobs/hero/fallsmall.ogg"),
    };
    commands.insert_resource(hero_sounds);

    // Load skeleton sound effects
    let skeleton_sounds = crate::resources::asset_handles::SkeletonSounds {
        hurt1: asset_server.load("audio/sfx/mobs/skeleton/hurt1.ogg"),
        hurt2: asset_server.load("audio/sfx/mobs/skeleton/hurt2.ogg"),
        hurt3: asset_server.load("audio/sfx/mobs/skeleton/hurt3.ogg"),
        hurt4: asset_server.load("audio/sfx/mobs/skeleton/hurt4.ogg"),
        death: asset_server.load("audio/sfx/mobs/skeleton/death.ogg"),
    };
    commands.insert_resource(skeleton_sounds);

    // Load villager sound effects
    let villager_sounds = crate::resources::asset_handles::VillagerSounds {
        hit1: asset_server.load("audio/sfx/mobs/villager/hit1.ogg"),
        hit2: asset_server.load("audio/sfx/mobs/villager/hit2.ogg"),
        hit3: asset_server.load("audio/sfx/mobs/villager/hit3.ogg"),
        hit4: asset_server.load("audio/sfx/mobs/villager/hit4.ogg"),
        death: asset_server.load("audio/sfx/mobs/villager/death.ogg"),
    };
    commands.insert_resource(villager_sounds);

    // Load bow sound effects
    let bow_sounds = crate::resources::asset_handles::BowSounds {
        shoot: asset_server.load("audio/sfx/bow/bow.ogg"),
    };
    commands.insert_resource(bow_sounds);

    // Load block breaking sound effects
    let block_sounds = crate::resources::asset_handles::BlockSounds {
        stone_break1: asset_server.load("audio/sfx/blocks/stone/break1.ogg"),
        stone_break2: asset_server.load("audio/sfx/blocks/stone/break2.ogg"),
        stone_break3: asset_server.load("audio/sfx/blocks/stone/break3.ogg"),
        wood_break: asset_server.load("audio/sfx/blocks/wood/break1.ogg"),
        leaves_break1: asset_server.load("audio/sfx/blocks/leaves/break1.ogg"),
        leaves_break2: asset_server.load("audio/sfx/blocks/leaves/break2.ogg"),
        leaves_break3: asset_server.load("audio/sfx/blocks/leaves/break3.ogg"),
        leaves_break4: asset_server.load("audio/sfx/blocks/leaves/break4.ogg"),
        leaves_break5: asset_server.load("audio/sfx/blocks/leaves/break5.ogg"),
        sand_break1: asset_server.load("audio/sfx/blocks/sand/break1.ogg"),
        sand_break2: asset_server.load("audio/sfx/blocks/sand/break2.ogg"),
        sand_break3: asset_server.load("audio/sfx/blocks/sand/break3.ogg"),
        sand_break4: asset_server.load("audio/sfx/blocks/sand/break4.ogg"),
        snow_break1: asset_server.load("audio/sfx/blocks/snow/break1.ogg"),
        snow_break2: asset_server.load("audio/sfx/blocks/snow/break2.ogg"),
        snow_break3: asset_server.load("audio/sfx/blocks/snow/break3.ogg"),
        cactus_break1: asset_server.load("audio/sfx/blocks/cactus/break1.ogg"),
        cactus_break2: asset_server.load("audio/sfx/blocks/cactus/break2.ogg"),
        cactus_break3: asset_server.load("audio/sfx/blocks/cactus/break3.ogg"),
        cactus_break4: asset_server.load("audio/sfx/blocks/cactus/break4.ogg"),
        cactus_break5: asset_server.load("audio/sfx/blocks/cactus/break5.ogg"),
        pumpkin_carve1: asset_server.load("audio/sfx/blocks/pumpkin/carve1.ogg"),
        pumpkin_carve2: asset_server.load("audio/sfx/blocks/pumpkin/carve2.ogg"),
    };
    commands.insert_resource(block_sounds);

    info!("✅ All assets loaded successfully!");
}
