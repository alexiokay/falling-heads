use bevy::prelude::*;
use crate::components::buff::{Buff, BuffType, Buffable};
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
    mut query: Query<(&mut Buff, Option<&mut Health>, Option<&mut Velocity>), With<Buffable>>,
    mut damage_events: MessageWriter<EntityDamaged>,
    time: Res<Time>,
) {
    for (mut buff, health_opt, velocity_opt) in query.iter_mut() {
        buff.tick_timer.tick(time.delta());

        // Apply damage/healing on tick
        if buff.tick_timer.just_finished() {
            let damage = buff.buff_type.damage_per_tick();

            if damage != 0 {
                if let Some(mut health) = health_opt {
                    if damage > 0 {
                        // Apply damage
                        health.current = health.current.saturating_sub(damage as u32);
                    } else {
                        // Apply healing
                        health.current = (health.current as i32 - damage).min(health.max as i32) as u32;
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
