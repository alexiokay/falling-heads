use super::*;

pub(crate) fn smoke_probe(world: &mut World) {
    use crate::resources::graphics_settings::GraphicsSettings;
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    if !world.contains_resource::<LauncherProbe>() {
        world.init_resource::<LauncherProbe>();
    }
    let time = world.resource::<Time<Real>>().elapsed_secs();
    let step = world.resource::<LauncherProbe>().step;
    let since = world.resource::<LauncherProbe>().since;
    assert!(time < 115., "Launcher probe timed out at stage {step}");
    let capture = |world: &mut World, name: &str| {
        world
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("target/visual-launcher-{name}.png")));
    };
    if step == 4 || step == 8 {
        for (i, delay) in [0.2, 0.5, 1.0].into_iter().enumerate() {
            let bit = 1 << (i + if step == 8 { 3 } else { 0 });
            if time > since + delay && world.resource::<LauncherProbe>().breakup_captures & bit == 0
            {
                capture(world, &format!("breakup-{step}-{i}"));
                world.resource_mut::<LauncherProbe>().breakup_captures |= bit;
            }
        }
    }
    match step {
        0 if time > 8. => {
            let sounds = world.resource::<crate::systems::rocket_audio::RocketSounds>();
            let audio = world.resource::<Assets<AudioSource>>();
            for handle in [&sounds.ignition, &sounds.engine, &sounds.crash] {
                assert!(
                    audio.get(handle).is_some(),
                    "Rocket audio asset must decode"
                );
            }
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(1360., 900.);
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.dlss = true;
            g.dlss_model_45 = true;
            g.dlss_45_performance = false;
            g.ray_tracing = false;
            g.frame_generation = false;
            let mut command = PowerCommand::new(PowerCommandType::Launcher, "Launcher test".into());
            command.launcher.direction = LaunchDirection::Right;
            crate::plugins::external_api_plugin::get_command_queue()
                .unwrap()
                .lock()
                .unwrap()
                .push_back(command);
        }
        1 | 5 => {
            let Some(f) = world.resource::<LauncherState>().active.as_ref() else {
                return;
            };
            if f.elapsed < 0.4 {
                return;
            }
            let expected = (f.start, f.end, f.seed, f.count);
            if step == 1 {
                assert_eq!(f.count, 1, "Default is exactly one villager");
            }
            let point = path(f.start, f.end, 0.3);
            let watch = world
                .spawn((
                    Block {
                        health: 100,
                        max_health: 100,
                    },
                    BlockType::Obsidian,
                    Transform::from_translation(point),
                ))
                .id();
            world.resource_mut::<LauncherProbe>().watch = Some(watch);
            world.resource_mut::<LauncherProbe>().expected = Some(expected);
            capture(world, if step == 1 { "warmup" } else { "left-warmup" });
        }
        2 | 6 => {
            let Some(f) = world.resource::<LauncherState>().active.as_ref() else {
                return;
            };
            if f.elapsed < WARMUP + 1.9 {
                return;
            }
            assert!(world.get::<LauncherPassenger>(f.hero).is_some());
            assert_eq!(
                *world.get::<RigidBody>(f.hero).unwrap(),
                RigidBody::Kinematic
            );
            assert!(
                world.get::<Transform>(f.ship).unwrap().translation.z
                    - world.get::<Transform>(f.hero).unwrap().translation.z
                    > 100.
            );
            let watch = world.resource_mut::<LauncherProbe>().watch.unwrap();
            assert_eq!(
                world
                    .get::<Block>(watch)
                    .expect("Flight must leave crossed terrain intact")
                    .health,
                100
            );
            world.despawn(watch);
            capture(world, if step == 2 { "flight" } else { "left-flight" });
        }
        3 | 7 => {
            if world.resource::<LauncherState>().active.is_some() {
                return;
            }
            let (start, end, seed, count) = world.resource::<LauncherProbe>().expected.unwrap();
            let (hero, t) = world
                .query_filtered::<(Entity, &Transform), With<Hero>>()
                .single(world)
                .unwrap();
            let landing = (end.xy() / TILE).round() * TILE;
            assert!(
                t.translation.xy().distance(landing) < 20.,
                "Hero failed to land"
            );
            assert!(world.get::<LauncherPassenger>(hero).is_none());
            assert_eq!(*world.get::<RigidBody>(hero).unwrap(), RigidBody::Dynamic);
            assert_ne!(*world.get::<Visibility>(hero).unwrap(), Visibility::Hidden);
            let center = (landing / TILE).as_ivec2();
            assert!(
                !world
                    .query_filtered::<&Transform, With<Block>>()
                    .iter(world)
                    .any(|t| {
                        let c = (t.translation.xy() / TILE).round().as_ivec2() - center;
                        crater_cell(c.x, c.y, seed)
                    })
            );
            let villagers = world
                .query::<(&RocketArrival, &Transform)>()
                .iter(world)
                .filter(|(arrival, _)| arrival.0 == seed)
                .collect::<Vec<_>>();
            let queued = world
                .resource::<Arrivals>()
                .0
                .iter()
                .filter(|a| a.seed == seed)
                .count();
            assert_eq!(villagers.len() + queued, count as usize);
            assert!(
                villagers
                    .iter()
                    .all(|(_, t)| t.translation.xy().distance(landing) < 100.)
            );
            assert!((47. * TILE..=52. * TILE).contains(&(end.x - start.x).abs()));
            eprintln!(
                "[LAUNCHER TEST] landed stage={step} passengers={count} dx={} dy={}",
                (end.x - start.x) / TILE,
                (end.y - start.y) / TILE
            );
            capture(world, if step == 3 { "crash" } else { "left-crash" });
        }
        4 if time > since + 3. => {
            verify_arrivals(world);
            world.resource_mut::<LauncherState>().cooldown = 0.;
            let mut command =
                PowerCommand::new(PowerCommandType::Launcher, "Eight passengers".into());
            command.launcher.direction = LaunchDirection::Left;
            command.launcher.villagers = 8;
            crate::plugins::external_api_plugin::get_command_queue()
                .unwrap()
                .lock()
                .unwrap()
                .push_back(command);
        }
        8 if time > since + 3. => {
            verify_arrivals(world);
            world.resource_mut::<GraphicsSettings>().dlss_45_performance = true;
        }
        9 if time > since + 3. => {
            capture(world, "dlss45-performance");
            world.resource_mut::<GraphicsSettings>().dlss_model_45 = false;
        }
        10 if time > since + 3. => {
            world.resource_mut::<GraphicsSettings>().dlss_45_performance = false;
        }
        11 if time > since + 3. => {
            world.resource_mut::<GraphicsSettings>().ray_tracing = true;
        }
        12 if time > since + 4. => {
            capture(world, "rt");
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.ray_tracing = false;
            g.dlss_model_45 = true;
        }
        13 if time > since + 3. => {
            assert_eq!(
                world
                    .resource::<crate::systems::native_gpu::NativeStatus>()
                    .mode,
                1
            );
            world
                .resource_mut::<NextState<GameState>>()
                .set(GameState::Settings);
        }
        14 if time > since + 1. => {
            world
                .resource_mut::<crate::components::settings_menu::SettingsNavigation>()
                .0 = crate::components::settings_menu::SettingsPage::Graphics;
        }
        15 if time > since + 3. => {
            capture(world, "settings");
        }
        16 if time > since + 2. => {
            eprintln!(
                "[LAUNCHER TEST] completed: right/default passenger, left/eight passengers, restored physics, clear crater, M Quality/Performance, K Performance/Quality, RR, M Quality"
            );
            world.write_message(AppExit::Success);
        }
        _ => return,
    }
    let mut probe = world.resource_mut::<LauncherProbe>();
    probe.step += 1;
    probe.since = time;
}

pub(super) fn verify_arrivals(world: &mut World) {
    let (_, _, seed, count) = world.resource::<LauncherProbe>().expected.unwrap();
    assert!(
        world
            .resource::<Arrivals>()
            .0
            .iter()
            .all(|a| a.seed != seed)
    );
    assert_eq!(
        world
            .query::<&RocketArrival>()
            .iter(world)
            .filter(|a| a.0 == seed)
            .count(),
        count as usize
    );
}
