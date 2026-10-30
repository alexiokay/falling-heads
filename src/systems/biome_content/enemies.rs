// Explicit ECS query filters document the non-overlap required by Bevy.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]
use super::{
    art::Art,
    projectiles::{self, Kind},
};
use crate::components::{
    buff::Buffable,
    bundles::avian_physics,
    damage_visual::DeathAnimation,
    enemy::{Enemy, EnemyBehavior, EnemyType as E, Flying, Health},
};
use avian3d::prelude::*;
use bevy::prelude::*;
pub(super) const SPECIES: [E; 10] = [
    E::Slime,
    E::Creeper,
    E::Bogged,
    E::Husk,
    E::Breeze,
    E::Silverfish,
    E::Blaze,
    E::MagmaCube,
    E::Stray,
    E::Shulker,
];
impl E {
    pub(super) fn asset(self) -> &'static str {
        match self {
            E::Slime => "slime",
            E::Creeper => "creeper",
            E::Bogged => "bogged",
            E::Husk => "husk",
            E::Breeze => "breeze",
            E::Silverfish => "silverfish",
            E::Blaze => "blaze",
            E::MagmaCube => "magma_cube",
            E::Stray => "stray",
            E::Shulker => "shulker",
            _ => "slime",
        }
    }
}
#[derive(Component)]
pub(crate) struct BiomeFoe {
    pub(super) cooldown: f32,
    age: f32,
    fuse: f32,
    burst: u8,
    aim: Vec2,
    previous_y: f32,
    anchor: Option<Entity>,
    pub(super) generation: u8,
}
#[derive(Component)]
pub(super) struct Rod(f32);
pub(crate) fn spawn(commands: &mut Commands, p: Vec3, kind: E, generation: u8) -> Entity {
    commands
        .spawn((
            Enemy::new(kind),
            Health::new(kind.max_health()),
            Buffable,
            crate::components::chunk::ChunkMember {
                chunk_x: (p.x / 1024.).floor() as i32,
                chunk_y: (p.y / 1024.).floor() as i32,
            },
            avian_physics::enemy_with_velocity(Vec3::ZERO),
            Transform::from_translation(p),
            Visibility::Inherited,
            BiomeFoe {
                cooldown: 1.5,
                age: 0.,
                fuse: 0.,
                burst: 0,
                aim: Vec2::X,
                previous_y: 0.,
                anchor: None,
                generation,
            },
        ))
        .id()
}
pub(super) fn attach(
    mut commands: Commands,
    art: Res<Art>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    q: Query<(Entity, &Enemy, Option<&BiomeFoe>), Added<Enemy>>,
) {
    for (e, enemy, foe) in &q {
        let Some(index) = SPECIES.iter().position(|s| *s == enemy.enemy_type) else {
            continue;
        };
        let kind = enemy.enemy_type;
        // Damage/fuse tint is per actor; never mutate a species-wide material.
        let source = mats.get(&art.faces[index]).unwrap().clone();
        let face = mats.add(source);
        commands.entity(e).insert((
            Mesh3d(if kind == E::Silverfish {
                art.small_face.clone()
            } else {
                art.face.clone()
            }),
            MeshMaterial3d(face),
        ));
        if foe.is_none() {
            commands.entity(e).insert(BiomeFoe {
                cooldown: 1.5 + (e.to_bits() % 7) as f32 * 0.17,
                age: 0.,
                fuse: 0.,
                burst: 0,
                aim: Vec2::X,
                previous_y: 0.,
                anchor: None,
                generation: 0,
            });
        }
        if kind == E::Blaze {
            commands.entity(e).insert((Flying, GravityScale(0.)));
        }
        if kind == E::Silverfish {
            commands.entity(e).insert((
                Collider::cuboid(42., 28., 40.),
                crate::components::enemy::TerrainFootprint(Vec2::new(21., 14.)),
            ));
        }
        if kind == E::Blaze {
            for i in 0..6 {
                let phase = i as f32 * std::f32::consts::TAU / 6.;
                commands.spawn((
                    ChildOf(e),
                    Rod(phase),
                    Mesh3d(art.quad.clone()),
                    MeshMaterial3d(art.drops[1].clone()),
                    Transform::from_xyz(phase.cos() * 43., phase.sin() * 30., -2.)
                        .with_scale(Vec3::new(12., 34., 1.)),
                    bevy::light::NotShadowCaster,
                ));
            }
        }
    }
}
pub(super) fn think(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<Art>,
    targets: Res<crate::systems::target_index::ActorTargets>,
    spatial: Res<crate::resources::spatial_hash::SpatialHash>,
    blocks: Query<
        (
            &Transform,
            &crate::components::block::BlockType,
            Option<&super::Tile>,
        ),
        With<crate::components::block::Block>,
    >,
    mut foes: Query<
        (
            Entity,
            &Enemy,
            &Transform,
            &mut LinearVelocity,
            &mut BiomeFoe,
            &Health,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Without<DeathAnimation>,
    >,
    mut rods: Query<
        (&Rod, &mut Transform),
        (Without<Enemy>, Without<crate::components::block::Block>),
    >,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut damage: MessageWriter<crate::events::damage_events::EntityDamaged>,
) {
    let dt = time.delta_secs();
    let sound = |commands: &mut Commands, index: usize, at: Vec3| {
        commands.spawn((
            AudioPlayer::new(art.sounds[index].clone()),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.28)),
            crate::systems::audio::effects::SoundAt(at.xy()),
        ));
    };
    for (rod, mut t) in &mut rods {
        let a = rod.0 + time.elapsed_secs() * 1.8;
        t.translation = Vec3::new(a.cos() * 43., a.sin() * 28., a.sin() * 8. - 3.);
    }
    for (e, enemy, t, mut v, mut foe, hp, mat) in &mut foes {
        if hp.current == 0 {
            continue;
        }
        let kind = enemy.enemy_type;
        foe.age += dt;
        foe.cooldown = (foe.cooldown - dt).max(0.);
        if kind == E::MagmaCube && foe.previous_y < 0. && v.y > 0. {
            v.y = if kind == E::Slime { 430. } else { 550. };
            sound(
                &mut commands,
                if kind == E::Slime { 5 } else { 6 },
                t.translation,
            );
        }
        if kind == E::Slime && foe.previous_y <= 0. && v.y > 200. {
            sound(&mut commands, 5, t.translation);
        }
        foe.previous_y = v.y;
        if kind == E::Shulker {
            if let Some(anchor) = foe.anchor {
                if !blocks.contains(anchor) {
                    foe.anchor = None;
                    commands
                        .entity(e)
                        .insert((RigidBody::Dynamic, GravityScale(1.5)))
                        .remove::<Flying>();
                }
            } else {
                let nearest = spatial
                    .nearby_iter(t.translation.x, t.translation.y)
                    .filter_map(|b| {
                        blocks
                            .get(b)
                            .ok()
                            .filter(|(_, kind, tile)| {
                                !crate::systems::plant_rules::soft(**kind, *tile)
                            })
                            .map(|(bt, _, _)| (b, bt.translation))
                    })
                    .filter(|(_, p)| p.xy().distance(t.translation.xy()) < 120.)
                    .min_by(|a, b| {
                        a.1.distance_squared(t.translation)
                            .total_cmp(&b.1.distance_squared(t.translation))
                    });
                if let Some((block, p)) = nearest {
                    let d = t.translation - p;
                    let normal = if d.x.abs() > d.y.abs() {
                        Vec3::X * d.x.signum()
                    } else {
                        Vec3::Y * d.y.signum()
                    };
                    let p = Vec3::new(p.x, p.y, t.translation.z) + normal * 65.;
                    foe.anchor = Some(block);
                    commands.entity(e).insert((
                        Position(p),
                        Transform::from_translation(p),
                        RigidBody::Static,
                        GravityScale(0.),
                        Flying,
                    ));
                    v.0 = Vec3::ZERO;
                }
            }
        }
        let Some(target) = targets.nearest(t.translation, 850.) else {
            continue;
        };
        let d = (target.position - t.translation).xy();
        let range = d.length();
        let direction = d.normalize_or_zero();
        let clear =
            projectiles::terrain_hit(t.translation.xy(), target.position.xy(), &spatial, &blocks)
                .is_none();
        if kind == E::Creeper {
            if range < 155. && clear {
                if foe.fuse == 0. {
                    sound(&mut commands, 4, t.translation);
                }
                foe.fuse += dt;
            } else {
                foe.fuse = (foe.fuse - dt * 2.).max(0.);
            }
            if let Some(mut m) = mats.get_mut(&mat.0) {
                m.base_color = if foe.fuse > 0. && (foe.age * (8. + foe.fuse * 12.)).sin() > 0. {
                    Color::srgb(2., 2., 2.)
                } else {
                    Color::WHITE
                };
            }
            if foe.fuse >= 1.45 {
                projectiles::blast(&mut commands, t.translation, 180., Some(e));
                damage.write(crate::events::damage_events::EntityDamaged {
                    entity: e,
                    damage: 100,
                });
                continue;
            }
        }
        let ranged = matches!(
            kind,
            E::Blaze | E::Breeze | E::Bogged | E::Stray | E::Shulker
        );
        if kind == E::Blaze {
            let charging = foe.cooldown <= 0.8 || foe.burst > 0;
            if charging && foe.fuse == 0. {
                foe.aim = direction;
                foe.fuse = 1.;
            }
            // Stop to telegraph a locked-aim burst; drift sideways and settle
            // during recovery, leaving a clear opening between volleys.
            let desired = if charging {
                Vec2::new(0., 16.)
            } else {
                Vec2::new(
                    if range < 250. {
                        -direction.x * 95.
                    } else if range > 440. {
                        direction.x * 90.
                    } else {
                        (foe.age * 0.6).sin() * 80.
                    },
                    (target.position.y + 100. - t.translation.y).clamp(-55., 55.),
                )
            };
            v.0 = v.0.lerp(desired.extend(0.), 1. - (-4. * dt).exp());
            if let Some(mut material) = mats.get_mut(&mat.0) {
                material.emissive = if charging {
                    LinearRgba::rgb(0.8, 0.22, 0.01)
                } else {
                    LinearRgba::BLACK
                };
            }
        } else if kind == E::Breeze {
            // A ground-contact skirmisher: fast lateral leaps, then brake near
            // the apex to fire a knockback charge. Real terrain still blocks it.
            let side = if range < 200. {
                -d.x.signum()
            } else {
                d.x.signum()
            };
            let desired = if foe.cooldown > 1.2 { side * 240. } else { 0. };
            v.x += (desired - v.x) * (1. - (-5. * dt).exp());
        } else if kind != E::Shulker {
            let speed = if kind == E::Silverfish {
                210.
            } else if kind == E::Husk {
                70.
            } else {
                125.
            };
            let sign = if ranged && range < 240. {
                -d.x.signum()
            } else {
                d.x.signum()
            };
            if v.x.abs() < 230. {
                v.x += (sign * speed - v.x) * (1. - (-3. * dt).exp());
            }
        }
        if !ranged || !clear || foe.cooldown > 0. || range < 90. {
            continue;
        }
        if kind == E::Breeze && (v.y > 50. || v.y < -120.) {
            continue;
        }
        let shot = match kind {
            E::Bogged => Kind::Poison,
            E::Stray => Kind::Frost,
            E::Breeze => Kind::Wind,
            E::Shulker => Kind::Shulker,
            _ => Kind::Fire,
        };
        if kind == E::Blaze && foe.burst == 0 {
            sound(&mut commands, 0, t.translation);
        } else if kind == E::Shulker {
            sound(&mut commands, 3, t.translation);
        }
        let direction = if kind == E::Blaze {
            Vec2::from_angle((foe.burst as f32 - 1.) * 0.055).rotate(foe.aim)
        } else {
            direction
        };
        let start = t.translation + direction.extend(0.) * 45.;
        projectiles::spawn(
            &mut commands,
            &art,
            start,
            direction,
            shot,
            Some(target.entity),
            Some(e),
        );
        if kind == E::Blaze {
            foe.burst += 1;
            if foe.burst < 3 {
                foe.cooldown = 0.22;
            } else {
                foe.burst = 0;
                foe.cooldown = 4.0;
                foe.fuse = 0.;
            }
        } else {
            foe.cooldown = if kind == E::Shulker { 4.5 } else { 3.2 };
        }
    }
}
pub(super) fn deaths(
    mut commands: Commands,
    mut events: MessageReader<crate::events::damage_events::EntityDied>,
    q: Query<(&Enemy, &Transform, &BiomeFoe)>,
    art: Res<Art>,
) {
    for event in events.read() {
        let Ok((enemy, t, foe)) = q.get(event.entity) else {
            continue;
        };
        if enemy.enemy_type == E::Blaze {
            commands.spawn((
                AudioPlayer::new(art.sounds[2].clone()),
                PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.35)),
                crate::systems::audio::effects::SoundAt(t.translation.xy()),
            ));
        }
        if matches!(enemy.enemy_type, E::Slime | E::MagmaCube) && foe.generation == 0 {
            for side in [-1., 1.] {
                let child = spawn(
                    &mut commands,
                    t.translation + Vec3::X * side * 35.,
                    enemy.enemy_type,
                    1,
                );
                commands.entity(child).insert((
                    Transform::from_translation(t.translation + Vec3::X * side * 35.)
                        .with_scale(Vec3::splat(0.55)),
                    Health::new(1),
                    LinearVelocity(Vec3::new(side * 150., 220., 0.)),
                ));
            }
        }
        let drop = match enemy.enemy_type {
            E::Blaze => Some(1),
            E::Slime | E::MagmaCube => Some(2),
            E::Shulker => Some(3),
            _ => None,
        };
        if let Some(kind) = drop {
            super::interactions::drop_item(&mut commands, &art, t.translation, kind);
        }
    }
}

pub(super) fn hurt_audio(
    mut commands: Commands,
    mut events: MessageReader<crate::events::damage_events::EntityDamaged>,
    foes: Query<&Enemy, With<BiomeFoe>>,
    art: Res<Art>,
) {
    for event in events.read() {
        if foes
            .get(event.entity)
            .is_ok_and(|e| e.enemy_type == E::Blaze)
        {
            commands.spawn((
                AudioPlayer::new(art.sounds[1].clone()),
                PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.25)),
            ));
        }
    }
}
