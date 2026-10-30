use crate::{
    components::{
        block::Block,
        buff::{Buff, BuffType, Buffable},
        enemy::{Enemy, Health},
        hero::Hero,
        villager::Villager,
    },
    events::damage_events::EntityDamaged,
    resources::{asset_handles::GameTextures, graphics_settings::GraphicsSettings},
};
use avian3d::prelude::*;
use bevy::prelude::*;
#[derive(Resource, Default)]
struct Probe {
    step: u8,
    since: f32,
    anchor: Vec3,
    actors: Vec<Entity>,
}
fn capture(w: &mut World, name: &str) {
    w.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(format!(
            "target/visual-headsize-{name}.png"
        )));
}
pub fn run(w: &mut World) {
    if !w.contains_resource::<Probe>() {
        w.init_resource::<Probe>();
    }
    let now = w.resource::<Time<Real>>().elapsed_secs();
    let step = w.resource::<Probe>().step;
    let since = w.resource::<Probe>().since;
    assert!(now < 60.);
    let anchor = w.resource::<Probe>().anchor;
    if step > 0 {
        for (mut p, mut v) in w
            .query_filtered::<(&mut Position, &mut LinearVelocity), With<Hero>>()
            .iter_mut(w)
        {
            p.0 = anchor;
            v.0 = Vec3::ZERO;
        }
        let enemies: Vec<_> = w.query_filtered::<Entity, With<Enemy>>().iter(w).collect();
        for e in enemies {
            w.despawn(e);
        }
        let mut weather = w.resource_mut::<crate::components::weather::ActiveWeather>();
        weather.weather_type = crate::components::weather::WeatherType::Clear;
        weather.intensity = 0.;
        weather.duration_remaining = 999.;
    }
    match step {
        0 if now > 8. => {
            let anchor = w
                .query_filtered::<&Transform, With<Hero>>()
                .single(w)
                .unwrap()
                .translation;
            w.resource_mut::<Probe>().anchor = anchor;
            w.query::<&mut Window>()
                .single_mut(w)
                .unwrap()
                .resolution
                .set(1360., 900.);
            let mut g = w.resource_mut::<GraphicsSettings>();
            g.ray_tracing = false;
            g.dlss = false;
            g.frame_generation = false;
            let old: Vec<_> = w
                .query::<(Entity, &Transform, Option<&Block>, Option<&Villager>)>()
                .iter(w)
                .filter(|(_, t, b, v)| {
                    (b.is_some() || v.is_some()) && t.translation.xy().distance(anchor.xy()) < 900.
                })
                .map(|(e, _, _, _)| e)
                .collect();
            for e in old {
                w.despawn(e);
            }
            let image = w.resource::<GameTextures>().villager.clone();
            let mut actors = Vec::new();
            for (i, (size, scale)) in [
                (16., 1.),
                (32., 1.),
                (64., 1.),
                (64., 0.5),
                (64., 1.),
                (64., 1.5),
            ]
            .into_iter()
            .enumerate()
            {
                let mesh = w
                    .resource_mut::<Assets<Mesh>>()
                    .add(Rectangle::new(size, size));
                let material = w
                    .resource_mut::<Assets<StandardMaterial>>()
                    .add(StandardMaterial {
                        base_color_texture: Some(image.clone()),
                        unlit: true,
                        alpha_mode: AlphaMode::Mask(0.5),
                        ..default()
                    });
                let x = (i % 3) as f32 * 320. - 320.;
                let y = if i < 3 { 200. } else { -180. };
                actors.push(
                    w.spawn((
                        Villager {
                            ai_timer: Timer::default(),
                            move_direction: 0.,
                            direction_change_timer: Timer::default(),
                        },
                        Health::new(6),
                        Buffable,
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        Transform::from_translation(anchor + Vec3::new(x, y, 0.))
                            .with_scale(Vec3::splat(scale)),
                    ))
                    .id(),
                );
            }
            w.resource_mut::<Probe>().actors = actors;
        }
        1 if now > since + 2. => {
            let actors = w.resource::<Probe>().actors.clone();
            for e in actors {
                w.write_message(EntityDamaged {
                    entity: e,
                    damage: 1,
                });
            }
        }
        2 if now > since + 0.12 => {
            capture(w, "damage-and-hearts");
        }
        3 if now > since + 1.2 => {
            let actors = w.resource::<Probe>().actors.clone();
            for e in actors {
                w.entity_mut(e).insert(Buff::new(BuffType::Burning, 2.));
            }
        }
        4 if now > since + 0.5 => {
            capture(w, "burning");
        }
        5 if now > since + 2. => {
            let actors = w.resource::<Probe>().actors.clone();
            for e in actors {
                w.write_message(EntityDamaged {
                    entity: e,
                    damage: 20,
                });
            }
        }
        6 if now > since + 0.12 => {
            capture(w, "death");
        }
        7 if now > since + 2. => {
            assert!(
                w.resource::<Probe>()
                    .actors
                    .iter()
                    .all(|e| w.get_entity(*e).is_err())
            );
            eprintln!(
                "[HEADSIZE TEST] completed: mesh dimensions, scaled heads, damage, hearts, fire and death"
            );
            w.write_message(AppExit::Success);
        }
        _ => return,
    }
    let mut p = w.resource_mut::<Probe>();
    p.step += 1;
    p.since = now;
}
