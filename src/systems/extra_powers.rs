//! Chat powers added in October 2026: FEATHER, ANVIL, GOLEM, PEARL, QUAKE,
//! SHRINK, WIND, BLACKHOLE, ICEAGE, BEES, BOLT and FUSS. Each reacts to
//! `PowerActivated` (API, dev panel) and runs while the world runs (`InGame`).
use crate::components::block::{Block, BlockType};
use crate::components::buff::Buffable;
use crate::components::bundles::avian_physics;
use crate::components::enemy::{Enemy, Flying, Health};
use crate::components::hero::Hero;
use crate::components::resource::Resource as Drop;
use crate::components::villager::Villager;
use crate::events::block_events::{BlockDamaged, BlockDestroyed};
use crate::events::damage_events::EntityDamaged;
use crate::events::power_events::PowerActivated;
use crate::resources::command_queue::PowerCommandType as P;
use crate::resources::spatial_hash::SpatialHash;
use crate::systems::audio::effects::SoundAt;
use crate::systems::world_portals::PowerNotice;
use avian3d::prelude::{GravityScale, LinearVelocity, Position};
use bevy::prelude::*;
use rand::RngExt;
use std::collections::HashSet;

pub struct ExtraPowersPlugin;

impl Plugin for ExtraPowersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Fuss>()
            .init_resource::<Quake>()
            .init_resource::<CameraShake>()
            .init_resource::<Wind>()
            .init_resource::<IceAge>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    (
                        start_simple,
                        start_spawned,
                        start_pearl,
                        start_quake,
                        start_bolt,
                    ),
                    (
                        feather_gravity,
                        fall_anvils,
                        golem_ai.after(crate::plugins::physics_plugin::PhysicsSet::AI),
                        blow_wind,
                        black_holes,
                        quake_pulses,
                        pearl_flight,
                        spin_parts,
                        spiral_matter,
                        ice_age,
                        bees,
                        fuss,
                        fade_fx,
                    ),
                )
                    .chain()
                    .run_if(in_state(crate::states::InGame)),
            );
    }
}

// ---------------------------------------------------------------- shared art

#[derive(Resource)]
struct Art {
    quad: Handle<Mesh>,
    cube: Handle<Mesh>,
    anvil: Handle<StandardMaterial>,
    golem_face: Handle<Mesh>,
    golem: Handle<StandardMaterial>,
    bee_face: Handle<Mesh>,
    bee: Handle<StandardMaterial>,
    hole: HoleArt,
    pearl: Handle<StandardMaterial>,
    dust: Handle<Image>,
    sounds: Sounds,
}

struct Sounds {
    anvil: Handle<AudioSource>,
    golem_hit: Handle<AudioSource>,
    golem_summon: Handle<AudioSource>,
    bee_loop: Handle<AudioSource>,
    bee_sting: Handle<AudioSource>,
    pearl: Handle<AudioSource>,
    quake: Handle<AudioSource>,
    wind: Handle<AudioSource>,
    hole: Handle<AudioSource>,
    hole_pop: Handle<AudioSource>,
    ice: Handle<AudioSource>,
    bolt: Handle<AudioSource>,
    fuss: Handle<AudioSource>,
    feather: Handle<AudioSource>,
    shrink: Handle<AudioSource>,
}

/// A 64x64 quad showing one rectangle of a creature skin (u0,v0)..(u1,v1).
fn face_quad(size: Vec2, uv: [f32; 4]) -> Mesh {
    let [u0, v0, u1, v1] = uv;
    Mesh::from(Rectangle::new(size.x, size.y)).with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[u1, v0], [u0, v0], [u0, v1], [u1, v1]],
    )
}

fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut texture = |path: &str, blend: bool| {
        materials.add(StandardMaterial {
            base_color_texture: Some(
                server
                    .load_builder()
                    .with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
                        s.sampler = bevy::image::ImageSampler::nearest()
                    })
                    .load(path.to_string()),
            ),
            unlit: true,
            alpha_mode: if blend {
                AlphaMode::Blend
            } else {
                AlphaMode::Mask(0.5)
            },
            ..default()
        })
    };
    let anvil = texture("textures/powers/anvil_side.png", false);
    let golem = texture("textures/powers/iron_golem.png", false);
    let bee = texture("textures/powers/bee.png", false);
    let pearl = texture("textures/powers/ender_pearl.png", true);
    // Black hole layers: additive glow and swirls around a black core.
    let mut glow = |path: &str, color: Color, strength: f32| {
        materials.add(StandardMaterial {
            base_color: color,
            base_color_texture: Some(server.load(path.to_string())),
            emissive: color.to_linear() * strength,
            unlit: true,
            alpha_mode: AlphaMode::Add,
            ..default()
        })
    };
    let hole = HoleArt {
        glow: glow(
            "textures/powers/blackhole_glow.png",
            Color::srgb(0.55, 0.15, 1.0),
            1.5,
        ),
        swirl: glow(
            "textures/powers/blackhole_swirl.png",
            Color::srgb(0.85, 0.25, 1.0),
            2.0,
        ),
        disk: glow(
            "textures/powers/blackhole_disk.png",
            Color::srgb(1.0, 0.55, 0.15),
            3.0,
        ),
        spark: server.load("textures/powers/blackhole_spark.png"),
        core: materials.add(StandardMaterial {
            base_color: Color::BLACK,
            unlit: true,
            ..default()
        }),
        sphere: meshes.add(Sphere::new(0.5).mesh().uv(32, 16)),
        disc: meshes.add(Circle::new(0.5)),
        ring: meshes.add(Annulus::new(0.46, 0.5)),
    };
    let sound = |name: &str| server.load(format!("audio/powers/{name}.ogg"));
    commands.insert_resource(Art {
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        anvil,
        // Iron golem skin is 128 units square: head front at (8,8)-(16,18).
        golem_face: meshes.add(face_quad(
            Vec2::new(64.0, 80.0),
            [0.0625, 0.0625, 0.125, 0.140625],
        )),
        golem,
        // Bee skin is 64 units square: body front at (10,10)-(17,17).
        bee_face: meshes.add(face_quad(
            Vec2::splat(30.0),
            [0.15625, 0.15625, 0.265625, 0.265625],
        )),
        bee,
        hole,
        pearl,
        dust: server.load("textures/powers/quake_dust.png"),
        sounds: Sounds {
            anvil: sound("anvil_land"),
            golem_hit: sound("golem_hit"),
            golem_summon: sound("golem_summon"),
            bee_loop: sound("bee_loop"),
            bee_sting: sound("bee_sting"),
            pearl: sound("pearl_teleport"),
            quake: sound("quake"),
            wind: sound("wind"),
            hole: sound("black_hole"),
            hole_pop: sound("black_hole_pop"),
            ice: sound("ice_age"),
            bolt: sound("bolt"),
            fuss: sound("fuss"),
            feather: sound("feather"),
            shrink: sound("shrink"),
        },
    });
}

fn play(commands: &mut Commands, clip: &Handle<AudioSource>, at: Vec3, volume: f32, speed: f32) {
    commands.spawn((
        AudioPlayer::new(clip.clone()),
        PlaybackSettings::DESPAWN
            .with_volume(bevy::audio::Volume::Linear(volume))
            .with_speed(speed),
        SoundAt(at.xy()),
    ));
}

fn announce(notice: &mut PowerNotice, text: String) {
    notice.text = text;
    notice.remaining = 3.5;
}

fn by(name: &str, user: &str) -> String {
    format!("{name}! (from {user})")
}

/// Meshes and materials for the black hole: core, glow, swirls and sparks.
struct HoleArt {
    core: Handle<StandardMaterial>,
    glow: Handle<StandardMaterial>,
    swirl: Handle<StandardMaterial>,
    disk: Handle<StandardMaterial>,
    spark: Handle<Image>,
    sphere: Handle<Mesh>,
    disc: Handle<Mesh>,
    ring: Handle<Mesh>,
}

/// A black hole layer that turns in place.
#[derive(Component)]
struct Spin(f32);

/// Matter spiralling into a black hole along its tilted disk.
#[derive(Component)]
struct Spiral {
    center: Vec3,
    angle: f32,
    radius: f32,
}

/// The accretion disk is tilted towards the camera: its height is squashed.
const DISK_TILT: f32 = 0.38;

/// Short-lived visual: fades out, optionally drifting and spinning.
#[derive(Component)]
struct Fx {
    life: f32,
    max: f32,
    velocity: Vec3,
    spin: f32,
    /// Scale gained per second (shockwaves and flashes grow as they fade).
    grow: f32,
    /// Downward pull per second (debris arcs and falls).
    gravity: f32,
}

#[allow(clippy::too_many_arguments)]
fn spawn_fx(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    mesh: &Handle<Mesh>,
    color: Color,
    at: Vec3,
    size: Vec2,
    rotation: f32,
    velocity: Vec3,
    life: f32,
) {
    let material = materials.add(StandardMaterial {
        base_color: color,
        emissive: color.to_linear() * 2.0,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.spawn((
        Fx {
            life,
            max: life,
            velocity,
            spin: 0.0,
            grow: 0.0,
            gravity: 0.0,
        },
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material),
        Transform::from_translation(at)
            .with_rotation(Quat::from_rotation_z(rotation))
            .with_scale(size.extend(1.0)),
        bevy::light::NotShadowCaster,
    ));
}

fn fade_fx(
    mut commands: Commands,
    time: Res<Time>,
    mut fx: Query<(
        Entity,
        &mut Fx,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    for (e, mut f, mut t, material) in &mut fx {
        f.life -= dt;
        if f.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        f.velocity.y -= f.gravity * dt;
        t.translation += f.velocity * dt;
        t.rotate_z(f.spin * dt);
        if f.grow != 0.0 {
            let g = 1.0 + f.grow * dt;
            t.scale.x *= g;
            t.scale.y *= g;
        }
        if let Some(mut m) = materials.get_mut(&material.0) {
            m.base_color.set_alpha((f.life / f.max).clamp(0.0, 1.0));
        }
    }
}

// --------------------------------------------------------- timed effects

/// FEATHER: low gravity until `until`; `base` is restored afterwards.
#[derive(Component)]
pub struct Feather {
    until: f32,
    base: f32,
}

/// SHRINK: half-size head until `until` (scaling lives in buff_core).
#[derive(Component)]
pub struct Shrunk {
    pub until: f32,
}

/// WIND: one gusty blow. Strength, direction and length vary per use.
#[derive(Resource, Default)]
struct Wind {
    start: f32,
    until: f32,
    /// Mostly sideways, sometimes with a little lift or downdraft.
    direction: Vec2,
    /// Sideways speed it pushes bodies towards at full gust.
    strength: f32,
    /// Offsets the gust rhythm so no two winds pulse alike.
    phase: f32,
}

/// One kind of wind: how hard, how long and how it sounds.
#[derive(Clone, Copy)]
struct WindKind {
    name: &'static str,
    speed: f32,
    seconds: (f32, f32),
    volume: f32,
    pitch: f32,
}

const WIND_KINDS: [WindKind; 3] = [
    WindKind {
        name: "breeze",
        speed: 260.0,
        seconds: (7.0, 9.0),
        volume: 0.5,
        pitch: 1.15,
    },
    WindKind {
        name: "strong",
        speed: 480.0,
        seconds: (9.0, 12.0),
        volume: 0.8,
        pitch: 0.9,
    },
    WindKind {
        name: "STORM",
        speed: 720.0,
        seconds: (11.0, 14.0),
        volume: 1.0,
        pitch: 0.7,
    },
];

impl Wind {
    /// 0..1: builds up over 0.4 s, surges and eases, dies down over the last 0.6 s.
    fn gust(&self, now: f32) -> f32 {
        let ramp = ((now - self.start) / 0.4)
            .min((self.until - now) / 0.6)
            .clamp(0.0, 1.0);
        let t = now - self.start + self.phase;
        let surge = 0.78 + 0.22 * (t * 2.3).sin() + 0.12 * (t * 5.7).sin();
        (ramp * surge).clamp(0.0, 1.0)
    }
}

#[derive(Resource, Default)]
pub struct IceAge {
    until: f32,
    next_grip: f32,
}

impl IceAge {
    /// Whether the ice age is on: terrain landings then keep heads on ice.
    pub fn active(&self, now: f32) -> bool {
        now < self.until
    }
}

/// EARTHQUAKE: pulses every [`QUAKE_PULSE`] seconds until `until`.
#[derive(Resource, Default)]
struct Quake {
    until: f32,
    next_pulse: f32,
}

const QUAKE_SECONDS: f32 = 4.0;
const QUAKE_PULSE: f32 = 0.5;
const QUAKE_RADIUS: f32 = 520.0;

/// Screen shake, added on top of the camera follow (`camera_follow_action_center`).
#[derive(Resource, Default)]
pub struct CameraShake {
    until: f32,
    length: f32,
    strength: f32,
    /// Offset added to the camera last frame, removed before the next follow step.
    pub applied: Vec2,
}

impl CameraShake {
    pub fn start(&mut self, now: f32, seconds: f32, strength: f32) {
        self.until = now + seconds;
        self.length = seconds;
        self.strength = strength;
    }

    /// This frame's offset in world units; it fades out over the shake.
    pub fn offset(&self, now: f32) -> Vec2 {
        if now >= self.until || self.length <= 0.0 {
            return Vec2::ZERO;
        }
        let fade = (self.until - now) / self.length;
        let wobble = Vec2::new(
            (now * 47.0).sin() + (now * 83.0).sin() * 0.5,
            (now * 59.0).cos() + (now * 97.0).sin() * 0.5,
        );
        wobble * self.strength * fade / 1.5
    }
}

/// FUSS: mirrored hero controls until `until` (read by `hero_movement`).
#[derive(Resource, Default)]
pub struct Fuss {
    pub until: f32,
    pub active: bool,
}

impl Fuss {
    /// -1 while controls are mirrored, otherwise 1.
    pub fn steer_sign(&self, now: f32) -> f32 {
        if now < self.until { -1.0 } else { 1.0 }
    }
}

const FEATHER_GRAVITY: f32 = 0.35;
const FEATHER_SECONDS: f32 = 8.0;
const SHRINK_SECONDS: f32 = 10.0;
const ICE_AGE_SECONDS: f32 = 15.0;
/// Water within this distance of the hero freezes when an ice age starts.
const ICE_AGE_FREEZE_RADIUS: f32 = 1600.0;
const FUSS_SECONDS: f32 = 5.0;

type Heads = Or<(With<Hero>, With<Villager>)>;
type Actors = Or<(With<Hero>, With<Villager>, With<Enemy>)>;

/// FEATHER, SHRINK, WIND, ICEAGE and FUSS: timers and one-off kicks.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn start_simple(
    mut commands: Commands,
    mut powers: MessageReader<PowerActivated>,
    time: Res<Time>,
    art: Res<Art>,
    mut notice: ResMut<PowerNotice>,
    mut wind: ResMut<Wind>,
    mut ice: ResMut<IceAge>,
    mut fuss_state: ResMut<Fuss>,
    actors: Query<
        (
            Entity,
            &Transform,
            Option<&GravityScale>,
            Option<&Feather>,
            Has<Hero>,
        ),
        (Actors, Without<Flying>),
    >,
    friends: Query<Entity, Heads>,
    mut weather: ResMut<crate::components::weather::ActiveWeather>,
    modes: Option<Res<crate::systems::game_modes::GameModes>>,
    mut freeze: MessageWriter<crate::systems::fluids::FreezeWaterNear>,
) {
    let now = time.elapsed_secs();
    let hero_at = actors
        .iter()
        .find(|a| a.4)
        .map_or(Vec3::ZERO, |a| a.1.translation);
    for power in powers.read() {
        let user = &power.command.username;
        match power.command.command_type {
            P::Feather => {
                for (e, _, gravity, feather, _) in &actors {
                    let base = feather.map_or(gravity.map_or(1.5, |g| g.0), |f| f.base);
                    commands.entity(e).try_insert(Feather {
                        until: now + FEATHER_SECONDS,
                        base,
                    });
                }
                play(&mut commands, &art.sounds.feather, hero_at, 0.6, 1.0);
                announce(&mut notice, by("FEATHER: low gravity", user));
            }
            P::Shrink => {
                for e in &friends {
                    commands.entity(e).try_insert(Shrunk {
                        until: now + SHRINK_SECONDS,
                    });
                }
                play(&mut commands, &art.sounds.shrink, hero_at, 0.8, 1.4);
                announce(&mut notice, by("SHRINK: tiny heads", user));
            }
            P::Wind => {
                let mut rng = rand::rng();
                // Breezes and strong winds are common, storms rarer.
                let roll = rng.random_range(0.0..1.0);
                let WindKind {
                    name,
                    speed,
                    seconds: (shortest, longest),
                    volume,
                    pitch,
                } = WIND_KINDS[if roll < 0.4 {
                    0
                } else if roll < 0.8 {
                    1
                } else {
                    2
                }];
                let side = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                let lift = if rng.random_bool(0.3) {
                    rng.random_range(-0.15..0.3)
                } else {
                    0.0
                };
                *wind = Wind {
                    start: now,
                    until: now + rng.random_range(shortest..longest),
                    direction: Vec2::new(side, lift).normalize(),
                    strength: speed,
                    phase: rng.random_range(0.0..std::f32::consts::TAU),
                };
                play(&mut commands, &art.sounds.wind, hero_at, volume, pitch);
                let arrow = if side > 0.0 { ">>>" } else { "<<<" };
                announce(&mut notice, by(&format!("WIND: {name} {arrow}"), user));
            }
            P::IceAge => {
                ice.until = now + ICE_AGE_SECONDS;
                ice.next_grip = 0.0;
                if crate::systems::game_modes::is_on(
                    modes.as_deref(),
                    crate::systems::game_modes::GameMode::Weather,
                ) {
                    weather.weather_type = crate::components::weather::WeatherType::Snow;
                    weather.intensity = 0.85;
                    weather.duration_remaining = ICE_AGE_SECONDS;
                }
                // Lakes and pools freeze over for the whole ice age (10 steps a second).
                freeze.write(crate::systems::fluids::FreezeWaterNear {
                    center: hero_at.xy(),
                    radius: ICE_AGE_FREEZE_RADIUS,
                    ticks: (ICE_AGE_SECONDS * 10.0) as u16 + 30,
                });
                play(&mut commands, &art.sounds.ice, hero_at, 0.7, 0.8);
                announce(
                    &mut notice,
                    by("ICE AGE: slippery floors, frozen water", user),
                );
            }
            P::Fuss => {
                fuss_state.until = now + FUSS_SECONDS;
                play(&mut commands, &art.sounds.fuss, hero_at, 0.6, 1.0);
                announce(&mut notice, by("FUSS: controls mirrored", user));
            }
            _ => {}
        }
    }
}

fn feather_gravity(
    mut commands: Commands,
    time: Res<Time>,
    mut actors: Query<(Entity, &Feather, &mut GravityScale)>,
) {
    let now = time.elapsed_secs();
    for (e, feather, mut gravity) in &mut actors {
        if now >= feather.until {
            gravity.0 = feather.base;
            commands.entity(e).try_remove::<Feather>();
        } else if gravity.0 != feather.base * FEATHER_GRAVITY {
            gravity.0 = feather.base * FEATHER_GRAVITY;
        }
    }
}

fn blow_wind(
    time: Res<Time>,
    wind: Res<Wind>,
    art: Res<Art>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut actors: Query<&mut LinearVelocity, (Actors, Without<Flying>)>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
) {
    let now = time.elapsed_secs();
    if now >= wind.until || now < wind.start {
        return;
    }
    let dt = time.delta_secs();
    let gust = wind.gust(now);
    let push = wind.direction * wind.strength * gust;
    for mut v in &mut actors {
        v.x += (push.x - v.x) * (dt * 3.0).min(1.0);
        // Lift or downdraft nudges, never cancels a jump or a fall outright.
        v.y += push.y * dt * 2.5;
    }
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let c = camera.translation();
    let mut rng = rand::rng();
    let power = wind.strength / 720.0 * gust;
    // Streaks across the view: more, longer and faster the harder it blows.
    let streaks = power * 2.4;
    let count = streaks as u32 + u32::from(rng.random_bool(f64::from(streaks.fract())));
    for _ in 0..count {
        let at = Vec3::new(
            c.x - wind.direction.x * 760.0,
            c.y + rng.random_range(-620.0..620.0),
            60.0,
        );
        let speed = 1300.0 + 1300.0 * power;
        spawn_fx(
            &mut commands,
            &mut materials,
            &art.quad,
            Color::srgba(0.9, 0.95, 1.0, 0.35 + 0.35 * power),
            at,
            Vec2::new(
                rng.random_range(50.0..110.0) * (0.7 + power),
                2.0 + 2.0 * power,
            ),
            wind.direction.y.atan2(wind.direction.x.abs()) * wind.direction.x.signum(),
            wind.direction.extend(0.0) * speed,
            0.9,
        );
    }
    // Strong winds and storms carry leaves and dust along.
    if power > 0.45 && rng.random_bool(f64::from(power * 0.5)) {
        let leaf = rng.random_bool(0.6);
        let color = if leaf {
            Color::srgba(0.35, rng.random_range(0.55..0.75), 0.2, 0.95)
        } else {
            Color::srgba(0.6, 0.5, 0.35, 0.6)
        };
        let material = materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let at = Vec3::new(
            c.x - wind.direction.x * 760.0,
            c.y + rng.random_range(-500.0..500.0),
            55.0,
        );
        let size = if leaf {
            Vec3::new(10.0, 6.0, 1.0)
        } else {
            Vec3::splat(rng.random_range(18.0..34.0))
        };
        commands.spawn((
            Fx {
                life: 1.6,
                max: 1.6,
                velocity: (wind.direction * wind.strength * rng.random_range(1.1..1.6)).extend(0.0)
                    + Vec3::Y * rng.random_range(-60.0..80.0),
                spin: if leaf {
                    rng.random_range(-9.0..9.0)
                } else {
                    0.5
                },
                grow: if leaf { 0.0 } else { 0.6 },
                gravity: if leaf { 60.0 } else { 0.0 },
            },
            Mesh3d(art.quad.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(at)
                .with_rotation(Quat::from_rotation_z(rng.random_range(0.0..6.0)))
                .with_scale(size),
            bevy::light::NotShadowCaster,
        ));
    }
}

fn ice_age(
    time: Res<Time<Virtual>>,
    mut ice: ResMut<IceAge>,
    mut commands: Commands,
    heads: Query<Entity, Heads>,
) {
    let now = time.elapsed_secs();
    if now >= ice.until || now < ice.next_grip {
        return;
    }
    // Ice grip lasts 1 s; refresh it twice a second while the ice age lasts.
    ice.next_grip = now + 0.5;
    for e in &heads {
        commands
            .entity(e)
            .try_insert(crate::components::surface_grip::SurfaceGrip::ice(now));
    }
}

fn fuss(
    time: Res<Time>,
    mut state: ResMut<Fuss>,
    mut heroes: Query<&mut Hero>,
    mut villagers: Query<&mut Villager>,
) {
    let active = time.elapsed_secs() < state.until;
    if active != state.active {
        // AI heads turn around when the fuss starts and again when it ends.
        state.active = active;
        for mut hero in &mut heroes {
            hero.move_direction = -hero.move_direction;
        }
        for mut villager in &mut villagers {
            villager.move_direction = -villager.move_direction;
        }
    }
}

// ------------------------------------------------- ANVIL, GOLEM, HOLE, BEES

#[derive(Component)]
struct Anvil {
    velocity: f32,
    broken: u8,
    fallen: f32,
    hit: HashSet<Entity>,
    landed: Option<f32>,
}

#[derive(Component)]
struct Golem {
    until: f32,
    punch_ready: f32,
}

#[derive(Component)]
struct BlackHole {
    born: f32,
    pop_at: f32,
}

/// Seconds a black hole takes to open and to collapse before it pops.
const HOLE_OPEN: f32 = 0.5;
const HOLE_CLOSE: f32 = 0.35;
/// Core size in world units when fully open.
const HOLE_SIZE: f32 = 64.0;

/// `hum` loops on the hole for its whole life and stops when it pops (the clip
/// is shorter than the hole lasts).
fn spawn_black_hole(
    commands: &mut Commands,
    art: &HoleArt,
    hum: &Handle<AudioSource>,
    at: Vec3,
    now: f32,
) {
    let layer = |mesh: &Handle<Mesh>, material: &Handle<StandardMaterial>, size: f32, z: f32| {
        (
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0.0, 0.0, z).with_scale(Vec3::splat(size)),
            bevy::light::NotShadowCaster,
        )
    };
    commands
        .spawn((
            BlackHole {
                born: now,
                pop_at: now + HOLE_SECONDS,
            },
            Transform::from_translation(at).with_scale(Vec3::ZERO),
            Visibility::Inherited,
        ))
        .with_children(|hole| {
            // Child offsets are scaled by the hole's size, so depth steps stay tiny.
            // Purple halo, then a magenta swirl turning against the disk. Each swirl
            // texture is a crescent: two half a turn apart make a full ring.
            hole.spawn(layer(&art.disc, &art.glow, 4.4, -0.03));
            for start in [0.0, std::f32::consts::PI] {
                let mut swirl = layer(&art.disc, &art.swirl, 3.2, -0.02);
                swirl.2.rotate_z(start);
                hole.spawn((swirl, Spin(-1.6)));
            }
            // Hot accretion disk, tilted towards the camera.
            hole.spawn((
                Transform::from_xyz(0.0, 0.0, -0.01)
                    .with_rotation(Quat::from_rotation_z(0.22))
                    .with_scale(Vec3::new(1.0, DISK_TILT, 1.0)),
                Visibility::Inherited,
            ))
            .with_children(|disk| {
                for start in [0.0, std::f32::consts::PI] {
                    let mut ring = layer(&art.disc, &art.disk, 3.0, 0.0);
                    ring.2.rotate_z(start);
                    disk.spawn((ring, Spin(3.2)));
                }
            });
            // The black core sits in front of everything it swallows.
            hole.spawn(layer(&art.sphere, &art.core, 1.0, 0.02));
            hole.spawn((
                AudioPlayer::new(hum.clone()),
                PlaybackSettings::LOOP
                    .with_volume(bevy::audio::Volume::Linear(0.8))
                    .with_speed(0.7),
                SoundAt(at.xy()),
            ));
        });
}

#[derive(Component)]
struct Bee {
    target: Entity,
    until: f32,
    sting_ready: f32,
    dazed_until: f32,
    phase: f32,
}

const ANVIL_MAX_BLOCKS: u8 = 6;
const GOLEM_SECONDS: f32 = 30.0;
const HOLE_SECONDS: f32 = 10.0;
const HOLE_RADIUS: f32 = 800.0;
const BEE_SECONDS: f32 = 12.0;

#[allow(clippy::too_many_arguments)]
fn start_spawned(
    mut commands: Commands,
    mut powers: MessageReader<PowerActivated>,
    time: Res<Time>,
    art: Res<Art>,
    mut notice: ResMut<PowerNotice>,
    hero: Query<&Transform, With<Hero>>,
    friends: Query<Entity, Heads>,
) {
    let Ok(hero) = hero.single() else {
        return;
    };
    let now = time.elapsed_secs();
    let at = hero.translation;
    let mut rng = rand::rng();
    for power in powers.read() {
        let user = &power.command.username;
        match power.command.command_type {
            P::Anvil => {
                for dx in [-160.0, 0.0, 160.0] {
                    commands.spawn((
                        Anvil {
                            velocity: 300.0,
                            broken: 0,
                            fallen: 0.0,
                            hit: HashSet::new(),
                            landed: None,
                        },
                        Mesh3d(art.cube.clone()),
                        MeshMaterial3d(art.anvil.clone()),
                        Transform::from_translation(Vec3::new(at.x + dx, at.y + 650.0, 20.0))
                            .with_scale(Vec3::new(60.0, 44.0, 60.0)),
                    ));
                }
                announce(&mut notice, by("ANVIL", user));
            }
            P::Golem => {
                let p = Vec3::new(at.x + 90.0, at.y + 60.0, at.z);
                commands.spawn((
                    Villager {
                        ai_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
                        move_direction: 1.0,
                        direction_change_timer: Timer::from_seconds(99.0, TimerMode::Repeating),
                    },
                    Golem {
                        until: now + GOLEM_SECONDS,
                        punch_ready: now,
                    },
                    Health::new(20),
                    Buffable,
                    avian_physics::character(),
                    Mesh3d(art.golem_face.clone()),
                    MeshMaterial3d(art.golem.clone()),
                    Transform::from_translation(p).with_scale(Vec3::splat(1.2)),
                ));
                play(&mut commands, &art.sounds.golem_summon, p, 0.8, 1.0);
                announce(&mut notice, by("IRON GOLEM joins", user));
            }
            P::BlackHole => {
                let side = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                // Well away from the hero (330-440 px) but still on screen: it pulls
                // from a distance, not on top of them.
                let p = Vec3::new(
                    at.x + side * rng.random_range(200.0..280.0),
                    at.y + rng.random_range(260.0..340.0),
                    40.0,
                );
                spawn_black_hole(&mut commands, &art.hole, &art.sounds.hole, p, now);
                announce(&mut notice, by("BLACK HOLE", user));
            }
            P::Bees => {
                let targets: Vec<Entity> = friends.iter().collect();
                if targets.is_empty() {
                    continue;
                }
                let target = targets[rng.random_range(0..targets.len())];
                for i in 0..6 {
                    let angle = i as f32 * std::f32::consts::TAU / 6.0;
                    commands.spawn((
                        Bee {
                            target,
                            until: now + BEE_SECONDS,
                            sting_ready: now + 1.0,
                            dazed_until: 0.0,
                            phase: angle,
                        },
                        Mesh3d(art.bee_face.clone()),
                        MeshMaterial3d(art.bee.clone()),
                        Transform::from_translation(
                            at + Vec3::new(angle.cos() * 400.0, 300.0 + angle.sin() * 120.0, 50.0),
                        ),
                    ));
                }
                play(&mut commands, &art.sounds.bee_loop, at, 0.7, 1.0);
                announce(&mut notice, by("BEES", user));
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn fall_anvils(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<Art>,
    grid: Res<SpatialHash>,
    mut anvils: Query<(Entity, &mut Anvil, &mut Transform)>,
    blocks: Query<
        (
            &Transform,
            &BlockType,
            Option<&crate::systems::biome_content::Tile>,
        ),
        (With<Block>, Without<Anvil>),
    >,
    mut actors: Query<
        (Entity, &Transform, &mut LinearVelocity, Has<Enemy>),
        (Actors, Without<Anvil>),
    >,
    mut destroyed: MessageWriter<BlockDestroyed>,
    mut damage: MessageWriter<EntityDamaged>,
    mut broken: Local<HashSet<Entity>>,
) {
    broken.clear();
    let now = time.elapsed_secs();
    let dt = time.delta_secs();
    for (e, mut anvil, mut t) in &mut anvils {
        if let Some(landed) = anvil.landed {
            if now - landed > 1.0 {
                commands.entity(e).despawn();
            }
            continue;
        }
        anvil.velocity = (anvil.velocity + 1400.0 * dt).min(1100.0);
        let step = anvil.velocity * dt;
        t.translation.y -= step;
        anvil.fallen += step;
        let p = t.translation;
        // Smash blocks it falls into, up to a limit, through the normal path.
        let reach = Vec2::new(30.0, 22.0) + Vec2::splat(32.0);
        for block in grid.rect_iter(p.xy() - reach, p.xy() + reach) {
            if anvil.broken >= ANVIL_MAX_BLOCKS {
                break;
            }
            let Ok((bt, kind, tile)) = blocks.get(block) else {
                continue;
            };
            let d = (bt.translation - p).xy().abs();
            if d.x < 62.0 && d.y < 54.0 {
                crate::plugins::gameplay_plugin::break_block(
                    &mut commands,
                    &mut destroyed,
                    &mut broken,
                    block,
                    *kind,
                    bt.translation,
                    tile.copied(),
                );
                anvil.broken += 1;
            }
        }
        // Bonk whoever is under it.
        for (actor, at, mut v, enemy) in &mut actors {
            if (at.translation - p)
                .xy()
                .abs()
                .cmplt(Vec2::new(62.0, 56.0))
                .all()
                && anvil.hit.insert(actor)
            {
                v.y = v.y.min(-450.0);
                if enemy {
                    damage.write(EntityDamaged {
                        entity: actor,
                        damage: 2,
                    });
                }
            }
        }
        if anvil.broken >= ANVIL_MAX_BLOCKS || anvil.fallen > 1600.0 {
            anvil.landed = Some(now);
            play(&mut commands, &art.sounds.anvil, p, 0.7, 1.0);
        }
    }
}

#[allow(clippy::type_complexity)]
fn golem_ai(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<Art>,
    mut golems: Query<(Entity, &mut Golem, &Transform, &mut LinearVelocity), Without<Enemy>>,
    mut enemies: Query<(Entity, &Transform, &mut LinearVelocity), (With<Enemy>, Without<Golem>)>,
    hero: Query<&Transform, (With<Hero>, Without<Golem>)>,
    mut damage: MessageWriter<EntityDamaged>,
) {
    let now = time.elapsed_secs();
    let hero = hero.single().ok().map(|t| t.translation);
    for (e, mut golem, t, mut v) in &mut golems {
        if now >= golem.until {
            commands.entity(e).despawn();
            continue;
        }
        let p = t.translation;
        let nearest = enemies
            .iter()
            .filter(|(_, et, _)| et.translation.distance(p) < 420.0)
            .min_by(|a, b| {
                a.1.translation
                    .distance_squared(p)
                    .total_cmp(&b.1.translation.distance_squared(p))
            })
            .map(|(id, et, _)| (id, et.translation));
        // Walk to the nearest enemy, otherwise stay near the hero.
        let goal = nearest.map(|n| n.1).or(hero);
        if let Some(goal) = goal {
            let dx = goal.x - p.x;
            if dx.abs() > 50.0 {
                v.x = dx.signum() * 240.0;
            }
        }
        if let Some((target, at)) = nearest
            && at.distance(p) < 100.0
            && now >= golem.punch_ready
        {
            golem.punch_ready = now + 1.0;
            damage.write(EntityDamaged {
                entity: target,
                damage: 2,
            });
            if let Ok((_, _, mut ev)) = enemies.get_mut(target) {
                let away = (at - p).x.signum();
                ev.x = away * 520.0;
                ev.y = 380.0;
            }
            play(&mut commands, &art.sounds.golem_hit, p, 0.8, 1.0);
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn black_holes(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<Art>,
    mut holes: Query<(Entity, &BlackHole, &mut Transform)>,
    mut bodies: Query<
        (&Transform, &mut LinearVelocity),
        (
            Or<(With<Hero>, With<Villager>, With<Enemy>, With<Drop>)>,
            Without<BlackHole>,
        ),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let now = time.elapsed_secs();
    let dt = time.delta_secs();
    let mut rng = rand::rng();
    for (e, hole, mut t) in &mut holes {
        let center = t.translation;
        if now >= hole.pop_at {
            for (bt, mut v) in &mut bodies {
                let d = bt.translation - center;
                if d.length() < HOLE_RADIUS {
                    let away = d.xy().normalize_or(Vec2::Y);
                    v.x = away.x * 550.0;
                    v.y = away.y * 550.0 + 250.0;
                }
            }
            play(&mut commands, &art.sounds.hole_pop, center, 0.9, 0.8);
            pop_black_hole(&mut commands, &mut materials, &art, center);
            commands.entity(e).despawn();
            continue;
        }
        // Pull everything nearby towards the centre.
        for (bt, mut v) in &mut bodies {
            let d = center - bt.translation;
            let distance = d.length();
            if distance < HOLE_RADIUS && distance > 1.0 {
                let pull = d.xy().normalize() * 1100.0 * dt;
                v.x = (v.x + pull.x).clamp(-650.0, 650.0);
                v.y = (v.y + pull.y).clamp(-650.0, 650.0);
            }
        }
        // Open, breathe, then collapse into the pop.
        let opening = ((now - hole.born) / HOLE_OPEN).clamp(0.0, 1.0);
        let closing = ((hole.pop_at - now) / HOLE_CLOSE).clamp(0.0, 1.0);
        let ease = |x: f32| 1.0 - (1.0 - x).powi(3);
        let breathe = 1.0 + (now * 7.0).sin() * 0.04;
        t.scale = Vec3::splat(HOLE_SIZE * ease(opening) * ease(closing) * breathe);
        // Matter streams in along the disk while it is open.
        if closing >= 1.0 {
            for _ in 0..2 {
                let angle = rng.random_range(0.0..std::f32::consts::TAU);
                let color = if rng.random_bool(0.35) {
                    Color::srgba(1.0, 0.7, 0.35, 0.95)
                } else {
                    Color::srgba(0.7, 0.3, 1.0, 0.9)
                };
                let material = materials.add(StandardMaterial {
                    base_color: color,
                    base_color_texture: Some(art.hole.spark.clone()),
                    emissive: color.to_linear() * 3.0,
                    unlit: true,
                    alpha_mode: AlphaMode::Add,
                    ..default()
                });
                commands.spawn((
                    Spiral {
                        center,
                        angle,
                        radius: rng.random_range(260.0..420.0),
                    },
                    Mesh3d(art.quad.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(center).with_scale(Vec3::new(26.0, 6.0, 1.0)),
                    bevy::light::NotShadowCaster,
                ));
            }
        }
    }
}

/// One additive flash or ring that grows while it fades.
#[allow(clippy::too_many_arguments)]
fn spawn_burst(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    mesh: &Handle<Mesh>,
    color: Color,
    center: Vec3,
    size: f32,
    life: f32,
    grow: f32,
) {
    let material = materials.add(StandardMaterial {
        base_color: color,
        emissive: color.to_linear() * 4.0,
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    });
    commands.spawn((
        Fx {
            life,
            max: life,
            velocity: Vec3::ZERO,
            spin: 0.0,
            grow,
            gravity: 0.0,
        },
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material),
        Transform::from_translation(center + Vec3::Z * 2.0).with_scale(Vec3::splat(size)),
        bevy::light::NotShadowCaster,
    ));
}

/// Flash, shockwave ring over the pull radius and a ring of sparks.
fn pop_black_hole(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    art: &Art,
    center: Vec3,
) {
    let flash = Color::srgba(1.0, 0.9, 1.0, 1.0);
    spawn_burst(
        commands,
        materials,
        &art.hole.disc,
        flash,
        center,
        90.0,
        0.25,
        9.0,
    );
    // The ring reaches the pull radius as it fades.
    let ring_life = 0.55;
    let ring_grow = ((HOLE_RADIUS * 2.0 / 40.0).powf(1.0 / (ring_life * 60.0)) - 1.0) * 60.0;
    let ring = Color::srgba(0.75, 0.4, 1.0, 0.9);
    spawn_burst(
        commands,
        materials,
        &art.hole.ring,
        ring,
        center,
        40.0,
        ring_life,
        ring_grow,
    );
    for i in 0..24 {
        let a = i as f32 * std::f32::consts::TAU / 24.0;
        let color = if i % 3 == 0 {
            Color::srgba(1.0, 0.75, 0.4, 0.95)
        } else {
            Color::srgba(0.75, 0.35, 1.0, 0.9)
        };
        spawn_fx(
            commands,
            materials,
            &art.quad,
            color,
            center,
            Vec2::new(30.0, 6.0),
            a,
            Vec3::new(a.cos(), a.sin(), 0.0) * 820.0,
            0.6,
        );
    }
}

/// Turns the black hole's swirl layers.
fn spin_parts(time: Res<Time>, mut parts: Query<(&Spin, &mut Transform)>) {
    for (spin, mut t) in &mut parts {
        t.rotate_z(spin.0 * time.delta_secs());
    }
}

/// Moves matter around and into the hole, faster and hotter as it falls in.
fn spiral_matter(
    mut commands: Commands,
    time: Res<Time>,
    holes: Query<&Transform, (With<BlackHole>, Without<Spiral>)>,
    mut matter: Query<(
        Entity,
        &mut Spiral,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    for (e, mut s, mut t, material) in &mut matter {
        // Follow the hole; when it pops the stream ends with it.
        let Some(hole) = holes
            .iter()
            .find(|h| h.translation.distance(s.center) < 1.0)
        else {
            commands.entity(e).despawn();
            continue;
        };
        s.angle += dt * (2.0 + 700.0 / s.radius.max(20.0));
        s.radius -= dt * (90.0 + 26000.0 / s.radius.max(20.0));
        if s.radius < HOLE_SIZE * 0.45 {
            commands.entity(e).despawn();
            continue;
        }
        let offset = Vec2::new(s.angle.cos(), s.angle.sin() * DISK_TILT) * s.radius;
        t.translation = hole.translation + offset.extend(0.5);
        // Point along the orbit.
        let tangent = Vec2::new(-s.angle.sin(), s.angle.cos() * DISK_TILT);
        t.rotation = Quat::from_rotation_z(tangent.y.atan2(tangent.x));
        let heat = (1.0 - s.radius / 420.0).clamp(0.0, 1.0);
        t.scale = Vec3::new(18.0 + 22.0 * heat, 5.0 + 2.0 * heat, 1.0);
        if let Some(mut m) = materials.get_mut(&material.0) {
            let near = Color::srgba(1.0, 0.85, 0.6, 1.0).to_linear();
            let far = Color::srgba(0.65, 0.25, 1.0, 0.85).to_linear();
            let mixed = far.mix(&near, heat);
            m.base_color = mixed.into();
            m.emissive = mixed * (2.0 + 3.0 * heat);
        }
    }
}

#[allow(clippy::type_complexity)]
fn bees(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<Art>,
    mut swarm: Query<(Entity, &mut Bee, &mut Transform)>,
    targets: Query<(&Transform, &LinearVelocity), (Heads, Without<Bee>)>,
    mut damage: MessageWriter<EntityDamaged>,
) {
    let now = time.elapsed_secs();
    let dt = time.delta_secs();
    for (e, mut bee, mut t) in &mut swarm {
        let Ok((target, target_v)) = targets.get(bee.target) else {
            commands.entity(e).despawn();
            continue;
        };
        if now >= bee.until {
            commands.entity(e).despawn();
            continue;
        }
        // A big bounce shakes the swarm off for a moment.
        if target_v.y > 380.0 {
            bee.dazed_until = now + 1.2;
        }
        bee.phase += dt * 7.0;
        let wobble = Vec3::new(bee.phase.cos() * 40.0, (bee.phase * 1.7).sin() * 30.0, 0.0);
        let goal = if now < bee.dazed_until {
            t.translation + Vec3::Y * 60.0 + wobble
        } else {
            target.translation + wobble
        };
        let speed = if now < bee.dazed_until { 120.0 } else { 420.0 };
        let step = (goal - t.translation).xy().clamp_length_max(speed * dt);
        t.translation += step.extend(0.0);
        t.translation.z = 50.0;
        // Wings: quick flap.
        t.scale = Vec3::new(1.0, 1.0 + (now * 40.0).sin() * 0.12, 1.0);
        if now >= bee.sting_ready
            && now >= bee.dazed_until
            && t.translation.xy().distance(target.translation.xy()) < 40.0
        {
            bee.sting_ready = now + 1.5;
            damage.write(EntityDamaged {
                entity: bee.target,
                damage: 1,
            });
            play(
                &mut commands,
                &art.sounds.bee_sting,
                t.translation,
                0.6,
                1.0,
            );
        }
    }
}

// ------------------------------------------------------ PEARL, QUAKE, BOLT

/// Free spot for the pearl: no block within a head's reach of `p`.
fn spot_is_free(grid: &SpatialHash, blocks: &Query<&Transform, With<Block>>, p: Vec2) -> bool {
    grid.rect_iter(p - Vec2::splat(72.0), p + Vec2::splat(72.0))
        .filter_map(|e| blocks.get(e).ok())
        .all(|b| (b.translation.xy() - p).abs().max_element() >= 70.0)
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn start_pearl(
    mut commands: Commands,
    mut powers: MessageReader<PowerActivated>,
    time: Res<Time>,
    art: Res<Art>,
    grid: Res<SpatialHash>,
    blocks: Query<&Transform, With<Block>>,
    hero: Query<(Entity, &Transform), (With<Hero>, Without<Block>)>,
    mut notice: ResMut<PowerNotice>,
) {
    for power in powers.read() {
        if power.command.command_type != P::Pearl {
            continue;
        }
        let Ok((hero, t)) = hero.single() else {
            continue;
        };
        let from = t.translation;
        let mut rng = rand::rng();
        let spot = (0..40)
            .map(|_| {
                let a = rng.random_range(0.0..std::f32::consts::TAU);
                let r = rng.random_range(192.0..640.0);
                from.xy() + Vec2::new(a.cos(), a.sin()) * r
            })
            .find(|p| spot_is_free(&grid, &blocks, *p));
        let Some(to) = spot else {
            announce(&mut notice, "PEARL found no free spot".into());
            continue;
        };
        // The pearl flies first; the hero follows when it lands.
        commands.spawn((
            PearlFlight {
                hero,
                from,
                to: to.extend(from.z),
                born: time.elapsed_secs(),
            },
            Mesh3d(art.quad.clone()),
            MeshMaterial3d(art.pearl.clone()),
            Transform::from_translation(from + Vec3::Z * 45.0).with_scale(Vec3::splat(26.0)),
            bevy::light::NotShadowCaster,
        ));
        announce(&mut notice, by("ENDER PEARL", &power.command.username));
    }
}

/// An ender pearl in flight; the hero teleports where it lands.
#[derive(Component)]
struct PearlFlight {
    hero: Entity,
    from: Vec3,
    to: Vec3,
    born: f32,
}

const PEARL_FLIGHT: f32 = 0.45;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn pearl_flight(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<Art>,
    mut pearls: Query<(Entity, &PearlFlight, &mut Transform), Without<Hero>>,
    mut heroes: Query<(&mut Transform, &mut Position, &mut LinearVelocity), With<Hero>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let now = time.elapsed_secs();
    let mut rng = rand::rng();
    let ender = |rng: &mut rand::rngs::ThreadRng| {
        if rng.random_bool(0.5) {
            Color::srgba(0.75, 0.3, 1.0, 0.9)
        } else {
            Color::srgba(0.25, 0.95, 0.75, 0.9)
        }
    };
    for (e, pearl, mut t) in &mut pearls {
        let k = ((now - pearl.born) / PEARL_FLIGHT).clamp(0.0, 1.0);
        let arc = 90.0 + pearl.from.distance(pearl.to) * 0.25;
        let at = pearl.from.lerp(pearl.to, k) + Vec3::Y * arc * 4.0 * k * (1.0 - k);
        t.translation = at + Vec3::Z * 45.0;
        t.rotate_z(time.delta_secs() * 12.0);
        // Sparkling trail behind the pearl.
        for _ in 0..2 {
            let jitter = Vec3::new(
                rng.random_range(-8.0..8.0),
                rng.random_range(-8.0..8.0),
                40.0,
            );
            spawn_fx(
                &mut commands,
                &mut materials,
                &art.quad,
                ender(&mut rng),
                at + jitter,
                Vec2::splat(rng.random_range(6.0..12.0)),
                rng.random_range(0.0..6.0),
                Vec3::new(0.0, rng.random_range(-20.0..20.0), 0.0),
                0.45,
            );
        }
        if k < 1.0 {
            continue;
        }
        commands.entity(e).despawn();
        let Ok((mut ht, mut position, mut v)) = heroes.get_mut(pearl.hero) else {
            continue;
        };
        let left = ht.translation;
        ht.translation = pearl.to;
        position.0 = pearl.to;
        v.0 = Vec3::ZERO;
        // Arrival: flash, a purple ring and sparks flying out.
        let arrive = pearl.to + Vec3::Z * 40.0;
        let flash = Color::srgba(0.85, 0.6, 1.0, 1.0);
        spawn_burst(
            &mut commands,
            &mut materials,
            &art.hole.disc,
            flash,
            arrive,
            70.0,
            0.22,
            7.0,
        );
        let ring = Color::srgba(0.55, 0.95, 0.8, 0.9);
        spawn_burst(
            &mut commands,
            &mut materials,
            &art.hole.ring,
            ring,
            arrive,
            30.0,
            0.4,
            7.5,
        );
        for i in 0..18 {
            let a = i as f32 * std::f32::consts::TAU / 18.0;
            spawn_fx(
                &mut commands,
                &mut materials,
                &art.quad,
                ender(&mut rng),
                arrive,
                Vec2::new(16.0, 5.0),
                a,
                Vec3::new(a.cos(), a.sin(), 0.0) * rng.random_range(260.0..420.0),
                0.5,
            );
        }
        // Departure: particles fall into the spot the hero left.
        for i in 0..14 {
            let a = i as f32 * std::f32::consts::TAU / 14.0;
            let out = Vec3::new(a.cos(), a.sin(), 0.0) * 90.0;
            spawn_fx(
                &mut commands,
                &mut materials,
                &art.quad,
                ender(&mut rng),
                left + out + Vec3::Z * 40.0,
                Vec2::splat(9.0),
                a,
                -out / 0.35,
                0.35,
            );
        }
        play(&mut commands, &art.sounds.pearl, pearl.to, 0.8, 1.0);
    }
}

fn is_ore(kind: BlockType) -> bool {
    matches!(
        kind,
        BlockType::Coal
            | BlockType::Iron
            | BlockType::Gold
            | BlockType::Diamond
            | BlockType::Emerald
            | BlockType::Redstone
    )
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn start_quake(
    mut commands: Commands,
    mut powers: MessageReader<PowerActivated>,
    time: Res<Time>,
    art: Res<Art>,
    hero: Query<&Transform, With<Hero>>,
    mut quake: ResMut<Quake>,
    mut shake: ResMut<CameraShake>,
    mut notice: ResMut<PowerNotice>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let now = time.elapsed_secs();
    for power in powers.read() {
        if power.command.command_type != P::Quake {
            continue;
        }
        let Ok(t) = hero.single() else {
            continue;
        };
        quake.until = now + QUAKE_SECONDS;
        quake.next_pulse = now;
        shake.start(now, QUAKE_SECONDS, 14.0);
        // A dusty shockwave rolls out over the ground.
        let ring = Color::srgba(0.85, 0.65, 0.4, 0.8);
        let center = t.translation - Vec3::Y * 30.0;
        let grow = ((QUAKE_RADIUS * 2.0 / 50.0).powf(1.0 / 36.0) - 1.0) * 60.0;
        spawn_burst(
            &mut commands,
            &mut materials,
            &art.hole.ring,
            ring,
            center,
            50.0,
            0.6,
            grow,
        );
        play(&mut commands, &art.sounds.quake, t.translation, 0.9, 0.55);
        announce(&mut notice, by("EARTHQUAKE", &power.command.username));
    }
}

/// Every half second while a quake lasts: blocks crack, ores shake loose, dust
/// and rocks burst out of the ground, bodies bounce and enemies are staggered.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn quake_pulses(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<Art>,
    mut quake: ResMut<Quake>,
    grid: Res<SpatialHash>,
    blocks: Query<
        (
            &Transform,
            &BlockType,
            Option<&crate::systems::biome_content::Tile>,
        ),
        With<Block>,
    >,
    hero: Query<&Transform, (With<Hero>, Without<Block>)>,
    mut bodies: Query<
        (Entity, &Transform, &mut LinearVelocity, Has<Enemy>),
        (
            Or<(With<Hero>, With<Villager>, With<Enemy>, With<Drop>)>,
            Without<Block>,
            Without<Flying>,
        ),
    >,
    mut destroyed: MessageWriter<BlockDestroyed>,
    mut damaged: MessageWriter<BlockDamaged>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let now = time.elapsed_secs();
    if now >= quake.until || now < quake.next_pulse {
        return;
    }
    quake.next_pulse = now + QUAKE_PULSE;
    let Ok(center) = hero.single().map(|t| t.translation) else {
        return;
    };
    let mut rng = rand::rng();
    let reach = Vec2::splat(QUAKE_RADIUS);
    let nearby: Vec<(
        Entity,
        Vec3,
        BlockType,
        Option<crate::systems::biome_content::Tile>,
    )> = grid
        .rect_iter(center.xy() - reach, center.xy() + reach)
        .filter_map(|e| {
            blocks
                .get(e)
                .ok()
                .map(|(t, k, tile)| (e, t.translation, *k, tile.copied()))
        })
        .filter(|(_, p, ..)| p.xy().distance(center.xy()) <= QUAKE_RADIUS)
        .collect();
    let cells: HashSet<(i32, i32)> = nearby
        .iter()
        .map(|(_, p, ..)| ((p.x / 64.0).round() as i32, (p.y / 64.0).round() as i32))
        .collect();
    let mut broken = HashSet::new();
    let mut surface = Vec::new();
    for (block, p, kind, tile) in &nearby {
        let cell = ((p.x / 64.0).round() as i32, (p.y / 64.0).round() as i32);
        let exposed = !cells.contains(&(cell.0, cell.1 + 1));
        if exposed {
            surface.push(*p);
        }
        if !rng.random_bool(if exposed { 0.35 } else { 0.12 }) {
            continue;
        }
        if is_ore(*kind) {
            // Ores shake loose as drops.
            crate::plugins::gameplay_plugin::break_block(
                &mut commands,
                &mut destroyed,
                &mut broken,
                *block,
                *kind,
                *p,
                *tile,
            );
        } else {
            damaged.write(BlockDamaged {
                entity: *block,
                damage: 1,
                remaining_health: 0,
            });
        }
    }
    // Dust bursts and flying rocks along the ground.
    for _ in 0..surface.len().min(9) {
        let p = surface[rng.random_range(0..surface.len())] + Vec3::new(0.0, 34.0, 30.0);
        let tone = rng.random_range(0.45..0.7);
        let dust = materials.add(StandardMaterial {
            base_color: Color::srgba(tone + 0.15, tone, tone - 0.1, 0.75),
            base_color_texture: Some(art.dust.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        commands.spawn((
            Fx {
                life: 1.1,
                max: 1.1,
                velocity: Vec3::new(rng.random_range(-30.0..30.0), 60.0, 0.0),
                spin: rng.random_range(-1.0..1.0),
                grow: 0.9,
                gravity: 0.0,
            },
            Mesh3d(art.quad.clone()),
            MeshMaterial3d(dust),
            Transform::from_translation(p).with_scale(Vec3::splat(rng.random_range(50.0..80.0))),
            bevy::light::NotShadowCaster,
        ));
        for _ in 0..2 {
            let rock = materials.add(StandardMaterial {
                base_color: Color::srgb(tone, tone * 0.8, tone * 0.6),
                ..default()
            });
            commands.spawn((
                Fx {
                    life: 1.0,
                    max: 1.0,
                    velocity: Vec3::new(
                        rng.random_range(-160.0..160.0),
                        rng.random_range(300.0..520.0),
                        0.0,
                    ),
                    spin: rng.random_range(-8.0..8.0),
                    grow: 0.0,
                    gravity: 1100.0,
                },
                Mesh3d(art.cube.clone()),
                MeshMaterial3d(rock),
                Transform::from_translation(p).with_scale(Vec3::splat(rng.random_range(8.0..15.0))),
            ));
        }
    }
    // Everything on the ground bounces; enemies are staggered.
    for (e, t, mut v, enemy) in &mut bodies {
        if t.translation.xy().distance(center.xy()) > QUAKE_RADIUS * 1.3 || v.y.abs() > 60.0 {
            continue;
        }
        v.y = rng.random_range(200.0..340.0);
        v.x += rng.random_range(-120.0..120.0);
        if enemy {
            commands
                .entity(e)
                .try_insert(crate::systems::biome_encounters::Stunned { remaining: 0.6 });
        }
    }
    play(
        &mut commands,
        &art.sounds.quake,
        center,
        0.35,
        rng.random_range(0.4..0.55),
    );
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn start_bolt(
    mut commands: Commands,
    mut powers: MessageReader<PowerActivated>,
    art: Res<Art>,
    hero: Query<&Transform, With<Hero>>,
    enemies: Query<(Entity, &Transform, &Health), (With<Enemy>, Without<Hero>)>,
    mut damage: MessageWriter<EntityDamaged>,
    mut notice: ResMut<PowerNotice>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for power in powers.read() {
        if power.command.command_type != P::Bolt {
            continue;
        }
        let Ok(hero) = hero.single() else {
            continue;
        };
        let Some((target, at)) = pick_bolt_target(
            hero.translation,
            enemies
                .iter()
                .map(|(e, t, h)| (e, t.translation, h.max, h.current)),
        ) else {
            announce(&mut notice, "BOLT: no enemy in range".into());
            continue;
        };
        damage.write(EntityDamaged {
            entity: target,
            damage: 4,
        });
        // Zigzag bolt from the sky down to the target.
        let mut rng = rand::rng();
        let mut top = at + Vec3::new(rng.random_range(-60.0..60.0), 900.0, 70.0);
        for i in 0..10 {
            let bottom = if i == 9 {
                at + Vec3::Z * 70.0
            } else {
                at.lerp(top, 1.0 - (i + 1) as f32 / 10.0)
                    + Vec3::new(rng.random_range(-45.0..45.0), 0.0, 70.0)
            };
            let mid = (top + bottom) / 2.0;
            let d = bottom - top;
            spawn_fx(
                &mut commands,
                &mut materials,
                &art.quad,
                Color::srgba(0.85, 0.9, 1.0, 1.0),
                mid,
                Vec2::new(d.xy().length(), 7.0),
                d.y.atan2(d.x),
                Vec3::ZERO,
                0.35,
            );
            top = bottom;
        }
        play(&mut commands, &art.sounds.bolt, at, 0.9, 1.0);
        announce(&mut notice, by("LIGHTNING BOLT", &power.command.username));
    }
}

/// BOLT target: the enemy with the most max health within 900 of the hero
/// (ties: the nearest), skipping ones already dead.
fn pick_bolt_target(
    hero: Vec3,
    enemies: impl Iterator<Item = (Entity, Vec3, u32, u32)>,
) -> Option<(Entity, Vec3)> {
    enemies
        .filter(|(_, p, _, current)| *current > 0 && p.distance(hero) < 900.0)
        .max_by(|a, b| {
            a.2.cmp(&b.2)
                .then(b.1.distance(hero).total_cmp(&a.1.distance(hero)))
        })
        .map(|(e, p, _, _)| (e, p))
}

/// Live power objects, for the `--powers-test` probe.
#[cfg(feature = "probes")]
pub(crate) struct PowerCounts {
    pub anvils: usize,
    pub golems: usize,
    pub holes: usize,
    pub bees: usize,
    pub fx: usize,
}

#[cfg(feature = "probes")]
pub(crate) fn power_counts(w: &mut World) -> PowerCounts {
    PowerCounts {
        anvils: w.query::<&Anvil>().iter(w).count(),
        golems: w.query::<&Golem>().iter(w).count(),
        holes: w.query::<&BlackHole>().iter(w).count(),
        bees: w.query::<&Bee>().iter(w).count(),
        fx: w.query::<&Fx>().iter(w).count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bolt_hits_the_toughest_enemy_in_range_nearest_on_ties() {
        let hero = Vec3::ZERO;
        let a = Entity::from_raw_u32(1).unwrap();
        let b = Entity::from_raw_u32(2).unwrap();
        let c = Entity::from_raw_u32(3).unwrap();
        let d = Entity::from_raw_u32(4).unwrap();
        let enemies = vec![
            (a, Vec3::new(100.0, 0.0, 0.0), 4, 4),
            (b, Vec3::new(500.0, 0.0, 0.0), 10, 10),
            (c, Vec3::new(300.0, 0.0, 0.0), 10, 10),
            (d, Vec3::new(2000.0, 0.0, 0.0), 50, 50), // out of range
        ];
        assert_eq!(
            pick_bolt_target(hero, enemies.into_iter()).map(|t| t.0),
            Some(c)
        );
        assert!(pick_bolt_target(hero, std::iter::empty()).is_none());
    }

    #[test]
    fn fuss_mirrors_controls_only_while_active() {
        let fuss = Fuss {
            until: 5.0,
            active: true,
        };
        assert_eq!(fuss.steer_sign(1.0), -1.0);
        assert_eq!(fuss.steer_sign(6.0), 1.0);
    }

    #[test]
    fn wind_builds_up_surges_and_dies_down() {
        let wind = Wind {
            start: 10.0,
            until: 14.0,
            direction: Vec2::X,
            strength: 480.0,
            phase: 1.0,
        };
        assert_eq!(wind.gust(10.0), 0.0);
        assert!(wind.gust(14.0) <= 0.001);
        let middle: Vec<f32> = (0..40).map(|i| wind.gust(10.6 + i as f32 * 0.07)).collect();
        assert!(middle.iter().all(|g| (0.0..=1.0).contains(g)));
        // It surges and eases rather than blowing flat.
        let (lo, hi) = middle
            .iter()
            .fold((1.0_f32, 0.0_f32), |(l, h), g| (l.min(*g), h.max(*g)));
        assert!(hi - lo > 0.2, "gust range {lo}..{hi}");
    }

    #[test]
    fn only_ores_shake_loose_in_a_quake() {
        assert!(is_ore(BlockType::Diamond));
        assert!(!is_ore(BlockType::Stone));
    }
}
