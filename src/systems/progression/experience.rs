use bevy::prelude::*;
use crate::components::experience::*;
use crate::components::hero::Hero;
use crate::components::resource::ResourceType;
use crate::components::enemy::EnemyType;
use crate::events::xp_events::*;
use crate::events::damage_events::EntityDied;
use crate::events::resource_events::ResourceCollected;
use crate::events::block_events::BlockDestroyed;
use crate::states::GameState;

pub struct ExperiencePlugin;

impl Plugin for ExperiencePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_message::<XpGained>()
            .add_message::<LevelUp>()
            .add_systems(Update,
                (
                    award_xp_for_mob_kills,
                    award_xp_for_resources,
                    award_xp_for_blocks,
                    apply_xp_gains,
                    update_xp_bar_ui,
                ).run_if(in_state(GameState::Playing))
            );

        info!("ExperiencePlugin loaded");
    }
}

/// Award XP when mobs are killed
fn award_xp_for_mob_kills(
    mut death_events: MessageReader<EntityDied>,
    enemy_query: Query<&crate::components::enemy::Enemy>,
    mut xp_events: MessageWriter<XpGained>,
) {
    for event in death_events.read() {
        if let Ok(enemy) = enemy_query.get(event.entity) {
            let xp_amount = match enemy.enemy_type {
                EnemyType::Skeleton => 15,
                EnemyType::Pigman => 10,
                EnemyType::Yeti => 20,
                EnemyType::Zombie => 10,
                EnemyType::Enderman => 30,
                EnemyType::Ghast => 35, // Flying enemy worth good XP
                EnemyType::Dragon => 100,
            };

            xp_events.write(XpGained {
                amount: xp_amount,
                source: XpSource::MobKill,
            });

            info!("💀 Mob killed! Awarded {} XP for {:?}", xp_amount, enemy.enemy_type);
        }
    }
}

/// Award XP when resources are collected
fn award_xp_for_resources(
    mut resource_events: MessageReader<ResourceCollected>,
    mut xp_events: MessageWriter<XpGained>,
) {
    for event in resource_events.read() {
        let xp_amount = match event.resource_type {
            ResourceType::Coal => 2,
            ResourceType::Iron => 3,
            ResourceType::Gold => 5,
            ResourceType::Diamond => 10,
            ResourceType::Emerald => 15,
            ResourceType::Redstone => 3,
            ResourceType::Netherite => 25,
        };

        xp_events.write(XpGained {
            amount: xp_amount,
            source: XpSource::ResourceCollected,
        });
    }
}

/// Award XP when blocks are destroyed
fn award_xp_for_blocks(
    mut block_events: MessageReader<BlockDestroyed>,
    mut xp_events: MessageWriter<XpGained>,
) {
    for _event in block_events.read() {
        // Award 1 XP for breaking any block
        xp_events.write(XpGained {
            amount: 1,
            source: XpSource::BlockBroken,
        });
    }
}

/// Apply XP gains to hero and check for level ups
fn apply_xp_gains(
    mut xp_events: MessageReader<XpGained>,
    mut hero_query: Query<&mut Experience, With<Hero>>,
    mut level_up_events: MessageWriter<LevelUp>,
) {
    if let Ok(mut experience) = hero_query.single_mut() {
        for event in xp_events.read() {
            let leveled_up = experience.add_xp(event.amount);

            if leveled_up {
                info!("🎉 LEVEL UP! Now level {}", experience.level);
                level_up_events.write(LevelUp {
                    new_level: experience.level,
                });
            }
        }
    }
}

/// Update XP bar UI to show current progress
fn update_xp_bar_ui(
    hero_query: Query<&Experience, With<Hero>>,
    mut progress_query: Query<&mut Node, With<XpBarProgress>>,
    mut level_text_query: Query<&mut Text, With<LevelText>>,
) {
    if let Ok(experience) = hero_query.single() {
        // Update progress bar width
        if let Ok(mut node) = progress_query.single_mut() {
            let progress = experience.get_progress();
            node.width = Val::Percent(progress * 100.0);
        }

        // Update level text
        if let Ok(mut text) = level_text_query.single_mut() {
            text.0 = format!("Level {} ({}/{})",
                experience.level,
                experience.current_xp,
                experience.xp_to_next_level
            );
        }
    }
}
