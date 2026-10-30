//! Native regression scene: supported snow motion and rendered ghast transitions.
use crate::{
    components::{
        block::{Block, BlockType},
        chunk::ChunkMember,
        enemy::{Enemy, EnemyType},
        hero::Hero,
        snow_slide::SnowSlide,
        villager::Villager,
        weather::{ActiveWeather, WeatherType},
    },
    resources::{
        asset_handles::{EnemyTextures, GameTextures},
        command_queue::{PowerCommand, PowerCommandType},
        graphics_settings::GraphicsSettings,
        material_cache::MaterialCache,
        mesh_cache::MeshCache,
        settings::GameSettings,
    },
};
use avian3d::prelude::*;
use bevy::{
    ecs::system::SystemState,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
#[derive(Resource, Default)]
struct Probe {
    stage: u8,
    since: f32,
    anchor: Vec3,
    ghast: Option<Entity>,
    snow_head: Option<Entity>,
    last_open: Option<bool>,
    edges: u8,
    remaining: u8,
    captures: Vec<serde_json::Value>,
    snow_start: Option<Vec3>,
    snow_since: Option<f32>,
    snow_mid: bool,
    snow_done: bool,
}
fn capture(w: &mut World, name: &str) {
    w.spawn(Screenshot::primary_window())
        .observe(save_to_disk(format!("target/visual-snowghast-{name}.png")));
}
pub fn run(w: &mut World) {
    if !w.contains_resource::<Probe>() {
        w.init_resource::<Probe>();
    }
    let now = w.resource::<Time<Real>>().elapsed_secs();
    let stage = w.resource::<Probe>().stage;
    assert!(now < 70., "Snow/ghast test timed out at stage {stage}");
    if let Some(mut events) =
        w.get_resource_mut::<crate::systems::random_events::RandomEventTimer>()
    {
        events.timer.reset();
    }
    {
        let mut weather = w.resource_mut::<ActiveWeather>();
        weather.weather_type = WeatherType::Clear;
        weather.intensity = 0.;
        weather.duration_remaining = 999.;
    }
    if stage > 0 {
        let anchor = w.resource::<Probe>().anchor;
        for (mut p, mut v) in w
            .query_filtered::<(&mut Position, &mut LinearVelocity), With<Hero>>()
            .iter_mut(w)
        {
            p.x = anchor.x + 230.;
            p.y = anchor.y + 30.;
            v.0 = Vec3::ZERO;
        }
        if let Some(e) = w.resource::<Probe>().ghast {
            if let Some(mut p) = w.get_mut::<Position>(e) {
                p.x = anchor.x;
                p.y = anchor.y + 30.;
            }
            if let Some(mut v) = w.get_mut::<LinearVelocity>(e) {
                v.0 = Vec3::ZERO;
            }
        }
        // Exercise actual shooting, but remove emitted projectiles before they
        // can disturb the white-face image measurement or the snow platform.
        let shots: Vec<_> = w
            .query_filtered::<Entity, With<crate::components::projectile::Fireball>>()
            .iter(w)
            .collect();
        for e in shots {
            w.despawn(e);
        }
        if let Some(head) = w.resource::<Probe>().snow_head {
            let position = w.get::<Position>(head).map(|p| p.0);
            if let Some(p) = position {
                let sliding = w.get::<SnowSlide>(head).is_some();
                let start = w.resource::<Probe>().snow_start;
                if !w.resource::<Probe>().snow_done {
                    if sliding && start.is_none() {
                        let time = w.resource::<Time<Virtual>>().elapsed_secs();
                        let mut probe = w.resource_mut::<Probe>();
                        probe.snow_start = Some(p);
                        probe.snow_since = Some(time);
                        capture(w, "snow-contact");
                    } else if sliding
                        && start.is_some_and(|s| p.x - s.x > 1.5)
                        && !w.resource::<Probe>().snow_mid
                    {
                        assert!((p.y - start.unwrap().y).abs() < 0.2);
                        w.resource_mut::<Probe>().snow_mid = true;
                        capture(w, "snow-slide");
                    } else if !sliding && start.is_some() {
                        assert!(
                            p.x > start.unwrap().x + 2.,
                            "Short snow contact must still travel sideways"
                        );
                        let contact = w.resource::<Time<Virtual>>().elapsed_secs()
                            - w.resource::<Probe>().snow_since.unwrap();
                        assert!(contact <= 0.12, "Snow contact lasted too long: {contact}");
                        eprintln!(
                            "[SNOW CONTACT] observed {:.1} ms before normal rebound",
                            contact * 1000.
                        );
                        assert!(
                            w.get::<LinearVelocity>(head).unwrap().y > 300.,
                            "Normal rebound must follow slide"
                        );
                        w.resource_mut::<Probe>().snow_done = true;
                        capture(w, "snow-rebound");
                    }
                }
            }
        }
    }
    if stage == 0 && now > 8. {
        let anchor = w
            .query_filtered::<&Transform, With<Hero>>()
            .single(w)
            .unwrap()
            .translation;
        let anchor = (anchor.xy() / 64.).round().extend(0.) * 64.;
        w.resource_mut::<Probe>().anchor = anchor;
        w.query::<&mut Window>()
            .single_mut(w)
            .unwrap()
            .resolution
            .set(1360., 900.);
        {
            let mut g = w.resource_mut::<GraphicsSettings>();
            g.ray_tracing = false;
            g.dlss = false;
            g.frame_generation = false;
            g.fog = false;
        }
        {
            let mut s = w.resource_mut::<GameSettings>();
            s.effects_volume = 0.;
            s.music_volume = 0.;
            s.fps_limit = 60;
        }
        let remove: Vec<_> = w
            .query::<(
                Entity,
                &Transform,
                Option<&Block>,
                Option<&Enemy>,
                Option<&Villager>,
            )>()
            .iter(w)
            .filter(|(_, t, b, e, v)| {
                e.is_some()
                    || v.is_some()
                    || (b.is_some() && t.translation.xy().distance(anchor.xy()) < 1100.)
            })
            .map(|(e, _, _, _, _)| e)
            .collect();
        for e in remove {
            w.despawn(e);
        }
        w.insert_resource(crate::systems::fluids::FluidWorld::default());
        let mesh = w.resource::<MeshCache>().block_mesh.clone();
        let mat = w.resource::<MaterialCache>().materials[&BlockType::Snow].clone();
        let chunk = (anchor.xy() / 1024.).floor().as_ivec2();
        for x in -9..=9 {
            w.spawn((
                Block {
                    health: 10000,
                    max_health: 10000,
                },
                BlockType::Snow,
                ChunkMember {
                    chunk_x: chunk.x,
                    chunk_y: chunk.y,
                },
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(anchor + Vec3::new(x as f32 * 64., -320., -10.))
                    .with_scale(Vec3::splat(64.)),
            ));
        }
        // A dark backing card makes a missing/black face objectively detectable.
        let mesh = w
            .resource_mut::<Assets<Mesh>>()
            .add(Rectangle::new(150., 150.));
        let mat = w
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                unlit: true,
                base_color: Color::srgb(0.01, 0.01, 0.01),
                ..default()
            });
        w.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(anchor + Vec3::new(0., 30., 24.)),
            bevy::light::NotShadowCaster,
        ));
        let mut state = SystemState::<(
            Commands,
            Res<GameTextures>,
            ResMut<Assets<Mesh>>,
            ResMut<Assets<StandardMaterial>>,
            Res<AssetServer>,
        )>::new(w);
        let (mut commands, textures, mut meshes, mut materials, server) = state.get_mut(w).unwrap();
        let head = crate::systems::entity_spawn::spawn_villager(
            &mut commands,
            anchor + Vec3::new(-200., -170., 30.),
            &textures,
            &mut meshes,
            &mut materials,
            None,
            &server,
        );
        commands
            .entity(head)
            .insert(LinearVelocity(Vec3::new(100., -100., 0.)));
        state.apply(w);
        w.get_mut::<Villager>(head).unwrap().move_direction = 1.;
        w.resource_mut::<Probe>().snow_head = Some(head);
        crate::plugins::external_api_plugin::get_command_queue()
            .unwrap()
            .lock()
            .unwrap()
            .push_back(PowerCommand {
                command_type: PowerCommandType::Ghast,
                username: "probe".into(),
                launcher: default(),
                style: None,
            });
        let mut p = w.resource_mut::<Probe>();
        p.stage = 1;
        p.since = now;
    } else if stage == 1 {
        if let Some(e) = w
            .query::<(Entity, &Enemy)>()
            .iter(w)
            .find(|(_, e)| e.enemy_type == EnemyType::Ghast)
            .map(|(e, _)| e)
        {
            w.get_mut::<Enemy>(e).unwrap().shoot_timer =
                Some(Timer::from_seconds(0.9, TimerMode::Repeating));
            let mut p = w.resource_mut::<Probe>();
            p.ghast = Some(e);
            p.stage = 2;
            p.since = now;
        }
    } else if (stage == 2 || stage == 3) && now > w.resource::<Probe>().since + 3. {
        let entity = w.resource::<Probe>().ghast.unwrap();
        let material = w
            .get::<MeshMaterial3d<StandardMaterial>>(entity)
            .unwrap()
            .0
            .clone();
        let frame = w
            .resource::<Assets<StandardMaterial>>()
            .get(&material)
            .unwrap()
            .base_color_texture
            .clone()
            .unwrap();
        let textures = w.resource::<EnemyTextures>();
        assert!(frame == textures.ghast || frame == textures.ghast_shooting);
        assert!(w.resource::<Assets<Image>>().contains(frame.id()));
        let open = frame == textures.ghast_shooting;
        let last = w.resource::<Probe>().last_open;
        if last.is_some_and(|previous| previous != open) {
            let mut p = w.resource_mut::<Probe>();
            p.edges += 1;
            p.remaining = 3;
        }
        w.resource_mut::<Probe>().last_open = Some(open);
        if w.resource::<Probe>().remaining > 0 {
            let p = w.resource::<Probe>();
            let mode = if stage == 2 { "raster" } else { "rt" };
            let name = format!("{mode}-edge{}-frame{}", p.edges, 3 - p.remaining);
            let center = w.get::<GlobalTransform>(entity).unwrap().translation();
            let (camera, ct) = w
                .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>()
                .single(w)
                .unwrap();
            let a = camera
                .world_to_viewport(ct, center + Vec3::new(-28., -28., 0.))
                .unwrap();
            let b = camera
                .world_to_viewport(ct, center + Vec3::new(28., 28., 0.))
                .unwrap();
            let min = a.min(b);
            let max = a.max(b);
            w.resource_mut::<Probe>()
                .captures
                .push(serde_json::json!({"name":name,"open":open,"box":[min.x,min.y,max.x,max.y]}));
            capture(w, &name);
            w.resource_mut::<Probe>().remaining -= 1;
        }
        if w.resource::<Probe>().edges >= 4 && w.resource::<Probe>().remaining == 0 {
            if stage == 2 {
                w.resource_mut::<GraphicsSettings>().ray_tracing = true;
                let mut p = w.resource_mut::<Probe>();
                p.stage = 3;
                p.since = now;
                p.edges = 0;
                p.last_open = None;
            } else {
                assert_eq!(
                    w.resource::<crate::systems::native_gpu::NativeStatus>()
                        .mode,
                    2
                );
                std::fs::write(
                    "target/snowghast-captures.json",
                    serde_json::to_vec_pretty(&w.resource::<Probe>().captures).unwrap(),
                )
                .unwrap();
                let mut p = w.resource_mut::<Probe>();
                p.stage = 4;
                p.since = now;
            }
        }
    } else if stage == 4 && now > w.resource::<Probe>().since + 2. {
        assert!(w.resource::<Probe>().snow_done && w.resource::<Probe>().snow_mid);
        assert_eq!(w.resource::<Probe>().captures.len(), 24);
        eprintln!(
            "[SNOWGHAST TEST] completed: supported slide/rebound; 24 mouth-transition frames in raster and RT"
        );
        w.write_message(AppExit::Success);
    }
}
