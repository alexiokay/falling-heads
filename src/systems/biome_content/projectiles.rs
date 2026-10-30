// Explicit ECS query filters document the non-overlap required by Bevy.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]
//! Swept projectiles: earliest terrain/actor hit wins, including a long frame.
use super::art::Art;
use crate::{
    components::{
        block::Block,
        buff::{Buff, BuffType},
        damage_visual::DeathAnimation,
        enemy::{Enemy, Health},
    },
    events::{block_events::BlockDamaged, damage_events::EntityDamaged},
    resources::spatial_hash::SpatialHash,
};
use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Poison,
    Frost,
    Wind,
    Shulker,
    Crystal,
    Fire,
}
impl Kind {
    fn index(self) -> usize {
        match self {
            Self::Poison => 0,
            Self::Frost => 1,
            Self::Wind => 2,
            Self::Shulker => 3,
            Self::Crystal => 4,
            Self::Fire => 5,
        }
    }
    fn speed(self) -> f32 {
        match self {
            Self::Shulker => 150.,
            Self::Fire => 390.,
            Self::Wind => 310.,
            Self::Crystal => 280.,
            Self::Poison | Self::Frost => crate::utils::constants::ARROW_SPEED,
        }
    }
}
#[derive(Component)]
pub(super) struct Shot {
    kind: Kind,
    pub velocity: Vec2,
    age: f32,
    target: Option<Entity>,
    owner: Option<Entity>,
    pub friendly: bool,
}
#[derive(Component)]
pub(super) struct Lift(pub f32);
pub(super) fn spawn(
    commands: &mut Commands,
    art: &Art,
    p: Vec3,
    direction: Vec2,
    kind: Kind,
    target: Option<Entity>,
    owner: Option<Entity>,
) -> Entity {
    let arrow = matches!(kind, Kind::Poison | Kind::Frost);
    commands
        .spawn((
            Shot {
                kind,
                velocity: direction.normalize_or_zero() * kind.speed(),
                age: 0.,
                target,
                owner,
                friendly: false,
            },
            Mesh3d(art.quad.clone()),
            MeshMaterial3d(art.shots[kind.index()].clone()),
            Transform::from_translation(Vec3::new(p.x, p.y, 60.)).with_scale(if arrow {
                Vec3::new(44., 44., 1.)
            } else {
                Vec3::new(26., 26., 1.)
            }),
            bevy::light::NotShadowCaster,
        ))
        .id()
}
pub(super) fn terrain_hit(
    from: Vec2,
    to: Vec2,
    spatial: &SpatialHash,
    blocks: &Query<
        (
            &Transform,
            &crate::components::block::BlockType,
            Option<&super::Tile>,
        ),
        With<Block>,
    >,
) -> Option<(Entity, f32)> {
    spatial
        .rect_iter(
            from.min(to) - Vec2::splat(36.),
            from.max(to) + Vec2::splat(36.),
        )
        .filter_map(|e| {
            let (t, kind, tile) = blocks.get(e).ok()?;
            if crate::systems::plant_rules::soft(*kind, tile) {
                return None;
            }
            crate::systems::terrain_shape::hit(from, to, t.translation.xy(), *kind, tile)
                .map(|u| (e, u))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}
pub(super) fn turn(current: Vec2, target: Vec2, max_angle: f32) -> Vec2 {
    if current.length_squared() < 0.01 || target.length_squared() < 0.01 {
        return current;
    }
    let a = current.y.atan2(current.x);
    let b = target.y.atan2(target.x);
    let delta =
        (b - a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    Vec2::from_angle(a + delta.clamp(-max_angle, max_angle)) * current.length()
}
pub(super) fn advance(
    mut commands: Commands,
    time: Res<Time>,
    gravity: Res<avian3d::prelude::Gravity>,
    spatial: Res<SpatialHash>,
    blocks: Query<
        (
            &Transform,
            &crate::components::block::BlockType,
            Option<&super::Tile>,
        ),
        With<Block>,
    >,
    mut shots: Query<(Entity, &mut Shot, &mut Transform), Without<Block>>,
    actors: Query<
        (
            Entity,
            &Transform,
            Option<&Enemy>,
            Option<&Buff>,
            Option<&Mesh3d>,
        ),
        (With<Health>, Without<Shot>, Without<DeathAnimation>),
    >,
    barriers: Query<
        (
            Entity,
            &Transform,
            &crate::components::projectile::MagicBarrier,
        ),
        Without<Shot>,
    >,
    meshes: Res<Assets<Mesh>>,
    mut damage: MessageWriter<EntityDamaged>,
    mut heat: MessageWriter<crate::systems::dragon_flyover::TerrainHeat>,
) {
    let dt = time.delta_secs();
    for (entity, mut shot, mut t) in &mut shots {
        shot.age += dt;
        if shot.age > 6. {
            commands.entity(entity).despawn();
            continue;
        }
        if shot.kind == Kind::Shulker
            && shot.age < 4.
            && let Some(target) = shot.target.and_then(|e| actors.get(e).ok())
        {
            shot.velocity = turn(
                shot.velocity,
                target.1.translation.xy() - t.translation.xy(),
                0.85 * dt,
            );
        }
        let from = t.translation.xy();
        if matches!(shot.kind, Kind::Poison | Kind::Frost) {
            shot.velocity += gravity.0.xy() * dt;
        }
        let to = from + shot.velocity * dt;
        let terrain = terrain_hit(from, to, &spatial, &blocks);
        let victim = actors
            .iter()
            .filter(|(e, _, enemy, _, _)| {
                Some(*e) != shot.owner
                    && ((shot.kind == Kind::Crystal && !shot.friendly)
                        || enemy.is_some() == shot.friendly)
            })
            .filter_map(|(e, t, _, buff, mesh)| {
                let half = crate::systems::head_geometry::bounds(&meshes, mesh)
                    .size
                    .xy()
                    * t.scale.xy().abs()
                    * 0.5
                    + Vec2::splat(5.);
                crate::systems::projectile_system::segment_box(from, to, t.translation.xy(), half)
                    .map(|u| (e, u, buff.is_some()))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let actor_first = victim.is_some_and(|(_, u, _)| terrain.is_none_or(|(_, v)| u <= v));
        let impact = if actor_first {
            victim.map(|(_, u, _)| u)
        } else {
            terrain.map(|(_, u)| u)
        };
        if !shot.friendly
            && let Some((barrier, bt, owner, u)) = barriers
                .iter()
                .filter_map(|(e, t, b)| {
                    circle_entry(from, to, t.translation.xy(), b.radius)
                        .map(|u| (e, t, b.owner_entity, u))
                })
                .min_by(|a, b| a.3.total_cmp(&b.3))
            && impact.is_none_or(|v| u <= v)
        {
            let p = from.lerp(to, u);
            let n = (p - bt.translation.xy()).normalize_or_zero();
            shot.velocity = n * shot.velocity.length();
            shot.friendly = true;
            shot.target = shot.owner;
            shot.owner = Some(owner);
            t.translation = (p + n * 2.).extend(t.translation.z);
            crate::plugins::powers_plugin::spawn_barrier_impact(
                &mut commands,
                barrier,
                t.translation,
                bt.translation,
            );
            continue;
        }
        if let Some(u) = impact {
            let p = from.lerp(to, u);
            if actor_first {
                let (e, _, buff) = victim.unwrap();
                if shot.kind != Kind::Wind {
                    damage.write(EntityDamaged {
                        entity: e,
                        damage: 1,
                    });
                }
                if !buff {
                    let effect = match shot.kind {
                        Kind::Poison => Some((BuffType::Poisoned, 2.)),
                        Kind::Frost => Some((BuffType::Slowness, 2.5)),
                        Kind::Fire => Some((BuffType::Burning, 2.)),
                        _ => None,
                    };
                    if let Some((kind, duration)) = effect {
                        commands.entity(e).try_insert(Buff::new(kind, duration));
                    }
                }
                if shot.kind == Kind::Shulker {
                    commands.entity(e).try_insert(Lift(1.2));
                }
            }
            if shot.kind == Kind::Fire {
                heat.write(crate::systems::dragon_flyover::TerrainHeat {
                    position: p,
                    radius: 85.,
                });
            }
            if shot.kind == Kind::Wind {
                wind(&mut commands, p);
            }
            commands.entity(entity).despawn();
        } else {
            t.translation.x = to.x;
            t.translation.y = to.y;
            // Pack arrow icon points up/right; align its diagonal with travel.
            t.rotation = Quat::from_rotation_z(
                shot.velocity.y.atan2(shot.velocity.x)
                    - if matches!(shot.kind, Kind::Poison | Kind::Frost) {
                        std::f32::consts::FRAC_PI_4
                    } else {
                        0.
                    },
            );
        }
    }
}
fn impulse(origin: Vec2, p: Vec2) -> Vec2 {
    let delta = p - origin;
    let distance = delta.length();
    if distance > 210. {
        return Vec2::ZERO;
    }
    (delta.normalize_or_zero() + Vec2::Y * 0.25) * 380. * (1. - distance / 210.)
}
fn wind(commands: &mut Commands, p: Vec2) {
    commands.queue(move |world: &mut World| {
        let mut actors = world.query::<(&Transform, &mut LinearVelocity)>();
        for (t, mut v) in actors.iter_mut(world) {
            v.0 += impulse(p, t.translation.xy()).extend(0.);
        }
        let mut shots = world.query::<(&Transform, &mut Shot)>();
        for (t, mut s) in shots.iter_mut(world) {
            s.velocity += impulse(p, t.translation.xy());
        }
    });
}
pub(super) fn blast(commands: &mut Commands, p: Vec3, radius: f32, owner: Option<Entity>) {
    commands.queue(move |world: &mut World| {
        let victims: Vec<_> = world
            .query_filtered::<(Entity, &Transform), With<Health>>()
            .iter(world)
            .filter(|(e, t)| Some(*e) != owner && t.translation.xy().distance(p.xy()) < radius)
            .map(|(e, _)| e)
            .collect();
        for entity in victims {
            world.write_message(EntityDamaged { entity, damage: 3 });
        }
        let blocks: Vec<_> = world
            .query_filtered::<(Entity, &Transform), With<Block>>()
            .iter(world)
            .filter(|(_, t)| t.translation.xy().distance(p.xy()) < radius)
            .map(|(e, _)| e)
            .collect();
        for entity in blocks {
            world.write_message(BlockDamaged {
                entity,
                damage: 3,
                remaining_health: 0,
            });
        }
        let frames = world
            .get_resource::<crate::resources::asset_handles::ExplosionTextures>()
            .map(|t| t.frames.clone())
            .unwrap_or_default();
        if let Some(texture) = frames.first() {
            let mesh = world.resource::<Art>().quad.clone();
            let material = world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color_texture: Some(texture.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                });
            world.spawn((
                crate::components::explosive::ExplosionAnimation {
                    timer: Timer::from_seconds(0.035, TimerMode::Repeating),
                    current_frame: 0,
                    total_frames: frames.len(),
                    delay_timer: Timer::from_seconds(0., TimerMode::Once),
                    started: false,
                },
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_translation(p + Vec3::Z * 70.).with_scale(Vec3::new(
                    radius * 2.,
                    radius * 2.,
                    1.,
                )),
                Visibility::Hidden,
                bevy::light::NotShadowCaster,
            ));
        }
        if world.contains_resource::<crate::systems::rocket_audio::RocketSounds>() {
            crate::systems::rocket_audio::play(world, 2);
        }
    });
}

fn circle_entry(from: Vec2, to: Vec2, center: Vec2, radius: f32) -> Option<f32> {
    let d = to - from;
    let q = from - center;
    let a = d.length_squared();
    if a < 0.00001 || q.dot(d) >= 0. {
        return None;
    }
    let c = q.length_squared() - radius * radius;
    if c <= 0. {
        return Some(0.);
    }
    let b = q.dot(d);
    let discriminant = b * b - a * c;
    if discriminant < 0. {
        return None;
    }
    let u = (-b - discriminant.sqrt()) / a;
    (0.0..=1.0).contains(&u).then_some(u)
}
#[cfg(test)]
mod contact_tests {
    use super::*;
    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(avian3d::prelude::Gravity(Vec3::ZERO))
            .insert_resource(SpatialHash::new(256.))
            .init_resource::<Assets<Mesh>>()
            .add_message::<EntityDamaged>()
            .add_message::<crate::systems::dragon_flyover::TerrainHeat>()
            .add_systems(Update, advance);
        app
    }
    fn shot(app: &mut App, kind: Kind) -> Entity {
        app.world_mut()
            .spawn((
                Shot {
                    kind,
                    velocity: Vec2::new(500., 0.),
                    age: 0.,
                    target: None,
                    owner: None,
                    friendly: false,
                },
                Transform::from_xyz(0., 0., 60.),
            ))
            .id()
    }
    #[test]
    fn terrain_occludes_actor_and_emits_shared_fire_heat() {
        let mut app = app();
        let wall = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                Transform::from_xyz(100., 0., -10.),
                crate::components::block::BlockType::Stone,
            ))
            .id();
        app.world_mut()
            .resource_mut::<SpatialHash>()
            .insert(wall, 100., 0.);
        app.world_mut()
            .spawn((Health::new(5), Transform::from_xyz(200., 0., 30.)));
        let projectile = shot(&mut app, Kind::Fire);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.5));
        app.update();
        assert!(app.world().get_entity(projectile).is_err());
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<EntityDamaged>>()
                .drain()
                .count(),
            0
        );
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<crate::systems::dragon_flyover::TerrainHeat>>()
                .drain()
                .count(),
            1
        );
    }
    #[test]
    fn poison_hits_once_on_contact_not_on_a_timer() {
        let mut app = app();
        let actor = app
            .world_mut()
            .spawn((Health::new(5), Transform::from_xyz(100., 0., 30.)))
            .id();
        let projectile = shot(&mut app, Kind::Poison);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.2));
        app.update();
        assert!(app.world().get_entity(projectile).is_err());
        assert_eq!(
            app.world().get::<Buff>(actor).unwrap().buff_type,
            BuffType::Poisoned
        );
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<EntityDamaged>>()
                .drain()
                .count(),
            1
        );
        app.update();
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<EntityDamaged>>()
                .drain()
                .count(),
            0
        );
    }
    #[test]
    fn mirror_intercepts_fast_projectiles_before_the_owner() {
        assert!(
            (circle_entry(Vec2::ZERO, Vec2::new(500., 0.), Vec2::new(300., 0.), 100.).unwrap()
                - 0.4)
                .abs()
                < 0.0001
        );
        assert_eq!(
            circle_entry(
                Vec2::new(220., 0.),
                Vec2::new(150., 0.),
                Vec2::new(300., 0.),
                100.
            ),
            None
        );
    }
}

#[cfg(test)]
mod speed_tests {
    use super::*;
    #[test]
    fn tipped_arrows_use_normal_arrow_launch_speed() {
        assert_eq!(Kind::Poison.speed(), crate::utils::constants::ARROW_SPEED);
        assert_eq!(Kind::Frost.speed(), crate::utils::constants::ARROW_SPEED);
    }
}
