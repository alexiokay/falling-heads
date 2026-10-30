// Explicit ECS query filters document the non-overlap required by Bevy.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]
use super::{
    art::Art,
    catalog::Tile,
    projectiles::{self, Kind, Lift},
};
use crate::components::{
    buff::{Buff, BuffType},
    damage_visual::DeathAnimation,
    enemy::{EnemyType, Health},
    hero::Hero,
    villager::Villager,
};
use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
#[derive(Component)]
pub(super) struct Pickup {
    kind: usize,
    life: f32,
}
pub(super) fn drop_item(commands: &mut Commands, art: &Art, p: Vec3, kind: usize) {
    commands.spawn((
        Pickup { kind, life: 25. },
        Mesh3d(art.quad.clone()),
        MeshMaterial3d(art.drops[kind].clone()),
        Transform::from_xyz(p.x, p.y, 60.).with_scale(Vec3::new(26., 26., 1.)),
        bevy::light::NotShadowCaster,
    ));
}
pub(super) fn broken(
    mut commands: Commands,
    mut events: MessageReader<crate::events::block_events::BlockDestroyed>,
    art: Res<Art>,
) {
    for e in events.read() {
        match e.tile {
            Some(Tile::Infested) => {
                super::enemies::spawn(
                    &mut commands,
                    e.position + Vec3::Z * 50.,
                    EnemyType::Silverfish,
                    0,
                );
            }
            Some(Tile::Crystal | Tile::Amethyst) => {
                for i in 0..5 {
                    let angle = std::f32::consts::PI * (0.15 + i as f32 * 0.175);
                    let d = Vec2::from_angle(angle);
                    projectiles::spawn(
                        &mut commands,
                        &art,
                        e.position + d.extend(0.) * 36.,
                        d,
                        Kind::Crystal,
                        None,
                        None,
                    );
                }
                drop_item(&mut commands, &art, e.position, 4);
            }
            Some(Tile::Berries) => drop_item(&mut commands, &art, e.position, 0),
            _ => {}
        }
    }
}
pub(super) fn pickups(
    mut commands: Commands,
    time: Res<Time>,
    mut drops: Query<(Entity, &mut Pickup, &Transform)>,
    mut actors: Query<
        (Entity, &Transform, &mut Health, Option<&Buff>),
        (Or<(With<Hero>, With<Villager>)>, Without<DeathAnimation>),
    >,
    enemies: Query<
        (Entity, &Transform),
        (
            With<crate::components::enemy::Enemy>,
            Without<DeathAnimation>,
        ),
    >,
    art: Res<Art>,
) {
    for (e, mut drop, t) in &mut drops {
        drop.life -= time.delta_secs();
        if drop.life <= 0. {
            commands.entity(e).despawn();
            continue;
        }
        // Wait a short beat so splitting enemies do not instantly consume drops.
        if drop.life > 24.5 {
            continue;
        }
        let nearest = actors
            .iter()
            .filter(|(_, a, h, _)| {
                h.current > 0 && a.translation.xy().distance(t.translation.xy()) < 90.
            })
            .min_by(|a, b| {
                a.1.translation
                    .distance_squared(t.translation)
                    .total_cmp(&b.1.translation.distance_squared(t.translation))
            })
            .map(|a| a.0);
        let Some(actor) = nearest else {
            continue;
        };
        let Ok((_, at, mut hp, _buff)) = actors.get_mut(actor) else {
            continue;
        };
        match drop.kind {
            0 | 3 => hp.current = (hp.current + if drop.kind == 0 { 1 } else { 2 }).min(hp.max),
            2 => {
                commands
                    .entity(actor)
                    .try_insert(crate::systems::slime_physics::SlimeCoat {
                        until: time.elapsed_secs() + 8.,
                    });
            }
            1 | 4 => {
                if let Some((target, to)) = enemies.iter().min_by(|a, b| {
                    a.1.translation
                        .distance_squared(at.translation)
                        .total_cmp(&b.1.translation.distance_squared(at.translation))
                }) {
                    let direction = (to.translation - at.translation).xy().normalize_or_zero();
                    for offset in [-0.16_f32, 0., 0.16] {
                        let d = Vec2::from_angle(direction.y.atan2(direction.x) + offset);
                        let shot = projectiles::spawn(
                            &mut commands,
                            &art,
                            at.translation + d.extend(0.) * 40.,
                            d,
                            if drop.kind == 1 {
                                Kind::Fire
                            } else {
                                Kind::Crystal
                            },
                            Some(target),
                            Some(actor),
                        );
                        commands
                            .entity(shot)
                            .entry::<projectiles::Shot>()
                            .and_modify(|mut s| s.friendly = true);
                    }
                }
            }
            _ => {}
        }
        commands.entity(e).despawn();
    }
}
pub(super) fn hazards(
    mut commands: Commands,
    time: Res<Time>,
    tiles: Query<(Entity, &Tile, &Transform)>,
    mut actors: Query<
        (
            Entity,
            &Transform,
            &mut Health,
            &mut LinearVelocity,
            Option<&mut Lift>,
            Option<&Buff>,
        ),
        (Without<Tile>, Without<DeathAnimation>),
    >,
    spatial: Res<crate::resources::spatial_hash::SpatialHash>,
    mut damage: MessageWriter<crate::events::damage_events::EntityDamaged>,
    mut last: Local<std::collections::HashMap<Entity, f32>>,
) {
    let now = time.elapsed_secs();
    last.retain(|e, _| actors.contains(*e));
    let mut harvested = std::collections::HashSet::new();
    for (e, t, mut health, mut velocity, lift, buff) in &mut actors {
        if let Some(mut lift) = lift {
            lift.0 -= time.delta_secs();
            if lift.0 <= 0. {
                commands.entity(e).remove::<Lift>();
            } else {
                velocity.y = velocity.y.max(190.);
            }
        }
        for block in spatial.nearby_iter(t.translation.x, t.translation.y) {
            let Ok((_, tile, bt)) = tiles.get(block) else {
                continue;
            };
            let delta = (t.translation - bt.translation).xy();
            if delta.x.abs() > 32. * t.scale.x.abs() + 20.
                || delta.y.abs() > 32. * t.scale.y.abs() + 32.
            {
                continue;
            }
            match tile {
                Tile::Berries
                    if health.current > 0
                        && health.current < health.max
                        && harvested.insert(block) =>
                {
                    health.current += 1;
                    commands.entity(block).despawn();
                }
                Tile::DripstoneUp | Tile::DripstoneDown
                    if last.get(&e).is_none_or(|v| now - v > 1.) =>
                {
                    damage.write(crate::events::damage_events::EntityDamaged {
                        entity: e,
                        damage: 1,
                    });
                    last.insert(e, now);
                }
                Tile::PowderSnow if buff.is_none() => {
                    commands
                        .entity(e)
                        .try_insert(Buff::new(BuffType::Slowness, 0.7));
                }
                _ => {}
            }
        }
    }
}
