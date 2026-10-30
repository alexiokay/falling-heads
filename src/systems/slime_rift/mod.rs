//! Isolated minigame lifecycle. The main world remains intact and paused.
use crate::{
    components::{
        block::{Block, BlockType},
        enemy::Health,
        hero::Hero,
    },
    events::power_events::PowerActivated,
    resources::{action_center::ActionCenter, command_queue::PowerCommandType},
    states::GameState,
};
use avian3d::prelude::*;
use bevy::{camera::visibility::RenderLayers, prelude::*};
mod course;
pub(crate) mod probe;
mod visuals;
use course::{COUNT, Course, STEP};

/// The hero while it waits (hidden and frozen) in the live world during a rift.
#[derive(Component)]
pub(crate) struct RiftParked;
#[derive(Component)]
pub(super) struct Scene;
#[derive(Component)]
pub(super) struct Runner;
#[derive(Component)]
pub(super) struct PadVisual(pub usize);
#[derive(Component)]
pub(super) struct Gem(pub usize);
#[derive(Component)]
pub(super) struct Hud;
#[derive(Component)]
struct Gate;
#[derive(Resource)]
struct Rift {
    request: bool,
    rewind_request: bool,
    gate: Option<(Entity, Vec3)>,
    cooldown: f32,
    check_at: f32,
    session: Option<Session>,
}
impl Default for Rift {
    fn default() -> Self {
        Self {
            request: false,
            rewind_request: false,
            gate: None,
            cooldown: 0.,
            check_at: 30.,
            session: None,
        }
    }
}
struct Session {
    hero: Entity,
    start: Transform,
    exit: Vec3,
    visibility: Visibility,
    camera: Entity,
    camera_start: Transform,
    layers: Option<RenderLayers>,
    suns: Vec<(Entity, f32)>,
    origin: Vec3,
    course: Course,
    elapsed: f32,
    accumulator: f32,
    returning: Option<(f32, Transform, Vec3)>,
    success: bool,
    paused: bool,
}
pub(crate) struct SlimeRiftPlugin;
impl Plugin for SlimeRiftPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Rift>()
            .add_systems(Startup, visuals::setup)
            .add_systems(Update, requests)
            .add_systems(
                PostUpdate,
                discover
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::Rift), enter)
            .add_systems(
                PostUpdate,
                update
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(GameState::Rift)),
            )
            .add_systems(OnExit(GameState::Rift), leave);
    }
}
fn requests(
    mut events: MessageReader<PowerActivated>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut rift: ResMut<Rift>,
) {
    if *state.get() == GameState::Playing && keys.just_pressed(KeyCode::F8) {
        rift.request = true;
    }
    if *state.get() == GameState::Rift && keys.just_pressed(KeyCode::KeyR) {
        rift.rewind_request = true;
    }
    if *state.get() == GameState::Rift
        && let Some(queue) = crate::plugins::external_api_plugin::get_command_queue()
        && let Ok(mut q) = queue.lock()
    {
        q.retain(|cmd| match cmd.command_type {
            PowerCommandType::Rewind => {
                rift.rewind_request = true;
                false
            }
            PowerCommandType::Rift => false,
            _ => true,
        });
    }
    for e in events.read() {
        match e.command.command_type {
            PowerCommandType::Rift if *state.get() == GameState::Playing => rift.request = true,
            PowerCommandType::Rewind if *state.get() == GameState::Rift => {
                rift.rewind_request = true
            }
            _ => {}
        }
    }
}
fn discover(w: &mut World) {
    let dt = w.resource::<Time>().delta_secs();
    let now = w.resource::<Time>().elapsed_secs();
    let mut r = w.remove_resource::<Rift>().unwrap();
    r.cooldown = (r.cooldown - dt).max(0.);
    let hero = w
        .query_filtered::<(Entity, &Transform), With<Hero>>()
        .single(w)
        .ok()
        .map(|(e, t)| (e, *t));
    if !crate::systems::game_modes::is_on(
        w.get_resource::<crate::systems::game_modes::GameModes>(),
        crate::systems::game_modes::GameMode::SlimeRift,
    ) {
        // Turned off in Settings > Game modes: no natural gates, requests refused.
        if r.request {
            r.request = false;
            let mut n = w.resource_mut::<super::world_portals::PowerNotice>();
            n.text = "Slime Rift is turned off (Settings > Game modes)".into();
            n.remaining = 3.;
        }
        if let Some((gate, _)) = r.gate.take() {
            w.despawn(gate);
        }
        w.insert_resource(r);
        return;
    }
    if let Some((hero, t)) = hero {
        let busy = w.get::<super::launcher::LauncherPassenger>(hero).is_some()
            || w.resource::<super::biome_teleport::BiomeTeleport>().phase
                != super::biome_teleport::TeleportPhase::Idle;
        if r.request && busy {
            r.request = false;
            let mut n = w.resource_mut::<super::world_portals::PowerNotice>();
            n.text = "Finish travelling before entering a rift".into();
            n.remaining = 3.;
        }
        if !busy && (r.request || now >= r.check_at) {
            r.check_at = now + 2.;
            if r.request {
                r.request = false;
                if r.cooldown <= 0. || std::env::args().any(|a| a == "--rift-test") {
                    // A requested encounter opens around the hero with a visible pull-in.
                    if let Some((gate, _)) = r.gate.take() {
                        w.despawn(gate);
                    }
                    let gate = visuals::gate(w, t.translation + Vec3::Z * 50., false);
                    r.gate = Some((gate, t.translation));
                    w.resource_mut::<NextState<GameState>>()
                        .set(GameState::Rift);
                } else {
                    let mut n = w.resource_mut::<super::world_portals::PowerNotice>();
                    n.text = format!("Rift ready in {:.0}s", r.cooldown.ceil());
                    n.remaining = 3.;
                }
            } else if r.cooldown <= 0. && r.gate.is_none() {
                let biome = crate::utils::biome_utils::determine_biome_at_position(
                    t.translation.x,
                    t.translation.y,
                );
                if matches!(
                    biome,
                    crate::components::block::BiomeType::Forest
                        | crate::components::block::BiomeType::Plains
                ) {
                    let candidates: Vec<_> = w
                        .query::<(&Transform, &BlockType, &Block)>()
                        .iter(w)
                        .filter(|(_, k, b)| {
                            b.health > 0 && matches!(k, BlockType::Grass | BlockType::Dirt)
                        })
                        .map(|(b, _, _)| b.translation + Vec3::Y * 100.)
                        .filter(|p| {
                            let d = *p - t.translation;
                            d.x.abs() > 250. && d.x.abs() < 800. && d.y.abs() < 350.
                        })
                        .collect();
                    if let Some(p) = candidates.into_iter().find(|p| clear(w, *p)) {
                        let gate = visuals::gate(w, p + Vec3::Z * 50., false);
                        r.gate = Some((gate, p));
                    }
                }
            }
        }
        if let Some((gate, p)) = r.gate {
            if p.xy().distance(t.translation.xy()) > 2200. {
                w.despawn(gate);
                r.gate = None;
            } else if !busy && p.xy().distance(t.translation.xy()) < 85. {
                w.resource_mut::<NextState<GameState>>()
                    .set(GameState::Rift);
            } else if let Some(mut gt) = w.get_mut::<Transform>(gate) {
                gt.rotation = Quat::from_rotation_z((now * 0.6).sin() * 0.12);
            }
        }
    }
    w.insert_resource(r);
}
fn clear(w: &World, p: Vec3) -> bool {
    w.resource::<crate::resources::spatial_hash::SpatialHash>()
        .rect_iter(p.xy() - Vec2::splat(76.), p.xy() + Vec2::splat(76.))
        .all(|e| {
            let Some(kind) = w.get::<BlockType>(e) else {
                return true;
            };
            let Some(t) = w.get::<Transform>(e) else {
                return true;
            };
            super::plant_rules::soft(*kind, w.get::<super::biome_content::Tile>(e))
                || (t.translation.x - p.x).abs() >= 76.
                || (t.translation.y - p.y).abs() >= 76.
        })
}

fn enter(w: &mut World) {
    let Some((hero, start, visibility)) = w
        .query_filtered::<(Entity, &Transform, &Visibility), With<Hero>>()
        .single(w)
        .ok()
        .map(|(e, t, v)| (e, *t, *v))
    else {
        w.resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        return;
    };
    // A rocket or teleport can start later in the same frame as the request.
    if w.get::<super::launcher::LauncherPassenger>(hero).is_some()
        || w.resource::<super::biome_teleport::BiomeTeleport>().phase
            != super::biome_teleport::TeleportPhase::Idle
        || w.get::<Health>(hero).is_none_or(|hp| hp.current == 0)
    {
        w.resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        return;
    }
    let Some((camera, camera_start, layers)) = w
        .query_filtered::<(Entity, &Transform, Option<&RenderLayers>), With<Camera3d>>()
        .single(w)
        .ok()
        .map(|(e, t, l)| (e, *t, l.cloned()))
    else {
        w.resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        return;
    };
    let mut exit = start.translation;
    // Only already loaded, unobstructed openings; no terrain is erased by travel.
    for dx in [640., -640., 448., -448., 256., -256.] {
        let p = start.translation + Vec3::X * dx;
        let c = crate::components::chunk::world_to_chunk_coords(p.x, p.y, 64.);
        if w.resource::<crate::components::chunk::LoadedChunks>()
            .is_loaded(c.0, c.1)
            && clear(w, p)
            && !w
                .resource::<super::fluids::FluidWorld>()
                .is_water_at(p.xy())
        {
            exit = p;
            break;
        }
    }
    // The world keeps running behind the course. Park the hero instead of
    // pausing time: hidden, frozen in place, no collision, not a target.
    w.entity_mut(hero).insert((
        Visibility::Hidden,
        RiftParked,
        RigidBody::Static,
        LinearVelocity::ZERO,
        ColliderDisabled,
    ));
    w.entity_mut(camera)
        .insert(RenderLayers::from_layers(&[0, 1]));
    let origin = start.translation + Vec3::new(0., 60., 500.);
    // The real sun stays on: the real world is visible behind the course. The
    // values are still recorded so leaving the rift restores them exactly.
    let suns: Vec<_> = w
        .query::<(Entity, &DirectionalLight)>()
        .iter(w)
        .map(|(e, light)| (e, light.illuminance))
        .collect();
    visuals::scene(w, origin, start.translation);
    w.resource_mut::<Rift>().session = Some(Session {
        hero,
        start,
        exit,
        visibility,
        camera,
        camera_start,
        layers,
        suns,
        origin,
        course: Course::default(),
        elapsed: 0.,
        accumulator: 0.,
        returning: None,
        success: false,
        paused: false,
    });
}
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * t * (t * (t * 6. - 15.) + 10.)
}
fn blend(a: Transform, b: Transform, t: f32) -> Transform {
    Transform {
        translation: a.translation.lerp(b.translation, t),
        rotation: a.rotation.slerp(b.rotation, t),
        scale: Vec3::ONE,
    }
}
fn update(w: &mut World) {
    let mut r = w.remove_resource::<Rift>().unwrap();
    let Some(s) = r.session.as_mut() else {
        w.insert_resource(r);
        return;
    };
    let keys = w.resource::<ButtonInput<KeyCode>>();
    let cancel = keys.just_pressed(KeyCode::Escape);
    if keys.just_pressed(KeyCode::KeyP) {
        s.paused = !s.paused;
    }
    let steer = Vec2::new(
        (keys.pressed(KeyCode::KeyD) as i8 - keys.pressed(KeyCode::KeyA) as i8) as f32,
        (keys.pressed(KeyCode::KeyS) as i8 - keys.pressed(KeyCode::KeyW) as i8) as f32,
    );
    let dt = if s.paused {
        0.
    } else {
        w.resource::<Time<Real>>().delta_secs().min(0.1)
    };
    s.elapsed += dt;
    if r.rewind_request {
        r.rewind_request = false;
        s.course.rewind();
    }
    if (cancel || s.course.frame.complete) && s.returning.is_none() {
        s.success = s.course.frame.complete;
        let camera = *w.get::<Transform>(s.camera).unwrap();
        let p = s.origin + s.course.frame.p;
        s.returning = Some((0., camera, p));
        s.paused = false;
    }
    let desired;
    let runner_pos;
    if let Some((elapsed, from, p)) = s.returning.as_mut() {
        *elapsed += w.resource::<Time<Real>>().delta_secs().min(0.1);
        let t = smooth(*elapsed / 1.4);
        if *elapsed >= 0.55 {
            w.entity_mut(s.camera)
                .insert(RenderLayers::from_layers(&[0, 1]));
        }
        visuals::transition(w, 1. - smooth((*elapsed - 0.55).abs() / 0.35));
        let endpoint = if s.success {
            s.exit
        } else {
            s.start.translation
        };
        let camera = Transform::from_translation(endpoint + Vec3::new(30., 80., 1500.))
            .looking_at(endpoint, Vec3::Y);
        desired = blend(*from, camera, t);
        runner_pos = p.lerp(endpoint, t);
        if *elapsed >= 1.4 {
            w.resource_mut::<NextState<GameState>>()
                .set(GameState::Playing);
        }
    } else {
        visuals::transition(w, 1. - smooth((s.elapsed - 1.6).abs() / 0.4));
        if s.elapsed > 1.6 {
            s.accumulator += dt;
            while s.accumulator >= STEP {
                s.course.tick(steer);
                s.accumulator -= STEP;
            }
        }
        let p = s.origin + s.course.pose(s.accumulator / STEP);
        let camera = Transform::from_translation(p + Vec3::new(-490., 620., 1030.))
            .looking_at(p + Vec3::new(190., 30., 0.), Vec3::Y);
        desired = if s.elapsed < 1.6 {
            blend(s.camera_start, camera, smooth(s.elapsed / 1.6))
        } else {
            let current = *w.get::<Transform>(s.camera).unwrap();
            blend(current, camera, 1. - (-4.5 * dt).exp())
        };
        let pull = smooth(s.elapsed / 1.35);
        runner_pos = s.start.translation.lerp(p, pull)
            + Vec3::Y * (pull * std::f32::consts::PI).sin() * 150.;
    }
    *w.get_mut::<Transform>(s.camera).unwrap() = desired;
    visuals::animate(
        w,
        s.origin,
        &s.course,
        runner_pos,
        s.paused,
        s.returning.is_some(),
    );
    w.insert_resource(r);
}
fn leave(w: &mut World) {
    let mut r = w.remove_resource::<Rift>().unwrap();
    if let Some(s) = r.session.take() {
        if let Some((gate, _)) = r.gate.take() {
            w.despawn(gate);
        }
        let endpoint = if s.success {
            s.exit
        } else {
            s.start.translation
        };
        if let Ok(mut hero) = w.get_entity_mut(s.hero) {
            hero.remove::<(RiftParked, ColliderDisabled)>();
            hero.insert((RigidBody::Dynamic, LinearVelocity::ZERO));
            hero.insert((
                s.visibility,
                Transform {
                    translation: endpoint,
                    ..s.start
                },
                Position(endpoint),
            ));
            if s.success {
                hero.insert(LinearVelocity(Vec3::new(120., 300., 0.)));
            }
        }
        if s.success {
            if let Some(mut hp) = w.get_mut::<Health>(s.hero) {
                hp.current = (hp.current + 1).min(hp.max);
            }
            let amount = 50 + s.course.frame.gems.count_ones() * 15;
            if let Some(mut xp) = w.get_mut::<crate::components::experience::Experience>(s.hero) {
                xp.add_xp(amount);
            }
            let mut notice = w.resource_mut::<super::world_portals::PowerNotice>();
            notice.text = format!("Slime rift complete! +{amount} XP, +1 heart");
            notice.remaining = 5.;
        }
        if let Some(mut center) = w.get_resource_mut::<ActionCenter>() {
            center.center = endpoint;
            center.raw_center = endpoint;
        }
        if let Ok(mut camera) = w.get_entity_mut(s.camera) {
            camera.insert(
                Transform::from_translation(endpoint + Vec3::new(30., 80., 1500.))
                    .looking_at(endpoint, Vec3::Y),
            );
            if let Some(layers) = s.layers {
                camera.insert(layers);
            } else {
                camera.remove::<RenderLayers>();
            }
        }
        for (entity, illuminance) in s.suns {
            if let Some(mut light) = w.get_mut::<DirectionalLight>(entity) {
                light.illuminance = illuminance;
            }
        }
        r.cooldown = 60.;
    }
    let entities: Vec<_> = w.query_filtered::<Entity, With<Scene>>().iter(w).collect();
    for e in entities {
        if w.get_entity(e).is_ok() {
            w.despawn(e);
        }
    }
    w.insert_resource(r);
}
