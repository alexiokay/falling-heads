//! `--visual-smoke-test --powers-test`: fires each new chat power in turn,
//! checks it had its effect and saves `target/visual-power-<name>.png`.
use crate::components::hero::Hero;
use crate::resources::command_queue::{PowerCommand, PowerCommandType as P};
use avian3d::prelude::*;
use bevy::prelude::*;

const POWERS: [(P, &str); 12] = [
    (P::Feather, "feather"),
    (P::Anvil, "anvil"),
    (P::Golem, "golem"),
    (P::Pearl, "pearl"),
    (P::Quake, "quake"),
    (P::Shrink, "shrink"),
    (P::Wind, "wind"),
    (P::BlackHole, "blackhole"),
    (P::IceAge, "iceage"),
    (P::Bees, "bees"),
    (P::Bolt, "bolt"),
    (P::Fuss, "fuss"),
];
/// Seconds after firing before the effect is checked and captured.
const CHECK_AFTER: f32 = 0.7;
const NEXT_AFTER: f32 = 2.2;

#[derive(Resource, Default)]
struct Probe {
    index: usize,
    fired_at: Option<f32>,
    checked: bool,
    hero_start: Vec3,
    started: bool,
    cooldown_at: Option<f32>,
    second_bolt: bool,
}

fn capture(w: &mut World, name: &str) {
    w.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(format!(
            "target/visual-power-{name}.png"
        )));
}

fn hero(w: &mut World) -> Option<(Entity, Vec3, f32)> {
    w.query_filtered::<(Entity, &Position, &Transform), With<Hero>>()
        .iter(w)
        .next()
        .map(|(e, p, t)| (e, p.0, t.scale.x))
}

fn notice(w: &World) -> String {
    w.resource::<crate::systems::world_portals::PowerNotice>()
        .text
        .clone()
}

/// Checks that a power did what it should, shortly after firing it.
fn verify(w: &mut World, power: P, start: Vec3) {
    let (hero_entity, hero_at, scale) = hero(w).expect("hero");
    let counts = crate::systems::extra_powers::power_counts(w);
    let text = notice(w);
    match power {
        P::Feather => assert!(
            w.get::<GravityScale>(hero_entity)
                .is_some_and(|g| g.0 < 1.0),
            "FEATHER: hero gravity not lowered"
        ),
        P::Anvil => assert!(counts.anvils >= 1, "ANVIL: no anvils"),
        P::Golem => assert!(counts.golems >= 1, "GOLEM: no golem"),
        P::Pearl => assert!(
            hero_at.xy().distance(start.xy()) > 150.0 || text.contains("no free spot"),
            "PEARL: hero did not move ({start} -> {hero_at})"
        ),
        P::Quake => assert!(text.contains("EARTHQUAKE"), "QUAKE: no notice ({text})"),
        P::Shrink => assert!(scale < 0.75, "SHRINK: hero scale {scale}"),
        P::Wind => assert!(text.contains("WIND"), "WIND: no notice ({text})"),
        P::BlackHole => assert!(counts.holes >= 1, "BLACKHOLE: no hole"),
        P::IceAge => assert!(
            w.get::<crate::components::surface_grip::SurfaceGrip>(hero_entity)
                .is_some(),
            "ICEAGE: hero not on ice"
        ),
        P::Bees => assert_eq!(counts.bees, 6, "BEES: swarm size"),
        P::Bolt => assert!(
            text.contains("LIGHTNING") || text.contains("no enemy"),
            "BOLT: nothing happened ({text})"
        ),
        P::Fuss => {
            let now = w.resource::<Time>().elapsed_secs();
            assert!(
                w.resource::<crate::systems::extra_powers::Fuss>()
                    .steer_sign(now)
                    < 0.0,
                "FUSS: controls not mirrored"
            );
        }
        _ => {}
    }
    eprintln!("[POWERS] {power:?} ok (fx={})", counts.fx);
}

pub fn run(w: &mut World) {
    if !w.contains_resource::<Probe>() {
        w.init_resource::<Probe>();
    }
    let now = w.resource::<Time<Real>>().elapsed_secs();
    assert!(now < 120.0, "Powers test timed out");
    let blocks = w
        .query_filtered::<(), With<crate::components::block::Block>>()
        .iter(w)
        .count();
    if !w.resource::<Probe>().started {
        if now < 8.0 || blocks < 100 || hero(w).is_none() {
            return;
        }
        w.resource_mut::<Probe>().started = true;
    }
    let probe = w.resource::<Probe>();
    let (index, fired_at, checked) = (probe.index, probe.fired_at, probe.checked);
    if index >= POWERS.len() {
        // Settings > Cooldowns: with BOLT at 30 s, a second BOLT straight after
        // the first is refused with a notice.
        let Some(at) = w.resource::<Probe>().cooldown_at else {
            w.resource_mut::<crate::systems::power_cooldowns::PowerCooldowns>()
                .set(P::Bolt, 30);
            if let Some(queue) = crate::plugins::external_api_plugin::get_command_queue() {
                queue
                    .lock()
                    .unwrap()
                    .push_back(PowerCommand::new(P::Bolt, "probe".into()));
            }
            w.resource_mut::<Probe>().cooldown_at = Some(now);
            return;
        };
        // The second BOLT goes in after the first one's own notice.
        if !w.resource::<Probe>().second_bolt {
            if now - at > 0.5 {
                if let Some(queue) = crate::plugins::external_api_plugin::get_command_queue() {
                    queue
                        .lock()
                        .unwrap()
                        .push_back(PowerCommand::new(P::Bolt, "probe".into()));
                }
                w.resource_mut::<Probe>().second_bolt = true;
            }
            return;
        }
        if now - at < 0.7 {
            return;
        }
        let text = notice(w);
        assert!(
            text.contains("Bolt recharging"),
            "COOLDOWN: no notice ({text})"
        );
        w.resource_mut::<crate::systems::power_cooldowns::PowerCooldowns>()
            .set(P::Bolt, 0);
        eprintln!("[POWERS] cooldown ok ({text})");
        eprintln!("[POWERS] all {} powers passed", POWERS.len());
        w.write_message(AppExit::Success);
        return;
    }
    let (power, name) = POWERS[index];
    match fired_at {
        None => {
            let start = hero(w).map_or(Vec3::ZERO, |h| h.1);
            if let Some(queue) = crate::plugins::external_api_plugin::get_command_queue() {
                queue
                    .lock()
                    .unwrap()
                    .push_back(PowerCommand::new(power, "probe".into()));
            }
            let mut p = w.resource_mut::<Probe>();
            p.hero_start = start;
            p.fired_at = Some(now);
            p.checked = false;
        }
        Some(at) if !checked && now - at > CHECK_AFTER => {
            let start = w.resource::<Probe>().hero_start;
            verify(w, power, start);
            capture(w, name);
            w.resource_mut::<Probe>().checked = true;
        }
        Some(at) if checked && now - at > NEXT_AFTER => {
            let mut p = w.resource_mut::<Probe>();
            p.index += 1;
            p.fired_at = None;
        }
        _ => {}
    }
}
