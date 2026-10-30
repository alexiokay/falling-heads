use crate::components::{
    block::CobwebBlock, enemy::Enemy, hero::Hero, power_effect::CobwebSlow, villager::Villager,
};
use avian3d::prelude::*;
use bevy::prelude::*;

const COBWEB_SLOW_MULTIPLIER: f32 = 0.25; // 75% speed reduction
const COBWEB_MAX_FALL_SPEED: f32 = -20.0; // Clamp falling speed when in cobweb (very slow to prevent clipping)
const COBWEB_MIN_HORIZONTAL_SPEED: f32 = 5.0; // Minimum horizontal speed to prevent phasing through blocks
const BLOCK_SIZE: f32 = 64.0;
const COLLISION_THRESHOLD: f32 = BLOCK_SIZE * 0.8; // Slightly smaller for better feel

/// System that detects when entities enter cobwebs and applies slow effect
pub fn cobweb_collision_system(
    mut commands: Commands,
    entity_query: Query<
        (Entity, &Transform),
        (
            Or<(With<Hero>, With<Villager>, With<Enemy>)>,
            Without<crate::systems::spiders::Spider>,
        ),
    >,
    cobweb_query: Query<(&Transform, Has<crate::systems::spiders::Web>), With<CobwebBlock>>,
    cobweb_slow_query: Query<Entity, With<CobwebSlow>>,
) {
    for (entity, entity_transform) in entity_query.iter() {
        let entity_pos = entity_transform.translation;
        let mut in_cobweb = false;

        // Check all cobweb blocks to see if entity is touching one
        for (cobweb_transform, silk) in cobweb_query.iter() {
            let cobweb_pos = cobweb_transform.translation;
            let distance = entity_pos.xy().distance(cobweb_pos.xy());

            if distance < if silk { 76. } else { COLLISION_THRESHOLD } {
                in_cobweb = true;
                break;
            }
        }

        // Add or remove CobwebSlow component based on collision
        let has_slow = cobweb_slow_query.contains(entity);

        if in_cobweb && !has_slow {
            // Add slow effect
            commands.entity(entity).insert(CobwebSlow {
                slow_multiplier: COBWEB_SLOW_MULTIPLIER,
            });
            info!("🕸️ Entity entered cobweb - applying slow effect");
        } else if !in_cobweb && has_slow {
            // Remove slow effect
            commands.entity(entity).remove::<CobwebSlow>();
            info!("🕸️ Entity left cobweb - removing slow effect");
        }
    }
}

/// System that applies the cobweb slow effect to entity movement
pub fn apply_cobweb_slow(
    mut query: Query<
        (&mut LinearVelocity, &CobwebSlow),
        Or<(With<Hero>, With<Villager>, With<Enemy>)>,
    >,
) {
    for (mut velocity, cobweb_slow) in query.iter_mut() {
        // Apply slow multiplier to horizontal velocity
        // This creates friction that fights against movement input
        velocity.x = (velocity.x * cobweb_slow.slow_multiplier).clamp(-30., 30.);

        // Prevent velocity from decaying TOO much - maintain minimum collision detection speed
        // This prevents phasing through blocks when velocity gets near-zero
        if velocity.x.abs() > 0.1 && velocity.x.abs() < COBWEB_MIN_HORIZONTAL_SPEED {
            // Clamp to minimum speed in the direction of movement
            velocity.x = velocity.x.signum() * COBWEB_MIN_HORIZONTAL_SPEED;
        } else if velocity.x.abs() <= 0.1 {
            // Complete stop - no minimum needed
            velocity.x = 0.0;
        }

        // Clamp falling speed (like Minecraft) - prevents sinking through blocks
        // Only slow falling, don't slow jumping up
        if velocity.y < 0.0 {
            velocity.y = velocity.y.max(COBWEB_MAX_FALL_SPEED);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn web_works_at_different_render_depth_and_with_an_existing_poison_buff() {
        let mut app = App::new();
        app.add_systems(Update, (cobweb_collision_system, apply_cobweb_slow).chain());
        let hero = app
            .world_mut()
            .spawn((
                Hero {
                    move_direction: 1.,
                    direction_change_timer: Timer::default(),
                },
                Transform::from_xyz(0., 0., 90.),
                LinearVelocity(Vec3::new(250., -400., 0.)),
                crate::components::buff::Buff::new(crate::components::buff::BuffType::Poisoned, 4.),
            ))
            .id();
        let web = app
            .world_mut()
            .spawn((CobwebBlock, Transform::from_xyz(0., 0., -10.)))
            .id();
        app.update();
        let v = app.world().get::<LinearVelocity>(hero).unwrap();
        assert!(v.x <= 30. && v.y >= -20.);
        assert_eq!(
            app.world()
                .get::<crate::components::buff::Buff>(hero)
                .unwrap()
                .buff_type,
            crate::components::buff::BuffType::Poisoned
        );
        app.world_mut().despawn(web);
        app.update();
        assert!(app.world().get::<CobwebSlow>(hero).is_none());
    }
}
