//! Native opt-in lifecycle, rewind, pause, support-loss and command-isolation test.
use super::*;
#[derive(Resource, Default)]
struct Probe {
    step: u8,
    at: f32,
    hero: Option<Entity>,
    runner_time: f32,
    world_actor: Option<(Entity, Vec3)>,
    returned: bool,
    plant: Option<(Entity, Entity)>,
}
fn capture(w: &mut World, name: &str) {
    w.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(format!(
            "target/visual-rift-{name}.png"
        )));
}
pub(crate) fn run(w: &mut World) {
    w.init_resource::<Probe>();
    let now = w.resource::<Time<Real>>().elapsed_secs();
    let step = w.resource::<Probe>().step;
    let at = w.resource::<Probe>().at;
    assert!(now < 110., "Rift test timeout at stage {step}");
    match step {
        0 if now > 8. => {
            {
                let mut g =
                    w.resource_mut::<crate::resources::graphics_settings::GraphicsSettings>();
                g.ray_tracing = std::env::args().any(|a| a == "--rift-rt");
                g.dlss = false;
                g.frame_generation = false;
            }
            w.query::<&mut Window>()
                .single_mut(w)
                .unwrap()
                .resolution
                .set(1360., 900.);
            let hero = w.query_filtered::<Entity, With<Hero>>().single(w).unwrap();
            w.resource_mut::<Probe>().hero = Some(hero);
            let hero_p = w.get::<Transform>(hero).unwrap().translation;
            let p = (hero_p.xy() / 64.).round().extend(0.) * 64. + Vec3::new(384., 256., -10.);
            let old: Vec<_> = w
                .query_filtered::<(Entity, &Transform), With<Block>>()
                .iter(w)
                .filter(|(_, t)| {
                    (t.translation.x - p.x).abs() < 40. && (t.translation.y - p.y - 32.).abs() < 75.
                })
                .map(|(e, _)| e)
                .collect();
            for e in old {
                w.despawn(e);
            }
            let chunk = crate::components::chunk::ChunkMember {
                chunk_x: (p.x / 1024.).floor() as i32,
                chunk_y: (p.y / 1024.).floor() as i32,
            };
            w.resource_mut::<crate::components::chunk::LoadedChunks>()
                .add(chunk.chunk_x, chunk.chunk_y);
            let support = w
                .spawn((
                    Block {
                        health: 10,
                        max_health: 10,
                    },
                    BlockType::Dirt,
                    chunk,
                    Transform::from_translation(p),
                    Visibility::Inherited,
                ))
                .id();
            let plant = w
                .spawn((
                    Block {
                        health: 2,
                        max_health: 2,
                    },
                    BlockType::DeadBush,
                    super::super::biome_content::Tile::Fern,
                    chunk,
                    Transform::from_translation(p + Vec3::Y * 64.),
                    Visibility::Inherited,
                ))
                .id();
            w.resource_mut::<Probe>().plant = Some((support, plant));
        }
        1 if now > at + 0.5 => {
            let (support, plant) = w.resource::<Probe>().plant.unwrap();
            assert!(w.get_entity(plant).is_ok(), "Supported fern was removed");
            w.despawn(support);
        }
        2 if now > at + 0.6 => {
            let (_, plant) = w.resource::<Probe>().plant.unwrap();
            assert!(
                w.get_entity(plant).is_err(),
                "Fern survived destruction of its ground"
            );
            w.resource_mut::<Rift>().request = true;
        }
        3 if *w.resource::<State<GameState>>().get() == GameState::Rift && now > at + 0.7 => {
            capture(w, "entry");
            // Remember where the hero was parked; it must not move during the rift.
            let actor = w
                .resource::<Probe>()
                .hero
                .and_then(|h| w.get::<Position>(h).map(|p| (h, p.0)));
            w.resource_mut::<Probe>().world_actor = actor;
            if let Some(q) = crate::plugins::external_api_plugin::get_command_queue() {
                q.lock()
                    .unwrap()
                    .push_back(crate::resources::command_queue::PowerCommand::new(
                        PowerCommandType::Fast,
                        "rift-probe".into(),
                    ));
            }
        }
        4 if now > at + 6. => {
            // The world keeps running behind the course; the hero waits parked.
            assert!(
                !w.resource::<Time<Virtual>>().is_paused(),
                "World paused in rift"
            );
            assert!(
                !w.resource::<Time<Physics>>().is_paused(),
                "Physics paused in rift"
            );
            if let Some((e, p)) = w.resource::<Probe>().world_actor {
                assert!(w.get::<super::RiftParked>(e).is_some(), "Hero not parked");
                assert!(matches!(
                    w.get::<avian3d::prelude::RigidBody>(e),
                    Some(avian3d::prelude::RigidBody::Static)
                ));
                // Gameplay position only: depth (Z) follows the live head count,
                // which changes while the world keeps running.
                let now_at = w.get::<Position>(e).unwrap().0;
                assert!(
                    now_at.xy().distance(p.xy()) < 0.01,
                    "Parked hero moved while inside rift: {p} -> {now_at}"
                );
            }
            // Powers sent during the rift act in the live world instead of waiting.
            if let Some(q) = crate::plugins::external_api_plugin::get_command_queue() {
                assert!(
                    !q.lock()
                        .unwrap()
                        .iter()
                        .any(|c| c.command_type == PowerCommandType::Fast),
                    "Power sent during the rift did not reach the live world"
                );
            }
            if std::env::args().any(|a| a == "--rift-rt") {
                assert_eq!(
                    w.resource::<crate::systems::native_gpu::NativeStatus>()
                        .mode,
                    2,
                    "RT was requested but not active"
                );
                let owners: Vec<_> = w
                    .query_filtered::<&ChildOf, With<bevy::solari::prelude::RaytracingMesh3d>>()
                    .iter(w)
                    .map(|p| p.parent())
                    .collect();
                // The live world is visible behind the course, so its meshes are traced too.
                assert!(!owners.is_empty());
            }
            capture(w, "course");
            let time = w
                .resource::<Rift>()
                .session
                .as_ref()
                .unwrap()
                .course
                .frame
                .time;
            w.resource_mut::<Probe>().runner_time = time;
            let mut r = w.resource_mut::<Rift>();
            r.rewind_request = true;
        }
        5 if now > at + 2. => {
            let r = w.resource::<Rift>();
            let s = r.session.as_ref().unwrap();
            assert!(s.course.rewinding);
            assert!(s.course.frame.time < w.resource::<Probe>().runner_time - 1.5);
            capture(w, "rewind");
            w.resource_mut::<Rift>().session.as_mut().unwrap().paused = true;
            w.resource_mut::<Probe>().runner_time = w
                .resource::<Rift>()
                .session
                .as_ref()
                .unwrap()
                .course
                .frame
                .time;
        }
        6 if now > at + 0.8 => {
            assert_eq!(
                w.resource::<Rift>()
                    .session
                    .as_ref()
                    .unwrap()
                    .course
                    .frame
                    .time,
                w.resource::<Probe>().runner_time
            );
            w.resource_mut::<Rift>().session.as_mut().unwrap().paused = false;
        }
        7 if *w.resource::<State<GameState>>().get() == GameState::Playing => {
            assert!(w.resource::<Rift>().session.is_none());
            assert!(!w.resource::<Time<Virtual>>().is_paused());
            assert!(!w.resource::<Time<Physics>>().is_paused());
            assert!(
                w.query_filtered::<Entity, With<Scene>>()
                    .iter(w)
                    .next()
                    .is_none()
            );
            let hero = w.resource::<Probe>().hero.unwrap();
            assert_ne!(*w.get::<Visibility>(hero).unwrap(), Visibility::Hidden);
            capture(w, "return");
            w.resource_mut::<Probe>().returned = true;
        }
        8 if now > at + 0.6 => {
            // A second entry is cancelled, proving both success and early-return paths.
            w.resource_mut::<Rift>().request = true;
        }
        9 if *w.resource::<State<GameState>>().get() == GameState::Rift && now > at + 2. => {
            w.resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Escape);
            super::update(w); // Exercise the real Escape path before InputPlugin clears injected edges.
            w.resource_mut::<ButtonInput<KeyCode>>()
                .release(KeyCode::Escape);
        }
        10 if *w.resource::<State<GameState>>().get() == GameState::Playing && now > at + 2. => {
            assert!(w.resource::<Probe>().returned);
            assert!(w.resource::<Rift>().session.is_none());
            eprintln!(
                "[RIFT TEST] completed: camera/course, plant support loss, paused world, deferred command, rewind, pause, successful exit and cancellation"
            );
            w.write_message(AppExit::Success);
        }
        _ => return,
    }
    let mut p = w.resource_mut::<Probe>();
    p.step += 1;
    p.at = now;
}
