//! Natural terrain, away from the origin, using real streaming and teleportation.
use crate::{
    components::{block::BiomeType, hero::Hero},
    resources::graphics_settings::GraphicsSettings,
    systems::biome_teleport::{BiomeTeleport, TeleportPhase},
};
use avian3d::prelude::*;
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
#[derive(Resource, Default)]
struct Probe {
    step: usize,
    since: f32,
    anchor: Option<Vec3>,
}
const BIOMES: [BiomeType; 3] = [BiomeType::Plains, BiomeType::Underground, BiomeType::Forest];
pub fn run(w: &mut World) {
    if !w.contains_resource::<Probe>() {
        w.init_resource::<Probe>();
    }
    let now = w.resource::<Time<Real>>().elapsed_secs();
    let step = w.resource::<Probe>().step;
    let since = w.resource::<Probe>().since;
    assert!(now < 100., "Ecology probe timed out");
    if let Some(p) = w.resource::<Probe>().anchor {
        for (mut pos, mut v) in w
            .query_filtered::<(&mut Position, &mut LinearVelocity), With<Hero>>()
            .iter_mut(w)
        {
            pos.0 = p;
            v.0 = Vec3::ZERO;
        }
    }
    if let Some(mut events) =
        w.get_resource_mut::<crate::systems::random_events::RandomEventTimer>()
    {
        events.timer.reset();
    }
    let mut weather = w.resource_mut::<crate::components::weather::ActiveWeather>();
    weather.weather_type = crate::components::weather::WeatherType::Clear;
    weather.intensity = 0.;
    weather.duration_remaining = 999.;
    if step == 0 && now < 8. {
        return;
    }
    let stage = step % 3;
    let biome = step / 3;
    if biome >= 3 {
        if now > since + 2. {
            eprintln!(
                "[ECOLOGY TEST] completed: natural Plains, Underground, Forest away from world origin"
            );
            w.write_message(AppExit::Success);
        }
        return;
    }
    if stage == 0 {
        w.query::<&mut Window>()
            .single_mut(w)
            .unwrap()
            .resolution
            .set(1360., 900.);
        let mut g = w.resource_mut::<GraphicsSettings>();
        g.frame_generation = false;
        g.ray_tracing = false;
        g.dlss = false;
        g.fog = true;
        w.resource_mut::<Probe>().anchor = None;
        let y = crate::utils::biome_utils::nearest_biome_y(0., BIOMES[biome]);
        w.resource_mut::<BiomeTeleport>().target = Some(Vec3::new(4096., y, 90.));
    } else if stage == 1 && now > since + 9. {
        assert!(w.resource::<BiomeTeleport>().phase == TeleportPhase::Idle);
        let p = w
            .query_filtered::<&Transform, With<Hero>>()
            .single(w)
            .unwrap()
            .translation;
        assert_eq!(
            crate::utils::biome_utils::determine_biome_at_position(p.x, p.y),
            BIOMES[biome]
        );
        w.resource_mut::<Probe>().anchor = Some(p);
    } else if stage == 2 && now > since + 4. {
        w.spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!(
                "target/visual-ecology-{:?}.png",
                BIOMES[biome]
            )));
    } else {
        return;
    }
    let mut p = w.resource_mut::<Probe>();
    p.step += 1;
    p.since = now;
}
