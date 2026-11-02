//! `--visual-smoke-test --night-test`: with Graphics > Lighting off, jumps to
//! midnight and checks the whole world dims evenly: every material made lit is
//! now flat (none left to turn black) and the night shade is up. Saves
//! `target/visual-night-day.png` and `target/visual-night-night.png`.
use crate::resources::day_night_cycle::DayNightCycle;
use crate::resources::graphics_settings::GraphicsSettings;
use crate::systems::graphics::NightShade;
use bevy::prelude::*;

#[derive(Resource)]
struct Probe {
    step: u8,
    since: f32,
}

fn capture(w: &mut World, name: &str) {
    w.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(format!(
            "target/visual-night-{name}.png"
        )));
}

fn set_time(w: &mut World, fraction: f32) {
    let mut cycle = w.resource_mut::<DayNightCycle>();
    cycle.current_time = cycle.cycle_duration * fraction;
}

pub fn run(w: &mut World) {
    let now = w.resource::<Time<Real>>().elapsed_secs();
    assert!(now < 60.0, "Night test timed out");
    if !w.contains_resource::<Probe>() {
        if now < 8.0 {
            return;
        }
        let mut g = w.resource_mut::<GraphicsSettings>();
        g.lighting = false;
        g.weather_lighting = true;
        set_time(w, 0.25);
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
        0 if now - since > 3.0 => {
            set_time(w, 0.25);
            capture(w, "day");
            next(w);
        }
        1 if now - since > 0.5 => {
            set_time(w, 0.75);
            next(w);
        }
        2 => {
            // Hold midnight while the shade fades in.
            set_time(w, 0.75);
            if now - since < 5.0 {
                return;
            }
            let alpha = w
                .query_filtered::<&BackgroundColor, With<NightShade>>()
                .single(w)
                .expect("night shade")
                .0
                .alpha();
            assert!(alpha > 0.3, "night shade only at {alpha}");
            let lit = w
                .resource::<Assets<StandardMaterial>>()
                .iter()
                .filter(|(_, m)| !m.unlit)
                .count();
            assert_eq!(lit, 0, "{lit} materials still lit with Lighting off");
            capture(w, "night");
            eprintln!("[NIGHT] shade {alpha:.2}, no lit materials left");
            next(w);
        }
        // With Lighting on the effects that need it must change the picture:
        // shots with shadows, AO and bloom on, then off (compared by the caller).
        3 if now - since > 1.0 => {
            set_time(w, 0.25);
            w.resource_mut::<GraphicsSettings>().lighting = true;
            next(w);
        }
        4 if now - since > 4.0 => {
            set_time(w, 0.25);
            capture(w, "lit-effects-on");
            let mut g = w.resource_mut::<GraphicsSettings>();
            g.shadows = false;
            g.ambient_occlusion = false;
            g.bloom = false;
            next(w);
        }
        5 if now - since > 3.0 => {
            set_time(w, 0.25);
            let lit = w
                .resource::<Assets<StandardMaterial>>()
                .iter()
                .filter(|(_, m)| !m.unlit)
                .count();
            assert!(lit > 0, "Lighting on left every material flat");
            capture(w, "lit-effects-off");
            eprintln!("[NIGHT] lighting on restored {lit} lit materials");
            next(w);
        }
        6 if now - since > 1.0 => {
            eprintln!("[NIGHT] passed");
            w.write_message(AppExit::Success);
        }
        _ => {}
    }
}
