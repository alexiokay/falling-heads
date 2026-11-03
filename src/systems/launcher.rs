//! Original, modeled rocket transport. Coordinates and physics stay in world space.
use crate::{
    components::{
        block::{Block, BlockType},
        buff::Buffable,
        bundles::avian_physics,
        chunk::{LoadedChunks, world_to_chunk_coords},
        enemy::{Flying, Health},
        hero::Hero,
        villager::Villager,
    },
    events::{block_events::BlockDestroyed, power_events::PowerActivated},
    resources::{
        asset_handles::GameTextures,
        command_queue::{LaunchDirection, PowerCommand, PowerCommandType},
    },
    states::GameState,
};
use avian3d::prelude::*;
use bevy::prelude::*;

const TILE: f32 = 64.;
const WARMUP: f32 = 0.75;
const FLIGHT: f32 = 2.6;
#[derive(Component)]
pub struct LauncherPassenger;
#[derive(Component)]
struct RocketArrival(u32);
struct Arrival {
    position: Vec3,
    delay: f32,
    seed: u32,
}
#[derive(Resource, Default)]
struct Arrivals(Vec<Arrival>);
/// Lands `count` villagers beside `at`, one after another, like rocket passengers
/// (the space prison uses it for villagers who escaped with the hero).
pub(crate) fn drop_villagers(world: &mut World, at: Vec3, count: u32, seed: u32) {
    let mut arrivals = world.get_resource_or_insert_with(Arrivals::default);
    for i in 0..count.min(8) {
        let side = if i % 2 == 0 { -1. } else { 1. };
        arrivals.0.push(Arrival {
            position: at + Vec3::new(side * TILE * (1 + i / 2) as f32, 0., 0.),
            delay: 0.3 + i as f32 * 0.2,
            seed: seed.wrapping_add(i),
        });
    }
}
fn release_passengers(world: &mut World, dt: f32) {
    let mut arrivals = world.remove_resource::<Arrivals>().unwrap_or_default();
    for a in &mut arrivals.0 {
        a.delay -= dt;
    }
    let texture = world.resource::<GameTextures>().villager.clone();
    let material = world.resource::<RocketAssets>().villager.clone();
    if let Some(mut m) = world
        .resource_mut::<Assets<StandardMaterial>>()
        .get_mut(&material)
    {
        if m.base_color_texture.as_ref() != Some(&texture) {
            m.base_color_texture = Some(texture);
        }
    }
    let mesh = world.resource::<RocketAssets>().quad.clone();
    arrivals.0.retain(|a| {
        if a.delay > 0. {
            return true;
        }
        world.spawn((
            RocketArrival(a.seed),
            super::entity_spawn::SharedVillagerFace,
            Villager {
                ai_timer: Timer::from_seconds(1., TimerMode::Repeating),
                move_direction: 1.,
                direction_change_timer: Timer::from_seconds(3., TimerMode::Repeating),
            },
            Health::new(6),
            Buffable,
            avian_physics::character(),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(a.position),
        ));
        false
    });
    world.insert_resource(arrivals);
}
#[derive(Resource, Default)]
pub struct LauncherState {
    pub preload: Option<Vec3>,
    pending: Option<PowerCommand>,
    active: Option<Flight>,
    serial: u32,
}
struct Flight {
    hero: Entity,
    ship: Entity,
    exhaust: Entity,
    start: Vec3,
    end: Vec3,
    audio: Option<Entity>,
    elapsed: f32,
    waiting: f32,
    heading: f32,
    seed: u32,
    count: u8,
    saved: Saved,
}
struct Saved {
    body: RigidBody,
    gravity: GravityScale,
    layers: CollisionLayers,
    visibility: Visibility,
    flying: bool,
}
#[derive(Component)]
struct Fragment {
    velocity: Vec3,
    spin: Vec3,
    life: f32,
    duration: f32,
}
#[derive(Resource)]
struct RocketAssets {
    cube: Handle<Mesh>,
    quad: Handle<Mesh>,
    cylinder: Handle<Mesh>,
    cone: Handle<Mesh>,
    metal: Handle<StandardMaterial>,
    red: Handle<StandardMaterial>,
    dark: Handle<StandardMaterial>,
    hull: Handle<StandardMaterial>,
    panel: Handle<StandardMaterial>,
    porthole: Handle<StandardMaterial>,
    nav_red: Handle<StandardMaterial>,
    nav_green: Handle<StandardMaterial>,
    flame: Handle<super::graphics::FlameMaterial>,
    villager: Handle<StandardMaterial>,
}
pub struct LauncherPlugin;
impl Plugin for LauncherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LauncherState>()
            .init_resource::<Arrivals>()
            .add_systems(Startup, (setup, super::rocket_audio::setup))
            .add_systems(Update, requests.run_if(in_state(GameState::Playing)))
            .add_systems(
                PostUpdate,
                update
                    .after(super::biome_teleport::finish_biome_teleport)
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn requests(
    mut events: MessageReader<PowerActivated>,
    mut state: ResMut<LauncherState>,
    mut notice: ResMut<super::world_portals::PowerNotice>,
) {
    for event in events
        .read()
        .filter(|e| e.command.command_type == PowerCommandType::Launcher)
    {
        // One flight at a time; any wait between flights is the Launcher cooldown
        // under Settings > Cooldowns.
        if state.active.is_some() || state.pending.is_some() {
            notice.text = "Launcher already flying".into();
            notice.remaining = 3.;
        } else {
            state.pending = Some(event.command.clone());
        }
    }
}
fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846ca68b);
    x ^ (x >> 16)
}
fn destination(start: Vec3, direction: LaunchDirection, seed: u32) -> Vec3 {
    let sign = match direction {
        LaunchDirection::Left => -1.,
        LaunchDirection::Right => 1.,
        LaunchDirection::Auto => {
            if hash(seed) & 1 == 0 {
                1.
            } else {
                -1.
            }
        }
    };
    // Choose a real landing cell inside the range, rather than rounding a
    // 52-block endpoint outward after the flight has already been planned.
    let a = start.x + sign * 47. * TILE;
    let b = start.x + sign * 52. * TILE;
    let lo = (a.min(b) / TILE).ceil() as i32;
    let hi = (a.max(b) / TILE).floor() as i32;
    let x = lo + (hash(seed.wrapping_add(1)) % ((hi - lo + 1) as u32)) as i32;
    let y = (start.y / TILE).round() + ((hash(seed.wrapping_add(2)) % 29) as f32 - 14.);
    Vec3::new(x as f32 * TILE, y * TILE, start.z)
}
fn roll_angle(elapsed: f32) -> f32 {
    let warm = (elapsed / WARMUP).clamp(0., 1.);
    let flight = ((elapsed - WARMUP) / FLIGHT).clamp(0., 1.);
    // A restrained 1/8 turn on ignition; matched angular velocity at liftoff.
    std::f32::consts::TAU
        * (0.125 * warm.powi(3) + 0.5 * FLIGHT * (flight - 0.25 * flight * flight))
}
fn path(start: Vec3, end: Vec3, u: f32) -> Vec3 {
    let t = u.clamp(0., 1.).powf(1.6);
    let seed = hash(start.x.to_bits() ^ end.y.to_bits().rotate_left(11));
    let phase = (seed % 628) as f32 * 0.01;
    let turns = 0.8 + ((seed >> 10) % 30) as f32 * 0.01;
    let envelope = 16. * t * t * (1. - t) * (1. - t);
    // Broad continuous weave; horizontal speed never reverses in a corkscrew.
    start.lerp(end, t)
        + Vec3::Y
            * (5. * TILE * 4. * t * (1. - t)
                + 1.5 * TILE * envelope * (std::f32::consts::TAU * turns * t + phase).sin())
}
fn nose_heading(start: Vec3, end: Vec3, u: f32) -> f32 {
    let tangent = path(start, end, (u + 0.003).min(1.)) - path(start, end, (u - 0.003).max(0.));
    tangent.y.atan2(tangent.x)
}
fn turn_toward(current: f32, target: f32, dt: f32) -> f32 {
    let difference = (target - current + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    // Shortest-angle damping avoids the +/- PI discontinuity on leftward flights.
    current + (difference * (1. - (-7. * dt).exp())).clamp(-2.4 * dt, 2.4 * dt)
}
fn crater_cell(x: i32, y: i32, seed: u32) -> bool {
    x.abs() <= 2
        && y.abs() <= 2
        && ((x.abs() <= 1 && y.abs() <= 1)
            || ((x.abs() != 2 || y.abs() != 2)
                && hash(seed.wrapping_add((x + 3) as u32 * 73 + (y + 3) as u32 * 139)) % 4 != 0))
}

fn destroy(world: &mut World, b: Vec3, seed: u32) {
    let center = (b.xy() / TILE).round().as_ivec2();
    let mut q = world.query_filtered::<(
        Entity,
        &Transform,
        &BlockType,
        Option<&MeshMaterial3d<StandardMaterial>>,
    ), With<Block>>();
    let hits: Vec<_> = q
        .iter(world)
        .filter(|(_, p, _, _)| {
            let c = (p.translation.xy() / TILE).round().as_ivec2() - center;
            crater_cell(c.x, c.y, seed)
        })
        .map(|(e, p, k, m)| (e, p.translation, *k, m.map(|m| m.0.clone())))
        .collect();
    for (i, (e, p, kind, material)) in hits.iter().enumerate() {
        if i < 12 {
            if let Some(material) = material {
                let h = hash(seed.wrapping_add(i as u32));
                fragment(
                    world,
                    *p + Vec3::Z * 40.,
                    Vec3::splat(15. + (h % 15) as f32),
                    material.clone(),
                    Vec3::new((h % 401) as f32 - 200., 180. + (h % 220) as f32, 30.),
                    1.3,
                    h,
                );
            }
        }
        let tile = world
            .get::<crate::systems::biome_content::Tile>(*e)
            .copied();
        world.write_message(BlockDestroyed {
            tile,
            entity: *e,
            block_type: *kind,
            position: *p,
        });
        world.despawn(*e);
    }
    if let Some(mut spatial) =
        world.get_resource_mut::<crate::resources::spatial_hash::SpatialHash>()
    {
        for hit in &hits {
            spatial.remove(hit.0);
        }
    }
    {
        let cells: Vec<_> = (-2..=2)
            .flat_map(|x| {
                (-2..=2).filter_map(move |y| {
                    crater_cell(x, y, seed).then_some((center.x + x, center.y + y))
                })
            })
            .collect();
        if let Some(mut fluids) = world.get_resource_mut::<super::fluids::FluidWorld>() {
            fluids.clear_transport_cells(&cells);
        }
    }
}
fn restore(world: &mut World, f: &Flight) {
    if let Ok(mut hero) = world.get_entity_mut(f.hero) {
        hero.insert((
            f.saved.body,
            f.saved.gravity,
            f.saved.layers,
            f.saved.visibility,
            LinearVelocity::ZERO,
        ));
        hero.remove::<LauncherPassenger>();
        if !f.saved.flying {
            hero.remove::<Flying>();
        }
    }
}

fn finish(world: &mut World, f: &Flight) {
    let landing = (f.end.xy() / TILE).round() * TILE;
    let p = landing.extend(f.start.z);
    destroy(world, p, f.seed);
    super::rocket_audio::play(world, 2);
    crash_burst(world, p, f.seed);
    // Break the modeled ship into its actual panels, with independent ballistic motion.
    let root = world.get::<Transform>(f.ship).copied().unwrap_or_default();
    let parts: Vec<_> = world
        .query::<(&ChildOf, &Transform, &MeshMaterial3d<StandardMaterial>)>()
        .iter(world)
        .filter(|(parent, _, _)| parent.parent() == f.ship)
        .map(|(_, t, m)| (*t, m.0.clone()))
        .collect();
    for (i, (t, m)) in parts.into_iter().enumerate() {
        let h = hash(f.seed.wrapping_add(i as u32));
        let mut part = root * t;
        part.translation.z += 50.;
        let debris = fragment(
            world,
            part.translation,
            part.scale,
            m,
            Vec3::new((h % 601) as f32 - 300., 130. + (h % 390) as f32, 20.),
            2.1,
            h,
        );
        world.get_mut::<Transform>(debris).unwrap().rotation = part.rotation;
    }
    world.despawn(f.ship);
    world.despawn(f.exhaust);
    if let Some(audio) = f.audio {
        let _ = world.despawn(audio);
    }
    restore(world, f);
    if let Some(mut transform) = world.get_mut::<Transform>(f.hero) {
        transform.translation = p;
    }
    if let Some(mut position) = world.get_mut::<Position>(f.hero) {
        position.0 = p;
    }
    let slots = [
        (-1, 0),
        (1, 0),
        (-1, 1),
        (1, 1),
        (0, 1),
        (-1, -1),
        (1, -1),
        (0, -1),
    ];
    for (i, &(x, y)) in slots.iter().take(f.count as usize).enumerate() {
        world.resource_mut::<Arrivals>().0.push(Arrival {
            position: p + Vec3::new(x as f32 * TILE, y as f32 * TILE, 0.),
            delay: 0.25 + i as f32 * 0.14,
            seed: f.seed,
        });
    }
    info!(
        "LAUNCHER landed: dx={:.1} blocks dy={:.1} blocks passengers={}",
        (f.end.x - f.start.x) / TILE,
        (f.end.y - f.start.y) / TILE,
        f.count
    );
}
fn update(world: &mut World) {
    let dt = world.resource::<Time>().delta_secs();
    release_passengers(world, dt);
    let debris: Vec<_> = world
        .query::<(
            Entity,
            &mut Fragment,
            &mut Transform,
            &MeshMaterial3d<StandardMaterial>,
        )>()
        .iter_mut(world)
        .map(|(e, mut f, mut t, m)| {
            f.life -= dt;
            f.velocity.y -= 330. * dt;
            t.translation += f.velocity * dt;
            t.rotate(Quat::from_euler(
                EulerRot::XYZ,
                f.spin.x * dt,
                f.spin.y * dt,
                f.spin.z * dt,
            ));
            (
                e,
                m.0.clone(),
                f.life,
                (f.life / (f.duration * 0.45)).clamp(0., 1.),
            )
        })
        .collect();
    for (e, m, life, alpha) in debris {
        if life <= 0. {
            world.despawn(e);
            world
                .resource_mut::<Assets<StandardMaterial>>()
                .remove(m.id());
        } else if let Some(mut mat) = world.resource_mut::<Assets<StandardMaterial>>().get_mut(&m) {
            mat.base_color.set_alpha(alpha);
        }
    }
    let mut state = world.remove_resource::<LauncherState>().unwrap();
    let teleporting = world
        .get_resource::<super::biome_teleport::BiomeTeleport>()
        .is_some_and(|t| {
            t.target.is_some() || t.phase != super::biome_teleport::TeleportPhase::Idle
        });
    if teleporting {
        state.pending = None;
        if let Some(f) = state.active.take() {
            restore(world, &f);
            world.despawn(f.ship);
            world.despawn(f.exhaust);
            if let Some(audio) = f.audio {
                let _ = world.despawn(audio);
            }
        }
        state.preload = None;
        world.insert_resource(state);
        return;
    }
    if let Some(command) = state.pending.take() {
        if let Some((hero, start)) = world
            .query_filtered::<(Entity, &Transform), With<Hero>>()
            .iter(world)
            .next()
            .map(|(e, t)| (e, t.translation))
        {
            state.serial = state.serial.wrapping_add(1);
            let seed = hash(
                state
                    .serial
                    .wrapping_add(start.x.to_bits())
                    .wrapping_add(start.y.to_bits()),
            );
            let end = destination(start, command.launcher.direction, seed);
            let saved = Saved {
                body: world
                    .get::<RigidBody>(hero)
                    .copied()
                    .unwrap_or(RigidBody::Dynamic),
                gravity: world
                    .get::<GravityScale>(hero)
                    .copied()
                    .unwrap_or(GravityScale(1.)),
                layers: world
                    .get::<CollisionLayers>(hero)
                    .copied()
                    .unwrap_or_default(),
                visibility: world.get::<Visibility>(hero).copied().unwrap_or_default(),
                flying: world.get::<Flying>(hero).is_some(),
            };
            world.entity_mut(hero).insert((
                LauncherPassenger,
                Flying,
                RigidBody::Kinematic,
                GravityScale(0.),
                CollisionLayers::NONE,
                Visibility::Hidden,
                LinearVelocity::ZERO,
            ));
            let ship = ship(
                world,
                start + Vec3::Z * 180.,
                hero,
                command.launcher.villagers.min(8),
            );
            super::rocket_audio::play(world, 0);
            let a = world.resource::<RocketAssets>();
            let exhaust = world
                .spawn((
                    Mesh3d(a.quad.clone()),
                    MeshMaterial3d(a.flame.clone()),
                    Transform::default(),
                    Visibility::Hidden,
                    bevy::light::NotShadowCaster,
                ))
                .id();
            state.active = Some(Flight {
                hero,
                ship,
                exhaust,
                start,
                end,
                audio: None,
                elapsed: 0.,
                waiting: 0.,
                heading: std::f32::consts::FRAC_PI_2,
                seed,
                count: command.launcher.villagers.min(8),
                saved,
            });
            state.preload = Some(end);
        }
    }
    if let Some(mut f) = state.active.take() {
        if world.get_entity(f.hero).is_err() {
            world.despawn(f.ship);
            world.despawn(f.exhaust);
            if let Some(audio) = f.audio {
                let _ = world.despawn(audio);
            }
            state.preload = None;
        } else {
            let (cx, cy) = world_to_chunk_coords(f.end.x, f.end.y, TILE);
            let ready = world
                .get_resource::<LoadedChunks>()
                .is_none_or(|l| (-1..=1).all(|x| (-1..=1).all(|y| l.is_loaded(cx + x, cy + y))));
            f.elapsed += dt;
            if !ready && f.elapsed >= WARMUP {
                f.waiting += dt;
                f.elapsed = WARMUP - 0.01;
                if f.waiting > 8. {
                    restore(world, &f);
                    world.despawn(f.ship);
                    world.despawn(f.exhaust);
                    if let Some(audio) = f.audio {
                        let _ = world.despawn(audio);
                    }
                    state.preload = None;
                    if let Some(mut notice) =
                        world.get_resource_mut::<super::world_portals::PowerNotice>()
                    {
                        notice.text = "Launcher cancelled: destination could not load".into();
                        notice.remaining = 4.;
                    }
                    world.insert_resource(state);
                    return;
                }
            }
            let warm = (f.elapsed / WARMUP).min(1.);
            let u = ((f.elapsed - WARMUP) / FLIGHT).clamp(0., 1.);
            let p = path(f.start, f.end, u)
                + Vec3::X
                    * ((f.end.x - f.start.x).signum()
                        * 14.
                        * (std::f32::consts::PI * warm).sin()
                        * (1. - warm));
            let target = nose_heading(f.start, f.end, u);
            f.heading = turn_toward(f.heading, target, dt);
            let turn = warm * warm * (3. - 2. * warm);
            let pitch = f.heading;
            // A gentle rocking on the roll rhythm (not full barrel rolls), so the
            // portholes and the heads in them stay in view.
            let roll = 0.18 * roll_angle(f.elapsed).sin();
            if let Some(mut t) = world.get_mut::<Transform>(f.ship) {
                t.translation = p + Vec3::Z * 180.;
                t.rotation = Quat::from_rotation_z(pitch)
                    * Quat::from_rotation_y(
                        -0.9 * (1. - turn)
                            + 0.12
                                * (u * std::f32::consts::TAU).sin()
                                * (std::f32::consts::PI * u).sin(),
                    )
                    * Quat::from_rotation_x(roll);
            }
            // Heads stay upright and face the camera inside their portholes.
            let ship_turn = world
                .get::<Transform>(f.ship)
                .map_or(Quat::IDENTITY, |t| t.rotation);
            let now = world.resource::<Time>().elapsed_secs();
            for (parent, mut t, head, light) in world
                .query::<(
                    &ChildOf,
                    &mut Transform,
                    Option<&visuals::RocketHead>,
                    Option<&visuals::NavLight>,
                )>()
                .iter_mut(world)
            {
                if parent.parent() != f.ship {
                    continue;
                }
                if head.is_some() {
                    t.rotation = ship_turn.inverse();
                }
                if let Some(light) = light {
                    let on = (now + light.0) % 1. < 0.3;
                    t.scale = Vec3::splat(if on { 9. } else { 0. });
                }
            }
            if u > 0. && f.audio.is_none() {
                f.audio = Some(super::rocket_audio::play(world, 1));
            }
            if let Some(mut t) = world.get_mut::<Transform>(f.hero) {
                t.translation = p;
            }
            if let Some(mut t) = world.get_mut::<Position>(f.hero) {
                t.0 = p;
            }
            if let Some(mut v) = world.get_mut::<LinearVelocity>(f.hero) {
                v.0 = Vec3::ZERO;
            }
            if let Some(mut center) =
                world.get_resource_mut::<crate::resources::action_center::ActionCenter>()
            {
                center.center = p;
                center.raw_center = p;
            }
            for mut camera in world
                .query_filtered::<&mut Transform, With<Camera3d>>()
                .iter_mut(world)
            {
                *camera = Transform::from_xyz(p.x + 30., p.y + 80., 1500.)
                    .looking_at(Vec3::new(p.x, p.y, 0.), Vec3::Y);
            }
            let dir = Vec3::new(pitch.cos(), pitch.sin(), 0.);
            let length = if u > 0. {
                160. + u * 80.
            } else {
                15. + warm.powi(3) * 65.
            };
            world.entity_mut(f.exhaust).insert((
                Visibility::Visible,
                Transform::from_translation(p - dir * (100. + length * 0.5) + Vec3::Z * 185.)
                    .with_rotation(Quat::from_rotation_z(pitch + std::f32::consts::FRAC_PI_2))
                    .with_scale(Vec3::new(0.65, length / 64., 1.)),
            ));
            if u >= 1. {
                finish(world, &f);
                state.preload = None;
            } else {
                state.active = Some(f);
            }
        }
    }
    world.insert_resource(state);
}
#[derive(Resource, Default)]
struct LauncherProbe {
    step: u8,
    since: f32,
    expected: Option<(Vec3, Vec3, u32, u8)>,
    watch: Option<Entity>,
    breakup_captures: u32,
}

#[path = "launcher/visuals.rs"]
mod visuals;
use visuals::*;

#[path = "launcher/probe.rs"]
mod probe;
pub(crate) use probe::*;

#[cfg(test)]
#[path = "launcher/tests.rs"]
mod tests;
