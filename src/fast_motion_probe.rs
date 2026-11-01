//! `--visual-smoke-test --fast-motion-test`: turns on the player's DLSS and
//! frame generation settings, makes the hero dash left and right fast and
//! saves `target/visual-fast-<n>.png` mid-dash, plus `target/fast-crops.png`
//! is built from them by the caller. Used to see smearing on fast heads.
use crate::components::hero::Hero;
use crate::resources::graphics_settings::GraphicsSettings;
use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;

#[derive(Resource)]
struct Probe {
    started: f32,
    shots: u32,
}

pub fn run(w: &mut World) {
    let now = w.resource::<Time<Real>>().elapsed_secs();
    assert!(now < 60.0, "Fast motion test timed out");
    if !w.contains_resource::<Probe>() {
        if now < 8.0 {
            return;
        }
        let mut g = w.resource_mut::<GraphicsSettings>();
        g.dlss = std::env::var("FAST_DLSS").map_or(true, |v| v != "0");
        g.frame_generation = true;
        g.anti_aliasing = false;
        w.insert_resource(Probe {
            started: now,
            shots: 0,
        });
        return;
    }
    let (started, shots) = {
        let p = w.resource::<Probe>();
        (p.started, p.shots)
    };
    let t = now - started;
    // Dash: alternate direction every 0.6 s at high speed, keep it airborne.
    if let Some(mut v) = w
        .query_filtered::<&mut LinearVelocity, With<Hero>>()
        .iter_mut(w)
        .next()
    {
        let side = if (t / 0.6) as i32 % 2 == 0 { 1.0 } else { -1.0 };
        v.x = side * 1100.0;
        if v.y < 0.0 {
            v.y = 120.0;
        }
    }
    // Shots mid-dash once the renderer has settled (after 3 s).
    if t > 3.0 + shots as f32 * 0.31 && shots < 6 {
        // Where the hero is on screen, for cropping the shot.
        let hero = w
            .query_filtered::<&GlobalTransform, With<Hero>>()
            .iter(w)
            .next()
            .map(|t| t.translation());
        let camera = w
            .query::<(&Camera, &GlobalTransform)>()
            .iter(w)
            .find(|(c, _)| c.is_active)
            .map(|(c, t)| (c.clone(), *t));
        if let (Some(hero), Some((camera, ct))) = (hero, camera)
            && let Ok(p) = camera.world_to_viewport(&ct, hero)
        {
            eprintln!("[FAST] shot {shots} hero_screen {:.0} {:.0}", p.x, p.y);
        }
        let path = format!("target/visual-fast-{shots}.png");
        w.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(path));
        w.resource_mut::<Probe>().shots += 1;
    }
    if shots >= 6 && t > 6.0 {
        let hero = w
            .query_filtered::<&Transform, With<Hero>>()
            .iter(w)
            .next()
            .map(|t| t.translation);
        eprintln!("[FAST] captured 6 shots; hero at {hero:?}");
        w.write_message(AppExit::Success);
    }
}
