use bevy::prelude::*;
use rand::Rng;
use crate::components::hero::Hero;
use crate::components::enemy::{Enemy, EnemyType};
use crate::components::villager::Villager;
use crate::components::block::BlockType;
use crate::events::damage_events::{EntityDamaged, EntityDied};
use crate::events::block_events::BlockDestroyed;
use crate::resources::asset_handles::{HeroSounds, SkeletonSounds, VillagerSounds, BowSounds, BlockSounds};

pub struct MobSoundPlugin;

impl Plugin for MobSoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            play_damage_sounds,
            play_death_sounds,
            play_bow_sounds,
            play_block_break_sounds,
        ));

        info!("MobSoundPlugin loaded");
    }
}

/// Play random damage/hit sounds when entities take damage
fn play_damage_sounds(
    mut commands: Commands,
    mut damage_events: MessageReader<EntityDamaged>,
    hero_query: Query<(), With<Hero>>,
    enemy_query: Query<&Enemy>,
    villager_query: Query<(), With<Villager>>,
    hero_sounds: Res<HeroSounds>,
    skeleton_sounds: Res<SkeletonSounds>,
    villager_sounds: Res<VillagerSounds>,
) {
    let mut rng = rand::thread_rng();

    for event in damage_events.read() {
        // Check what type of entity was damaged
        if hero_query.get(event.entity).is_ok() {
            // Hero was hit - play random hero hit sound
            let sound = match rng.gen_range(0..3) {
                0 => hero_sounds.hit1.clone(),
                1 => hero_sounds.hit2.clone(),
                _ => hero_sounds.hit3.clone(),
            };

            commands.spawn((
                AudioPlayer::new(sound),
                PlaybackSettings::DESPAWN,  // Auto-cleanup when finished
            ));
        } else if let Ok(enemy) = enemy_query.get(event.entity) {
            // Enemy was hit
            if enemy.enemy_type == EnemyType::Skeleton {
                // Play random skeleton hurt sound
                let sound = match rng.gen_range(0..4) {
                    0 => skeleton_sounds.hurt1.clone(),
                    1 => skeleton_sounds.hurt2.clone(),
                    2 => skeleton_sounds.hurt3.clone(),
                    _ => skeleton_sounds.hurt4.clone(),
                };

                commands.spawn((
                AudioPlayer::new(sound),
                PlaybackSettings::DESPAWN,  // Auto-cleanup when finished
            ));
            }
        } else if villager_query.get(event.entity).is_ok() {
            // Villager was hit - play random villager hit sound
            let sound = match rng.gen_range(0..4) {
                0 => villager_sounds.hit1.clone(),
                1 => villager_sounds.hit2.clone(),
                2 => villager_sounds.hit3.clone(),
                _ => villager_sounds.hit4.clone(),
            };

            commands.spawn((
                AudioPlayer::new(sound),
                PlaybackSettings::DESPAWN,  // Auto-cleanup when finished
            ));
        }
    }
}

/// Play death sounds when entities die
fn play_death_sounds(
    mut commands: Commands,
    mut death_events: MessageReader<EntityDied>,
    enemy_query: Query<&Enemy>,
    villager_query: Query<(), With<Villager>>,
    skeleton_sounds: Res<SkeletonSounds>,
    villager_sounds: Res<VillagerSounds>,
) {
    for event in death_events.read() {
        // Check what type of entity died
        if let Ok(enemy) = enemy_query.get(event.entity) {
            if enemy.enemy_type == EnemyType::Skeleton {
                commands.spawn((
                    AudioPlayer::new(skeleton_sounds.death.clone()),
                    PlaybackSettings::DESPAWN,  // Auto-cleanup when finished
                ));
            }
        } else if villager_query.get(event.entity).is_ok() {
            commands.spawn((
                AudioPlayer::new(villager_sounds.death.clone()),
                PlaybackSettings::DESPAWN,  // Auto-cleanup when finished
            ));
        }
    }
}

/// Play bow sound when projectiles (arrows) are spawned
fn play_bow_sounds(
    mut commands: Commands,
    projectile_query: Query<(), Added<crate::components::projectile::Arrow>>,
    bow_sounds: Res<BowSounds>,
) {
    // Play sound for each newly spawned projectile
    if !projectile_query.is_empty() {
        commands.spawn((
            AudioPlayer::new(bow_sounds.shoot.clone()),
            PlaybackSettings::DESPAWN,  // Auto-cleanup when finished
        ));
    }
}

/// Play block breaking sounds when blocks are destroyed
fn play_block_break_sounds(
    mut commands: Commands,
    mut block_destroyed_events: MessageReader<BlockDestroyed>,
    block_sounds: Res<BlockSounds>,
) {
    let mut rng = rand::thread_rng();

    for event in block_destroyed_events.read() {
        let sound = match event.block_type {
            // Stone and ore blocks
            BlockType::Stone | BlockType::Coal | BlockType::Iron | BlockType::Gold
            | BlockType::Diamond | BlockType::Emerald | BlockType::Redstone
            | BlockType::Sandstone | BlockType::Netherrack | BlockType::Netherite
            | BlockType::AncientDebris | BlockType::Obsidian | BlockType::EndStone => {
                // Random stone break sound
                match rng.gen_range(0..3) {
                    0 => block_sounds.stone_break1.clone(),
                    1 => block_sounds.stone_break2.clone(),
                    _ => block_sounds.stone_break3.clone(),
                }
            }

            // Sand blocks
            BlockType::Sand => {
                // Random sand break sound
                match rng.gen_range(0..4) {
                    0 => block_sounds.sand_break1.clone(),
                    1 => block_sounds.sand_break2.clone(),
                    2 => block_sounds.sand_break3.clone(),
                    _ => block_sounds.sand_break4.clone(),
                }
            }

            // Snow blocks
            BlockType::Snow => {
                // Random snow break sound
                match rng.gen_range(0..3) {
                    0 => block_sounds.snow_break1.clone(),
                    1 => block_sounds.snow_break2.clone(),
                    _ => block_sounds.snow_break3.clone(),
                }
            }

            // Wood blocks
            BlockType::Wood => block_sounds.wood_break.clone(),

            // Leaves, grass, plants
            BlockType::Leaves | BlockType::Grass | BlockType::Melon | BlockType::Pumpkin
            | BlockType::Cactus | BlockType::DeadBush | BlockType::Mushroom | BlockType::Vines
            | BlockType::ChorusFruit | BlockType::Purpur => {
                // Random leaves break sound
                match rng.gen_range(0..5) {
                    0 => block_sounds.leaves_break1.clone(),
                    1 => block_sounds.leaves_break2.clone(),
                    2 => block_sounds.leaves_break3.clone(),
                    3 => block_sounds.leaves_break4.clone(),
                    _ => block_sounds.leaves_break5.clone(),
                }
            }

            // Default to stone sound for others
            _ => block_sounds.stone_break1.clone(),
        };

        commands.spawn((
            AudioPlayer::new(sound),
            PlaybackSettings::DESPAWN,  // Auto-cleanup when finished
        ));
    }
}
