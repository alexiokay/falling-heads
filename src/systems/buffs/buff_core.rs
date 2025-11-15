use bevy::prelude::*;
use avian3d::prelude::LinearVelocity;
use crate::components::buff::{Buff, BuffType, Buffable, BuffBehavior};
use crate::components::block::{BlockType, BiomeType};
use crate::components::enemy::Health;
use crate::components::damage_visual::MarkedForDeath;
use crate::events::damage_events::EntityDamaged;
use crate::states::GameState;

pub struct BuffPlugin;

impl Plugin for BuffPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            update_buffs,
            apply_buff_effects,
            apply_buff_visual_effects,
            apply_buff_scale_effects,
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
    mut query: Query<(Entity, &mut Buff, Option<&Health>, Option<&mut LinearVelocity>), (With<Buffable>, Without<MarkedForDeath>)>,
    mut damage_events: MessageWriter<EntityDamaged>,
    time: Res<Time>,
) {
    for (entity, mut buff, health_opt, velocity_opt) in query.iter_mut() {
        buff.tick_timer.tick(time.delta());

        // Apply damage/healing on tick (multiplied by stacks)
        if buff.tick_timer.just_finished() {
            let base_damage = buff.buff_type.damage_per_tick();

            if base_damage != 0 {
                if let Some(_health) = health_opt {
                    if base_damage > 0 {
                        // Apply damage multiplied by stack count
                        let total_damage = (base_damage as f32 * buff.get_damage_multiplier()) as u32;
                        damage_events.write(EntityDamaged {
                            entity,
                            damage: total_damage,
                        });
                        info!("Buff {} dealing {} damage (base {} × {} stacks)",
                              buff.buff_type.name(), total_damage, base_damage, buff.stacks);
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
    mut query: Query<(&Buff, &mut Sprite), (With<Buffable>, Without<MarkedForDeath>)>,
) {
    for (buff, mut sprite) in query.iter_mut() {
        // Blend the buff color with the sprite's base color
        let buff_color = buff.buff_type.overlay_color();
        sprite.color = buff_color;
    }
}

/// Helper function to apply a buff to an entity (stacking logic)
pub fn apply_buff(
    commands: &mut Commands,
    entity: Entity,
    buff_type: BuffType,
    duration: f32,
    existing_buff: Option<&mut Buff>,
) {
    if let Some(mut buff) = existing_buff {
        // Buff already exists - add stack if same type
        if buff.buff_type == buff_type {
            buff.add_stack(duration);
            info!("Stacked {} buff (now {} stacks)", buff_type.name(), buff.stacks);
        }
    } else {
        // No buff exists - add new one
        commands.entity(entity).insert(Buff::new(buff_type, duration));
        info!("Applied new {} buff", buff_type.name());
    }
}

/// Apply block-specific buffs based on biome and block type
pub fn apply_block_buff(
    commands: &mut Commands,
    entity: Entity,
    block_type: BlockType,
    biome: BiomeType,
    existing_buff: Option<&mut Buff>,
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
        apply_buff(commands, entity, buff_type, duration, existing_buff);
    }
}

/// Apply scale changes based on Grow buff (visual and collision)
fn apply_buff_scale_effects(
    _commands: Commands,
    mut with_buff_query: Query<(Entity, &Buff, &mut Transform), (With<Buffable>, Without<MarkedForDeath>)>,
    mut without_buff_query: Query<(Entity, &mut Transform), (With<Buffable>, Without<Buff>, Without<MarkedForDeath>)>,
) {
    // Apply scale for entities WITH Grow buff
    for (_entity, buff, mut transform) in with_buff_query.iter_mut() {
        if buff.buff_type == BuffType::Grow {
            let scale_multiplier = buff.buff_type.scale_multiplier();
            // Apply scale based on stacks: 1 stack = 1.2x, 2 stacks = 1.44x (1.2^2)
            let total_scale = scale_multiplier.powi(buff.stacks as i32);

            // Only update if scale actually changed
            if transform.scale.x != total_scale {
                transform.scale = Vec3::splat(total_scale);
                info!("Grow buff applied: scale={:.2}x (collider auto-scales with Transform)", total_scale);
            }
        }
    }

    // Reset scale for entities WITHOUT any buff (buff expired)
    for (_entity, mut transform) in without_buff_query.iter_mut() {
        if transform.scale != Vec3::ONE {
            transform.scale = Vec3::ONE;
            // Collider auto-scales with Transform, no need to manually reset
        }
    }
}
