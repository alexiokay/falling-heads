use crate::components::block::BlockType;
use crate::components::enemy::{Enemy, EnemyType};
use crate::components::hero::Hero;
use crate::components::villager::Villager;
use crate::events::block_events::BlockDestroyed;
use crate::events::damage_events::{EntityDamaged, EntityDied};
use crate::resources::asset_handles::{
    BlockSounds, BowSounds, HeroSounds, SkeletonSounds, VillagerSounds,
};
use bevy::prelude::*;
use rand::RngExt;

pub struct MobSoundPlugin;

impl Plugin for MobSoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                play_damage_sounds,
                play_death_sounds,
                play_bow_sounds,
                play_block_break_sounds,
            ),
        );

        info!("MobSoundPlugin loaded");
    }
}

/// Where an entity's sound comes from, so far-away hits and deaths stay quiet.
fn sound_at(
    positions: &Query<&GlobalTransform>,
    entity: Entity,
) -> crate::systems::audio::effects::SoundAt {
    // Unknown position (already despawned): NaN means "play at full volume".
    crate::systems::audio::effects::SoundAt(
        positions
            .get(entity)
            .map_or(Vec2::NAN, |t| t.translation().xy()),
    )
}

/// Play random damage/hit sounds when entities take damage
#[allow(clippy::too_many_arguments)] // Bevy system parameters
fn play_damage_sounds(
    mut commands: Commands,
    mut damage_events: MessageReader<EntityDamaged>,
    hero_query: Query<(), With<Hero>>,
    enemy_query: Query<&Enemy>,
    villager_query: Query<(), With<Villager>>,
    positions: Query<&GlobalTransform>,
    hero_sounds: Res<HeroSounds>,
    skeleton_sounds: Res<SkeletonSounds>,
    villager_sounds: Res<VillagerSounds>,
) {
    let mut rng = rand::rng();

    for event in damage_events.read() {
        let at = sound_at(&positions, event.entity);
        // Check what type of entity was damaged
        if hero_query.get(event.entity).is_ok() {
            // Hero was hit - play random hero hit sound
            let sound = match rng.random_range(0..3) {
                0 => hero_sounds.hit1.clone(),
                1 => hero_sounds.hit2.clone(),
                _ => hero_sounds.hit3.clone(),
            };

            commands.spawn((
                AudioPlayer::new(sound),
                PlaybackSettings::DESPAWN, // Auto-cleanup when finished
            ));
        } else if let Ok(enemy) = enemy_query.get(event.entity) {
            // Enemy was hit
            if enemy.enemy_type == EnemyType::Skeleton {
                // Play random skeleton hurt sound
                let sound = match rng.random_range(0..4) {
                    0 => skeleton_sounds.hurt1.clone(),
                    1 => skeleton_sounds.hurt2.clone(),
                    2 => skeleton_sounds.hurt3.clone(),
                    _ => skeleton_sounds.hurt4.clone(),
                };

                commands.spawn((
                    AudioPlayer::new(sound),
                    PlaybackSettings::DESPAWN, // Auto-cleanup when finished
                    at,
                ));
            }
        } else if villager_query.get(event.entity).is_ok() {
            // Villager was hit - play random villager hit sound
            let sound = match rng.random_range(0..4) {
                0 => villager_sounds.hit1.clone(),
                1 => villager_sounds.hit2.clone(),
                2 => villager_sounds.hit3.clone(),
                _ => villager_sounds.hit4.clone(),
            };

            commands.spawn((
                AudioPlayer::new(sound),
                PlaybackSettings::DESPAWN, // Auto-cleanup when finished
                at,
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
    positions: Query<&GlobalTransform>,
    skeleton_sounds: Res<SkeletonSounds>,
    villager_sounds: Res<VillagerSounds>,
) {
    for event in death_events.read() {
        let at = sound_at(&positions, event.entity);
        // Check what type of entity died
        if let Ok(enemy) = enemy_query.get(event.entity) {
            if enemy.enemy_type == EnemyType::Skeleton {
                commands.spawn((
                    AudioPlayer::new(skeleton_sounds.death.clone()),
                    PlaybackSettings::DESPAWN, // Auto-cleanup when finished
                    at,
                ));
            }
        } else if villager_query.get(event.entity).is_ok() {
            commands.spawn((
                AudioPlayer::new(villager_sounds.death.clone()),
                PlaybackSettings::DESPAWN, // Auto-cleanup when finished
                at,
            ));
        }
    }
}

/// Play bow sound when projectiles (arrows) are spawned
fn play_bow_sounds(
    mut commands: Commands,
    projectile_query: Query<&Transform, Added<crate::components::projectile::Arrow>>,
    bow_sounds: Res<BowSounds>,
) {
    // One bow sound per frame of new arrows, placed at the first arrow so
    // off-screen skeletons are not heard.
    if let Some(arrow) = projectile_query.iter().next() {
        commands.spawn((
            AudioPlayer::new(bow_sounds.shoot.clone()),
            PlaybackSettings::DESPAWN, // Auto-cleanup when finished
            crate::systems::audio::effects::SoundAt(arrow.translation.xy()),
        ));
    }
}

/// Play block breaking sounds when blocks are destroyed
fn play_block_break_sounds(
    mut commands: Commands,
    mut block_destroyed_events: MessageReader<BlockDestroyed>,
    block_sounds: Res<BlockSounds>,
) {
    // Explosions and beams break dozens of blocks in one frame. A few
    // overlapping break sounds already read as "lots"; one sound per block
    // spawned 100+ audio decoders at once and stalled the frame.
    const MAX_BREAK_SOUNDS_PER_FRAME: usize = 4;

    let mut rng = rand::rng();

    for event in block_destroyed_events
        .read()
        .take(MAX_BREAK_SOUNDS_PER_FRAME)
    {
        let sound = match event.block_type {
            // Stone and ore blocks
            BlockType::Stone
            | BlockType::Coal
            | BlockType::Iron
            | BlockType::Gold
            | BlockType::Diamond
            | BlockType::Emerald
            | BlockType::Redstone
            | BlockType::Sandstone
            | BlockType::Netherrack
            | BlockType::Netherite
            | BlockType::AncientDebris
            | BlockType::Obsidian
            | BlockType::EndStone => {
                // Random stone break sound
                match rng.random_range(0..3) {
                    0 => block_sounds.stone_break1.clone(),
                    1 => block_sounds.stone_break2.clone(),
                    _ => block_sounds.stone_break3.clone(),
                }
            }

            // Sand blocks
            BlockType::Sand => {
                // Random sand break sound
                match rng.random_range(0..4) {
                    0 => block_sounds.sand_break1.clone(),
                    1 => block_sounds.sand_break2.clone(),
                    2 => block_sounds.sand_break3.clone(),
                    _ => block_sounds.sand_break4.clone(),
                }
            }

            // Snow blocks
            BlockType::Snow => {
                // Random snow break sound
                match rng.random_range(0..3) {
                    0 => block_sounds.snow_break1.clone(),
                    1 => block_sounds.snow_break2.clone(),
                    _ => block_sounds.snow_break3.clone(),
                }
            }

            // Wood blocks
            BlockType::Wood => block_sounds.wood_break.clone(),

            // Leaves, grass, plants
            BlockType::Leaves
            | BlockType::Grass
            | BlockType::Melon
            | BlockType::Pumpkin
            | BlockType::Cactus
            | BlockType::DeadBush
            | BlockType::Mushroom
            | BlockType::Vines
            | BlockType::ChorusFruit
            | BlockType::Purpur => {
                // Random leaves break sound
                match rng.random_range(0..5) {
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
            PlaybackSettings::DESPAWN, // Auto-cleanup when finished
            crate::systems::audio::effects::SoundAt(event.position.xy()),
        ));
    }
    // Drop the rest of this frame's breaks so they don't play late next frame.
    block_destroyed_events.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_big_blast_plays_a_few_break_sounds_not_one_per_block() {
        let mut app = App::new();
        app.init_resource::<BlockSounds>()
            .add_message::<BlockDestroyed>()
            .add_systems(Update, play_block_break_sounds);
        for i in 0..120 {
            app.world_mut().write_message(BlockDestroyed {
                tile: None,
                entity: Entity::PLACEHOLDER,
                block_type: BlockType::Stone,
                position: Vec3::X * i as f32,
            });
        }
        app.update();
        let mut players = app.world_mut().query::<&AudioPlayer>();
        assert_eq!(players.iter(app.world()).count(), 4);
        // The rest of the blast must not trickle in on later frames.
        app.update();
        assert_eq!(players.iter(app.world()).count(), 4);
    }
}
