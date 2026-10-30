use super::*;
pub(super) fn bucket_powers(
    mut powers: MessageReader<crate::events::power_events::PowerActivated>,
    time: Res<Time>,
    mut cooldown: Local<f32>,
    hero: Query<&Transform, With<crate::components::hero::Hero>>,
    blocks: Query<
        (
            &Transform,
            &BlockType,
            Option<&crate::systems::biome_content::Tile>,
        ),
        With<Block>,
    >,
    loaded: Res<LoadedChunks>,
    mut world: ResMut<FluidWorld>,
    mut notice: ResMut<crate::systems::world_portals::PowerNotice>,
) {
    use crate::resources::command_queue::PowerCommandType;
    *cooldown = (*cooldown - time.delta_secs()).max(0.);
    for power in powers.read() {
        let kind = match power.command.command_type {
            PowerCommandType::WaterBucket => Liquid::Water,
            PowerCommandType::LavaBucket => Liquid::Lava,
            _ => continue,
        };
        if *cooldown > 0. {
            notice.text = format!("BUCKET RECHARGING - {:.0}s", cooldown.ceil());
            notice.remaining = 2.;
            continue;
        }
        let Ok(hero) = hero.single() else {
            continue;
        };
        let origin = (
            (hero.translation.x / 64.).round() as i32,
            (hero.translation.y / 64.).round() as i32,
        );
        let occupied: HashSet<_> = blocks
            .iter()
            .filter(|(_, k, tile)| **k != BlockType::Cobweb && tile.is_none_or(|t| !t.decorative()))
            .map(|(t, _, _)| {
                (
                    (t.translation.x / 64.).round() as i32,
                    (t.translation.y / 64.).round() as i32,
                )
            })
            .collect();
        let mut poured = 0;
        // Pour beside the head, never inject into a block or opposite liquid.
        for (dx, dy) in [(1, 1), (-1, 1), (1, 2), (-1, 2), (0, 2), (2, 1), (-2, 1)] {
            let p = (origin.0 + dx, origin.1 + dy);
            let c = chunk(p);
            if occupied.contains(&p) || !loaded.is_loaded(c.0, c.1) {
                continue;
            }
            poured += world.deposit(p, kind, (16 - poured).min(8));
            if poured == 16 {
                break;
            }
        }
        if poured > 0 {
            *cooldown = 8.;
        }
        notice.text = if poured > 0 {
            format!("{:?} BUCKET", kind).to_uppercase()
        } else {
            "NO CLEAR SPACE FOR BUCKET".into()
        };
        notice.remaining = 2.;
    }
}
/// Only fill a small CLOSED natural basin. Any opening below its waterline,
/// unloaded cell, or oversized cavity rejects the pool, leaving a finite spring.
pub(super) fn basin(
    seed: Cell,
    blocked: &HashSet<Cell>,
    loaded: impl Fn(Cell) -> bool,
) -> Option<Vec<Cell>> {
    let mut seen = HashSet::new();
    let mut pending = vec![seed];
    while let Some(p) = pending.pop() {
        if p.1 > seed.1 || blocked.contains(&p) || !seen.insert(p) {
            continue;
        }
        if (p.0 - seed.0).abs() > 7 || seed.1 - p.1 > 3 || !loaded(p) || seen.len() > 64 {
            return None;
        }
        pending.extend([
            (p.0 - 1, p.1),
            (p.0 + 1, p.1),
            (p.0, p.1 - 1),
            (p.0, p.1 + 1),
        ]);
    }
    if seen.len() < 3 {
        return None;
    }
    let mut cells: Vec<_> = seen.into_iter().collect();
    cells.sort_unstable();
    Some(cells)
}
pub(super) fn ingest(
    mut commands: Commands,
    seeds: Query<(Entity, &FluidSeed)>,
    mut world: ResMut<FluidWorld>,
    blocks: Query<
        (
            &Transform,
            &BlockType,
            Option<&crate::systems::biome_content::Tile>,
        ),
        With<Block>,
    >,
    loaded: Res<LoadedChunks>,
) {
    if seeds.is_empty() {
        return;
    }
    let blocked: HashSet<_> = blocks
        .iter()
        .filter(|(_, k, tile)| **k != BlockType::Cobweb && tile.is_none_or(|t| !t.decorative()))
        .map(|(t, _, _)| {
            (
                (t.translation.x / 64.).round() as i32,
                (t.translation.y / 64.).round() as i32,
            )
        })
        .collect();
    let is_loaded = |p: Cell| {
        let c = chunk(p);
        loaded.is_loaded(c.0, c.1)
    };
    for (entity, seed) in &seeds {
        // Generation order must not turn a boundary basin into a leaking source.
        if ![
            (seed.cell.0 - 8, seed.cell.1 - 4),
            (seed.cell.0 + 8, seed.cell.1 + 1),
        ]
        .into_iter()
        .all(is_loaded)
        {
            continue;
        }
        if world.seeded.insert(seed.cell) {
            if let Some(pool) = basin(seed.cell, &blocked, is_loaded) {
                for p in pool {
                    world.seeded.insert(p);
                    world.cells.entry(p).or_insert(Volume {
                        kind: seed.kind,
                        amount: 8,
                        reserve: 0,
                    });
                }
            } else {
                world.cells.entry(seed.cell).or_insert(Volume {
                    kind: seed.kind,
                    amount: 8,
                    reserve: 40,
                });
            }
        }
        commands.entity(entity).despawn();
    }
}
pub(super) fn simulate(
    mut commands: Commands,
    time: Res<Time>,
    mut clock: Local<f32>,
    mut world: ResMut<FluidWorld>,
    blocks: Query<
        (
            Entity,
            &Transform,
            &BlockType,
            Option<&crate::systems::biome_content::Tile>,
        ),
        With<Block>,
    >,
    loaded: Res<LoadedChunks>,
    mesh: Option<Res<MeshCache>>,
    materials: Res<MaterialCache>,
    weather: Option<Res<crate::components::weather::ActiveWeather>>,
    hero: Query<&Transform, With<crate::components::hero::Hero>>,
    occupancy: Res<FluidOccupancy>,
) {
    *clock += time.delta_secs();
    if *clock < 0.1 {
        return;
    }
    *clock %= 0.1;
    let Some(mesh) = mesh else {
        return;
    };
    let blocked: HashSet<_> = blocks
        .iter()
        .filter(|(_, _, kind, tile)| {
            **kind != BlockType::Cobweb && tile.is_none_or(|t| !t.decorative())
        })
        .map(|(_, t, _, _)| {
            (
                (t.translation.x / 64.0).round() as i32,
                (t.translation.y / 64.0).round() as i32,
            )
        })
        .collect();
    let is_loaded = |p: Cell| {
        let c = chunk(p);
        loaded.is_loaded(c.0, c.1)
    };
    let old: Vec<_> = world.solids.iter().map(|(&p, &e)| (p, e)).collect();
    for (p, e) in old {
        if !is_loaded(p) {
            world.solids.remove(&p);
        } else if blocks.get(e).is_err() {
            world.solids.remove(&p);
            world.cooled.remove(&p);
            world.ice.remove(&p);
        }
    }
    world.step(&blocked, is_loaded);
    if let (Some(weather), Ok(hero)) = (weather.as_ref(), hero.single()) {
        world.react_weather(&weather, hero.translation.xy(), &blocked, &occupancy.0);
    }
    let ice: Vec<_> = world.ice.keys().copied().collect();
    for p in ice {
        if is_loaded(p) {
            let ticks = &mut world.ice.get_mut(&p).unwrap().ticks;
            let fiery = weather.as_ref().is_some_and(|w| {
                w.weather_type == crate::components::weather::WeatherType::FireRain
                    && w.duration_remaining > 0.
                    && w.intensity >= 0.1
            }) && hero.single().is_ok_and(|h| {
                h.translation
                    .xy()
                    .distance_squared(Vec2::new(p.0 as f32 * 64., p.1 as f32 * 64.))
                    < 2000_f32.powi(2)
            }) && (1..=2).all(|dy| !blocked.contains(&(p.0, p.1 + dy)));
            let snowing = weather.as_ref().is_some_and(|w| w.is_snowing())
                && hero.single().is_ok_and(|h| {
                    h.translation
                        .xy()
                        .distance_squared(Vec2::new(p.0 as f32 * 64., p.1 as f32 * 64.))
                        < 2000_f32.powi(2)
                });
            if snowing && !fiery {
                continue;
            }
            *ticks = ticks.saturating_sub(if fiery { 30 } else { 1 });
            if *ticks == 0 {
                world.thaw(p);
                if let Some(entity) = world.solids.remove(&p) {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
    let cooled: Vec<_> = world
        .cooled
        .iter()
        .chain(world.ice.keys())
        .copied()
        .filter(|p| is_loaded(*p) && !world.solids.contains_key(p))
        .collect();
    for p in cooled {
        if blocked.contains(&p) {
            continue;
        }
        if occupancy.0.contains(&p) {
            // Placement may be deferred for a frame; recheck at materialization.
            world.thaw(p);
            continue;
        }
        let kind = if world.ice.contains_key(&p) {
            BlockType::Ice
        } else {
            BlockType::Obsidian
        };
        let Some(material) = materials.get(kind) else {
            continue;
        };
        let c = chunk(p);
        let entity = commands
            .spawn((
                Block {
                    health: kind.starting_health(),
                    max_health: kind.starting_health(),
                },
                kind,
                ChunkMember {
                    chunk_x: c.0,
                    chunk_y: c.1,
                },
                Mesh3d(mesh.block_mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(
                    p.0 as f32 * 64.0,
                    p.1 as f32 * 64.0,
                    crate::systems::actor_depth::TERRAIN_CENTER,
                )
                .with_scale(Vec3::splat(64.0)),
            ))
            .id();
        world.solids.insert(p, entity);
    }
}
pub(super) fn frost_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut world: ResMut<FluidWorld>,
    snowballs: Query<
        (Entity, &Transform, &LinearVelocity),
        With<crate::components::projectile::Snowball>,
    >,
    occupancy: Res<FluidOccupancy>,
) {
    for (entity, t, velocity) in &snowballs {
        let end = t.translation.xy();
        let start = end - velocity.0.xy() * time.delta_secs();
        let steps = ((end - start).length() / 4.).ceil().clamp(1., 256.) as usize;
        for i in 0..=steps {
            let sample = start.lerp(end, i as f32 / steps as f32);
            let p = (sample / 64.).round().as_ivec2();
            if world.is_water_at(sample) && world.freeze((p.x, p.y), 120, &occupancy.0) {
                commands.entity(entity).try_despawn();
                break;
            }
        }
    }
}
pub(super) fn contact(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<FluidWorld>,
    mut damage: MessageWriter<crate::events::damage_events::EntityDamaged>,
    mut heat: Local<HashMap<Entity, f32>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    heroes: Query<(), With<crate::components::hero::Hero>>,
    mut strokes: Local<HashMap<Entity, f32>>,
    mut bodies: Query<
        (
            Entity,
            &Transform,
            &mut LinearVelocity,
            Option<&mut Buff>,
            Option<&Health>,
        ),
        Without<Block>,
    >,
) {
    heat.retain(|e, _| bodies.contains(*e));
    strokes.retain(|e, _| bodies.contains(*e));
    for (entity, t, mut velocity, buff, health) in &mut bodies {
        let half = if health.is_some() {
            32.0 * t.scale.x.abs()
        } else {
            16.0
        };
        let lo = ((t.translation.xy() - Vec2::splat(half)) / 64.0)
            .round()
            .as_ivec2();
        let hi = ((t.translation.xy() + Vec2::splat(half)) / 64.0)
            .round()
            .as_ivec2();
        let mut depth = 0.0_f32;
        let mut lava = false;
        for y in lo.y..=hi.y {
            for x in lo.x..=hi.x {
                let kind = world
                    .cells
                    .get(&(x, y))
                    .map(|v| v.kind)
                    .or_else(|| world.falling.get(&(x, y)).map(|v| v.0));
                let Some(kind) = kind else {
                    continue;
                };
                for (left, right, bottom, surface) in fluid_sections(&world, (x, y), kind) {
                    let width =
                        (t.translation.x + half).min(right) - (t.translation.x - half).max(left);
                    let height = (t.translation.y + half).min(surface)
                        - (t.translation.y - half).max(bottom);
                    if width > 0. && height > 0. {
                        depth += width * height / (4. * half * half);
                        lava |= kind == Liquid::Lava;
                    }
                }
            }
        }
        if depth <= 0.0 {
            heat.remove(&entity);
            strokes.remove(&entity);
            continue;
        }
        let depth = depth.min(1.0);
        let drag = (-time.delta_secs() * depth * if lava { 5.0 } else { 1.5 }).exp();
        velocity.x *= drag;
        velocity.y = velocity.y * drag + 520.0 * depth * time.delta_secs();
        if health.is_some() {
            let max_speed = if lava { 65. } else { 180. };
            velocity.x = velocity.x.clamp(-max_speed, max_speed);
            let elapsed = strokes.entry(entity).or_default();
            *elapsed += time.delta_secs();
            let manual = if heroes.contains(entity) {
                keys.as_ref().map_or(0., |k| {
                    if k.pressed(KeyCode::KeyS) || k.pressed(KeyCode::ArrowDown) {
                        -1.
                    } else if k.pressed(KeyCode::KeyW)
                        || k.pressed(KeyCode::ArrowUp)
                        || k.pressed(KeyCode::Space)
                    {
                        1.
                    } else {
                        0.
                    }
                })
            } else {
                0.
            };
            // Asynchronous dives keep autonomous heads mining the lake bed.
            // Rise phases allow them to reach banks; a strong stroke clears a lip.
            let phase = (*elapsed + (entity.index_u32() % 17) as f32 * 0.31) % 9.;
            let stroke = if manual != 0. {
                manual
            } else if phase < 5.5 {
                -1.
            } else {
                1.
            };
            let target = stroke
                * if lava {
                    55.
                } else if stroke > 0. {
                    if manual > 0. || phase > 8. {
                        430.
                    } else {
                        245.
                    }
                } else {
                    160.
                };
            velocity.y += (target - velocity.y) * (time.delta_secs() * 6.).min(1.);
        }
        if lava && health.is_some() {
            if !heat.contains_key(&entity) {
                damage.write(crate::events::damage_events::EntityDamaged { entity, damage: 1 });
                heat.insert(entity, 0.);
            }
            if let Some(mut buff) = buff {
                if buff.buff_type == BuffType::Burning {
                    buff.duration.reset();
                } else {
                    let clock = heat.entry(entity).or_default();
                    *clock += time.delta_secs();
                    if *clock >= 0.75 {
                        *clock %= 0.75;
                        damage.write(crate::events::damage_events::EntityDamaged {
                            entity,
                            damage: 1,
                        });
                    }
                }
            } else {
                commands
                    .entity(entity)
                    .insert(Buff::new(BuffType::Burning, 2.0));
            }
        } else if !lava {
            heat.remove(&entity);
            if buff.is_some_and(|b| b.buff_type == BuffType::Burning) {
                commands.entity(entity).remove::<Buff>();
            }
        }
    }
}
/// Connected stream strips and pooled surfaces share ONE mesh per liquid type.
/// A falling cell uses a vertical column; settled volume uses a horizontal surface.
/// Flux bridges cells during transport, without adding simulated water volume.
/// Shared front-face bounds: contact must match pools, streams and droplets exactly.

#[derive(Resource, Default)]
pub(super) struct FluidOccupancy(pub HashSet<Cell>);
pub(super) fn update_occupancy(
    mut input: ResMut<FluidOccupancy>,
    actors: Query<&Transform, With<Health>>,
) {
    input.0.clear();
    for t in &actors {
        let half = 32. * t.scale.x.abs() + 8.;
        let lo = ((t.translation.xy() - Vec2::splat(half)) / 64.)
            .round()
            .as_ivec2();
        let hi = ((t.translation.xy() + Vec2::splat(half)) / 64.)
            .round()
            .as_ivec2();
        for y in lo.y..=hi.y {
            for x in lo.x..=hi.x {
                input.0.insert((x, y));
            }
        }
    }
}
