use crate::components::block::{BiomeType, BlockType};
use crate::components::buff::{Buff, BuffBehavior, BuffType, Buffable, GrowthPower};
use crate::components::damage_visual::MarkedForDeath;
use crate::components::enemy::Health;
use crate::events::damage_events::EntityDamaged;
use bevy::prelude::*;

pub struct BuffPlugin;

impl Plugin for BuffPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_growth_power,
                update_buffs,
                apply_buff_effects,
                apply_buff_visual_effects,
                apply_buff_scale_effects,
            )
                .chain()
                .run_if(in_state(crate::states::InGame)),
        );

        info!("BuffPlugin loaded");
    }
}

/// Update buff timers and remove expired buffs
fn update_buffs(mut commands: Commands, mut query: Query<(Entity, &mut Buff)>, time: Res<Time>) {
    for (entity, mut buff) in query.iter_mut() {
        buff.duration.tick(time.delta());

        if buff.duration.is_finished() {
            commands.entity(entity).remove::<Buff>();
        }
    }
}

/// Apply buff effects (damage/healing, velocity changes)
fn apply_buff_effects(
    mut query: Query<
        (Entity, &mut Buff, Option<&Health>),
        (With<Buffable>, Without<MarkedForDeath>),
    >,
    mut damage_events: MessageWriter<EntityDamaged>,
    time: Res<Time>,
) {
    for (entity, mut buff, health_opt) in query.iter_mut() {
        buff.tick_timer.tick(time.delta());

        // Apply damage/healing on tick (multiplied by stacks)
        if buff.tick_timer.just_finished() {
            let base_damage = buff.buff_type.damage_per_tick();

            if base_damage != 0 {
                if let Some(_health) = health_opt {
                    if base_damage > 0 {
                        // Apply damage multiplied by stack count
                        let total_damage =
                            (base_damage as f32 * buff.get_damage_multiplier()) as u32;
                        damage_events.write(EntityDamaged {
                            entity,
                            damage: total_damage,
                        });
                        info!(
                            "Buff {} dealing {} damage (base {} × {} stacks)",
                            buff.buff_type.name(),
                            total_damage,
                            base_damage,
                            buff.stacks
                        );
                    } else {
                        // Healing not implemented yet - would need EntityHealed event
                        // For now, skip healing
                    }
                }
            }
        }
        // Speed buffs scale walking acceleration and caps in the movement systems;
        // multiplying stored velocity here compounded every frame.
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
    if let Some(buff) = existing_buff {
        // Buff already exists - add stack if same type
        if buff.buff_type == buff_type {
            buff.add_stack(duration);
            info!(
                "Stacked {} buff (now {} stacks)",
                buff_type.name(),
                buff.stacks
            );
        }
    } else {
        // No buff exists - add new one
        commands
            .entity(entity)
            .insert(Buff::new(buff_type, duration));
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
    use rand::RngExt;
    let mut rng = rand::rng();

    // 20% chance to apply buff on collision
    if !rng.random_bool(0.2) {
        return;
    }

    let buff = match biome {
        BiomeType::Nether => {
            // Nether: only special blocks give debuffs
            match block_type {
                BlockType::SoulSand => Some((BuffType::Slowness, 3.0)),
                _ => None,
            }
        }
        BiomeType::End => {
            // End: slowness from chorus
            match block_type {
                BlockType::ChorusFruit => Some((BuffType::Speed, 5.0)),
                _ => None,
            }
        }
        BiomeType::Mountains => {
            // Mountains: freezing from snow
            match block_type {
                BlockType::Snow => Some((BuffType::Freezing, 3.0)),
                _ => None,
            }
        }
        BiomeType::Underground => {
            // Underground: poisoned from mushrooms
            match block_type {
                BlockType::Mushroom => Some((BuffType::Poisoned, 4.0)),
                _ => None,
            }
        }
        _ => None,
    };

    if let Some((buff_type, duration)) = buff {
        apply_buff(commands, entity, buff_type, duration, existing_buff);
    }
}

pub(crate) fn activate_growth(commands: &mut Commands, entity: Entity) {
    // Queue against the world so multiple API commands in one frame stack too.
    commands.queue(move |world: &mut World| {
        let Ok(mut actor) = world.get_entity_mut(entity) else {
            return;
        };
        if let Some(mut growth) = actor.get_mut::<GrowthPower>() {
            growth.stacks = (growth.stacks + 1).min(2);
            growth.duration.reset();
        } else {
            actor.insert(GrowthPower::default());
        }
    });
}
fn update_growth_power(
    mut commands: Commands,
    time: Res<Time>,
    mut actors: Query<(Entity, &mut GrowthPower)>,
) {
    for (e, mut growth) in &mut actors {
        growth.duration.tick(time.delta());
        if growth.duration.is_finished() {
            commands.entity(e).remove::<GrowthPower>();
        }
    }
}
#[derive(Component)]
struct PreGrowthScale(Vec3);
/// SHRINK power: half size.
const SHRINK_SCALE: f32 = 0.5;
/// Preserve authored size when the grow buff or SHRINK starts, changes or expires.
fn apply_buff_scale_effects(
    mut commands: Commands,
    time: Option<Res<Time>>,
    mut actors: Query<
        (
            Entity,
            Option<&Buff>,
            &mut Transform,
            Option<&PreGrowthScale>,
            Option<&GrowthPower>,
            Option<&crate::systems::extra_powers::Shrunk>,
        ),
        (With<Buffable>, Without<MarkedForDeath>),
    >,
) {
    let now = time.map_or(0.0, |t| t.elapsed_secs());
    for (e, buff, mut t, saved, growth, shrunk) in &mut actors {
        let stacks = growth.map(|g| g.stacks).or_else(|| {
            buff.filter(|b| b.buff_type == BuffType::Grow)
                .map(|b| b.stacks)
        });
        let shrinking = shrunk.is_some_and(|s| now < s.until);
        if shrunk.is_some() && !shrinking {
            commands
                .entity(e)
                .remove::<crate::systems::extra_powers::Shrunk>();
        }
        if stacks.is_some() || shrinking {
            let base = saved.map_or(t.scale, |s| s.0);
            if saved.is_none() {
                commands.entity(e).insert(PreGrowthScale(base));
            }
            let grow = stacks.map_or(1.0, |s| BuffType::Grow.scale_multiplier().powi(s as i32));
            let desired = base * grow * if shrinking { SHRINK_SCALE } else { 1.0 };
            if t.scale != desired {
                t.scale = desired;
            }
        } else if let Some(saved) = saved {
            t.scale = saved.0;
            commands.entity(e).remove::<PreGrowthScale>();
        }
    }
}
#[cfg(test)]
mod size_regression {
    use super::*;
    #[test]
    fn commanded_growth_survives_other_statuses_stacks_refreshes_and_expires() {
        let mut app = App::new();
        app.init_resource::<Time>().add_systems(
            Update,
            (update_growth_power, apply_buff_scale_effects).chain(),
        );
        let e = app
            .world_mut()
            .spawn((Buffable, Transform::from_scale(Vec3::splat(0.5))))
            .id();
        activate_growth(&mut app.world_mut().commands(), e);
        app.world_mut().flush();
        app.update();
        app.world_mut()
            .entity_mut(e)
            .insert(Buff::new(BuffType::Burning, 2.));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(12));
        app.update();
        assert!(app.world().get::<GrowthPower>(e).is_some());
        assert!((app.world().get::<Transform>(e).unwrap().scale.x - 0.6).abs() < 0.001);
        activate_growth(&mut app.world_mut().commands(), e);
        activate_growth(&mut app.world_mut().commands(), e);
        app.world_mut().flush();
        assert_eq!(app.world().get::<GrowthPower>(e).unwrap().stacks, 2);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(29));
        app.update();
        assert!(app.world().get::<GrowthPower>(e).is_some());
        assert!((app.world().get::<Transform>(e).unwrap().scale.x - 0.72).abs() < 0.001);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(2));
        app.update();
        assert!(app.world().get::<GrowthPower>(e).is_none());
        assert_eq!(
            app.world().get::<Transform>(e).unwrap().scale,
            Vec3::splat(0.5)
        );
    }
    #[test]
    fn grow_restores_authored_size_and_does_not_reset_unbuffed_heads() {
        let mut app = App::new();
        app.add_systems(Update, apply_buff_scale_effects);
        let e = app
            .world_mut()
            .spawn((Buffable, Transform::from_scale(Vec3::splat(0.5))))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Transform>(e).unwrap().scale,
            Vec3::splat(0.5)
        );
        app.world_mut()
            .entity_mut(e)
            .insert(Buff::new(BuffType::Grow, 2.));
        app.update();
        assert!(app.world().get::<Transform>(e).unwrap().scale.x > 0.5);
        app.world_mut().entity_mut(e).remove::<Buff>();
        app.update();
        assert_eq!(
            app.world().get::<Transform>(e).unwrap().scale,
            Vec3::splat(0.5)
        );
    }
}
