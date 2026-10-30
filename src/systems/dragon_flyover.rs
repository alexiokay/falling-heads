//! Articulated voxel dragon shared by End encounters and the DRAGON power.
use crate::{
    components::{
        block::BiomeType,
        block::{Block, BlockType},
        buff::{Buff, BuffType},
        enemy::{Enemy, EnemyType, Flying, Health},
        hero::Hero,
        villager::Villager,
    },
    events::power_events::PowerActivated,
    resources::command_queue::PowerCommandType,
};
use bevy::prelude::*;
#[derive(Component)]
pub(crate) struct EndDragon {
    age: f32,
    origin: Vec3,
    phase: f32,
    last_health: u32,
    perch: Option<Vec3>,
    attack_tick: f32,
    facing_right: bool,
    yaw: f32,
    yaw_velocity: f32,
    velocity: Vec2,
    bank: f32,
    roll: f32,
}
#[derive(Component)]
enum JointMotion {
    Neck,
    Jaw,
    Wing,
    Tip,
    Leg,
    Tail,
}
#[derive(Component)]
struct Chest;
#[derive(Component)]
struct Joint {
    owner: Entity,
    phase: f32,
    amplitude: f32,
    axis: Vec3,
    base: Quat,
    motion: JointMotion,
}
#[derive(Resource)]
struct DragonAssets {
    stage: Entity,
    cube: Handle<Mesh>,
    quad: Handle<Mesh>,
    wing: Handle<Mesh>,
    skin: Handle<StandardMaterial>,
    membrane: Handle<StandardMaterial>,
    eyes: Handle<StandardMaterial>,
    scales: Handle<StandardMaterial>,
    belly: Handle<StandardMaterial>,
    bone: Handle<StandardMaterial>,
    breath: Handle<BreathMaterial>,
}
#[derive(Resource, Default)]
struct Encounter {
    elapsed: f32,
    count: u32,
}
pub struct DragonPlugin;
impl Plugin for DragonPlugin {
    fn build(&self, app: &mut App) {
        if std::env::args().any(|arg| arg == "--fire-impact-test") {
            app.add_systems(
                Update,
                fire_gallery_contacts.after(flight).before(surface_fire),
            );
        }
        app.add_plugins((
            MaterialPlugin::<BreathMaterial>::default(),
            MaterialPlugin::<SurfaceFireMaterial>::default(),
        ))
        .init_resource::<Encounter>()
        .init_resource::<BreathSurfaces>()
        .add_message::<TerrainHeat>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                encounters,
                flight,
                weather_heat,
                apply_heat,
                ground_fire,
                surface_fire,
                animate,
                boss_label,
            )
                .chain()
                .run_if(in_state(crate::states::InGame)),
        );
    }
}

fn encounters(
    mut commands: Commands,
    time: Res<Time>,
    mut state: ResMut<Encounter>,
    hero: Query<&Transform, With<Hero>>,
    dragons: Query<(), With<EndDragon>>,
    assets: Res<DragonAssets>,
    mut requests: MessageReader<PowerActivated>,
    modes: Option<Res<crate::systems::game_modes::GameModes>>,
) {
    let manual = requests
        .read()
        .any(|p| p.command.command_type == PowerCommandType::Dragon);
    if !crate::systems::game_modes::is_on(
        modes.as_deref(),
        crate::systems::game_modes::GameMode::Dragon,
    ) {
        return;
    }
    let Ok(hero) = hero.single() else {
        return;
    };
    let end = crate::utils::biome_utils::determine_biome_at_position(
        hero.translation.x,
        hero.translation.y,
    ) == BiomeType::End;
    if end {
        state.elapsed += time.delta_secs();
    } else {
        state.elapsed = 0.;
    }
    let interval = 80. + (state.count.wrapping_mul(37) % 61) as f32;
    if dragons.is_empty() && (manual || (end && state.elapsed > interval)) {
        spawn(&mut commands, hero.translation, &assets);
        state.elapsed = 0.;
        state.count += 1;
    }
}
#[derive(Component)]
pub(crate) struct DragonHitPart {
    pub owner: Entity,
    pub half: Vec2,
}
#[derive(Component)]
struct Breath(Entity);
#[derive(Component)]
struct GroundFire {
    owner: Entity,
    remaining: f32,
    tick: f32,
    origin: Vec2,
    hops: u8,
}
/// Shared terrain heat request: rendering never determines material behavior.
#[derive(Message)]
pub(crate) struct TerrainHeat {
    pub position: Vec2,
    pub radius: f32,
}
#[derive(Clone, Copy)]
struct SurfaceContact {
    position: Vec3,
    direction: Vec2,
    strength: f32,
}
#[derive(Resource, Default)]
struct BreathSurfaces(std::collections::HashMap<Entity, SurfaceContact>);
#[derive(Component)]
struct SurfaceFlame {
    owner: Entity,
    opacity: f32,
}

const BREATH_PENETRATION: usize = 3;
fn breath_intersections(mouth: Vec2, end: Vec2, blocks: &[(Entity, Vec2)]) -> Vec<(Entity, f32)> {
    let mut hits: Vec<_> = blocks
        .iter()
        .filter_map(|(entity, center)| {
            super::projectile_system::segment_box(mouth, end, *center, Vec2::splat(32.))
                .map(|f| (*entity, f))
        })
        .collect();
    hits.sort_by(|a, b| a.1.total_cmp(&b.1));
    hits
}
fn breath_reach(mouth: Vec2, direction: Vec2, blocks: &[(Entity, Vec2)]) -> f32 {
    let mut layers = Vec::new();
    for (_, f) in breath_intersections(mouth, mouth + direction * 360., blocks) {
        if layers
            .last()
            .is_none_or(|last: &f32| (f - *last).abs() > 0.001)
        {
            layers.push(f);
        }
    }
    layers.get(BREATH_PENETRATION).map_or(360., |f| f * 360.)
}
/// Each cone ray penetrates three cells and impacts the fourth surface.
fn exposed_breath_blocks(mouth: Vec2, direction: Vec2, blocks: &[(Entity, Vec2)]) -> Vec<Entity> {
    let perpendicular = Vec2::new(-direction.y, direction.x);
    let mut touched = Vec::new();
    for ray in -4..=4 {
        let end = mouth + direction * 360. + perpendicular * (ray as f32 * 12.);
        let mut layer = 0;
        let mut previous = -1.;
        for (entity, f) in breath_intersections(mouth, end, blocks) {
            if (f - previous).abs() > 0.001 {
                layer += 1;
                previous = f;
            }
            if layer > BREATH_PENETRATION + 1 {
                break;
            }
            if !touched.contains(&entity) {
                touched.push(entity);
            }
        }
    }
    touched
}
#[derive(
    Asset, bevy::reflect::TypePath, bevy::render::render_resource::AsBindGroup, Clone, Debug,
)]
struct BreathMaterial {
    #[uniform(0)]
    effect: Vec4,
}
impl Material for BreathMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/dragon_breath.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
fn flammable(kind: BlockType) -> bool {
    kind.combustible()
}
fn perch_clear(candidate: Vec3, blocks: &[Vec3]) -> bool {
    blocks.iter().all(|p| {
        let d = *p - candidate;
        !(d.x.abs() < 250. && d.y > -55. && d.y < 220.)
    })
}
/// Exact critically damped response: angular momentum is continuous even when
/// the target changes midway through a turn, independent of the frame rate.
fn turn_towards(current: f32, velocity: f32, target: f32, dt: f32) -> (f32, f32) {
    let omega = 5.;
    let offset = current - target;
    let impulse = velocity + omega * offset;
    let decay = (-omega * dt).exp();
    (
        target + (offset + impulse * dt) * decay,
        (velocity - omega * impulse * dt) * decay,
    )
}
fn neck_rotation(d: &EndDragon) -> Quat {
    // Lead the body with continuous angular velocity, so retargeting cannot
    // snap the neck to the opposite limit while the body is still banking.
    Quat::from_rotation_y((d.yaw_velocity * 0.12).clamp(-0.42, 0.42))
        * Quat::from_rotation_z(-0.06 * (d.yaw_velocity / 3.).abs().min(1.))
}
fn mouth_transform(d: &EndDragon, root: &Transform) -> Transform {
    *root * Transform::from_xyz(-48., 5., 0.).with_rotation(neck_rotation(d))
}
#[allow(clippy::too_many_arguments)] // Bevy system parameters
fn flight(
    mut commands: Commands,
    time: Res<Time>,
    mut dragons: Query<
        (Entity, &mut EndDragon, &mut Transform, &Health),
        (
            Without<Joint>,
            Without<Hero>,
            Without<Block>,
            Without<Breath>,
            Without<crate::components::damage_visual::DeathAnimation>,
        ),
    >,
    hero: Query<&Transform, (With<Hero>, Without<EndDragon>, Without<Breath>)>,
    actors: Query<
        (Entity, &Transform),
        (
            Or<(With<Hero>, With<Villager>)>,
            Without<EndDragon>,
            Without<Breath>,
        ),
    >,
    blocks: Query<
        (Entity, &Transform, &BlockType),
        (With<Block>, Without<EndDragon>, Without<Breath>),
    >,
    mut flames: Query<(Entity, &Breath, &mut Transform, &mut Visibility), Without<EndDragon>>,
    mut heat: MessageWriter<TerrainHeat>,
    mut surfaces: ResMut<BreathSurfaces>,
    a: Res<DragonAssets>,
    mut materials: ResMut<Assets<BreathMaterial>>,
    mut damage: MessageWriter<crate::events::damage_events::EntityDamaged>,
    mut block_damage: MessageWriter<crate::events::block_events::BlockDamaged>,
    grid: Res<crate::resources::spatial_hash::SpatialHash>,
) {
    surfaces.0.clear();
    let Ok(hero) = hero.single() else {
        return;
    };
    let dt = time.delta_secs();
    for (e, mut d, mut t, health) in &mut dragons {
        d.age += dt;
        d.phase = (d.phase + dt) % 18.;
        if t.translation.xy().distance(hero.translation.xy()) > 3200. {
            commands.entity(e).despawn();
            continue;
        }
        if health.current < d.last_health {
            d.phase = 0.;
            d.perch = None;
        }
        d.last_health = health.current;
        let side = if (d.age / 18.) as u32 % 2 == 0 {
            1.
        } else {
            -1.
        };
        let target = hero.translation;
        if d.phase < 0.15 {
            d.perch = None;
        }
        if d.phase >= 11. && d.perch.is_none() {
            // Candidates within 650 of the hero, plus the clearance box perch_clear checks
            let terrain: Vec<_> = grid
                .rect_iter(
                    target.xy() - Vec2::new(900., 700.),
                    target.xy() + Vec2::new(900., 900.),
                )
                .filter_map(|e| blocks.get(e).ok())
                .map(|(_, t, _)| t.translation)
                .collect();
            d.perch = terrain
                .iter()
                .filter(|p| p.xy().distance(target.xy()) < 650.)
                .map(|p| Vec3::new(p.x, p.y + 91., 90.))
                .filter(|p| perch_clear(*p, &terrain))
                .min_by(|a, b| {
                    a.distance_squared(t.translation)
                        .total_cmp(&b.distance_squared(t.translation))
                });
        }
        if let Some(p) = d.perch {
            if !grid
                .nearby_iter(p.x, p.y - 91.)
                .filter_map(|e| blocks.get(e).ok())
                .any(|(_, b, _)| {
                    (b.translation.x - p.x).abs() < 2. && (b.translation.y + 91. - p.y).abs() < 2.
                })
            {
                d.perch = None;
            }
        }
        let desired = if (11. ..15.).contains(&d.phase) && d.perch.is_some() {
            d.perch.unwrap()
        } else if (6. ..11.).contains(&d.phase) {
            target + Vec3::new(side * 330., 150., 0.)
        } else {
            target
                + Vec3::new(
                    (d.age * 0.52).cos() * 430.,
                    240. + (d.age * 0.8).sin() * 110.,
                    0.,
                )
        };
        let delta = desired.xy() - t.translation.xy();
        let effort = (d.yaw_velocity / 3.).clamp(-1., 1.);
        let desired_velocity = (delta * 1.8).clamp_length_max(210.) * (1. - 0.35 * effort.abs())
            + Vec2::Y * 28. * effort.abs();
        let previous_velocity = d.velocity;
        let decay = (-3. * dt).exp();
        d.velocity = desired_velocity + (previous_velocity - desired_velocity) * decay;
        t.translation += (desired_velocity * dt
            + (previous_velocity - desired_velocity) * (1. - decay) / 3.)
            .extend(0.);
        t.translation.z = 90.;
        let aim = (target.xy() - t.translation.xy()).normalize_or(Vec2::NEG_X);
        // Hysteresis prevents rapid left/right reversals when the target is overhead.
        let attacking = (6. ..11.).contains(&d.phase);
        let target_dx = if attacking {
            target.x - t.translation.x
        } else {
            delta.x
        };
        if target_dx.abs() > 45. {
            d.facing_right = target_dx > 0.;
        }
        let desired_yaw = if d.facing_right {
            std::f32::consts::PI
        } else {
            0.
        };
        (d.yaw, d.yaw_velocity) = turn_towards(d.yaw, d.yaw_velocity, desired_yaw, dt);
        let direction = if attacking {
            aim
        } else {
            d.velocity.normalize_or(Vec2::NEG_X)
        };
        let angle = (-direction.y * d.yaw.cos()).atan2(direction.x.abs().max(0.3));
        let target_bank = if (11. ..15.).contains(&d.phase) && delta.length() < 10. {
            0.
        } else {
            angle.clamp(-0.6, 0.6)
        };
        d.bank += (target_bank - d.bank) * (1. - (-5. * dt).exp());
        let target_roll = -0.5 * (d.yaw_velocity / 3.).clamp(-1., 1.);
        d.roll += (target_roll - d.roll) * (1. - (-6. * dt).exp());
        t.scale = Vec3::ONE;
        t.rotation = Quat::from_rotation_z(d.bank)
            * Quat::from_rotation_y(d.yaw)
            * Quat::from_rotation_x(d.roll);
        let head = mouth_transform(&d, &t);
        let mut mouth = head.transform_point(Vec3::new(-74., -13., 4.));
        mouth.z = dragon_stage_transform().transform_point(mouth).z;
        let projected_forward = (head.rotation * Vec3::NEG_X).xy();
        let dir = projected_forward.normalize_or(Vec2::NEG_X);
        // A side-view attack resumes once the mouth faces into the play plane.
        // Do not fire a full-length sideways beam while looking at the camera.
        let breathing = (8. ..11.).contains(&d.phase) && (desired_yaw - d.yaw).abs() < 0.18;
        let nearby: Vec<_> = grid
            .rect_iter(
                mouth.xy() - Vec2::splat(430.),
                mouth.xy() + Vec2::splat(430.),
            )
            .filter_map(|e| blocks.get(e).ok())
            .filter(|(_, bt, _)| bt.translation.xy().distance_squared(mouth.xy()) < 430. * 430.)
            .map(|(entity, bt, _)| (entity, bt.translation.xy()))
            .collect();
        let length = breath_reach(mouth.xy(), dir, &nearby);
        let strength = ((d.phase - 8.) * 4.)
            .min((11. - d.phase) * 4.)
            .clamp(0., 1.);
        let touched = if breathing {
            exposed_breath_blocks(mouth.xy(), dir, &nearby)
        } else {
            Vec::new()
        };
        for &owner in &touched {
            if let Ok((_, surface, _)) = blocks.get(owner) {
                surfaces.0.insert(
                    owner,
                    SurfaceContact {
                        position: surface.translation,
                        direction: dir,
                        strength,
                    },
                );
            }
        }
        let mut found = false;
        for (_, breath, mut transform, mut visible) in &mut flames {
            if breath.0 != e {
                continue;
            }
            found = true;
            *visible = if breathing {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            transform.translation = mouth + (dir * length * 0.5).extend(8.);
            transform.rotation = Quat::from_rotation_z(dir.y.atan2(dir.x));
            transform.scale = Vec3::new(length.max(1.), 110., 1.);
        }
        if !found {
            commands.spawn((
                Breath(e),
                Mesh3d(a.quad.clone()),
                MeshMaterial3d(a.breath.clone()),
                bevy::light::NotShadowCaster,
                Transform::IDENTITY,
                Visibility::Hidden,
            ));
        }
        if let Some(mut material) = materials.get_mut(&a.breath) {
            material.effect = Vec4::new(time.elapsed_secs(), strength, 0., 0.);
        }
        d.attack_tick += dt;
        if d.attack_tick >= 0.65 {
            d.attack_tick = 0.;
            for (actor, at) in &actors {
                let distance = at.translation.xy().distance(t.translation.xy());
                if distance < 98. {
                    damage.write(crate::events::damage_events::EntityDamaged {
                        entity: e,
                        damage: 1,
                    });
                }
                let rel = at.translation.xy() - mouth.xy();
                let along = rel.dot(dir);
                if breathing
                    && along >= 0.
                    && along < length + 25.
                    && (rel - dir * along).length() < 25. + along * 0.12
                {
                    damage.write(crate::events::damage_events::EntityDamaged {
                        entity: actor,
                        damage: 1,
                    });
                    commands
                        .entity(actor)
                        .try_insert(Buff::new(BuffType::Burning, 2.));
                }
            }
            if breathing {
                for &block in &touched {
                    block_damage.write(crate::events::block_events::BlockDamaged {
                        entity: block,
                        damage: 1,
                        remaining_health: 0,
                    });
                }
                // Cover the whole breath, including fluid-only cells between blocks.
                let steps = (length / 32.).ceil() as usize;
                for i in 0..=steps {
                    let along = (i as f32 * 32.).min(length);
                    heat.write(TerrainHeat {
                        position: mouth.xy() + dir * along,
                        radius: 25. + along * 0.12,
                    });
                }
            }
        }
    }
    for (entity, breath, _, _) in &mut flames {
        if !dragons.contains(breath.0) {
            commands.entity(entity).despawn();
        }
    }
}

fn dragon_stage_transform() -> Transform {
    // Compress only world depth AFTER the rig's rotation. Preserve world XY,
    // turn foreshortening and projectile silhouettes, with all parts in front.
    Transform::from_xyz(0., 0., 82.8).with_scale(Vec3::new(1., 1., 0.08))
}
#[derive(
    Asset, bevy::reflect::TypePath, bevy::render::render_resource::AsBindGroup, Clone, Debug,
)]
struct SurfaceFireMaterial {
    #[uniform(0)]
    effect: Vec4,
    #[uniform(1)]
    exposed: Vec4,
    #[uniform(2)]
    flow: Vec4,
}
impl Material for SurfaceFireMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/block_fire.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

fn spawn_ground_fire(commands: &mut Commands, owner: Entity, position: Vec3, _a: &DragonAssets) {
    commands.spawn((
        GroundFire {
            owner,
            remaining: 5.,
            tick: 0.,
            origin: position.xy(),
            hops: 0,
        },
        Transform::from_translation(position),
    ));
}
fn surface_exposure(position: Vec3, occupied: &std::collections::HashSet<IVec2>) -> Vec4 {
    let cell = position.xy().round().as_ivec2();
    Vec4::from_array(
        [
            IVec2::new(-64, 0),
            IVec2::new(64, 0),
            IVec2::new(0, -64),
            IVec2::new(0, 64),
        ]
        .map(|offset| {
            if occupied.contains(&(cell + offset)) {
                0.
            } else {
                1.
            }
        }),
    )
}
fn surface_fire(
    mut commands: Commands,
    time: Res<Time>,
    a: Res<DragonAssets>,
    contacts: Res<BreathSurfaces>,
    blocks: Query<(Entity, &Transform), (With<Block>, Without<SurfaceFlame>)>,
    fires: Query<&GroundFire>,
    mut visuals: Query<
        (
            Entity,
            &mut SurfaceFlame,
            &mut Transform,
            &MeshMaterial3d<SurfaceFireMaterial>,
        ),
        Without<Block>,
    >,
    mut materials: ResMut<Assets<SurfaceFireMaterial>>,
) {
    let mut wanted = contacts.0.clone();
    // A lower, independent flame remains only where a combustible block burns.
    for fire in &fires {
        if let Ok((_, transform)) = blocks.get(fire.owner) {
            wanted.entry(fire.owner).or_insert(SurfaceContact {
                position: transform.translation,
                direction: Vec2::Y,
                strength: 0.5 * (fire.remaining / 0.6).clamp(0., 1.),
            });
        }
    }
    if wanted.is_empty() && visuals.is_empty() {
        return;
    }
    let occupied: std::collections::HashSet<_> = blocks
        .iter()
        .map(|(_, t)| t.translation.xy().round().as_ivec2())
        .collect();
    let heated: std::collections::HashSet<_> = wanted
        .iter()
        .filter(|(owner, _)| blocks.contains(**owner))
        .map(|(_, contact)| contact.position.xy().round().as_ivec2())
        .collect();
    let flow = |contact: SurfaceContact| {
        let connected = (Vec4::ONE - surface_exposure(contact.position, &heated))
            .dot(Vec4::new(1., 2., 4., 8.));
        Vec4::new(contact.direction.x, contact.direction.y, connected, 0.)
    };
    for (entity, mut flame, mut transform, handle) in &mut visuals {
        let contact = wanted
            .remove(&flame.owner)
            .filter(|_| blocks.contains(flame.owner));
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.effect.x = time.elapsed_secs();
            if let Some(contact) = contact {
                flame.opacity +=
                    (contact.strength - flame.opacity) * (1. - (-20. * time.delta_secs()).exp());
                let impact = if contacts.0.contains_key(&flame.owner) {
                    1.
                } else {
                    0.
                };
                material.effect.z +=
                    (impact - material.effect.z) * (1. - (-14. * time.delta_secs()).exp());
                material.exposed = surface_exposure(contact.position, &occupied);
                material.flow = flow(contact);
                transform.translation = contact.position + Vec3::new(0., 12., 48.);
            } else {
                // Dissolve in world space: never scale or collapse the flame mesh.
                flame.opacity *= (-time.delta_secs() / 0.055).exp();
            }
            material.effect.y = flame.opacity;
        }
        if contact.is_none() && flame.opacity < 0.015 {
            commands.entity(entity).despawn();
            materials.remove(handle.0.id());
        }
    }
    for (owner, contact) in wanted {
        if !blocks.contains(owner) {
            continue;
        }
        let impact = if contacts.0.contains_key(&owner) {
            1.
        } else {
            0.
        };
        let opacity = contact.strength * (1. - (-20. * time.delta_secs()).exp());
        let material = materials.add(SurfaceFireMaterial {
            effect: Vec4::new(time.elapsed_secs(), opacity, impact, 0.),
            exposed: surface_exposure(contact.position, &occupied),
            flow: flow(contact),
        });
        commands.spawn((
            SurfaceFlame { owner, opacity },
            Mesh3d(a.quad.clone()),
            MeshMaterial3d(material),
            bevy::light::NotShadowCaster,
            Transform::from_translation(contact.position + Vec3::new(0., 12., 48.))
                .with_scale(Vec3::new(96., 112., 1.)),
        ));
    }
}
fn weather_heat(
    time: Res<Time>,
    weather: Res<crate::components::weather::ActiveWeather>,
    hero: Query<&Transform, With<Hero>>,
    blocks: Query<&Transform, With<Block>>,
    mut heat: MessageWriter<TerrainHeat>,
    mut clock: Local<f32>,
    mut serial: Local<usize>,
) {
    use crate::components::weather::WeatherType;
    if weather.weather_type != WeatherType::FireRain
        || weather.duration_remaining <= 0.
        || weather.intensity < 0.1
    {
        return;
    }
    *clock += time.delta_secs();
    if *clock < 1. {
        return;
    }
    *clock %= 1.;
    let Ok(hero) = hero.single() else {
        return;
    };
    let occupied: std::collections::HashSet<_> = blocks
        .iter()
        .map(|t| (t.translation.xy() / 64.).round().as_ivec2())
        .collect();
    let mut exposed: Vec<_> = occupied
        .iter()
        .copied()
        .filter(|p| {
            (p.as_vec2() * 64.).distance_squared(hero.translation.xy()) < 1200_f32.powi(2)
                && (1..=4).all(|dy| !occupied.contains(&(*p + IVec2::Y * dy)))
        })
        .collect();
    exposed.sort_by_key(|p| (p.x, p.y));
    if !exposed.is_empty() {
        let count = exposed.len();
        exposed.rotate_left(*serial % count);
        *serial += 3;
        for p in exposed.into_iter().take(3) {
            heat.write(TerrainHeat {
                position: p.as_vec2() * 64.,
                radius: 35.,
            });
        }
    }
}
fn apply_heat(
    mut commands: Commands,
    mut heat: MessageReader<TerrainHeat>,
    a: Res<DragonAssets>,
    mut fluids: ResMut<super::fluids::FluidWorld>,
    blocks: Query<
        (
            Entity,
            &Transform,
            &BlockType,
            Option<&crate::components::snow_layer::SnowLayer>,
            Option<&super::biome_content::Tile>,
        ),
        With<Block>,
    >,
    fires: Query<&GroundFire>,
    mut damage: MessageWriter<crate::events::block_events::BlockDamaged>,
    weather: Res<crate::components::weather::ActiveWeather>,
) {
    // Deduplicate overlapping samples: one reaction per cell per update.
    let mut cells = std::collections::HashSet::new();
    for h in heat.read() {
        let center = (h.position / 64.).round().as_ivec2();
        let r = (h.radius / 64.).ceil() as i32;
        for y in -r..=r {
            for x in -r..=r {
                let p = center + IVec2::new(x, y);
                if (p.as_vec2() * 64.).distance(h.position) <= h.radius + 32. {
                    cells.insert(p);
                }
            }
        }
    }
    if cells.is_empty() {
        return;
    }
    let wet: std::collections::HashSet<_> = cells
        .iter()
        .copied()
        .filter(|p| fluids.heat_cell((p.x, p.y)))
        .collect();
    let rainy = weather.weather_type == crate::components::weather::WeatherType::Rain
        && weather.intensity >= 0.1
        && weather.duration_remaining > 0.;
    let mut burning: std::collections::HashSet<_> = fires.iter().map(|f| f.owner).collect();
    for (entity, t, kind, cap, tile) in &blocks {
        let p = (t.translation.xy() / 64.).round().as_ivec2();
        if !cells.contains(&p) {
            continue;
        }
        if let Some(cap) = cap {
            let mut cap = *cap;
            cap.melt();
            if cap.has_snow() {
                commands.entity(entity).insert(cap);
            } else {
                commands
                    .entity(entity)
                    .remove::<crate::components::snow_layer::SnowLayer>();
            }
            fluids.melt_cap((p.x, p.y + 1));
        }
        if kind.family() == crate::components::block::MaterialFamily::Frozen {
            damage.write(crate::events::block_events::BlockDamaged {
                entity,
                damage: 1,
                remaining_health: 0,
            });
        }
        if !rainy
            && !wet.contains(&p)
            && tile.map_or_else(|| flammable(*kind), |t| t.combustible())
            && burning.len() < 24
            && burning.insert(entity)
        {
            spawn_ground_fire(&mut commands, entity, t.translation, &a);
        }
    }
}
fn ground_fire(
    mut commands: Commands,
    time: Res<Time>,
    _a: Res<DragonAssets>,
    mut fires: Query<(Entity, &mut GroundFire, &Transform)>,
    blocks: Query<
        (
            Entity,
            &Transform,
            &BlockType,
            Option<&super::biome_content::Tile>,
        ),
        (With<Block>, Without<GroundFire>),
    >,
    mut damage: MessageWriter<crate::events::block_events::BlockDamaged>,
    weather: Res<crate::components::weather::ActiveWeather>,
    fluids: Res<super::fluids::FluidWorld>,
    actors: Query<(Entity, &Transform), (With<Health>, Without<GroundFire>, Without<EndDragon>)>,
    mut hurt: MessageWriter<crate::events::damage_events::EntityDamaged>,
) {
    let mut burning: std::collections::HashSet<_> = fires.iter().map(|(_, f, _)| f.owner).collect();
    for (entity, mut fire, transform) in &mut fires {
        fire.remaining -= time.delta_secs();
        fire.tick += time.delta_secs();
        if fire.remaining <= 0.
            || fluids.is_water_at(transform.translation.xy())
            || (weather.weather_type == crate::components::weather::WeatherType::Rain
                && weather.duration_remaining > 0.
                && weather.intensity >= 0.1)
        {
            commands.entity(entity).despawn();
            continue;
        }
        let Ok((_, _, kind, tile)) = blocks.get(fire.owner) else {
            fire.remaining = fire.remaining.min(0.45);
            continue;
        };
        if !tile.map_or_else(|| flammable(*kind), |t| t.combustible()) {
            continue;
        }
        if fire.tick < 1. {
            continue;
        }
        fire.tick = 0.;
        for (actor, t) in &actors {
            if t.translation.xy().distance(transform.translation.xy()) < 65. {
                hurt.write(crate::events::damage_events::EntityDamaged {
                    entity: actor,
                    damage: 1,
                });
                commands
                    .entity(actor)
                    .try_insert(Buff::new(BuffType::Burning, 2.));
            }
        }
        damage.write(crate::events::block_events::BlockDamaged {
            entity: fire.owner,
            damage: 1,
            remaining_health: 0,
        });
        for (other, t, kind, tile) in &blocks {
            if burning.len() >= 24 {
                break;
            }
            if fire.hops < 4
                && t.translation.xy().distance(fire.origin) <= 4. * 64.
                && tile.map_or_else(|| flammable(*kind), |t| t.combustible())
                && !burning.contains(&other)
                && t.translation.xy().distance(transform.translation.xy()) < 70.
            {
                commands.spawn((
                    GroundFire {
                        owner: other,
                        remaining: 5.,
                        tick: 0.,
                        origin: fire.origin,
                        hops: fire.hops + 1,
                    },
                    Transform::from_translation(t.translation),
                ));
                burning.insert(other);
                break;
            }
        }
    }
}

impl DragonHitPart {
    pub(crate) fn hit(&self, transform: &GlobalTransform, from: Vec2, to: Vec2) -> Option<f32> {
        let a = transform.affine();
        if self.half.x >= 0. && a.matrix3.z_axis.xy().length_squared() > 0.0001 {
            let vertices: [Vec2; 8] = std::array::from_fn(|i| {
                transform
                    .transform_point(Vec3::new(
                        if i & 1 == 0 {
                            -self.half.x
                        } else {
                            self.half.x
                        },
                        if i & 2 == 0 {
                            -self.half.y
                        } else {
                            self.half.y
                        },
                        if i & 4 == 0 { -0.5 } else { 0.5 },
                    ))
                    .xy()
            });
            let min = vertices
                .iter()
                .copied()
                .fold(Vec2::splat(f32::INFINITY), Vec2::min);
            let max = vertices
                .iter()
                .copied()
                .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max);
            super::projectile_system::segment_box(from, to, (min + max) * 0.5, (max - min) * 0.5)?;
            return [
                [0, 1, 3, 2],
                [4, 5, 7, 6],
                [0, 1, 5, 4],
                [2, 3, 7, 6],
                [0, 2, 6, 4],
                [1, 3, 7, 5],
            ]
            .into_iter()
            .flat_map(|[a, b, c, d]| [[a, b, c], [a, c, d]])
            .filter_map(|[a, b, c]| {
                segment_triangle(from, to, [vertices[a], vertices[b], vertices[c]])
            })
            .min_by(f32::total_cmp);
        }
        let matrix = Mat2::from_cols(a.matrix3.x_axis.xy(), a.matrix3.y_axis.xy());
        if matrix.determinant().abs() < 0.001 {
            return None;
        }
        let inverse = matrix.inverse();
        let from = inverse * (from - transform.translation().xy());
        let to = inverse * (to - transform.translation().xy());
        if self.half.x >= 0. {
            return super::projectile_system::segment_box(from, to, Vec2::ZERO, self.half);
        }
        // Test the same triangle fan used by the visible articulated wing.
        let points = [
            Vec2::new(0., 0.),
            Vec2::new(55., 130.),
            Vec2::new(116., 107.),
            Vec2::new(86., 83.),
            Vec2::new(95., 66.),
            Vec2::new(65., 47.),
            Vec2::new(69., 28.),
            Vec2::new(40., 19.),
            Vec2::new(48., 0.),
        ];
        (1..points.len() - 1)
            .filter_map(|i| segment_triangle(from, to, [points[0], points[i], points[i + 1]]))
            .min_by(f32::total_cmp)
    }
}
fn segment_triangle(from: Vec2, to: Vec2, triangle: [Vec2; 3]) -> Option<f32> {
    let cross = |a: Vec2, b: Vec2| a.x * b.y - a.y * b.x;
    if cross(triangle[1] - triangle[0], triangle[2] - triangle[0]).abs() < 0.00001 {
        return None;
    }
    let sides = [
        cross(triangle[1] - triangle[0], from - triangle[0]),
        cross(triangle[2] - triangle[1], from - triangle[1]),
        cross(triangle[0] - triangle[2], from - triangle[2]),
    ];
    if sides.iter().all(|v| *v >= 0.) || sides.iter().all(|v| *v <= 0.) {
        return Some(0.);
    }
    let dir = to - from;
    (0..3)
        .filter_map(|i| {
            let a = triangle[i];
            let edge = triangle[(i + 1) % 3] - a;
            let denominator = cross(dir, edge);
            if denominator.abs() < 0.00001 {
                return None;
            }
            let t = cross(a - from, edge) / denominator;
            let u = cross(a - from, dir) / denominator;
            ((0. ..=1.).contains(&t) && (0. ..=1.).contains(&u)).then_some(t)
        })
        .min_by(f32::total_cmp)
}

#[derive(Component)]
struct BossLabel;

/// Deterministic surface and light gallery for the hidden visual probe only.

#[derive(Resource)]
struct FireGallery {
    mouth: Vec3,
    wall: Vec<Entity>,
    beam: Entity,
}

fn fire_gallery_contacts(
    time: Res<Time<Real>>,
    gallery: Option<Res<FireGallery>>,
    mut contacts: ResMut<BreathSurfaces>,
    blocks: Query<(Entity, &Transform), With<Block>>,
    mut visible: Query<&mut Visibility>,
    a: Res<DragonAssets>,
    mut materials: ResMut<Assets<BreathMaterial>>,
) {
    let Some(gallery) = gallery else {
        return;
    };
    let active = time.elapsed_secs() < 12.;
    if let Ok(mut v) = visible.get_mut(gallery.beam) {
        *v = if active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Some(mut mat) = materials.get_mut(&a.breath) {
        mat.effect = Vec4::new(time.elapsed_secs(), if active { 1. } else { 0. }, 0., 0.);
    }
    if !active {
        return;
    }
    let candidates: Vec<_> = gallery
        .wall
        .iter()
        .filter_map(|&e| blocks.get(e).ok().map(|(e, t)| (e, t.translation.xy())))
        .collect();
    for owner in exposed_breath_blocks(gallery.mouth.xy(), Vec2::X, &candidates) {
        let (_, t) = blocks.get(owner).unwrap();
        contacts.0.insert(
            owner,
            SurfaceContact {
                position: t.translation,
                direction: Vec2::X,
                strength: 1.,
            },
        );
    }
}

#[path = "dragon_flyover/visuals.rs"]
mod visuals;
use visuals::*;

#[path = "dragon_flyover/probe.rs"]
mod probe;
pub(crate) use probe::*;

#[cfg(test)]
#[path = "dragon_flyover/encounter_tests.rs"]
mod encounter_tests;
