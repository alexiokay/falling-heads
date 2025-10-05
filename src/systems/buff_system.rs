use bevy::prelude::*;
use crate::components::buff::{Buff, BuffType, Buffable};
use crate::components::block::{BlockType, BiomeType};
use crate::components::enemy::Health;
use crate::components::physics::Velocity;
use crate::events::damage_events::EntityDamaged;
use crate::states::GameState;

pub struct BuffPlugin;

impl Plugin for BuffPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            update_buffs,
            apply_buff_effects,
            apply_buff_visual_effects,
        ).run_if(in_state(GameState::Playing)));

        info!("BuffPlugin loaded");
    }
}

/// Update buff timers and remove expired buffs
fn update_buffs(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Buff)>,
    time: Res<Time>,
) {
    for (entity, mut buff) in query.iter_mut() {
        buff.duration.tick(time.delta());

        if buff.duration.is_finished() {
            commands.entity(entity).remove::<Buff>();
        }
    }
}

/// Apply buff effects (damage/healing, velocity changes)
fn apply_buff_effects(
    mut query: Query<(Entity, &mut Buff, Option<&Health>, Option<&mut Velocity>), With<Buffable>>,
    mut damage_events: MessageWriter<EntityDamaged>,
    time: Res<Time>,
) {
    for (entity, mut buff, health_opt, velocity_opt) in query.iter_mut() {
        buff.tick_timer.tick(time.delta());

        // Apply damage/healing on tick
        if buff.tick_timer.just_finished() {
            let damage = buff.buff_type.damage_per_tick();

            if damage != 0 {
                if let Some(_health) = health_opt {
                    if damage > 0 {
                        // Emit damage event - let health system handle death
                        damage_events.write(EntityDamaged {
                            entity,
                            damage: damage as u32,
                        });
                    } else {
                        // Healing not implemented yet - would need EntityHealed event
                        // For now, skip healing
                    }
                }
            }
        }

        // Apply velocity multiplier
        if let Some(mut velocity) = velocity_opt {
            let multiplier = buff.buff_type.velocity_multiplier();

            // Only apply if not default (1.0)
            if multiplier != 1.0 {
                velocity.x *= multiplier;
            }
        }
    }
}

/// Apply visual effects for buffs (color overlay)
fn apply_buff_visual_effects(
    mut query: Query<(&Buff, &mut Sprite), With<Buffable>>,
) {
    for (buff, mut sprite) in query.iter_mut() {
        // Blend the buff color with the sprite's base color
        let buff_color = buff.buff_type.overlay_color();
        sprite.color = buff_color;
    }
}

/// Helper function to apply a buff to an entity
pub fn apply_buff(
    commands: &mut Commands,
    entity: Entity,
    buff_type: BuffType,
    duration: f32,
) {
    commands.entity(entity).insert(Buff::new(buff_type, duration));
}

/// Apply block-specific buffs based on biome and block type
pub fn apply_block_buff(
    commands: &mut Commands,
    entity: Entity,
    block_type: BlockType,
    biome: BiomeType,
) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // 20% chance to apply buff on collision
    if !rng.gen_bool(0.2) {
        return;
    }

    let buff = match biome {
        BiomeType::Nether => {
            // Nether: only special blocks give debuffs
            match block_type {
                BlockType::SoulSand => Some((BuffType::Slowness, 3.0)),
                _ => None,
            }
        },
        BiomeType::End => {
            // End: slowness from chorus
            match block_type {
                BlockType::ChorusFruit => Some((BuffType::Speed, 5.0)),
                _ => None,
            }
        },
        BiomeType::Mountains => {
            // Mountains: freezing from snow
            match block_type {
                BlockType::Snow => Some((BuffType::Freezing, 3.0)),
                _ => None,
            }
        },
        BiomeType::Underground => {
            // Underground: poisoned from mushrooms
            match block_type {
                BlockType::Mushroom => Some((BuffType::Poisoned, 4.0)),
                _ => None,
            }
        },
        _ => None,
    };

    if let Some((buff_type, duration)) = buff {
        commands.entity(entity).insert(Buff::new(buff_type, duration));
    }
}
