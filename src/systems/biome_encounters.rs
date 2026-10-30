//! Seeded, accessible biome landmarks and readable enemy attack cycles.
mod mechanisms;
use crate::{
    components::{
        block::{BiomeType, Block, BlockType},
        buff::{Buff, BuffType, Buffable},
        bundles::avian_physics,
        chunk::ChunkMember,
        enemy::{Enemy, EnemyType, Health},
        hero::Hero,
        villager::Villager,
    },
    events::damage_events::EntityDamaged,
    resources::{material_cache::MaterialCache, mesh_cache::MeshCache},
};
use avian3d::prelude::*;
use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};
pub(crate) use mechanisms::Stunned;
pub(crate) use mechanisms::probe::run as mechanism_probe;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Witch,
    Miner,
    Sentinel,
}
#[derive(Component)]
pub struct EncounterFoe {
    pub role: Role,
    biome: BiomeType,
    phase: f32,
    cooldown: f32,
    target: Vec3,
}
impl EncounterFoe {
    pub fn received_damage(&self, amount: u32) -> u32 {
        if self.role != Role::Miner || amount == 0 {
            return amount;
        }
        // The miner's guard opens during the 1.5 seconds after a committed charge.
        if self.phase == 0. && self.cooldown > 2.3 {
            amount.saturating_add(1)
        } else {
            amount.saturating_sub(1).max(1)
        }
    }
}
const CLOUD_LIFETIME: f32 = 8.0;
const POTION_FLIGHT_TIME: f32 = 0.85;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Substance {
    Poison,
    Frost,
    Ember,
    Wither,
}
impl Substance {
    fn for_biome(b: BiomeType) -> Self {
        match b {
            BiomeType::Iceland => Self::Frost,
            BiomeType::Nether => Self::Ember,
            BiomeType::End => Self::Wither,
            _ => Self::Poison,
        }
    }
    fn index(self) -> usize {
        match self {
            Self::Poison => 0,
            Self::Frost => 1,
            Self::Ember => 2,
            Self::Wither => 3,
        }
    }
    fn tint(self) -> Vec4 {
        match self {
            Self::Poison => Vec4::new(0.28, 0.82, 0.12, 1.),
            Self::Frost => Vec4::new(0.3, 0.72, 1., 1.),
            Self::Ember => Vec4::new(1., 0.32, 0.06, 1.),
            Self::Wither => Vec4::new(0.65, 0.22, 0.9, 1.),
        }
    }
}
fn potion_position(p: &Potion, age: f32) -> Vec3 {
    // Continue ballistically beyond the aim point. No timer-triggered airburst.
    let u = age / POTION_FLIGHT_TIME;
    p.from.lerp(p.to, u) + Vec3::Y * (p.arc.max(35.) * 4. * u * (1. - u)) + Vec3::Z * 35.
}
fn cloud_opacity(age: f32) -> f32 {
    (age / 0.25).clamp(0., 1.) * ((CLOUD_LIFETIME - age) / 2.).clamp(0., 1.)
}
#[derive(Component)]
struct Potion {
    from: Vec3,
    to: Vec3,
    age: f32,
    biome: BiomeType,
    arc: f32,
}
#[derive(Component)]
struct Hazard {
    age: f32,
    tick: f32,
    biome: BiomeType,
}
#[derive(Component)]
struct RewardCache {
    biome: BiomeType,
}
#[derive(Component)]
struct RewardPulse {
    time: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Site {
    pub origin: IVec2,
    pub biome: BiomeType,
    pub seed: u32,
}
impl Site {
    pub fn width(&self) -> i32 {
        if self.biome == BiomeType::Desert {
            14
        } else {
            6
        }
    }
    pub fn reserved(&self, cell: IVec2) -> bool {
        let d = cell - self.origin;
        d.x.abs() <= self.width()
            && d.y >= -10
            && d.y <= 7
            && (d.y >= -5 || super::world_shapes::is_open(cell.x, cell.y, self.biome, self.seed))
    }
}
fn hash(x: u32) -> u32 {
    let x = (x ^ (x >> 16)).wrapping_mul(0x7feb352d);
    x ^ (x >> 15)
}
thread_local! {static SITES:std::cell::RefCell<std::collections::HashMap<(i32,i32,BiomeType,u32),Option<Site>>>=std::cell::RefCell::new(std::collections::HashMap::new());}
/// A region owns a whole landmark. Each streamed chunk builds only its cells.

#[derive(Component)]
struct StructureFinish(usize);
#[derive(Component)]
pub(crate) struct StructureBackdrop;

fn finish_blocks(mut commands: Commands, q: Query<(Entity, &StructureFinish)>, art: Res<Art>) {
    for (e, finish) in &q {
        let mesh = art.cube.clone();
        let material = art.finishes[finish.0].clone();
        commands.queue(move |world: &mut World| {
            if let Ok(mut e) = world.get_entity_mut(e) {
                let position = e.get::<Transform>().unwrap().translation;
                e.insert((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(position),
                ))
                .remove::<StructureFinish>();
            }
        });
    }
}

#[derive(Component)]
struct PendingLandmark(Site);
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
struct CloudMaterial {
    #[uniform(0)]
    effect: Vec4,
    #[uniform(1)]
    tint: Vec4,
}
impl Material for CloudMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/toxic_cloud.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
#[derive(Resource, Clone)]
struct Art {
    cube: Handle<Mesh>,
    wall: Handle<Mesh>,
    finishes: Vec<Handle<StandardMaterial>>,
    walls: Vec<Handle<StandardMaterial>>,
    face: Handle<Mesh>,
    quad: Handle<Mesh>,
    faces: [Option<Handle<Image>>; 3],
    potion: Handle<StandardMaterial>,
    liquids: [Handle<StandardMaterial>; 4],
}
pub struct BiomeEncountersPlugin;
impl Plugin for BiomeEncountersPlugin {
    fn build(&self, app: &mut App) {
        mechanisms::install(app);
        app.add_plugins(MaterialPlugin::<CloudMaterial>::default())
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (instantiate, finish_blocks, think, projectiles, rewards)
                    .chain()
                    .run_if(in_state(crate::states::InGame)),
            );
    }
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    server: Option<Res<AssetServer>>,
) {
    let load = |p: &'static str| {
        server.as_ref().map(|s| {
            s.load_builder()
                .with_settings(|settings: &mut bevy::image::ImageLoaderSettings| {
                    settings.sampler = bevy::image::ImageSampler::nearest()
                })
                .load(p)
        })
    };
    let textures = [
        "textures/structures/oak_planks.png",
        "textures/structures/dark_oak_planks.png",
        "textures/structures/stone_bricks.png",
        "textures/structures/nether_bricks.png",
        "textures/blocks/desert/sandstone/sandstone.png",
        "textures/structures/cut_sandstone.png",
        "textures/structures/chiseled_sandstone.png",
    ];
    let finishes = textures
        .iter()
        .map(|p| {
            mats.add(StandardMaterial {
                base_color_texture: load(p),
                perceptual_roughness: 0.92,
                ..default()
            })
        })
        .collect();
    let walls = textures
        .iter()
        .map(|p| {
            mats.add(StandardMaterial {
                base_color_texture: load(p),
                base_color: Color::srgb(0.68, 0.65, 0.61),
                perceptual_roughness: 0.92,
                ..default()
            })
        })
        .collect();
    commands.insert_resource(Art {
        cube: meshes.add(Cuboid::new(64., 64., 64.)),
        wall: meshes.add(Cuboid::new(
            64.,
            64.,
            super::actor_depth::STRUCTURE_THICKNESS,
        )),
        finishes,
        walls,
        face: meshes.add(Rectangle::new(64., 64.)),
        quad: meshes.add(Rectangle::new(1., 1.)),
        faces: [
            load("textures/entities/enemy/witch.png"),
            load("textures/entities/enemy/skeleton2.png"),
            load("textures/entities/enemy/EndermanFace.png"),
        ],
        liquids: [
            Substance::Poison,
            Substance::Frost,
            Substance::Ember,
            Substance::Wither,
        ]
        .map(|kind| {
            let c = kind.tint();
            mats.add(StandardMaterial {
                base_color: Color::srgb(c.x, c.y, c.z),
                base_color_texture: load("textures/projectiles/potion_overlay.png"),
                alpha_mode: AlphaMode::Mask(0.1),
                unlit: true,
                ..default()
            })
        }),
        potion: mats.add(StandardMaterial {
            base_color_texture: load("textures/projectiles/splash_potion.png"),
            alpha_mode: AlphaMode::Mask(0.1),
            unlit: true,
            ..default()
        }),
    });
}
fn model(
    commands: &mut Commands,
    entity: Entity,
    role: Role,
    art: &Art,
    mats: &mut Assets<StandardMaterial>,
) {
    let i = match role {
        Role::Witch => 0,
        Role::Miner => 1,
        Role::Sentinel => 2,
    };
    commands.entity(entity).insert((
        Mesh3d(art.face.clone()),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color_texture: art.faces[i].clone(),
            alpha_mode: AlphaMode::Mask(0.5),
            unlit: true,
            ..default()
        })),
    ));
}
fn spawn_foe(
    commands: &mut Commands,
    position: Vec3,
    biome: BiomeType,
    role: Role,
    chunk: IVec2,
    art: &Art,
    mats: &mut Assets<StandardMaterial>,
) -> Entity {
    let kind = match role {
        Role::Witch => EnemyType::Witch,
        Role::Miner => EnemyType::Miner,
        Role::Sentinel => EnemyType::Sentinel,
    };
    let entity = commands
        .spawn((
            Enemy::new(kind),
            Health::new(if role == Role::Miner { 6 } else { 4 }),
            Buffable,
            avian_physics::enemy_with_velocity(Vec3::ZERO),
            EncounterFoe {
                role,
                biome,
                phase: 0.,
                cooldown: 2.5,
                target: position,
            },
            Transform::from_translation(position),
            Visibility::Inherited,
            ChunkMember {
                chunk_x: chunk.x,
                chunk_y: chunk.y,
            },
        ))
        .id();
    model(commands, entity, role, art, mats);
    entity
}
fn instantiate(
    mut commands: Commands,
    pending: Query<(Entity, &PendingLandmark)>,
    art: Res<Art>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mechanisms: Res<mechanisms::MachineArt>,
    modes: Option<Res<crate::systems::game_modes::GameModes>>,
) {
    use crate::systems::game_modes::{GameMode, is_on};
    for (e, p) in &pending {
        let site = p.0;
        if is_on(modes.as_deref(), GameMode::Machines) {
            mechanisms::spawn(&mut commands, site, &mechanisms);
        }
        let chunk = site.origin.div_euclid(IVec2::splat(16));
        let pos = (site.origin.as_vec2() * 64.).extend(10.);
        for (offset, style) in backdrop(site) {
            commands.spawn((
                StructureBackdrop,
                ChunkMember {
                    chunk_x: chunk.x,
                    chunk_y: chunk.y,
                },
                Mesh3d(art.wall.clone()),
                MeshMaterial3d(art.walls[style].clone()),
                Transform::from_translation(
                    ((site.origin + offset).as_vec2() * 64.)
                        .extend(super::actor_depth::STRUCTURE_BACKING),
                ),
            ));
        }

        let role = match site.biome {
            BiomeType::Plains | BiomeType::Mountains => None,
            BiomeType::Forest => Some(Role::Witch),
            BiomeType::Underground => Some(Role::Miner),
            _ => Some(Role::Sentinel),
        };
        if let Some(role) = role.filter(|_| is_on(modes.as_deref(), GameMode::LandmarkEnemies)) {
            spawn_foe(&mut commands, pos, site.biome, role, chunk, &art, &mut mats);
        }
        if site.biome == BiomeType::Desert && is_on(modes.as_deref(), GameMode::LandmarkEnemies) {
            for x in [-8., 8.] {
                super::biome_content::spawn_specialist(
                    &mut commands,
                    pos + Vec3::new(x * 64., -3. * 64., 30.),
                    EnemyType::Husk,
                    0,
                );
            }
        }
        commands.entity(e).remove::<PendingLandmark>().insert((
            Text2d::new(title(site.biome)),
            TextFont {
                font_size: FontSize::Px(19.),
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.7, 0.6)),
            Transform::from_translation(pos + Vec3::new(0., 300., -80.)),
        ));
    }
}
fn visible(from: Vec3, to: Vec3, blocks: &[(Vec2, Entity)]) -> bool {
    !blocks.iter().any(|(p, _)| {
        super::projectile_system::segment_box(from.xy(), to.xy(), *p, Vec2::splat(30.)).is_some()
    })
}
fn think(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<Art>,
    mut foes: Query<(
        Entity,
        &mut Transform,
        &mut LinearVelocity,
        &mut EncounterFoe,
    )>,
    targets: Query<
        &Transform,
        (
            Or<(With<Hero>, With<Villager>)>,
            Without<EncounterFoe>,
            Without<super::launcher::LauncherPassenger>,
        ),
    >,
    blocks: Query<(Entity, &Transform), (With<Block>, Without<EncounterFoe>)>,
    grid: Res<crate::resources::spatial_hash::SpatialHash>,
) {
    // Blocks inside a rectangle, via the spatial hash instead of every block each frame
    let near = |lo: Vec2, hi: Vec2| -> Vec<(Vec2, Entity)> {
        grid.rect_iter(lo, hi)
            .filter_map(|e| blocks.get(e).ok().map(|(e, t)| (t.translation.xy(), e)))
            .collect()
    };
    let dt = time.delta_secs();
    for (entity, mut t, mut velocity, mut foe) in &mut foes {
        let Some(target) = targets.iter().min_by(|a, b| {
            a.translation
                .distance_squared(t.translation)
                .total_cmp(&b.translation.distance_squared(t.translation))
        }) else {
            continue;
        };
        let distance = target.translation.distance(t.translation);
        if distance > 850. {
            continue;
        }
        if foe.phase == 0. {
            foe.cooldown -= dt;
            if foe.cooldown > 0.
                || !visible(
                    t.translation,
                    target.translation,
                    &near(
                        t.translation.xy().min(target.translation.xy()) - Vec2::splat(64.),
                        t.translation.xy().max(target.translation.xy()) + Vec2::splat(64.),
                    ),
                )
            {
                continue;
            }
            foe.target = target.translation;
            foe.phase = 0.001;
        }
        foe.phase += dt;
        velocity.x *= 0.7;
        if foe.phase < 0.85 {
            continue;
        }
        match foe.role {
            Role::Witch | Role::Sentinel => {
                commands
                    .spawn((
                        Potion {
                            from: t.translation,
                            to: foe.target,
                            age: 0.,
                            biome: foe.biome,
                            arc: if foe.role == Role::Witch { 100. } else { 0. },
                        },
                        Mesh3d(art.quad.clone()),
                        MeshMaterial3d(art.potion.clone()),
                        bevy::light::NotShadowCaster,
                        Transform::from_translation(t.translation + Vec3::Z * 35.)
                            .with_scale(Vec3::new(32., 32., 1.)),
                    ))
                    .with_children(|p| {
                        p.spawn((
                            Mesh3d(art.quad.clone()),
                            MeshMaterial3d(
                                art.liquids[Substance::for_biome(foe.biome).index()].clone(),
                            ),
                            bevy::light::NotShadowCaster,
                            Transform::from_xyz(0., 0., 0.1),
                        ));
                    });
                if foe.role == Role::Sentinel && foe.biome == BiomeType::End {
                    let blink =
                        t.translation + Vec3::X * (t.translation.x - foe.target.x).signum() * 128.;
                    if near(blink.xy() - Vec2::splat(64.), blink.xy() + Vec2::splat(64.))
                        .iter()
                        .all(|(p, _)| (*p - blink.xy()).abs().max_element() > 64.)
                    {
                        t.translation = blink;
                        commands.entity(entity).insert(Position(blink));
                    }
                }
            }
            Role::Miner => {
                velocity.x = (foe.target.x - t.translation.x).signum() * 280.;
                velocity.y = 170.;
            }
        }
        foe.phase = 0.;
        foe.cooldown = if foe.role == Role::Witch { 4.5 } else { 3.8 };
    }
}
fn projectiles(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<Art>,
    mut cloud_materials: ResMut<Assets<CloudMaterial>>,
    cloud_handles: Query<&MeshMaterial3d<CloudMaterial>>,
    terrain: Res<crate::resources::spatial_hash::SpatialHash>,
    meshes: Res<Assets<Mesh>>,
    mut potions: Query<(Entity, &mut Transform, &mut Potion)>,
    mut hazards: Query<(Entity, &mut Transform, &mut Hazard), Without<Potion>>,
    mut targets: Query<
        (
            Entity,
            &Transform,
            &mut LinearVelocity,
            Option<&Buff>,
            Option<&Mesh3d>,
        ),
        (
            Or<(With<Hero>, With<Villager>)>,
            Without<Potion>,
            Without<Hazard>,
            Without<super::launcher::LauncherPassenger>,
        ),
    >,
    blocks: Query<
        (&Transform, &BlockType, Option<&Mesh3d>),
        (
            With<Block>,
            Without<Potion>,
            Without<Hazard>,
            Without<Hero>,
            Without<Villager>,
        ),
    >,
    mut damage: MessageWriter<EntityDamaged>,
) {
    let dt = time.delta_secs();
    for (e, mut t, mut p) in &mut potions {
        let previous = p.age;
        p.age += dt;
        let mut impact = None;
        // Sample the curved arc in short segments even through a render hitch.
        let steps = ((dt / 0.025).ceil() as usize).clamp(1, 160);
        let mut from = potion_position(&p, previous);
        for step in 1..=steps {
            let next = potion_position(&p, previous + dt * step as f32 / steps as f32);
            let pad = Vec2::splat(44.);
            let mut first: Option<f32> = None;
            let mut consider = |center: Vec2, half: Vec2| {
                if let Some(hit) = super::projectile_system::segment_box(
                    from.xy(),
                    next.xy(),
                    center,
                    half + Vec2::splat(6.),
                ) {
                    first = Some(first.map_or(hit, |old| old.min(hit)));
                }
            };
            for block in terrain.rect_iter(
                from.xy().min(next.xy()) - pad,
                from.xy().max(next.xy()) + pad,
            ) {
                let Ok((t, kind, mesh)) = blocks.get(block) else {
                    continue;
                };
                if *kind == BlockType::Cobweb {
                    continue;
                }
                if mesh.is_some() {
                    let face = super::head_geometry::bounds(&meshes, mesh);
                    consider(
                        t.transform_point(face.center).xy(),
                        face.size.xy() * t.scale.xy().abs() * 0.5,
                    );
                } else {
                    consider(t.translation.xy(), Vec2::splat(32.));
                }
            }
            for (_, t, _, _, mesh) in targets.iter() {
                let face = super::head_geometry::bounds(&meshes, mesh);
                consider(
                    t.transform_point(face.center).xy(),
                    face.size.xy() * t.scale.xy().abs() * 0.5,
                );
            }
            if let Some(hit) = first {
                impact = Some(from.lerp(next, hit));
                break;
            }
            from = next;
        }
        t.translation = impact.unwrap_or(from);
        t.rotation = Quat::from_rotation_z(p.age * 5.);
        if let Some(at) = impact {
            commands.entity(e).despawn();
            let tint = Substance::for_biome(p.biome).tint();
            commands.spawn((
                Hazard {
                    age: 0.,
                    tick: 0.0,
                    biome: p.biome,
                },
                Mesh3d(art.quad.clone()),
                MeshMaterial3d(cloud_materials.add(CloudMaterial {
                    effect: Vec4::new(0., 0., 0., at.x * 0.001),
                    tint,
                })),
                bevy::light::NotShadowCaster,
                Transform::from_translation(at).with_scale(Vec3::new(300., 220., 1.)),
            ));
        } else if p.age >= 4. {
            commands.entity(e).despawn();
        }
    }
    for (e, mut t, mut h) in &mut hazards {
        h.age += dt;
        h.tick -= dt;
        t.translation.y += dt * 3.;
        t.rotation = Quat::IDENTITY;
        if let Ok(handle) = cloud_handles.get(e) {
            if let Some(mut m) = cloud_materials.get_mut(&handle.0) {
                m.effect.x = h.age;
                m.effect.y = cloud_opacity(h.age);
            }
        }
        if h.age >= CLOUD_LIFETIME {
            if let Ok(handle) = cloud_handles.get(e) {
                cloud_materials.remove(handle.0.id());
            }
            commands.entity(e).despawn();
            continue;
        }
        if h.tick > 0. {
            continue;
        }
        h.tick = 0.8;
        for (entity, position, mut velocity, buff, _) in &mut targets {
            if position.translation.xy().distance(t.translation.xy()) < 75. {
                damage.write(EntityDamaged { entity, damage: 1 });
                velocity.x *= 0.55;
                if matches!(h.biome, BiomeType::Forest | BiomeType::Iceland)
                    && buff.is_none_or(|b| b.buff_type == BuffType::Slowness)
                {
                    commands
                        .entity(entity)
                        .insert(Buff::new(BuffType::Slowness, 1.2));
                }
                if h.biome == BiomeType::Iceland {
                    velocity.y *= 0.8;
                }
            }
        }
    }
}
fn rewards(
    mut commands: Commands,
    time: Res<Time>,
    hero: Query<&Transform, With<Hero>>,
    mut caches: Query<(Entity, &Transform, &RewardCache, Has<RewardPulse>)>,
    mut pulses: Query<(Entity, &mut RewardPulse)>,
    mut health: Query<&mut Health, With<Hero>>,
) {
    for (e, mut pulse) in &mut pulses {
        pulse.time -= time.delta_secs();
        if pulse.time <= 0. {
            commands.entity(e).remove::<RewardPulse>();
        }
    }
    let Ok(hero) = hero.single() else {
        return;
    };
    for (e, t, cache, pulse) in &mut caches {
        if pulse {
            continue;
        }
        if hero.translation.distance(t.translation) < 95.
            && matches!(cache.biome, BiomeType::Plains | BiomeType::Iceland)
        {
            for mut hp in &mut health {
                hp.current = (hp.current + 1).min(hp.max);
            }
            commands.entity(e).insert(RewardPulse { time: 20. });
        }
    }
}

// Opt-in fixture uses the same materials, AI, impacts and damage pipeline as play.
#[derive(Resource, Default)]
struct StyleProbe {
    step: u8,
    since: f32,
    anchor: Vec3,
    witch: Option<Entity>,
    mirror: Option<Entity>,
}

#[path = "biome_encounters/structures.rs"]
mod structures;
pub(crate) use structures::*;

#[path = "biome_encounters/probe.rs"]
mod probe;
pub(crate) use probe::*;

#[cfg(test)]
#[path = "biome_encounters/tests.rs"]
mod tests;
