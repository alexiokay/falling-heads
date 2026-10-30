use super::*;

pub(super) fn hero_ai_movement(
    time: Res<Time>,
    mut hero_query: Query<
        (
            &mut Hero,
            &mut LinearVelocity,
            Option<&crate::components::power_effect::SpeedModifier>,
            Option<&crate::components::buff::Buff>,
            Option<&crate::components::frozen::FrozenStacks>,
            Option<&crate::components::buff::GrowthPower>,
            Option<&SurfaceGrip>,
        ),
        Without<crate::systems::launcher::LauncherPassenger>,
    >,
) {
    for (mut hero, mut velocity, speed_mod, buff_opt, frozen, growth, grip) in hero_query.iter_mut()
    {
        // Skip AI movement if fully frozen
        if let Some(frozen_stacks) = frozen {
            if frozen_stacks.is_fully_frozen() {
                continue;
            }
        }

        // Tick direction change timer
        hero.direction_change_timer.tick(time.delta());

        // Calculate total speed multiplier from both SpeedModifier (powers) and Buff
        let mut speed_multiplier = 1.0;

        // Apply SpeedModifier from powers (e.g., !FAST command)
        if let Some(modifier) = speed_mod {
            speed_multiplier *= modifier.multiplier;
        }

        // Apply Buff velocity multiplier (e.g., Grow, Speed, Slowness buffs)
        if let Some(buff) = buff_opt {
            use crate::components::buff::BuffBehavior;
            speed_multiplier *= buff.buff_type.velocity_multiplier();
        }

        if growth.is_some() && buff_opt.is_none_or(|b| b.buff_type != BuffType::Grow) {
            speed_multiplier *= 1.3;
        }
        let has_turbo = speed_multiplier > 1.0;

        // Only change direction when velocity is VERY LOW (nearly stationary)
        // Don't change direction when character has momentum from physics/player
        if hero.direction_change_timer.just_finished() && velocity.x.abs() < 50.0 {
            hero.move_direction = -hero.move_direction; // Flip direction
        }

        // Only apply AI movement when:
        // 1. Nearly stationary (velocity < 50), OR
        // 2. Has turbo active (speed boost allows AI to help even when moving)
        if velocity.x.abs() < 50.0 || has_turbo {
            let move_speed = VILLAGER_MOVE_SPEED * speed_multiplier;
            let max_speed = VILLAGER_MAX_SPEED * speed_multiplier;

            velocity.x += hero.move_direction
                * move_speed
                * time.delta_secs()
                * grip.map_or(1., |g| {
                    g.steering(velocity.x, hero.move_direction, time.elapsed_secs())
                });

            // Clamp to AI max speed (adjusted for speed multipliers)
            velocity.x = velocity.x.clamp(-max_speed, max_speed);
        }
        // When velocity >= 50 and no turbo, AI doesn't touch it - full physics/player control
    }
}

/// Villager AI - walk back and forth with periodic direction changes
pub(super) fn villager_ai_movement(
    time: Res<Time>,
    mut villager_query: Query<
        (
            &mut Villager,
            &mut LinearVelocity,
            Option<&crate::components::frozen::FrozenStacks>,
            Option<&SurfaceGrip>,
            Option<&Buff>,
        ),
        Without<crate::systems::launcher::LauncherPassenger>,
    >,
) {
    for (mut villager, mut velocity, frozen, grip, buff) in villager_query.iter_mut() {
        let speed = buff_speed_multiplier(buff);
        // Skip AI movement if fully frozen
        if let Some(frozen_stacks) = frozen {
            if frozen_stacks.is_fully_frozen() {
                continue;
            }
        }

        // Tick direction change timer
        villager.direction_change_timer.tick(time.delta());

        // Only change direction when velocity is LOW (not bouncing from walls/collisions)
        if villager.direction_change_timer.just_finished() && velocity.x.abs() < 250.0 {
            villager.move_direction = -villager.move_direction; // Flip direction
        }

        // Only apply AI movement when velocity is low - don't fight against bounces
        if velocity.x.abs() < 300.0 {
            velocity.x += villager.move_direction
                * VILLAGER_MOVE_SPEED
                * speed
                * time.delta_secs()
                * grip.map_or(1., |g| {
                    g.steering(velocity.x, villager.move_direction, time.elapsed_secs())
                });

            // Clamp to AI max speed
            velocity.x = velocity
                .x
                .clamp(-VILLAGER_MAX_SPEED * speed, VILLAGER_MAX_SPEED * speed);
        }
        // When velocity > 300 (bouncing), AI doesn't touch it - full physics control
    }
}

/// Enemy AI - chase nearest hero or villager (melee enemies) OR keep distance (ranged skeletons)
pub(super) fn enemy_ai_movement(
    time: Res<Time>,
    mut enemy_query: Query<
        (
            &Transform,
            &mut LinearVelocity,
            &Enemy,
            Option<&SurfaceGrip>,
            Option<&Buff>,
        ),
        (
            Without<crate::systems::biome_encounters::EncounterFoe>,
            Without<crate::systems::spiders::Spider>,
            Without<crate::systems::bats::Bat>,
            Without<crate::systems::biome_content::BiomeFoe>,
            Without<crate::systems::biome_encounters::Stunned>,
        ),
    >,
    targets: Res<crate::systems::target_index::ActorTargets>,
) {
    for (enemy_transform, mut velocity, enemy, grip, buff) in enemy_query.iter_mut() {
        let speed = buff_speed_multiplier(buff);
        let max_speed = 400.0 * speed;
        let enemy_pos = enemy_transform.translation;

        if let Some(target) = targets.nearest(enemy_pos, f32::INFINITY) {
            let target_x = target.position.x;
            let nearest_distance = enemy_pos.distance(target.position);
            // SKELETON AI: Keep distance, never melee
            if enemy.enemy_type == crate::components::enemy::EnemyType::Skeleton {
                // Skeletons override physics when maintaining range (tactical positioning)
                if nearest_distance < SKELETON_MIN_RANGE {
                    // Too close - back away!
                    let direction = if target_x > enemy_pos.x { -1.0 } else { 1.0 }; // Move AWAY
                    velocity.x += direction
                        * VILLAGER_MOVE_SPEED
                        * speed
                        * 1.5
                        * time.delta_secs()
                        * grip.map_or(1., |g| {
                            g.steering(velocity.x, direction, time.elapsed_secs())
                        }); // Faster retreat
                    velocity.x = velocity.x.clamp(-max_speed, max_speed);
                } else if nearest_distance > SKELETON_MAX_RANGE {
                    // Too far - move closer (but not too much)
                    let direction = if target_x > enemy_pos.x { 1.0 } else { -1.0 }; // Move TOWARDS
                    velocity.x += direction
                        * VILLAGER_MOVE_SPEED
                        * speed
                        * 0.5
                        * time.delta_secs()
                        * grip.map_or(1., |g| {
                            g.steering(velocity.x, direction, time.elapsed_secs())
                        }); // Slow approach
                    velocity.x = velocity.x.clamp(-max_speed, max_speed);
                }
                // If in ideal range (300-600), don't move horizontally - just shoot
            } else {
                // MELEE ENEMIES (Pigman, etc.): Chase normally - blend with collision physics
                let direction = if target_x > enemy_pos.x { 1.0 } else { -1.0 };
                velocity.x += direction
                    * VILLAGER_MOVE_SPEED
                    * speed
                    * time.delta_secs()
                    * grip.map_or(1., |g| {
                        g.steering(velocity.x, direction, time.elapsed_secs())
                    });

                // Only clamp if moving in chase direction (don't kill collision bounces)
                if (direction > 0.0 && velocity.x > 0.0) || (direction < 0.0 && velocity.x < 0.0) {
                    velocity.x = velocity.x.clamp(-max_speed, max_speed);
                }
            }
        }
    }
}

pub(super) fn clamp_entity_velocities(
    mut query: Query<&mut LinearVelocity, Or<(With<Hero>, With<Villager>, With<Enemy>)>>,
) {
    for mut velocity in query.iter_mut() {
        // Clamp horizontal velocity
        velocity.x = velocity
            .x
            .clamp(-MAX_ENTITY_HORIZONTAL_SPEED, MAX_ENTITY_HORIZONTAL_SPEED);

        // Clamp vertical velocity
        velocity.y = velocity
            .y
            .clamp(-MAX_ENTITY_VERTICAL_SPEED, MAX_ENTITY_VERTICAL_SPEED);

        // Z should always be near zero (2D game)
        velocity.z = velocity.z.clamp(-10.0, 10.0);
    }
}

/// Walking-speed factor from an actor's buff (Speed, Slowness, Freezing, Grow).
/// Scales AI acceleration and its speed cap, never the stored velocity, so it
/// cannot compound from frame to frame.
fn buff_speed_multiplier(buff: Option<&Buff>) -> f32 {
    use crate::components::buff::BuffBehavior;
    buff.map_or(1.0, |b| b.buff_type.velocity_multiplier())
}
