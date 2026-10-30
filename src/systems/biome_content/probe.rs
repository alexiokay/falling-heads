//! Opt-in native rendering/AI verification, invoked by --content-test.
use super::*;
use crate::components::enemy::Flying;
use crate::components::{block::Block, chunk::ChunkMember, enemy::Enemy, hero::Hero};
use avian3d::prelude::*;
#[derive(Resource, Default)]
struct Probe {
    step: u8,
    since: f32,
    anchor: Vec3,
    max_shots: usize,
    charge_seen: bool,
    reveal_seen: bool,
}
fn capture(w: &mut World, name: &str) {
    w.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(format!(
            "target/visual-content-{name}.png"
        )));
}
fn clear(w: &mut World, p: Vec3) {
    let old: Vec<_> = w
        .query::<(
            Entity,
            &Transform,
            Option<&Block>,
            Option<&Enemy>,
            Option<&Label>,
            Option<&projectiles::Shot>,
        )>()
        .iter(w)
        .filter(|(_, t, b, e, l, s)| {
            (b.is_some() || e.is_some() || l.is_some() || s.is_some())
                && t.translation.xy().distance(p.xy()) < 1400.
        })
        .map(|a| a.0)
        .collect();
    for e in old {
        w.despawn(e);
    }
    let cells: Vec<_> = (-20..20)
        .flat_map(|x| (-16..16).map(move |y| ((p.x / 64.) as i32 + x, (p.y / 64.) as i32 + y)))
        .collect();
    w.resource_mut::<crate::systems::fluids::FluidWorld>()
        .clear_transport_cells(&cells);
}
#[derive(Component)]
struct Label;
pub(crate) fn run(w: &mut World) {
    w.init_resource::<Probe>();
    let now = w.resource::<Time<Real>>().elapsed_secs();
    let step = w.resource::<Probe>().step;
    let since = w.resource::<Probe>().since;
    assert!(now < 90., "Content probe timeout stage {step}");
    let p = w.resource::<Probe>().anchor;
    let shots = w.query::<&projectiles::Shot>().iter(w).count();
    w.resource_mut::<Probe>().max_shots = w.resource::<Probe>().max_shots.max(shots);
    if step > 0 {
        let mut weather = w.resource_mut::<crate::components::weather::ActiveWeather>();
        weather.weather_type = crate::components::weather::WeatherType::Clear;
        weather.intensity = 0.;
        weather.duration_remaining = 999.;
    }
    if step >= 8 {
        use crate::systems::biome_teleport::{BiomeTeleport, TeleportPhase};
        let phase = w.resource::<BiomeTeleport>().phase;
        if phase == TeleportPhase::Charging {
            w.resource_mut::<Probe>().charge_seen = true;
        }
        if phase == TeleportPhase::Revealing {
            w.resource_mut::<Probe>().reveal_seen = true;
            let h = w
                .query_filtered::<&Transform, With<Hero>>()
                .single(w)
                .unwrap()
                .translation;
            let c = w
                .query_filtered::<&Transform, With<Camera3d>>()
                .single(w)
                .unwrap()
                .translation;
            assert!(
                (c.xy() - h.xy() - Vec2::new(30., 80.)).length() < 250.,
                "Camera flew between biomes"
            );
        }
        let art = w.resource::<art::Art>();
        let materials = w.resource::<Assets<StandardMaterial>>();
        let expected: Vec<_> = art
            .faces
            .iter()
            .map(|h| materials.get(h).unwrap().base_color_texture.clone())
            .collect();
        let foes: Vec<_> = w
            .query::<(&Enemy, &MeshMaterial3d<StandardMaterial>, &BiomeFoe)>()
            .iter(w)
            .map(|(e, m, _)| (e.enemy_type, m.0.clone()))
            .collect();
        let materials = w.resource::<Assets<StandardMaterial>>();
        for (kind, mat) in foes {
            if let Some(i) = enemies::SPECIES.iter().position(|k| *k == kind) {
                assert_eq!(
                    materials.get(&mat).unwrap().base_color_texture,
                    expected[i],
                    "Species face changed after teleport: {kind:?}"
                );
            }
        }
    }
    match step {
        0 if now > 8. => {
            let hero = w.query_filtered::<Entity, With<Hero>>().single(w).unwrap();
            let p = w.get::<Transform>(hero).unwrap().translation;
            let p = (p.xy() / 64.).round().extend(0.) * 64.;
            w.resource_mut::<Probe>().anchor = p;
            w.entity_mut(hero)
                .insert((
                    RigidBody::Static,
                    Position(p + Vec3::new(0., 0., 30.)),
                    Transform::from_translation(p + Vec3::new(0., 0., 30.)),
                    Visibility::Hidden,
                ))
                .remove::<LinearVelocity>();
            w.query::<&mut Window>()
                .single_mut(w)
                .unwrap()
                .resolution
                .set(1360., 900.);
            {
                let mut g =
                    w.resource_mut::<crate::resources::graphics_settings::GraphicsSettings>();
                g.ray_tracing = std::env::args().any(|a| a == "--rift-rt");
                g.dlss = false;
                g.frame_generation = false;
            }
            clear(w, p);
            let art = w.resource::<art::Art>();
            let faces = art.faces.clone();
            let images = w.resource::<Assets<Image>>();
            let mats = w.resource::<Assets<StandardMaterial>>();
            for material in faces {
                assert!(
                    images
                        .get(
                            mats.get(&material)
                                .unwrap()
                                .base_color_texture
                                .as_ref()
                                .unwrap()
                        )
                        .is_some(),
                    "Missing enemy texture"
                );
            }
            for (i, kind) in enemies::SPECIES.into_iter().enumerate() {
                let pos = p + Vec3::new(
                    (i % 5) as f32 * 180. - 360.,
                    220. - (i / 5) as f32 * 230.,
                    30.,
                );
                let e = enemies::spawn(&mut w.commands(), pos, kind, 0);
                w.flush();
                w.entity_mut(e)
                    .insert(RigidBody::Static)
                    .remove::<LinearVelocity>();
                w.spawn((
                    Label,
                    Text2d::new(format!("{kind:?}")),
                    TextFont {
                        font_size: FontSize::Px(19.),
                        ..default()
                    },
                    Transform::from_translation(pos + Vec3::new(0., -70., 20.)),
                ));
            }
        }
        1 if now > since + 2. => {
            let foes: Vec<_> = w
                .query::<(&Enemy, Option<&Flying>, &MeshMaterial3d<StandardMaterial>)>()
                .iter(w)
                .map(|(e, f, m)| (e.enemy_type, f.is_some(), m.0.clone()))
                .collect();
            let art = w.resource::<art::Art>();
            let materials = w.resource::<Assets<StandardMaterial>>();
            for (kind, flying, mat) in foes {
                if let Some(i) = enemies::SPECIES.iter().position(|k| *k == kind) {
                    assert_ne!(
                        mat, art.faces[i],
                        "Damage must not tint all members of a species"
                    );
                    assert_eq!(
                        materials.get(&mat).unwrap().base_color_texture,
                        materials.get(&art.faces[i]).unwrap().base_color_texture
                    );
                    if kind == crate::components::enemy::EnemyType::Breeze {
                        assert!(!flying);
                    }
                    if kind == crate::components::enemy::EnemyType::Blaze {
                        assert!(flying);
                    }
                }
            }
            capture(w, "enemies");
        }
        2 if now > since + 0.6 => {
            clear(w, p);
            for (i, &tile) in catalog::TILES.iter().enumerate() {
                let pos = p + Vec3::new(
                    (i % 8) as f32 * 110. - 385.,
                    260. - (i / 8) as f32 * 125.,
                    -10.,
                );
                let c = (pos.xy() / 1024.).floor().as_ivec2();
                w.spawn((
                    tile,
                    tile.base(),
                    Block {
                        health: 20,
                        max_health: 20,
                    },
                    ChunkMember {
                        chunk_x: c.x,
                        chunk_y: c.y,
                    },
                    Transform::from_translation(pos),
                    Visibility::Inherited,
                ));
                w.spawn((
                    Label,
                    Text2d::new(format!("{tile:?}")),
                    TextFont {
                        font_size: FontSize::Px(13.),
                        ..default()
                    },
                    Transform::from_translation(pos + Vec3::new(0., -50., 60.)),
                ));
            }
        }
        3 if now > since + 2. => {
            capture(w, "blocks");
        }
        4 if now > since + 0.6 => {
            clear(w, p);
            let hero = w.query_filtered::<Entity, With<Hero>>().single(w).unwrap();
            w.entity_mut(hero).insert((
                Visibility::Inherited,
                Position(p + Vec3::new(-320., 0., 30.)),
                Transform::from_translation(p + Vec3::new(-320., 0., 30.)),
            ));
            let blaze = enemies::spawn(
                &mut w.commands(),
                p + Vec3::new(270., 80., 30.),
                crate::components::enemy::EnemyType::Blaze,
                0,
            );
            w.flush();
            w.get_mut::<enemies::BiomeFoe>(blaze).unwrap().cooldown = 0.4;
        }
        5 if now > since + 1.1 => {
            capture(w, "blaze-burst");
        }
        6 if now > since + 1.5 => {
            assert!(
                w.resource::<Probe>().max_shots >= 3,
                "Blaze must produce a three-shot burst"
            );
            capture(w, "combat");
            eprintln!(
                "[CONTENT TEST] max simultaneous shots={}",
                w.resource::<Probe>().max_shots
            );
        }
        7 if now > since + 1. => {
            let hero = w.query_filtered::<Entity, With<Hero>>().single(w).unwrap();
            w.entity_mut(hero).insert((
                LinearVelocity::ZERO,
                crate::components::enemy::Health::new(1000),
            ));
            let y = crate::utils::biome_utils::nearest_biome_y(
                p.y,
                crate::components::block::BiomeType::Forest,
            );
            w.resource_mut::<crate::systems::biome_teleport::BiomeTeleport>()
                .target = Some(Vec3::new(0., y, 30.));
        }
        8 if now > since + 8. => {
            assert!(w.resource::<Probe>().charge_seen && w.resource::<Probe>().reveal_seen);
            assert!(
                w.resource::<crate::systems::biome_teleport::BiomeTeleport>()
                    .phase
                    == crate::systems::biome_teleport::TeleportPhase::Idle
            );
            capture(w, "teleport-arrival");
            let hero = w.query_filtered::<Entity, With<Hero>>().single(w).unwrap();
            let mut ice = crate::components::frozen::FrozenStacks::new();
            for _ in 0..4 {
                ice.add_stack();
            }
            w.entity_mut(hero).insert((
                ice,
                crate::components::buff::Buff::new(crate::components::buff::BuffType::Burning, 8.),
            ));
        }
        9 if now > since + 0.6 => {
            capture(w, "fire-ice-a");
        }
        10 if now > since + 0.4 => {
            capture(w, "fire-ice-b");
        }
        11 if now > since + 2. => {
            capture(w, "forest-layers");
            eprintln!(
                "[CONTENT TEST] completed: species, burst, teleport cut, species restoration, stacked fire/ice"
            );
            w.write_message(AppExit::Success);
        }
        _ => return,
    }
    let mut probe = w.resource_mut::<Probe>();
    probe.step += 1;
    probe.since = now;
}
