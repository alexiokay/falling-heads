fn embedding_depth(seed: u64, block: bool, tip: f32) -> f32 {
    let hash = seed.wrapping_mul(0x9e3779b97f4a7c15);
    let t = ((hash ^ (hash >> 32)) & 1023) as f32 / 1023.;
    let (min, max) = if block { (3., 8.) } else { (7., 13.) };
    (min + (max - min) * t).min(tip * 0.65)
}
use crate::components::block::Block;
use crate::components::enemy::Enemy;
use crate::components::hero::Hero;
use crate::components::projectile::{Arrow, Fireball, StuckArrow};
use crate::components::villager::Villager;
use crate::events::damage_events::EntityDamaged;
use crate::resources::spatial_hash::SpatialHash;
use crate::utils::constants::{ARROW_IMPACT_LIFT, ARROW_IMPACT_PUSH};
use avian3d::prelude::*;
use bevy::prelude::*;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_arrow_lifetime, update_fireball_lifetime)
                .run_if(in_state(crate::states::InGame)),
        )
        // Run arrow systems in PostUpdate
        .add_systems(
            PostUpdate,
            (
                check_arrow_collisions,    // Manual collision detection for arrow game logic
                check_fireball_collisions, // Fireball collision detection
                update_arrow_rotation,
                update_stuck_arrows, // Run AFTER physics updates to get latest entity positions
            )
                .chain()
                .before(TransformSystems::Propagate)
                .run_if(in_state(crate::states::InGame)),
        );

        info!("ProjectilePlugin loaded");
    }
}

// REMOVED: apply_arrow_gravity - Now using Avian's GravityScale instead of manual gravity
// REMOVED: apply_arrow_velocity - Avian handles velocity integration automatically

/// Update arrow rotation to match velocity direction (realistic arrow physics)
fn update_arrow_rotation(mut query: Query<(&LinearVelocity, &mut Transform), With<Arrow>>) {
    for (velocity, mut transform) in query.iter_mut() {
        // Calculate angle from velocity direction
        // atan2(y, x) gives the angle where the velocity is pointing
        let angle = velocity.y.atan2(velocity.x);

        // Rotate arrow to point in direction of travel
        // Arrow texture points right (→), so no offset needed
        transform.rotation = Quat::from_rotation_z(angle);
    }
}

/// Update arrow lifetime and despawn when expired
fn update_arrow_lifetime(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Arrow), Without<StuckArrow>>,
    time: Res<Time>,
) {
    for (entity, mut arrow) in query.iter_mut() {
        arrow.lifetime.tick(time.delta());

        if arrow.lifetime.is_finished() {
            info!("ARROW DESPAWNED BY LIFETIME: entity={:?}", entity);
            commands.entity(entity).despawn();
        }
    }
}

/// Update arrows that are stuck to entities - make them follow the entity
fn update_stuck_arrows(
    mut commands: Commands,
    mut stuck_query: Query<(Entity, &mut StuckArrow), With<Arrow>>,
    entity_query: Query<Entity>,
    time: Res<Time>,
) {
    for (arrow_entity, mut stuck) in stuck_query.iter_mut() {
        stuck.despawn_timer.tick(time.delta());

        if stuck.despawn_timer.is_finished() {
            // Timer expired - despawn the stuck arrow
            commands.queue(move |world: &mut World| {
                if let Ok(entity) = world.get_entity_mut(arrow_entity) {
                    entity.despawn();
                }
            });
            continue;
        }

        // Check if the entity still exists
        if entity_query.get(stuck.stuck_to_entity).is_err() {
            // Entity doesn't exist anymore (died/despawned) - despawn arrow
            commands.queue(move |world: &mut World| {
                if let Ok(entity) = world.get_entity_mut(arrow_entity) {
                    entity.despawn();
                }
            });
        }
        // Arrow position is now automatically updated by Bevy's transform hierarchy (it's a child)
    }
}

/// Segment versus 2D box, returning the earliest impact fraction (including corners).
pub(crate) fn segment_box(from: Vec2, to: Vec2, center: Vec2, half: Vec2) -> Option<f32> {
    let direction = to - from;
    let mut enter = 0.0_f32;
    let mut leave = 1.0_f32;
    for axis in 0..2 {
        let lo = center[axis] - half[axis];
        let hi = center[axis] + half[axis];
        if direction[axis].abs() < 0.00001 {
            if from[axis] < lo || from[axis] > hi {
                return None;
            }
        } else {
            let a = (lo - from[axis]) / direction[axis];
            let b = (hi - from[axis]) / direction[axis];
            enter = enter.max(a.min(b));
            leave = leave.min(a.max(b));
            if enter > leave {
                return None;
            }
        }
    }
    Some(enter)
}

/// One attachment path for terrain and actors. Parent scale/rotation are inverted,
/// so a 64x terrain mesh cannot enlarge or offset its embedded arrow.
fn lodge_arrow(
    commands: &mut Commands,
    arrow: Entity,
    target: Entity,
    target_transform: &Transform,
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
    seconds: f32,
) {
    let world = Transform {
        translation: position,
        rotation,
        scale,
    };
    let local = Transform::from_matrix(Mat4::from(
        target_transform.compute_affine().inverse() * world.compute_affine(),
    ));
    // A laser/death system can remove either entity before deferred commands flush.
    // Validate and attach atomically, rather than enqueueing commands for a stale ID.
    commands.queue(move |world: &mut World| {
        if world.get_entity(target).is_err() {
            if let Ok(entity) = world.get_entity_mut(arrow) {
                entity.despawn();
            }
            return;
        }
        let Ok(mut entity) = world.get_entity_mut(arrow) else {
            return;
        };
        entity.remove::<(LinearVelocity, RigidBody, Collider, Sensor, AngularVelocity)>();
        entity.insert((
            ChildOf(target),
            local,
            StuckArrow {
                despawn_timer: Timer::from_seconds(seconds, TimerMode::Once),
                stuck_to_entity: target,
            },
        ));
    });
}

/// Sweep the arrow tip from the previous frame and select the FIRST impact.
/// Blocks occlude targets; fast projectiles cannot skip a thin wall.
fn check_arrow_collisions(
    mut commands: Commands,
    spatial_hash: Res<SpatialHash>,
    arrows: Query<(Entity, &Transform, &Arrow, &LinearVelocity), Without<StuckArrow>>,
    blocks: Query<
        (
            &Transform,
            &crate::components::block::BlockType,
            Option<&crate::systems::biome_content::Tile>,
        ),
        With<Block>,
    >,
    mut actors: Query<
        (
            Entity,
            &Transform,
            &mut LinearVelocity,
            Option<&Hero>,
            Option<&Villager>,
            Option<&Enemy>,
        ),
        (
            Without<Arrow>,
            Or<(With<Hero>, With<Villager>, With<Enemy>)>,
            Without<crate::components::damage_visual::MarkedForDeath>,
            Without<crate::components::damage_visual::DeathAnimation>,
        ),
    >,
    dragon_parts: Query<(&super::dragon_flyover::DragonHitPart, &GlobalTransform)>,
    mut damage: MessageWriter<EntityDamaged>,
    time: Res<Time>,
    mut previous: Local<std::collections::HashMap<Entity, Vec2>>,
) {
    previous.retain(|entity, _| arrows.contains(*entity));
    for (entity, transform, arrow, velocity) in &arrows {
        let direction = velocity.0.xy().normalize_or_zero();
        // arrow2.glb has local X bounds [-1.6, 1.6] after its node transforms.
        let tip_length = 1.6 * transform.scale.x.abs();
        let tip = transform.translation.xy() + direction * tip_length;
        let from = previous
            .insert(entity, tip)
            .unwrap_or(tip - velocity.0.xy() * time.delta_secs());
        let mut hit: Option<(f32, Entity, Transform, bool)> = None;
        let mut nearby = std::collections::HashSet::new();
        let steps = ((tip - from).length() / 64.0).ceil().max(1.0) as usize;
        for step in 0..=steps {
            let p = from.lerp(tip, step as f32 / steps as f32);
            nearby.extend(spatial_hash.query_nearby(p.x, p.y));
        }
        for target in nearby {
            let Ok((t, kind, tile)) = blocks.get(target) else {
                continue;
            };
            if *kind == crate::components::block::BlockType::Cobweb
                || crate::systems::plant_rules::soft(*kind, tile)
            {
                continue;
            }
            if let Some(fraction) =
                super::terrain_shape::hit(from, tip, t.translation.xy(), *kind, tile)
            {
                if hit.as_ref().is_none_or(|h| fraction < h.0) {
                    hit = Some((fraction, target, *t, true));
                }
            }
        }
        for (target, t, _, hero, villager, enemy) in &actors {
            let friendly = hero.is_some() || villager.is_some();
            if (friendly && arrow.team == crate::components::projectile::Team::Friendly)
                || (enemy.is_some() && arrow.team == crate::components::projectile::Team::Enemy)
            {
                continue;
            }
            let fraction = if enemy
                .is_some_and(|e| e.enemy_type == crate::components::enemy::EnemyType::Dragon)
            {
                dragon_parts
                    .iter()
                    .filter(|(part, _)| part.owner == target)
                    .filter_map(|(part, gt)| part.hit(gt, from, tip))
                    .min_by(f32::total_cmp)
            } else {
                segment_box(
                    from,
                    tip,
                    t.translation.xy(),
                    Vec2::splat(32.0 * t.scale.x.abs()),
                )
            };
            if let Some(fraction) = fraction {
                if hit.as_ref().is_none_or(|h| fraction < h.0) {
                    hit = Some((fraction, target, *t, false));
                }
            }
        }
        if let Some((fraction, target, target_transform, is_block)) = hit {
            let mut surface = from.lerp(tip, fraction);
            if is_block {
                // Recover the same silhouette surface, never a square fallback.
                let behind = surface - direction * 256.;
                let (_, kind, tile) = blocks.get(target).unwrap();
                if let Some(entry) = super::terrain_shape::hit(
                    behind,
                    surface + direction * 0.01,
                    target_transform.translation.xy(),
                    *kind,
                    tile,
                ) {
                    surface = behind.lerp(surface + direction * 0.01, entry);
                }
            }
            let penetration = embedding_depth(entity.to_bits(), is_block, tip_length);
            let point = surface - direction * (tip_length - penetration);
            let position = Vec3::new(point.x, point.y, transform.translation.z);
            lodge_arrow(
                &mut commands,
                entity,
                target,
                &target_transform,
                position,
                Quat::from_rotation_z(direction.y.atan2(direction.x)),
                transform.scale,
                if is_block { 8.0 } else { 2.0 },
            );
            if !is_block {
                damage.write(EntityDamaged {
                    entity: target,
                    damage: arrow.damage,
                });
                if let Ok((_, _, mut target_velocity, _, _, _)) = actors.get_mut(target) {
                    target_velocity.x += direction.x * ARROW_IMPACT_PUSH;
                    target_velocity.y += direction.y * ARROW_IMPACT_LIFT;
                }
            }
        }
    }
}

#[cfg(test)]
mod arrow_tests {
    use super::*;

    #[test]
    fn attachment_handles_parent_or_arrow_removed_before_command_flush() {
        for remove_parent in [true, false] {
            let mut world = World::new();
            let target = world.spawn(Transform::IDENTITY).id();
            let arrow = world.spawn(Transform::IDENTITY).id();
            let mut queue = bevy::ecs::world::CommandQueue::default();
            {
                let mut commands = Commands::new(&mut queue, &world);
                lodge_arrow(
                    &mut commands,
                    arrow,
                    target,
                    &Transform::IDENTITY,
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    Vec3::ONE,
                    8.,
                );
            }
            world
                .entity_mut(if remove_parent { target } else { arrow })
                .despawn();
            queue.apply(&mut world);
            assert!(world.get_entity(arrow).is_err());
        }
    }
    #[test]
    fn actual_arrow_system_lodges_fast_arrow_in_the_first_wall() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(SpatialHash::new(256.));
        app.add_message::<EntityDamaged>()
            .add_systems(Update, check_arrow_collisions);
        let wall = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                crate::components::block::BlockType::Stone,
                Transform::from_scale(Vec3::splat(64.)),
            ))
            .id();
        app.world_mut()
            .resource_mut::<SpatialHash>()
            .insert(wall, 0., 0.);
        let arrow = app
            .world_mut()
            .spawn((
                Arrow::new(1),
                Transform::from_xyz(-120., 0., 10.).with_scale(Vec3::splat(15.)),
                LinearVelocity(Vec3::X * 1200.),
            ))
            .id();
        app.update();
        app.world_mut()
            .get_mut::<Transform>(arrow)
            .unwrap()
            .translation
            .x = 120.;
        app.update();
        assert_eq!(
            app.world()
                .get::<StuckArrow>(arrow)
                .unwrap()
                .stuck_to_entity,
            wall
        );
        assert!(app.world().get::<LinearVelocity>(arrow).is_none());
        assert!(
            (3.0..=8.0).contains(
                &(app.world().get::<Transform>(arrow).unwrap().translation.x * 64. + 56.)
            )
        );
    }
    #[test]
    fn friendly_arrow_hits_articulated_dragon_head_outside_old_root_hitbox() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(SpatialHash::new(256.))
            .add_message::<EntityDamaged>()
            .add_systems(Update, check_arrow_collisions);
        let dragon = app
            .world_mut()
            .spawn((
                Enemy::new(crate::components::enemy::EnemyType::Dragon),
                Transform::IDENTITY,
                LinearVelocity::ZERO,
            ))
            .id();
        app.world_mut().spawn((
            super::super::dragon_flyover::DragonHitPart {
                owner: dragon,
                half: Vec2::splat(0.5),
            },
            GlobalTransform::from(
                Transform::from_xyz(-83., 10., 90.).with_scale(Vec3::new(62., 39., 40.)),
            ),
        ));
        let arrow = app
            .world_mut()
            .spawn((
                Arrow::new_friendly(2),
                Transform::from_xyz(-160., 10., 10.).with_scale(Vec3::splat(4.)),
                LinearVelocity(Vec3::X * 300.),
            ))
            .id();
        app.update();
        app.world_mut()
            .get_mut::<Transform>(arrow)
            .unwrap()
            .translation
            .x = -90.;
        app.update();
        assert_eq!(
            app.world()
                .get::<StuckArrow>(arrow)
                .unwrap()
                .stuck_to_entity,
            dragon
        );
        let hits: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<EntityDamaged>>()
            .drain()
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, dragon);
        assert_eq!(hits[0].damage, 2);
    }
    #[test]
    fn overlapping_arrows_recover_surface_with_bounded_varied_penetration() {
        for scale in [7.5, 15., 30.] {
            let mut app = App::new();
            app.insert_resource(Time::<()>::default())
                .insert_resource(SpatialHash::new(256.))
                .add_message::<EntityDamaged>()
                .add_systems(Update, check_arrow_collisions);
            let wall = app
                .world_mut()
                .spawn((
                    Block {
                        health: 3,
                        max_health: 3,
                    },
                    crate::components::block::BlockType::Stone,
                    Transform::from_scale(Vec3::splat(64.)),
                ))
                .id();
            app.world_mut()
                .resource_mut::<SpatialHash>()
                .insert(wall, 0., 0.);
            // The tip begins inside the wall, so the sweep alone would report t=0.
            let arrow = app
                .world_mut()
                .spawn((
                    Arrow::new(1),
                    Transform::from_xyz(-1.6 * scale, 0., 10.).with_scale(Vec3::splat(scale)),
                    LinearVelocity(Vec3::X * 100.),
                ))
                .id();
            app.update();
            let t = app.world().get::<Transform>(arrow).unwrap();
            let world_tip = t.translation.x * 64. + 1.6 * scale;
            let depth = world_tip + 32.;
            assert!(depth >= 3. && depth <= 8. && depth <= 1.6 * scale * 0.65 + 0.01);
            assert_eq!(app.world().get::<Block>(wall).unwrap().health, 3);
            assert!(app.world().resource::<Messages<EntityDamaged>>().is_empty());
        }
    }
    #[test]
    fn swept_tip_hits_thin_wall_and_corners_without_false_near_misses() {
        assert_eq!(
            segment_box(
                Vec2::new(-200., 0.),
                Vec2::new(200., 0.),
                Vec2::ZERO,
                Vec2::splat(32.)
            ),
            Some(0.42)
        );
        assert!(
            segment_box(
                Vec2::new(-100., 31.),
                Vec2::new(100., 31.),
                Vec2::ZERO,
                Vec2::splat(32.)
            )
            .is_some()
        );
        assert!(
            segment_box(
                Vec2::new(-100., 34.),
                Vec2::new(100., 34.),
                Vec2::ZERO,
                Vec2::splat(32.)
            )
            .is_none()
        );
    }
    #[test]
    fn embedded_arrow_preserves_world_size_follows_block_and_expires() {
        let mut app = App::new();
        app.add_plugins(bevy::transform::TransformPlugin);
        app.insert_resource(Time::<()>::default())
            .add_systems(Update, update_stuck_arrows);
        let block_transform = Transform::from_xyz(100., 200., -10.).with_scale(Vec3::splat(64.));
        let block = app.world_mut().spawn(block_transform).id();
        let arrow = app
            .world_mut()
            .spawn((Arrow::new(1), Transform::IDENTITY, LinearVelocity::ZERO))
            .id();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, app.world());
            lodge_arrow(
                &mut commands,
                arrow,
                block,
                &block_transform,
                Vec3::new(54., 200., 10.),
                Quat::IDENTITY,
                Vec3::splat(15.),
                8.,
            );
        }
        queue.apply(app.world_mut());
        app.update();
        let t = app.world().get::<GlobalTransform>(arrow).unwrap();
        assert!(t.translation().distance(Vec3::new(54., 200., 10.)) < 0.001);
        assert!((t.compute_transform().scale.x - 15.).abs() < 0.001);
        assert!(app.world().get::<LinearVelocity>(arrow).is_none());
        app.world_mut()
            .get_mut::<Transform>(block)
            .unwrap()
            .translation
            .x += 100.;
        app.update();
        assert!(
            (app.world()
                .get::<GlobalTransform>(arrow)
                .unwrap()
                .translation()
                .x
                - 154.)
                .abs()
                < 0.001
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(9));
        app.update();
        assert!(app.world().get_entity(arrow).is_err());
    }
}

/// Update fireball lifetime and despawn when expired
fn update_fireball_lifetime(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Fireball)>,
    time: Res<Time>,
) {
    for (entity, mut fireball) in query.iter_mut() {
        fireball.lifetime.tick(time.delta());

        if fireball.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Spawn explosion particles at the given position (fewer particles than TNT)
fn spawn_fireball_explosion(
    commands: &mut Commands,
    position: Vec3,
    explosion_textures: &Option<Res<crate::resources::asset_handles::ExplosionTextures>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    if let Some(textures) = explosion_textures.as_ref() {
        if !textures.frames.is_empty() {
            use rand::RngExt;
            let mut rng = rand::rng();

            // Spawn 8-10 explosion particles (bigger explosion)
            let particle_count = rng.random_range(8..=10);
            for _i in 0..particle_count {
                // Random offset from explosion center - larger area
                let max_offset = 60.0;
                let offset_x = rng.random_range(-max_offset..max_offset);
                let offset_y = rng.random_range(-max_offset..max_offset);

                // Staggered delays
                let delay_ms: f32 = rng.random_range(0.0..0.4);

                // Bigger size
                let size = rng.random_range(70.0..120.0);

                // Random Z position for depth
                let z_offset = rng.random_range(15.0..25.0);

                commands.spawn((
                    crate::components::explosive::ExplosionAnimation {
                        timer: Timer::from_seconds(1.0 / 60.0, TimerMode::Repeating), // One frame per 1/60 s at any FPS
                        current_frame: 0,
                        total_frames: 24,
                        delay_timer: Timer::from_seconds(delay_ms, TimerMode::Once),
                        started: false,
                    },
                    Mesh3d(meshes.add(Rectangle::new(size, size))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        base_color_texture: Some(textures.frames[0].clone()),
                        unlit: true,
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    })),
                    Transform::from_translation(Vec3::new(
                        position.x + offset_x,
                        position.y + offset_y,
                        z_offset,
                    )),
                    Visibility::Hidden,
                ));
            }
        }
    }
}

/// Check fireball collisions with blocks, enemies, heroes, and villagers
#[allow(clippy::too_many_arguments)] // Bevy system parameters
fn check_fireball_collisions(
    mut heat: MessageWriter<super::dragon_flyover::TerrainHeat>,
    mut destroyed: MessageWriter<crate::events::block_events::BlockDestroyed>,
    mut broken: Local<std::collections::HashSet<Entity>>,
    fluids: Res<super::fluids::FluidWorld>,
    mut commands: Commands,
    spatial_hash: Res<SpatialHash>,
    mut fireball_query: Query<(Entity, &mut Transform, &Fireball, &LinearVelocity), Without<Block>>,
    block_query: Query<
        (
            Entity,
            &Transform,
            &crate::components::block::BlockType,
            Option<&crate::systems::biome_content::Tile>,
        ),
        With<Block>,
    >,
    mut entities: ParamSet<(
        Query<(Entity, &Transform, &mut LinearVelocity), (With<Hero>, Without<Fireball>)>,
        Query<(Entity, &Transform, &mut LinearVelocity), (With<Villager>, Without<Fireball>)>,
        Query<
            (Entity, &Transform, &mut LinearVelocity),
            (
                With<Enemy>,
                Without<Fireball>,
                Without<crate::components::damage_visual::DeathAnimation>,
                Without<crate::components::damage_visual::MarkedForDeath>,
            ),
        >,
    )>,
    mut damage_events: MessageWriter<EntityDamaged>,
    explosion_textures: Option<Res<crate::resources::asset_handles::ExplosionTextures>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const FIREBALL_HIT_RADIUS: f32 = 64.0; // Increased to match character size (was 20.0, too small)
    const FIREBALL_PUSH_FORCE: f32 = 250.0;
    const FIREBALL_EXPLOSION_RADIUS: f32 = 100.0; // Area damage radius
    broken.clear();

    for (fireball_entity, fireball_transform, fireball, _fireball_velocity) in
        fireball_query.iter_mut()
    {
        let fireball_pos = fireball_transform.translation;
        let mut should_explode = fluids.is_water_at(fireball_pos.xy());

        // Check collision with blocks (always explodes on blocks regardless of team)
        let nearby_blocks = spatial_hash.query_nearby(fireball_pos.x, fireball_pos.y);
        for block_entity in nearby_blocks {
            if let Ok((_, block_transform, kind, tile)) = block_query.get(block_entity) {
                if crate::systems::plant_rules::soft(*kind, tile) {
                    continue;
                }
                let distance = fireball_pos.distance(block_transform.translation);
                if distance < 64.0 {
                    // Match character hit radius
                    should_explode = true;
                    break;
                }
            }
        }

        // Check collision with heroes - only if enemy fireball (friendly fire prevention)
        if !should_explode && fireball.team == crate::components::projectile::Team::Enemy {
            for (_hero_entity, hero_transform, _) in entities.p0().iter() {
                if fireball_pos.distance(hero_transform.translation) < FIREBALL_HIT_RADIUS {
                    should_explode = true;
                    break;
                }
            }
        }

        // Check collision with villagers - only if enemy fireball (friendly fire prevention)
        if !should_explode && fireball.team == crate::components::projectile::Team::Enemy {
            for (_, villager_transform, _) in entities.p1().iter() {
                if fireball_pos.distance(villager_transform.translation) < FIREBALL_HIT_RADIUS {
                    should_explode = true;
                    break;
                }
            }
        }

        // Check collision with enemies - only if friendly fireball (from reflected fireballs)
        if !should_explode && fireball.team == crate::components::projectile::Team::Friendly {
            for (_, enemy_transform, _) in entities.p2().iter() {
                if fireball_pos.distance(enemy_transform.translation) < FIREBALL_HIT_RADIUS {
                    should_explode = true;
                    break;
                }
            }
        }

        // If we hit something, explode with area damage
        if should_explode {
            heat.write(super::dragon_flyover::TerrainHeat {
                position: fireball_pos.xy(),
                radius: 100.,
            });
            spawn_fireball_explosion(
                &mut commands,
                fireball_pos,
                &explosion_textures,
                &mut meshes,
                &mut materials,
            );

            // Destroy blocks in explosion radius (spatial hash, not every block in the world)
            let reach = Vec2::splat(FIREBALL_EXPLOSION_RADIUS);
            for entity in
                spatial_hash.rect_iter(fireball_pos.xy() - reach, fireball_pos.xy() + reach)
            {
                let Ok((_, block_transform, kind, tile)) = block_query.get(entity) else {
                    continue;
                };
                if fireball_pos.distance(block_transform.translation) < FIREBALL_EXPLOSION_RADIUS {
                    crate::plugins::gameplay_plugin::break_block(
                        &mut commands,
                        &mut destroyed,
                        &mut broken,
                        entity,
                        *kind,
                        block_transform.translation,
                        tile.copied(),
                    );
                }
            }

            // Damage and push all heroes in radius - only if enemy fireball
            if fireball.team == crate::components::projectile::Team::Enemy {
                for (hero_entity, hero_transform, mut hero_velocity) in entities.p0().iter_mut() {
                    let distance = fireball_pos.distance(hero_transform.translation);
                    if distance < FIREBALL_EXPLOSION_RADIUS {
                        damage_events.write(EntityDamaged {
                            entity: hero_entity,
                            damage: 2,
                        });
                        commands
                            .entity(hero_entity)
                            .insert(crate::components::buff::Buff::new(
                                crate::components::buff::BuffType::Burning,
                                5.0,
                            ));
                        let push_direction =
                            (hero_transform.translation - fireball_pos).normalize_or_zero();
                        hero_velocity.x += push_direction.x * FIREBALL_PUSH_FORCE;
                        hero_velocity.y += push_direction.y * FIREBALL_PUSH_FORCE;
                    }
                }
            }

            // Damage and push all villagers in radius - only if enemy fireball
            if fireball.team == crate::components::projectile::Team::Enemy {
                for (villager_entity, villager_transform, mut villager_velocity) in
                    entities.p1().iter_mut()
                {
                    let distance = fireball_pos.distance(villager_transform.translation);
                    if distance < FIREBALL_EXPLOSION_RADIUS {
                        damage_events.write(EntityDamaged {
                            entity: villager_entity,
                            damage: 2,
                        });
                        commands.entity(villager_entity).insert(
                            crate::components::buff::Buff::new(
                                crate::components::buff::BuffType::Burning,
                                5.0,
                            ),
                        );
                        let push_direction =
                            (villager_transform.translation - fireball_pos).normalize_or_zero();
                        villager_velocity.x += push_direction.x * FIREBALL_PUSH_FORCE;
                        villager_velocity.y += push_direction.y * FIREBALL_PUSH_FORCE;
                    }
                }
            }

            // Damage and push all enemies in radius - only if friendly fireball (from reflected fireballs)
            if fireball.team == crate::components::projectile::Team::Friendly {
                for (enemy_entity, enemy_transform, mut enemy_velocity) in entities.p2().iter_mut()
                {
                    let distance = fireball_pos.distance(enemy_transform.translation);
                    if distance < FIREBALL_EXPLOSION_RADIUS {
                        damage_events.write(EntityDamaged {
                            entity: enemy_entity,
                            damage: 2,
                        });
                        let push_direction =
                            (enemy_transform.translation - fireball_pos).normalize_or_zero();
                        enemy_velocity.x += push_direction.x * FIREBALL_PUSH_FORCE;
                        enemy_velocity.y += push_direction.y * FIREBALL_PUSH_FORCE;
                    }
                }
            }

            commands.entity(fireball_entity).despawn();
            info!("🔥 Fireball exploded in area!");
        }
    }
}
