//! Opt-in hidden-window GPU and schedule regression check: --smoke-test.
use crate::{
    components::{
        block::{BiomeType, Block},
        hero::Hero,
        snow_layer::SnowLayer,
        weather::{ActiveWeather, WeatherType},
    },
    events::block_events::BlockDamaged,
    states::GameState,
    systems::biome_teleport::BiomeTeleport,
    utils::biome_utils::{determine_biome_at_position, nearest_biome_y},
};
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
pub struct SmokeTestPlugin;
impl Plugin for SmokeTestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Probe>()
            .add_systems(Update, probe)
            .add_systems(Last, verify_label_projection);
    }
}
#[derive(Resource, Default)]
struct Probe {
    step: usize,
    since: f64,
    awaiting: Option<BiomeType>,
    initialized: bool,
    portal_captured: bool,
    portal_expected: Option<BiomeType>,
}
const BIOMES: [BiomeType; 8] = [
    BiomeType::Plains,
    BiomeType::Forest,
    BiomeType::Desert,
    BiomeType::Mountains,
    BiomeType::Underground,
    BiomeType::Nether,
    BiomeType::End,
    BiomeType::Iceland,
];
fn probe(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut probe: ResMut<Probe>,
    mut teleport: ResMut<BiomeTeleport>,
    hero: Query<(Entity, &Transform), With<Hero>>,
    mut blocks: Query<(Entity, &Transform, &mut Block), Without<Hero>>,
    snow: Query<&SnowLayer>,
    mut damage: MessageWriter<BlockDamaged>,
    mut weather: ResMut<ActiveWeather>,
    mut state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
) {
    let elapsed = time.elapsed_secs_f64();
    assert!(
        elapsed < 180.0,
        "Smoke test timed out at step {}",
        probe.step
    );
    let Ok((hero_entity, hero)) = hero.single() else {
        return;
    };
    if !probe.initialized {
        if elapsed < 8.0 || blocks.iter().count() < 100 {
            return;
        }
        probe.initialized = true;
        commands.spawn((
            crate::components::username_label::UsernameLabel::with_offset(
                "Tracking test".into(),
                hero_entity,
                45.,
            ),
            Text2d::new("Tracking test"),
            TextFont {
                font_size: FontSize::Px(16.),
                ..default()
            },
            Transform::default(),
        ));
        if std::env::args().any(|arg| arg == "--vfx-smoke-test") {
            probe.step = 15;
        }
        for (entity, transform, mut block) in &mut blocks {
            if transform.translation.distance(hero.translation) < 600.0 {
                block.health = block.max_health.saturating_sub(1);
                // Duplicate events must still produce exactly one crack overlay.
                for _ in 0..2 {
                    damage.write(BlockDamaged {
                        entity,
                        damage: 0,
                        remaining_health: block.health,
                    });
                }
            }
        }
        probe.since = elapsed;
        return;
    }
    if probe.awaiting.is_none()
        && elapsed - probe.since
            < if probe.step == 16 || probe.step == 18 {
                0.25
            } else if probe.step == 10 {
                10.0
            } else if probe.step == 30 {
                8.0
            } else if probe.step == 33 {
                1.0
            } else if probe.step == 22 {
                0.4
            } else {
                3.0
            }
    {
        return;
    }
    if let Some(expected) = probe.awaiting {
        if teleport.target.is_some() {
            if !probe.portal_captured && elapsed - probe.since > 0.25 {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk("mineheads-smoke-teleport.png"));
                probe.portal_captured = true;
            }
            return;
        }
        assert_eq!(
            determine_biome_at_position(hero.translation.x, hero.translation.y),
            expected,
            "Teleport landed in wrong biome"
        );
        eprintln!(
            "SMOKE teleport {:?} passed in {:.3}s",
            expected,
            elapsed - probe.since
        );
        probe.awaiting = None;
        probe.step += 1;
        probe.since = elapsed;
        return;
    }
    match probe.step {
        0 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("mineheads-smoke-world.png"));
        }
        1..=8 => {
            if probe.step == 7 {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk("mineheads-smoke-nether.png"));
            }
            let biome = BIOMES[probe.step - 1];
            teleport.target = Some(Vec3::new(
                hero.translation.x,
                nearest_biome_y(hero.translation.y, biome),
                hero.translation.z,
            ));
            probe.awaiting = Some(biome);
            probe.since = elapsed;
            return;
        }
        9 => {
            weather.weather_type = WeatherType::Snow;
            weather.intensity = 1.0;
            weather.duration_remaining = 9999.0;
        }
        10 => {
            assert!(
                snow.iter().count() > 0,
                "Actual snowfall must produce visible caps without manually inserting layers"
            );
            eprintln!("SMOKE natural snow caps: {}", snow.iter().count());
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("mineheads-smoke-snow.png"));
        }
        11 => {
            state.set(GameState::Paused);
        }
        12 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("mineheads-smoke-pause.png"));
            state.set(GameState::Settings);
        }
        13 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("mineheads-smoke-settings.png"));
        }
        14 => {
            state.set(GameState::Playing);
            weather.weather_type = WeatherType::Clear;
            weather.intensity = 0.0;
        }
        15 => {
            if let Some(queue) = crate::plugins::external_api_plugin::get_command_queue() {
                let mut queue = queue.lock().unwrap();
                for power in [
                    crate::resources::command_queue::PowerCommandType::Mirror,
                    crate::resources::command_queue::PowerCommandType::LaserBurst,
                ] {
                    queue.push_back(crate::resources::command_queue::PowerCommand::new(
                        power,
                        "Visual QA".into(),
                    ));
                }
            }
        }
        16 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("mineheads-smoke-energy.png"));
        }
        17 => {
            if let Some(queue) = crate::plugins::external_api_plugin::get_command_queue() {
                queue.lock().unwrap().push_back(
                    crate::resources::command_queue::PowerCommand::new(
                        crate::resources::command_queue::PowerCommandType::Laser,
                        "Visual QA".into(),
                    ),
                );
            }
        }
        18 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("mineheads-smoke-laser.png"));
        }
        19..=20 => {}
        21 => {
            commands.queue(liquid_gallery);
            commands.queue(crate::systems::graphics::smoke_torches);
        }
        22 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("mineheads-smoke-waterfall.png"));
        }
        23 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("mineheads-smoke-liquids.png"));
        }
        24 => {
            probe.portal_expected = Some(crate::systems::world_portals::next_biome(
                determine_biome_at_position(hero.translation.x, hero.translation.y),
            ));
            commands.queue(portal_gallery);
        }
        25 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("mineheads-smoke-world-portal.png"));
        }
        26 => {
            let mut t = *hero;
            t.translation += Vec3::new(224., 128., 0.);
            commands.entity(hero_entity).insert(t);
        }
        27 => {
            assert_eq!(
                Some(determine_biome_at_position(
                    hero.translation.x,
                    hero.translation.y
                )),
                probe.portal_expected,
                "Always-active portal did not transport to its advertised biome"
            );
            eprintln!("SMOKE background portal and safe destination passed");
        }
        28 => {
            commands.queue(crate::systems::dragon_flyover::smoke_spawn);
        }
        29 => {
            commands.queue(crate::systems::dragon_flyover::smoke_verify);
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("mineheads-smoke-dragon.png"));
        }
        30 => {
            commands.queue(|world: &mut World| {
                let mut cycle =
                    world.resource_mut::<crate::resources::day_night_cycle::DayNightCycle>();
                cycle.current_time = cycle.cycle_duration * 0.75;
            });
        }
        31 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("mineheads-smoke-night.png"));
        }
        32 => {
            commands.queue(crate::systems::dragon_flyover::smoke_breath);
        }
        33 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("mineheads-smoke-dragon-breath.png"));
        }
        34 => {
            commands.queue(crate::systems::dragon_flyover::smoke_damage);
        }
        35 => {
            commands.queue(crate::systems::dragon_flyover::smoke_damaged);
        }
        36 => {}
        _ => {
            eprintln!(
                "SMOKE PASSED: {}",
                if std::env::args().any(|arg| arg == "--vfx-smoke-test") {
                    "energy, liquid, arrow and same-frame label rendering"
                } else {
                    "all eight biomes, damage shader, snow and menu schedules"
                }
            );
            exit.write(AppExit::Success);
        }
    }
    probe.step += 1;
    probe.since = elapsed;
}

/// A disposable render fixture in hidden smoke runs, never the user's saved world.
pub(crate) fn liquid_gallery(world: &mut World) {
    use crate::{
        components::{
            block::{Block, BlockType},
            enemy::Health,
            hero::Hero,
        },
        resources::{material_cache::MaterialCache, mesh_cache::MeshCache},
        systems::fluids::{FluidSeed, FluidWorld, Liquid},
    };
    let (hero, position) = world
        .query_filtered::<(Entity, &Transform), With<Hero>>()
        .iter(world)
        .map(|(e, t)| (e, t.translation))
        .next()
        .unwrap();
    let center = (position.xy() / 64.0).round().as_ivec2();
    let origin = Vec3::new(center.x as f32 * 64.0, center.y as f32 * 64.0, 10.0);
    let remove: Vec<_> = world
        .query::<(Entity, &Transform, Option<&Block>, Option<&Health>)>()
        .iter(world)
        .filter(|(e, t, b, h)| {
            *e != hero
                && (h.is_some()
                    || (b.is_some()
                        && (t.translation - origin).truncate().abs().max_element() < 640.0))
        })
        .map(|(e, _, _, _)| e)
        .collect();
    for e in remove {
        world.entity_mut(e).despawn();
    }
    world.entity_mut(hero).insert((
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::LinearVelocity::ZERO,
        Transform::from_translation(origin),
    ));
    world.insert_resource(FluidWorld::default());
    let cube = world.resource::<MeshCache>().block_mesh.clone();
    let cactus = world.resource::<MeshCache>().cactus_mesh.clone();
    let stone = world
        .resource::<MaterialCache>()
        .get(BlockType::Stone)
        .unwrap();
    for x in -5_i32..=5 {
        for y in -4..=-1 {
            if y != -4 && x != -5 && x != 0 && x != 5 {
                continue;
            }
            world.spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                BlockType::Stone,
                Mesh3d(cube.clone()),
                MeshMaterial3d(stone.clone()),
                Transform::from_xyz(
                    origin.x + x as f32 * 64.0,
                    origin.y + y as f32 * 64.0,
                    -10.0,
                )
                .with_scale(Vec3::splat(64.0)),
            ));
        }
    }
    // Two ledges force the same sideways-to-falling joins as the user report.
    for (x, y) in [(-2, 1), (-1, 0)] {
        world.spawn((
            Block {
                health: 3,
                max_health: 3,
            },
            BlockType::Stone,
            Mesh3d(cube.clone()),
            MeshMaterial3d(stone.clone()),
            Transform::from_xyz(origin.x + x as f32 * 64., origin.y + y as f32 * 64., -10.)
                .with_scale(Vec3::splat(64.)),
        ));
    }
    let material = world
        .resource::<MaterialCache>()
        .get(BlockType::Cactus)
        .unwrap();
    for y in 1..=2 {
        world.spawn((
            Block {
                health: 3,
                max_health: 3,
            },
            BlockType::Cactus,
            Mesh3d(cactus.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(origin.x - 192.0, origin.y + y as f32 * 64.0, -10.0),
        ));
    }
    world.spawn(FluidSeed {
        cell: (center.x - 2, center.y + 3),
        kind: Liquid::Water,
    });
    world.spawn(FluidSeed {
        cell: (center.x + 2, center.y - 2),
        kind: Liquid::Lava,
    });
    let arrow_model = world
        .resource::<crate::resources::asset_handles::ProjectileModels>()
        .arrow
        .clone();
    world.spawn((
        crate::components::projectile::Arrow::new_friendly(1),
        crate::components::bundles::avian_physics::arrow(Vec3::new(500., 0., 0.)),
        bevy::world_serialization::WorldAssetRoot(arrow_model),
        Visibility::Visible,
        Transform::from_translation(origin + Vec3::new(-128., -64., 10.))
            .with_scale(Vec3::splat(15.)),
    ));
}

/// Check actual camera projection AFTER the full render-frame schedule, including teleport.
fn verify_label_projection(
    labels: Query<(
        &crate::components::username_label::UsernameLabel,
        &GlobalTransform,
    )>,
    transforms: Query<&GlobalTransform>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    windows: Query<&Window>,
    mut samples: Local<u32>,
) {
    let (Ok((camera, view)), Ok(window)) = (cameras.single(), windows.single()) else {
        return;
    };
    for (label, global) in &labels {
        let Ok(owner) = transforms.get(label.owner) else {
            continue;
        };
        let Ok(screen) =
            camera.world_to_viewport(view, owner.translation() + Vec3::Y * label.offset_y)
        else {
            continue;
        };
        let expected = Vec2::new(
            screen.x - window.width() * 0.5,
            window.height() * 0.5 - screen.y,
        )
        .round();
        assert!(
            global.translation().xy().distance(expected) < 0.01,
            "Username projection used a stale head or camera transform"
        );
        *samples += 1;
        if *samples == 300 {
            eprintln!("SMOKE same-frame label projection passed 300 moving frames");
        }
    }
}

fn portal_gallery(world: &mut World) {
    use crate::components::{block::Block, enemy::Health, hero::Hero};
    use crate::resources::{material_cache::MaterialCache, mesh_cache::MeshCache};
    let (hero, p) = world
        .query_filtered::<(Entity, &Transform), With<Hero>>()
        .single(world)
        .map(|(e, t)| (e, t.translation))
        .unwrap();
    let origin = (p.xy() / 64.).round().as_ivec2();
    let remove: Vec<_> = world
        .query::<(Entity, &Transform, Option<&Block>, Option<&Health>)>()
        .iter(world)
        .filter(|(e, t, b, h)| {
            *e != hero
                && (h.is_some()
                    || (b.is_some() && (t.translation.xy() - p.xy()).abs().max_element() < 640.))
        })
        .map(|(e, _, _, _)| e)
        .collect();
    for e in remove {
        world.entity_mut(e).despawn();
    }
    world.insert_resource(crate::systems::fluids::FluidWorld::default());
    world.resource_scope(|world, materials: Mut<MaterialCache>| {
        world.resource_scope(|world, meshes: Mut<MeshCache>| {
            crate::systems::world_portals::spawn_ruin(
                &mut world.commands(),
                origin,
                determine_biome_at_position(p.x, p.y),
                &meshes,
                &materials,
            );
        });
    });
    world.entity_mut(hero).insert(Transform::from_xyz(
        origin.x as f32 * 64. - 192.,
        origin.y as f32 * 64. - 64.,
        10.,
    ));
}
