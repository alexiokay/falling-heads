//! Space prison event (`!PRISON`): the hero and one or two villagers are locked
//! in a spinning bedrock cell in zero gravity. Micro-blasts throw everyone
//! around, four fixed electromagnets take turns pulling heads to one side, a
//! button opens the trapdoor across the room for a moment, and guards come in
//! through the trapdoors. Fly out and an escape rocket picks you up and flies
//! you down to Earth (re-entry and all), landing you home with every villager
//! who got out; fail and the guards throw you out after 75 s. The world keeps
//! running behind it with the hero parked.
use crate::{
    components::{enemy::Health, hero::Hero},
    events::power_events::PowerActivated,
    resources::command_queue::PowerCommandType,
    states::GameState,
};
use avian3d::prelude::*;
use bevy::{camera::visibility::RenderLayers, prelude::*};
pub(crate) mod probe;
pub(crate) mod sim;
mod visuals;

#[derive(Component)]
pub(super) struct Scene;

#[derive(Resource, Default)]
pub(crate) struct Prison {
    request: Option<String>,
    session: Option<Session>,
}

pub(crate) struct Session {
    hero: Entity,
    start: Transform,
    visibility: Visibility,
    camera: Entity,
    camera_start: Transform,
    layers: Option<RenderLayers>,
    origin: Vec3,
    pub(crate) cell: sim::Cell,
    accumulator: f32,
    elapsed: f32,
    leaving: Option<f32>,
    villagers: usize,
    /// World text (ruin labels and the like) hidden for the visit, to restore.
    hidden_text: Vec<(Entity, Visibility)>,
    /// The escape ride to Earth, once the hero is out.
    ride: Option<Ride>,
    /// Seconds of screen shake left (blasts).
    shake: f32,
    magnet_was: Option<usize>,
    /// Where the camera looks: drifts toward the hero inside the cell.
    look: Vec3,
    /// World sounds (music included) paused for the visit, to resume after.
    paused_audio: Vec<Entity>,
}

/// Seconds from the hero leaving the cell to landing back in the world.
const RIDE_END: f32 = 4.4;

/// The rocket ride: it swoops in to pick the hero up at the trapdoor, then
/// dives for Earth, speeding up, until re-entry heat wraps it.
struct Ride {
    t: f32,
    exit: Vec3,
    side: Vec3,
    out: Vec3,
    target: Vec3,
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

impl Ride {
    fn at(&self, t: f32) -> Vec3 {
        if t < 0.9 {
            let k = 1. - (1. - t / 0.9).powi(3);
            (self.exit + self.side * 900.).lerp(self.exit, k)
        } else {
            let u = ((t - 0.9) / (RIDE_END - 1.2)).clamp(0., 1.);
            // Away from the prison first, then down, faster and faster.
            self.exit.lerp(self.target, u * u * u)
                + self.out * (u * std::f32::consts::PI).sin() * 320.
        }
    }
    /// Position, heading, flame (0..1) and re-entry heat (0..1).
    fn pose(&self) -> (Vec3, Vec3, f32, f32) {
        let p = self.at(self.t);
        let heading = if self.t < 0.9 {
            -self.side
        } else {
            self.at(self.t + 0.02) - p
        };
        let u = ((self.t - 0.9) / (RIDE_END - 1.2)).clamp(0., 1.);
        let flame = if self.t < 0.9 { 0.45 } else { 0.6 + u * 0.4 };
        (p, heading, flame, smooth((u - 0.55) / 0.4))
    }
}

impl Session {
    /// Seconds into the escape ride, once it has started.
    pub(crate) fn ride_time(&self) -> Option<f32> {
        self.ride.as_ref().map(|r| r.t)
    }
}

impl Prison {
    pub(crate) fn session_mut(&mut self) -> Option<&mut Session> {
        self.session.as_mut()
    }
}

pub(crate) struct PrisonPlugin;
impl Plugin for PrisonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Prison>()
            .add_plugins(MaterialPlugin::<visuals::PrisonField>::default())
            .add_systems(Startup, visuals::setup)
            .add_systems(Update, requests)
            .add_systems(OnEnter(GameState::Prison), enter)
            .add_systems(
                PostUpdate,
                update
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(GameState::Prison)),
            )
            .add_systems(OnExit(GameState::Prison), leave);
    }
}

fn requests(
    mut events: MessageReader<PowerActivated>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut prison: ResMut<Prison>,
) {
    // Repeat requests while locked up are dropped, not saved for later.
    if *state.get() == GameState::Prison
        && let Some(queue) = crate::plugins::external_api_plugin::get_command_queue()
        && let Ok(mut q) = queue.lock()
    {
        q.retain(|cmd| cmd.command_type != PowerCommandType::Prison);
    }
    for e in events.read() {
        if e.command.command_type == PowerCommandType::Prison
            && *state.get() == GameState::Playing
            && prison.session.is_none()
        {
            prison.request = Some(e.command.username.clone());
            next.set(GameState::Prison);
        }
    }
}

fn back_to_play(w: &mut World, why: &str) {
    if !why.is_empty() {
        let mut n = w.resource_mut::<super::world_portals::PowerNotice>();
        n.text = why.into();
        n.remaining = 3.;
    }
    w.resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
}

fn enter(w: &mut World) {
    let by = w
        .resource_mut::<Prison>()
        .request
        .take()
        .unwrap_or_else(|| "chat".into());
    let Some((hero, start, visibility)) = w
        .query_filtered::<(Entity, &Transform, &Visibility), With<Hero>>()
        .single(w)
        .ok()
        .map(|(e, t, v)| (e, *t, *v))
    else {
        return back_to_play(w, "");
    };
    // A rocket or teleport may have started in the same frame.
    if w.get::<super::launcher::LauncherPassenger>(hero).is_some()
        || w.resource::<super::biome_teleport::BiomeTeleport>().phase
            != super::biome_teleport::TeleportPhase::Idle
        || w.get::<Health>(hero).is_none_or(|hp| hp.current == 0)
    {
        return back_to_play(w, "Finish travelling before going to prison");
    }
    let Some((camera, camera_start, layers)) = w
        .query_filtered::<(Entity, &Transform, Option<&RenderLayers>), With<Camera3d>>()
        .single(w)
        .ok()
        .map(|(e, t, l)| (e, *t, l.cloned()))
    else {
        return back_to_play(w, "");
    };
    // Park the hero in the live world while the prison plays.
    w.entity_mut(hero).insert((
        Visibility::Hidden,
        crate::components::hero::Parked,
        RigidBody::Static,
        LinearVelocity::ZERO,
        ColliderDisabled,
    ));
    // The world is paused (like the pause menu); silence its music and sounds.
    let paused_audio: Vec<Entity> = w
        .query::<(Entity, &AudioSink)>()
        .iter(w)
        .filter(|(_, sink)| !sink.is_paused())
        .map(|(e, _)| e)
        .collect();
    for e in &paused_audio {
        if let Some(sink) = w.get::<AudioSink>(*e) {
            sink.pause();
        }
    }
    // Space: only the prison's own layer is drawn.
    w.entity_mut(camera).insert(RenderLayers::layer(1));
    let origin = start.translation + Vec3::new(0., 0., 4000.);
    let seed = (w.resource::<Time<Real>>().elapsed_secs() * 1000.) as u32 ^ 0x9e37_79b9;
    let villagers = 1 + (seed % 2) as usize;
    let mut cell = sim::Cell::new(seed, villagers);
    let limit = w
        .get_resource::<crate::systems::game_modes::GameModes>()
        .map_or(0, |m| m.prison_seconds);
    cell.limit = (limit > 0).then_some(limit as f32);
    visuals::scene(w, origin, hero);
    let mut n = w.resource_mut::<super::world_portals::PowerNotice>();
    n.text = format!("ARRESTED! (by {by}) Escape the space prison!");
    n.remaining = 4.;
    w.resource_mut::<Prison>().session = Some(Session {
        hero,
        start,
        visibility,
        camera,
        camera_start,
        layers,
        origin,
        cell,
        accumulator: 0.,
        elapsed: 0.,
        leaving: None,
        villagers,
        hidden_text: Vec::new(),
        ride: None,
        shake: 0.,
        magnet_was: None,
        look: origin,
        paused_audio,
    });
}

/// Camera distance that fits the spinning cell, the action up close.
fn camera_distance(w: &mut World) -> f32 {
    let aspect = w
        .query::<&Window>()
        .iter(w)
        .next()
        .map_or(1., |win| win.width() / win.height().max(1.));
    // Frame the cell (its corners as it spins) with a little room; the magnets
    // and station show around the edges. The camera also drifts to the hero.
    let reach = 640.;
    reach / ((std::f32::consts::FRAC_PI_8).tan() * aspect.min(1.))
}

fn update(w: &mut World) {
    let distance = camera_distance(w);
    let dt = w.resource::<Time<Real>>().delta_secs().min(0.1);
    let keys = w.resource::<ButtonInput<KeyCode>>();
    let give_up = keys.just_pressed(KeyCode::Escape);
    let key = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
    let steer = Vec2::new(
        key(KeyCode::KeyD, KeyCode::ArrowRight) as i8 as f32
            - key(KeyCode::KeyA, KeyCode::ArrowLeft) as i8 as f32,
        key(KeyCode::KeyW, KeyCode::ArrowUp) as i8 as f32
            - key(KeyCode::KeyS, KeyCode::ArrowDown) as i8 as f32,
    );
    let mut p = w.remove_resource::<Prison>().unwrap();
    let Some(s) = p.session.as_mut() else {
        w.insert_resource(p);
        return;
    };
    s.elapsed += dt;
    if give_up && s.cell.outcome.is_none() {
        s.cell.outcome = Some(sim::Outcome::Caught);
    }
    // After the outcome the cell keeps turning (the simulation only spins then).
    if s.elapsed > 0.6 {
        s.accumulator += dt;
        while s.accumulator >= sim::STEP {
            s.cell.tick(steer);
            s.accumulator -= sim::STEP;
        }
    }
    // World text is drawn by its own 2D pass, which the camera's layers do not
    // hide; keep it out of space (labels appearing mid-visit included).
    let shown: Vec<_> = w
        .query_filtered::<(Entity, &Visibility), With<Text2d>>()
        .iter(w)
        .filter(|(_, v)| **v != Visibility::Hidden)
        .map(|(e, v)| (e, *v))
        .collect();
    for (e, v) in shown {
        w.entity_mut(e).insert(Visibility::Hidden);
        if !s.hidden_text.iter().any(|(h, _)| *h == e) {
            s.hidden_text.push((e, v));
        }
    }
    let events = std::mem::take(&mut s.cell.events);
    for e in &events {
        match *e {
            sim::Event::HeroHit => {
                // A baton hit costs a heart, but the prison never kills.
                if let Some(mut hp) = w.get_mut::<Health>(s.hero) {
                    hp.current = hp.current.saturating_sub(1).max(1);
                }
            }
            sim::Event::GuardIn(_) => {
                let mut n = w.resource_mut::<super::world_portals::PowerNotice>();
                n.text = "GUARD INCOMING!".into();
                n.remaining = 1.5;
            }
            sim::Event::VillagerOut => {
                let mut n = w.resource_mut::<super::world_portals::PowerNotice>();
                n.text = "A villager escaped!".into();
                n.remaining = 2.;
            }
            _ => {}
        }
    }
    let magnet_was = s.magnet_was;
    s.magnet_was = s.cell.magnet;
    if events.iter().any(|e| matches!(e, sim::Event::Blast(_))) {
        s.shake = 0.35;
    }
    s.shake = (s.shake - dt).max(0.);
    // Out of the cell: the escape rocket swoops in and takes the hero to Earth.
    if matches!(s.cell.outcome, Some(sim::Outcome::Escaped { .. })) && s.ride.is_none() {
        let turn = Quat::from_rotation_z(s.cell.angle);
        let exit = s.origin + turn * s.cell.bodies[0].p.extend(30.);
        let out = (exit - s.origin).normalize_or(Vec3::Y);
        let earth = s.origin + visuals::EARTH_CENTER;
        let target = earth + (exit - earth).normalize() * visuals::EARTH_RADIUS * 1.02;
        let side = Vec3::new(-out.y, out.x, 0.);
        visuals::spawn_rocket(w, exit + side * 900., s.cell.saved);
        s.ride = Some(Ride {
            t: 0.,
            exit,
            side,
            out,
            target,
        });
    }
    let mut veil = Color::srgba(0., 0., 0., (1. - s.elapsed / 0.6).clamp(0., 1.));
    // Drift toward the hero so the action stays near the middle.
    if s.ride.is_none() && !s.cell.bodies[0].escaped {
        let hero = s.origin + Quat::from_rotation_z(s.cell.angle) * s.cell.bodies[0].p.extend(0.);
        let goal = s.origin.lerp(hero, 0.4);
        s.look = s.look.lerp(goal, 1. - (-3. * dt).exp());
    }
    let mut look = s.look;
    let mut back = distance;
    if let Some(r) = s.ride.as_mut() {
        r.t += dt;
        let (p, heading, flame, heat) = r.pose();
        visuals::pose_rocket(w, p, heading, flame, heat, r.t >= 0.9, dt);
        // Follow the rocket down, closing in as it dives.
        look = s.look.lerp(p, smooth(r.t / 1.2));
        back = distance + (950. - distance) * smooth((r.t - 0.9) / 1.6);
        // Re-entry: the screen flares orange-white before landing.
        let flare = ((r.t - (RIDE_END - 0.7)) / 0.6).clamp(0., 1.);
        veil = Color::srgba(1., 0.82, 0.55, flare);
        if r.t >= RIDE_END {
            w.resource_mut::<NextState<GameState>>()
                .set(GameState::Playing);
        }
    } else if s.cell.outcome.is_some() {
        // Caught (or gave up): fade to black.
        let t = s.leaving.get_or_insert(0.);
        *t += dt;
        veil = Color::srgba(0., 0., 0., (*t / 0.6).clamp(0., 1.));
        if *t >= 0.7 {
            w.resource_mut::<NextState<GameState>>()
                .set(GameState::Playing);
        }
    }
    // A slow sway sells the zero gravity; blasts shake the view.
    let sway = Vec3::new(
        (s.elapsed * 0.4).sin() * 40.,
        (s.elapsed * 0.3).cos() * 30.,
        0.,
    );
    let shake = Vec3::new((s.elapsed * 61.).sin(), (s.elapsed * 47.).cos(), 0.) * s.shake * 70.;
    let camera = Transform::from_translation(look + Vec3::Z * back + sway + shake)
        .looking_at(look + shake * 0.5, Vec3::Y);
    *w.get_mut::<Transform>(s.camera).unwrap() = if s.elapsed < 0.6 {
        Transform {
            translation: s
                .camera_start
                .translation
                .lerp(camera.translation, s.elapsed / 0.6),
            ..camera
        }
    } else {
        camera
    };
    visuals::animate(
        w,
        s.origin,
        &s.cell,
        &events,
        s.villagers,
        veil,
        s.ride.as_ref().map(|r| r.t),
        magnet_was,
    );
    w.insert_resource(p);
}

fn leave(w: &mut World) {
    let mut p = w.remove_resource::<Prison>().unwrap();
    if let Some(s) = p.session.take() {
        if let Ok(mut hero) = w.get_entity_mut(s.hero) {
            hero.remove::<(crate::components::hero::Parked, ColliderDisabled)>();
            hero.insert((
                RigidBody::Dynamic,
                LinearVelocity::ZERO,
                s.visibility,
                s.start,
                Position(s.start.translation),
            ));
        }
        for e in &s.paused_audio {
            if let Some(sink) = w.get::<AudioSink>(*e) {
                sink.play();
            }
        }
        for (e, v) in s.hidden_text {
            if let Ok(mut label) = w.get_entity_mut(e) {
                label.insert(v);
            }
        }
        if let Ok(mut camera) = w.get_entity_mut(s.camera) {
            camera.insert(s.camera_start);
            if let Some(layers) = s.layers {
                camera.insert(layers);
            } else {
                camera.remove::<RenderLayers>();
            }
        }
        let text = match s.cell.outcome {
            Some(sim::Outcome::Escaped { seconds, saved }) => {
                let amount = 100 + ((sim::QUICK_ESCAPE - seconds).max(0.) * 2.) as u32 + saved * 40;
                if let Some(mut hp) = w.get_mut::<Health>(s.hero) {
                    hp.current = (hp.current + 1).min(hp.max);
                }
                if let Some(mut xp) = w.get_mut::<crate::components::experience::Experience>(s.hero)
                {
                    xp.add_xp(amount);
                }
                // The villagers who got out land beside the hero.
                super::launcher::drop_villagers(
                    w,
                    s.start.translation,
                    saved,
                    s.cell.time.to_bits(),
                );
                format!(
                    "BACK ON EARTH! Escaped in {seconds:.0}s: +{amount} XP, +1 heart, {saved}/{} villagers saved",
                    s.villagers
                )
            }
            _ => "Caught! The guards threw you back out.".into(),
        };
        let mut n = w.resource_mut::<super::world_portals::PowerNotice>();
        n.text = text;
        n.remaining = 5.;
    }
    let entities: Vec<_> = w.query_filtered::<Entity, With<Scene>>().iter(w).collect();
    for e in entities {
        if w.get_entity(e).is_ok() {
            w.despawn(e);
        }
    }
    w.insert_resource(p);
}
