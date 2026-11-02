//! Space prison event (`!PRISON`): the hero and one or two villagers are locked
//! in a spinning bedrock cell in zero gravity. Micro-blasts throw everyone
//! around, magnets pull heads to the walls, a button opens the trapdoor across
//! the room for a moment, and guards come in through the trapdoors. Escape and a
//! rocket flies you (and any villager who got out) back home; fail and the
//! guards throw you out after 75 s. Like the Slime Rift, the world keeps running
//! behind it with the hero parked.
use crate::{
    components::{enemy::Health, hero::Hero},
    events::power_events::PowerActivated,
    resources::command_queue::{PowerCommand, PowerCommandType},
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
    by: String,
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
    // Park the hero in the live world, like the Slime Rift does.
    w.entity_mut(hero).insert((
        Visibility::Hidden,
        super::slime_rift::RiftParked,
        RigidBody::Static,
        LinearVelocity::ZERO,
        ColliderDisabled,
    ));
    // Space: only the prison's own layer is drawn.
    w.entity_mut(camera).insert(RenderLayers::layer(1));
    let origin = start.translation + Vec3::new(0., 0., 4000.);
    let seed = (w.resource::<Time<Real>>().elapsed_secs() * 1000.) as u32 ^ 0x9e37_79b9;
    let villagers = 1 + (seed % 2) as usize;
    let cell = sim::Cell::new(seed, villagers);
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
        by,
    });
}

/// Camera distance that fits the spinning cell (its corners included).
fn camera_distance(w: &mut World) -> f32 {
    let aspect = w
        .query::<&Window>()
        .iter(w)
        .next()
        .map_or(1., |win| win.width() / win.height().max(1.));
    let reach = (sim::HALF + sim::WALL) * std::f32::consts::SQRT_2 + 40.;
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
    if s.elapsed > 0.6 && s.cell.outcome.is_none() {
        s.accumulator += dt;
        while s.accumulator >= sim::STEP {
            s.cell.tick(steer);
            s.accumulator -= sim::STEP;
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
    let mut veil = (1. - s.elapsed / 0.6).clamp(0., 1.);
    if s.cell.outcome.is_some() {
        let t = s.leaving.get_or_insert(0.);
        *t += dt;
        veil = veil.max((*t / 0.6).clamp(0., 1.));
        if *t >= 0.7 {
            w.resource_mut::<NextState<GameState>>()
                .set(GameState::Playing);
        }
    }
    // A slow camera sway sells the zero gravity; the cell turns on its own.
    let sway = Vec3::new(
        (s.elapsed * 0.4).sin() * 40.,
        (s.elapsed * 0.3).cos() * 30.,
        0.,
    );
    let camera = Transform::from_translation(s.origin + Vec3::Z * distance + sway)
        .looking_at(s.origin, Vec3::Y);
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
    visuals::animate(w, s.origin, &s.cell, &events, s.villagers, veil);
    w.insert_resource(p);
}

fn leave(w: &mut World) {
    let mut p = w.remove_resource::<Prison>().unwrap();
    if let Some(s) = p.session.take() {
        if let Ok(mut hero) = w.get_entity_mut(s.hero) {
            hero.remove::<(super::slime_rift::RiftParked, ColliderDisabled)>();
            hero.insert((
                RigidBody::Dynamic,
                LinearVelocity::ZERO,
                s.visibility,
                s.start,
                Position(s.start.translation),
            ));
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
                let amount = 100 + ((sim::TIME_LIMIT - seconds).max(0.) * 2.) as u32 + saved * 40;
                if let Some(mut hp) = w.get_mut::<Health>(s.hero) {
                    hp.current = (hp.current + 1).min(hp.max);
                }
                if let Some(mut xp) = w.get_mut::<crate::components::experience::Experience>(s.hero)
                {
                    xp.add_xp(amount);
                }
                // Home by rocket, with every villager who got out.
                let mut command = PowerCommand::new(PowerCommandType::Launcher, s.by.clone());
                command.launcher.villagers = saved as u8;
                w.write_message(PowerActivated { command });
                format!(
                    "ESCAPED in {seconds:.0}s! +{amount} XP, +1 heart, {saved}/{} villagers saved",
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
