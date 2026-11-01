use super::*;

use super::terrain_motion;
#[derive(Resource, Default)]
pub(super) struct TerrainStepStarts(std::collections::HashMap<Entity, Vec2>);
/// Refresh on terrain mutations, never leave newly generated terrain unindexed for ten frames.
/// Updates only added, moved and removed blocks (mining no longer re-indexes the world).
pub(super) fn rebuild_spatial_hash(
    mut spatial_hash: ResMut<SpatialHash>,
    changed: Query<(Entity, &Transform), (With<Block>, Or<(Added<Block>, Changed<Transform>)>)>,
    mut removed: RemovedComponents<Block>,
) {
    for entity in removed.read() {
        spatial_hash.remove(entity);
    }
    for (entity, transform) in &changed {
        spatial_hash.insert(entity, transform.translation.x, transform.translation.y);
    }
}

/// Capture after Avian synchronizes explicit teleports, before integrating motion.
/// Resolve Position before writeback/interpolation, once for every physics step.
pub(super) fn install_terrain_contacts(app: &mut App) {
    app.init_resource::<TerrainStepStarts>()
        .add_systems(
            PhysicsSchedule,
            (
                rebuild_spatial_hash,
                expire_surface_grip,
                cancel_snow_slides,
                capture_terrain_starts,
            )
                .chain()
                .after(PhysicsStepSystems::First)
                .before(PhysicsStepSystems::BroadPhase)
                .run_if(in_state(crate::states::InGame)),
        )
        .add_systems(
            PhysicsSchedule,
            (check_all_entity_block_collisions, clamp_entity_velocities)
                .chain()
                .in_set(PhysicsStepSystems::Last)
                .run_if(in_state(crate::states::InGame)),
        );
}

pub(super) fn expire_surface_grip(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    actors: Query<(Entity, &SurfaceGrip)>,
) {
    for (entity, grip) in &actors {
        if time.elapsed_secs() >= grip.until {
            commands.entity(entity).remove::<SurfaceGrip>();
        }
    }
}

/// A plain floor landing ends snow/ice grip, unless the ICEAGE power keeps every floor icy.
fn clear_grip(commands: &mut Commands, entity: Entity, ice_age_on: bool, now: f32) {
    if ice_age_on {
        commands.entity(entity).insert(SurfaceGrip::ice(now));
    } else {
        commands.entity(entity).remove::<SurfaceGrip>();
    }
}

pub(super) fn cancel_snow_slides(
    mut commands: Commands,
    actors: Query<
        (
            Entity,
            Has<crate::components::enemy::Flying>,
            Has<crate::components::damage_visual::DeathAnimation>,
            Has<crate::systems::launcher::LauncherPassenger>,
            Option<&crate::components::frozen::FrozenStacks>,
        ),
        With<SnowSlide>,
    >,
) {
    for (entity, flying, dead, passenger, frozen) in &actors {
        if flying || dead || passenger || frozen.is_some_and(|f| f.is_fully_frozen()) {
            commands.entity(entity).remove::<SnowSlide>();
        }
    }
}

pub(super) fn capture_terrain_starts(
    actors: Query<
        (Entity, &Position),
        (
            Or<(With<Hero>, With<Villager>, With<Enemy>)>,
            Without<Block>,
            Without<crate::components::enemy::Flying>,
        ),
    >,
    mut starts: ResMut<TerrainStepStarts>,
) {
    starts.0.clear();
    starts.0.extend(actors.iter().map(|(e, p)| (e, p.0.xy())));
}

/// Lethal-contact decisions share one view of health, cooldown time and pending removals.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct TerrainDamageContext<'w, 's> {
    tuning: Option<Res<'w, ContactTuning>>,
    health: Query<'w, 's, &'static Block>,
    breaking: Local<'s, std::collections::HashSet<Entity>>,
    time: Res<'w, Time<Virtual>>,
    ice_age: Option<Res<'w, crate::systems::extra_powers::IceAge>>,
    hits: MessageWriter<'w, crate::events::block_events::HeadHitBlock>,
}

/// Actor/terrain collision uses authoritative positions and a swept local broad phase.
pub(super) fn check_all_entity_block_collisions(
    mut commands: Commands,
    spatial_hash: Res<SpatialHash>,
    starts: Res<TerrainStepStarts>,
    mut entity_query: Query<
        (
            Entity,
            &mut Transform,
            Option<&mut Position>,
            &mut LinearVelocity,
            &Restitution,
            Option<&Enemy>,
            Option<&SnowSlide>,
            Option<&crate::components::enemy::TerrainFootprint>,
            Option<&crate::systems::slime_physics::SlimeContact>,
            Option<&crate::systems::slime_physics::SlimeCoat>,
            Option<&crate::components::play_style::PlayStyle>,
            Has<crate::components::play_style::ClingSpent>,
            Has<crate::systems::extra_powers::Shrunk>,
        ),
        (
            Or<(
                With<Hero>,
                With<Villager>,
                With<Enemy>,
                With<crate::components::play_style::SplitTwin>,
            )>,
            Without<Block>,
            Without<crate::components::enemy::Flying>,
            Without<crate::systems::slime_rift::RiftParked>,
            Without<crate::components::play_style::TwinReturning>,
        ),
    >,
    block_query: Query<
        (
            Entity,
            &Transform,
            &BlockType,
            &ChunkMember,
            Option<&SnowLayer>,
            Option<&crate::systems::biome_content::Tile>,
        ),
        With<Block>,
    >,
    chunk_query: Query<&crate::components::chunk::Chunk>,
    mut buff_query: Query<&mut crate::components::buff::Buff>,
    mut damage_events: MessageWriter<BlockDamaged>,
    mut thorn_damage: MessageWriter<EntityDamaged>,
    mut thorns: Local<std::collections::HashMap<Entity, f32>>,
    mut block_cooldown: ResMut<DamageCooldown>,
    context: TerrainDamageContext,
    mut scratch: Local<ContactScratch>,
) {
    let TerrainDamageContext {
        tuning,
        health,
        mut breaking,
        time,
        ice_age,
        mut hits,
    } = context;
    let current_time = time.elapsed_secs();
    let ice_age_on = ice_age.is_some_and(|ice| ice.active(current_time));
    breaking.retain(|e| health.contains(*e));
    thorns.retain(|e, _| entity_query.contains(*e));
    let ContactScratch {
        candidates,
        bounds,
        contacts,
        damaged,
        biomes,
        last_free,
    } = &mut *scratch;
    last_free.retain(|e, _| entity_query.contains(*e));
    biomes.clear();
    biomes.extend(
        chunk_query
            .iter()
            .map(|c| ((c.chunk_x, c.chunk_y), c.biome)),
    );
    for (
        entity,
        mut transform,
        position,
        mut velocity,
        restitution,
        enemy,
        slide,
        footprint,
        sticky,
        coat,
        play_style,
        cling_spent,
        shrunk,
    ) in &mut entity_query
    {
        let end = position
            .as_ref()
            .map_or(transform.translation.xy(), |p| p.0.xy());
        let start = starts.0.get(&entity).copied().unwrap_or(end);
        let free = last_free
            .get(&entity)
            .copied()
            .filter(|f| f.distance(start) <= NUDGE_SWEEP_LIMIT);
        let half =
            transform.scale.xy().abs() * footprint.map_or(Vec2::splat(HERO_SIZE * 0.5), |f| f.0);
        // Query the entire swept rectangle, including actor growth and snow height.
        let padding = half + Vec2::splat(BLOCK_SIZE * 0.5 + 8.);
        let (lo, hi) = free.map_or((start.min(end), start.max(end)), |f| {
            (start.min(end).min(f), start.max(end).max(f))
        });
        candidates.clear();
        bounds.clear();
        for e in spatial_hash.rect_iter(lo - padding, hi + padding) {
            if breaking.contains(&e) {
                continue;
            }
            let Ok((_, t, kind, _, snow, tile)) = block_query.get(e) else {
                continue;
            };
            if crate::systems::plant_rules::passable(*kind, tile) {
                continue;
            }
            let (center, block_half) = block_contact_bounds(t.translation, snow);
            candidates.push(e);
            bounds.push(terrain_motion::Bounds {
                min: center.xy() - block_half.xy() - half,
                max: center.xy() + block_half.xy() + half,
            });
        }
        let start = sweep_start(start, free, bounds);
        let mut resolved = terrain_motion::resolve_into(start, end, bounds, contacts);
        if enemy.is_none() && tuning.as_ref().is_some_and(|t| t.break_through) {
            // Remove only actual reached, lethal contacts, then sweep again against
            // the remaining solids. A tougher block behind a fragile one still stops us.
            for _ in 0..candidates.len() {
                let lethal = contacts
                    .iter()
                    .find(|c| {
                        let e = candidates[c.index];
                        velocity.0.xy().dot(c.normal) < 0.
                            && health.get(e).is_ok_and(|b| b.health == 1)
                            && block_cooldown.can_damage(e, current_time, BLOCK_DAMAGE_COOLDOWN)
                            && !(c.normal.y > 0. && (slide.is_some() || sticky.is_some()))
                    })
                    .map(|c| c.index);
                let Some(index) = lethal else {
                    break;
                };
                let e = candidates.remove(index);
                bounds.remove(index);
                breaking.insert(e);
                hits.write(crate::events::block_events::HeadHitBlock {
                    head: entity,
                    block: e,
                });
                damage_events.write(BlockDamaged {
                    entity: e,
                    damage: 1,
                    remaining_health: 0,
                });
                block_cooldown.record_damage(e, current_time);
                resolved = terrain_motion::resolve_into(start, end, bounds, contacts);
            }
        }
        if coat.is_some_and(|c| current_time >= c.until) {
            commands
                .entity(entity)
                .remove::<crate::systems::slime_physics::SlimeCoat>();
        }
        if let Some(sticky) = sticky {
            let support = block_query
                .get(sticky.support)
                .ok()
                .and_then(|(_, t, _, _, snow, _)| {
                    let (center, extent) = block_contact_bounds(t.translation, snow);
                    let top = center.y + extent.y;
                    ((resolved.x - center.x).abs() < extent.x + half.x - 0.1
                        && (resolved.y - half.y - top).abs() < 6.)
                        .then_some(top)
                });
            if let Some(top) = support.filter(|_| velocity.y < 100.) {
                resolved.y = top + half.y + 1.;
                if current_time < sticky.release_at {
                    velocity.x = 0.;
                    velocity.y = 0.;
                } else {
                    velocity.x = sticky.horizontal;
                    velocity.y = sticky.rebound;
                    commands
                        .entity(entity)
                        .remove::<crate::systems::slime_physics::SlimeContact>();
                }
            } else {
                commands
                    .entity(entity)
                    .remove::<crate::systems::slime_physics::SlimeContact>();
            }
        }
        if let Some(slide) = slide {
            // Find current support after the sweep, so destroyed blocks, ledges,
            // steps and side walls cannot leave a head attached to empty space.
            let support = candidates
                .iter()
                .filter_map(|e| {
                    let (_, t, kind, _, snow, _) = block_query.get(*e).ok()?;
                    let (center, extent) = block_contact_bounds(t.translation, snow);
                    let top = center.y + extent.y;
                    let gap = resolved.y - half.y - top;
                    if gap < -0.1
                        || gap > 2.1
                        || (resolved.x - center.x).abs() >= extent.x + half.x - 0.1
                    {
                        return None;
                    }
                    let surface = if *kind == BlockType::Snow || snow.is_some_and(|s| s.stage > 0) {
                        SlideSurface::Snow
                    } else if *kind == BlockType::Ice {
                        SlideSurface::Ice
                    } else {
                        SlideSurface::Ground
                    };
                    Some((top, surface))
                })
                .max_by(|a, b| a.0.total_cmp(&b.0));
            match slide.step(
                current_time,
                support.map(|(_, surface)| surface),
                velocity.y,
            ) {
                SlideStep::Hold => {
                    resolved.y = support.unwrap().0 + half.y + 1.;
                    velocity.y = 0.;
                }
                SlideStep::Rebound(speed) => {
                    velocity.y = speed;
                    commands.entity(entity).remove::<SnowSlide>();
                    if matches!(support.map(|s| s.1), Some(SlideSurface::Snow)) {
                        commands
                            .entity(entity)
                            .insert(SurfaceGrip::snow(current_time));
                    } else if matches!(support.map(|s| s.1), Some(SlideSurface::Ice)) {
                        commands
                            .entity(entity)
                            .insert(SurfaceGrip::ice(current_time));
                    } else {
                        clear_grip(&mut commands, entity, ice_age_on, current_time);
                    }
                }
                SlideStep::Leave => {
                    commands.entity(entity).remove::<SnowSlide>();
                }
            }
        }
        if let Some(mut p) = position {
            p.x = resolved.x;
            p.y = resolved.y;
        } else {
            // Non-rigid fixtures/tests still share the same contact rules.
            transform.translation.x = resolved.x;
            transform.translation.y = resolved.y;
        }
        if bounds.iter().all(|b| !b.contains(resolved)) {
            last_free.insert(entity, resolved);
        }
        let style = play_style.copied().unwrap_or_default();
        let incoming = velocity.0.xy();
        let mut hit_x = false;
        let mut hit_y = false;
        damaged.clear();
        for contact in contacts.iter() {
            let (block_entity, _, kind, member, snow, tile) =
                block_query.get(candidates[contact.index]).unwrap();
            let normal = contact.normal;
            // Bounce only into a surface, and at most once per axis this step.
            if !hit_x && velocity.x * normal.x < 0. {
                velocity.x = (-velocity.x * restitution.coefficient * style.wall_bounce_factor())
                    .clamp(-MAX_BOUNCE_VELOCITY, MAX_BOUNCE_VELOCITY);
                if style == crate::components::play_style::PlayStyle::Sticky && !cling_spent {
                    // Grab the wall; the play-style system wall-jumps off it shortly after.
                    velocity.x = 0.;
                    velocity.y = 0.;
                    // One grab per jump: no new grab until the head lands on a floor.
                    commands.entity(entity).insert((
                        crate::components::play_style::WallCling {
                            side: normal.x.signum(),
                            release_at: current_time
                                + crate::components::play_style::WallCling::HOLD_SECONDS,
                        },
                        crate::components::play_style::ClingSpent,
                    ));
                }
                hit_x = true;
            }
            // Sustained floor contact is not another impact: do not restart the
            // slide, add carry again, or repeatedly mine the support underneath.
            if normal.y > 0. && (slide.is_some() || sticky.is_some()) {
                continue;
            }
            // Landing on a floor gives a Sticky head its wall grab back.
            if normal.y > 0. && cling_spent {
                commands
                    .entity(entity)
                    .remove::<crate::components::play_style::ClingSpent>();
            }
            if !hit_y && velocity.y * normal.y < 0. {
                if normal.y > 0. {
                    if enemy
                        .is_some_and(|e| e.enemy_type == crate::components::enemy::EnemyType::Slime)
                        || coat.is_some_and(|c| current_time < c.until)
                    {
                        commands.entity(entity).insert(
                            crate::systems::slime_physics::SlimeContact::new(
                                block_entity,
                                current_time,
                                velocity.0.xy(),
                            ),
                        );
                        velocity.x = 0.;
                        velocity.y = 0.;
                    } else if *kind == BlockType::Ice && snow.is_none_or(|s| s.stage == 0) {
                        commands
                            .entity(entity)
                            .insert(SurfaceGrip::ice(current_time));
                        velocity.y = (-velocity.y * 0.22).clamp(30., 85.);
                        if velocity.x.abs() < 90. {
                            velocity.x = if velocity.x < 0. { -115. } else { 115. };
                        }
                    } else {
                        // A Splitter splits on a hard landing (the play-style system decides).
                        if style == crate::components::play_style::PlayStyle::Splitter
                            && velocity.y < -crate::components::play_style::SPLIT_LANDING_SPEED
                        {
                            commands.entity(entity).try_insert(
                                crate::components::play_style::SplitRequest { speed: -velocity.y },
                            );
                        }
                        velocity.y = style.floor_bounce(velocity.y, restitution.coefficient);
                        // Settled snowfall and full snow blocks share one response.
                        if *kind == BlockType::Snow || snow.is_some_and(|s| s.stage > 0) {
                            velocity.x = snow_carry(velocity.x);
                            commands.entity(entity).insert((
                                SurfaceGrip::snow(current_time),
                                SnowSlide::new(current_time, velocity.y),
                            ));
                            velocity.y = 0.;
                        } else {
                            clear_grip(&mut commands, entity, ice_age_on, current_time);
                        }
                    }
                } else {
                    velocity.y = 0.;
                }
                hit_y = true;
            }
            if !damaged.insert(block_entity) {
                continue;
            }
            if (*kind == BlockType::Cactus
                || tile.is_some_and(|t| *t == crate::systems::biome_content::Tile::Magma)
                    && enemy.is_none_or(|e| {
                        !matches!(
                            e.enemy_type,
                            crate::components::enemy::EnemyType::MagmaCube
                                | crate::components::enemy::EnemyType::Blaze
                        )
                    }))
                && thorns
                    .get(&entity)
                    .is_none_or(|last| current_time - *last >= 0.8)
            {
                thorn_damage.write(EntityDamaged { entity, damage: 1 });
                thorns.insert(entity, current_time);
            }
            if enemy.is_none() {
                // SHRINK: tiny heads bounce off without mining.
                if !shrunk
                    && block_cooldown.can_damage(block_entity, current_time, BLOCK_DAMAGE_COOLDOWN)
                {
                    hits.write(crate::events::block_events::HeadHitBlock {
                        head: entity,
                        block: block_entity,
                    });
                    damage_events.write(BlockDamaged {
                        entity: block_entity,
                        // Heavy heads hit harder when they land fast.
                        damage: if normal.y > 0. {
                            style.landing_damage(incoming.y)
                        } else {
                            1
                        },
                        remaining_health: 0,
                    });
                    block_cooldown.record_damage(block_entity, current_time);
                }
                if let Some(&biome) = biomes.get(&(member.chunk_x, member.chunk_y)) {
                    if let Ok(mut buff) = buff_query.get_mut(entity) {
                        apply_block_buff(&mut commands, entity, *kind, biome, Some(&mut buff));
                    } else {
                        apply_block_buff(&mut commands, entity, *kind, biome, None);
                    }
                }
            }
        }
    }
}

pub(super) fn block_contact_bounds(center: Vec3, snow: Option<&SnowLayer>) -> (Vec3, Vec3) {
    let extra = snow.map_or(0., SnowLayer::thickness);
    (
        center + Vec3::Y * extra * 0.5,
        Vec3::new(BLOCK_SIZE, BLOCK_SIZE + extra, BLOCK_SIZE) * 0.5,
    )
}

#[derive(Default)]
pub(super) struct ContactScratch {
    candidates: Vec<Entity>,
    bounds: Vec<terrain_motion::Bounds>,
    contacts: Vec<terrain_motion::Contact>,
    damaged: std::collections::HashSet<Entity>,
    biomes: std::collections::HashMap<(i32, i32), crate::components::block::BiomeType>,
    /// Last resolved position of each actor that was outside every block.
    last_free: std::collections::HashMap<Entity, Vec2>,
}

/// Furthest unswept nudge (direct Transform/Position write, landing, spawn) that
/// is still swept from the last free position instead of pushed out of the
/// nearest face. Larger jumps are real teleports and use overlap recovery.
const NUDGE_SWEEP_LIMIT: f32 = BLOCK_SIZE * 2.0;

/// If an actor starts this step inside terrain but was free a short distance
/// away, sweep from that free spot so it stops on the side it came from. The
/// nearest-face push-out alone can pop it through to the far side of a wall.
pub(super) fn sweep_start(
    start: Vec2,
    last_free: Option<Vec2>,
    bounds: &[terrain_motion::Bounds],
) -> Vec2 {
    let inside = |p: Vec2| bounds.iter().any(|b| b.contains(p));
    match last_free {
        Some(free)
            if inside(start) && !inside(free) && free.distance(start) <= NUDGE_SWEEP_LIMIT =>
        {
            free
        }
        _ => start,
    }
}
