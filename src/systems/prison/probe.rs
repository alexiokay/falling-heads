//! `--visual-smoke-test --prison-test`: sends PRISON, checks the cell is up and
//! spinning, flies the hero into the bottom button and out through the top
//! trapdoor it opens, then checks the escape reward and the rocket home. A
//! second visit is given up with Esc and must end as caught. Saves
//! `target/visual-prison-<name>.png`.
use super::RIDE_END;
use super::*;
use crate::resources::command_queue::PowerCommand;

#[derive(Resource)]
struct Probe {
    step: u8,
    since: f32,
}

fn capture(w: &mut World, name: &str) {
    w.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(format!(
            "target/visual-prison-{name}.png"
        )));
}

fn send(cmd: PowerCommandType) {
    if let Some(queue) = crate::plugins::external_api_plugin::get_command_queue() {
        queue
            .lock()
            .unwrap()
            .push_back(PowerCommand::new(cmd, "probe".into()));
    }
}

fn state(w: &World) -> GameState {
    w.resource::<State<GameState>>().get().clone()
}

fn notice(w: &World) -> String {
    w.resource::<super::super::world_portals::PowerNotice>()
        .text
        .clone()
}

fn cell(w: &mut World) -> Option<&mut sim::Cell> {
    w.resource_mut::<Prison>()
        .into_inner()
        .session_mut()
        .map(|s| &mut s.cell)
}

pub(crate) fn run(w: &mut World) {
    let now = w.resource::<Time<Real>>().elapsed_secs();
    assert!(now < 90., "Prison test timed out");
    if !w.contains_resource::<Probe>() {
        let blocks = w
            .query_filtered::<(), With<crate::components::block::Block>>()
            .iter(w)
            .count();
        if now < 8. || blocks < 100 {
            return;
        }
        send(PowerCommandType::Prison);
        w.insert_resource(Probe {
            step: 0,
            since: now,
        });
        return;
    }
    let Probe { step, since } = *w.resource::<Probe>();
    let next = |w: &mut World| {
        let mut p = w.resource_mut::<Probe>();
        p.step += 1;
        p.since = now;
    };
    match step {
        0 => {
            if state(w) == GameState::Prison && now - since > 3. {
                let (angle, bodies) = {
                    let c = cell(w).expect("prison session");
                    (c.angle, c.bodies.len())
                };
                assert!(angle > 0.3, "cell is not spinning ({angle})");
                assert!(bodies >= 2, "no villager locked up with the hero");
                let labels = w
                    .query::<(&Text2d, &Visibility)>()
                    .iter(w)
                    .filter(|(_, v)| **v != Visibility::Hidden)
                    .count();
                assert_eq!(labels, 0, "world labels showing in the prison");
                capture(w, "cell");
                next(w);
            }
            assert!(now - since < 10., "PRISON did not lock the hero up");
        }
        // Into the bottom button: the top trapdoor opens.
        1 if now - since > 1. => {
            let c = cell(w).unwrap();
            c.bodies[0].p = Vec2::new(0., -sim::HALF + sim::RADIUS + 6.);
            c.bodies[0].v = Vec2::new(0., -500.);
            next(w);
        }
        2 if now - since > 0.1 => {
            let c = cell(w).unwrap();
            assert!(
                c.doors[0].open > 0.,
                "the button did not open the top trapdoor"
            );
            // Out through it before it shuts.
            c.bodies[0].p = Vec2::new(0., sim::HALF - 50.);
            c.bodies[0].v = Vec2::new(0., 1400.);
            capture(w, "trapdoor-open");
            next(w);
        }
        // The escape rocket: pick-up at the trapdoor, then re-entry over Earth.
        3 => {
            let ride = w
                .resource_mut::<Prison>()
                .into_inner()
                .session_mut()
                .and_then(|s| s.ride_time());
            if ride.is_some_and(|t| t > 0.6) {
                capture(w, "rocket-pickup");
                next(w);
            }
            assert!(now - since < 3., "no escape rocket came");
        }
        4 => {
            let ride = w
                .resource_mut::<Prison>()
                .into_inner()
                .session_mut()
                .and_then(|s| s.ride_time());
            if ride.is_some_and(|t| t > RIDE_END - 1.) {
                capture(w, "reentry");
                next(w);
            }
            assert!(now - since < 6., "the rocket never reached Earth");
        }
        // Landed home, in the world, with the reward.
        5 => {
            if state(w) == GameState::Playing {
                let text = notice(w);
                assert!(text.contains("BACK ON EARTH"), "no landing notice ({text})");
                let hero = w
                    .query_filtered::<&Visibility, With<crate::components::hero::Hero>>()
                    .single(w)
                    .copied()
                    .unwrap();
                assert!(hero != Visibility::Hidden, "the hero did not come back");
                eprintln!("[PRISON] {text}");
                next(w);
            }
            assert!(now - since < 4., "the ride did not end the visit");
        }
        // Second visit, given up with Esc.
        6 if now - since > 2. => {
            send(PowerCommandType::Prison);
            next(w);
        }
        7 => {
            // No time limit: hold the hero in the middle while waiting for a guard,
            // so blasts and buttons cannot let it escape on its own.
            if state(w) == GameState::Prison
                && let Some(c) = cell(w)
            {
                c.bodies[0].p = Vec2::ZERO;
                c.bodies[0].v = Vec2::ZERO;
            }
            if state(w) == GameState::Prison && now - since > 9. {
                let guards = cell(w).map_or(0, |c| c.guards());
                assert!(guards >= 1, "no guard came in");
                capture(w, "guards");
                w.resource_mut::<ButtonInput<KeyCode>>()
                    .press(KeyCode::Escape);
                // The real Escape path, before InputPlugin clears the injected edge.
                super::update(w);
                w.resource_mut::<ButtonInput<KeyCode>>()
                    .release(KeyCode::Escape);
                next(w);
            }
            assert!(now - since < 30., "second PRISON never started");
        }
        8 => {
            if state(w) == GameState::Playing {
                let text = notice(w);
                assert!(text.contains("Caught"), "giving up was not caught ({text})");
                eprintln!("[PRISON] gave up: {text}");
                eprintln!("[PRISON] passed");
                w.write_message(AppExit::Success);
            }
            assert!(now - since < 5., "Esc did not end the visit");
        }
        _ => {}
    }
}
