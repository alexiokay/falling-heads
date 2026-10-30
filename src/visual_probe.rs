//! Opt-in hidden scene comparisons and clear/snow performance capture.
use crate::{
    components::weather::{ActiveWeather, WeatherType},
    resources::{graphics_settings::GraphicsSettings, settings::GameSettings},
};
use bevy::{
    diagnostic::DiagnosticsStore,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
pub struct VisualProbePlugin;
impl Plugin for VisualProbePlugin {
    fn build(&self, app: &mut App) {
        if std::env::args().any(|a| a == "--depth-benchmark") {
            app.insert_resource(crate::resources::world_seed::WorldSeed::new(921));
        }
        app.world_mut()
            .resource_mut::<GraphicsSettings>()
            .frame_generation = false;
        if std::env::args().any(|a| {
            a == "--crowd-test"
                || a == "--ecology-test"
                || a == "--forest-trees-test"
                || a == "--forest-weather-rt-test"
                || a == "--torch-view-test"
        }) {
            app.insert_resource(crate::resources::world_seed::WorldSeed::new(921));
        }
        if std::env::args().any(|a| a == "--rt-flicker-test") {
            app.insert_resource(crate::resources::world_seed::WorldSeed::new(921));
            app.init_resource::<FlickerProbe>().add_systems(
                PostUpdate,
                flicker_probe.before(TransformSystems::Propagate),
            );
        }
        app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
            // Fixtures remove actors and terrain. Run after gameplay's deferred
            // commands have applied so a queued visual cannot target a removed actor.
            .init_resource::<Probe>()
            .add_systems(Last, probe);
    }
}
#[derive(Resource, Default)]
struct Probe {
    step: u8,
    night: bool,
    turn_started: bool,
    captures: u32,
    clear: Vec<f32>,
    snow: Vec<f32>,
    anchor: Option<Vec3>,
}
fn capture(world: &mut World, name: &str) {
    world
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(format!("target/visual-{name}.png")));
}
fn report(label: &str, values: &mut [f32]) {
    values.sort_by(f32::total_cmp);
    if !values.is_empty() {
        eprintln!(
            "[VISUAL PROFILE] {label} frames={} median_ms={:.3} p95_ms={:.3}",
            values.len(),
            values[values.len() / 2],
            values[values.len() * 95 / 100]
        );
    }
}
fn probe(world: &mut World) {
    if std::env::args().any(|a| a == "--depth-benchmark") {
        crate::depth_probe::run(world);
        return;
    }
    if std::env::args().any(|a| a == "--mechanism-test") {
        crate::systems::biome_encounters::mechanism_probe(world);
        return;
    }
    if std::env::args().any(|a| a == "--rift-test") {
        crate::systems::slime_rift::probe::run(world);
        return;
    }
    if std::env::args().any(|a| a == "--powers-test") {
        crate::powers_probe::run(world);
        return;
    }
    if std::env::args().any(|a| a == "--content-test") {
        crate::systems::biome_content::probe::run(world);
        return;
    }
    if std::env::args().any(|a| a == "--snowghast-test") {
        crate::snow_ghast_probe::run(world);
        return;
    }
    if std::env::args().any(|a| a == "--water-test") {
        crate::water_probe::run(world);
        return;
    }
    if std::env::args().any(|a| a == "--headsize-test") {
        crate::head_size_probe::run(world);
        return;
    }
    if std::env::args().any(|a| a == "--ecology-test") {
        crate::ecology_probe::run(world);
        return;
    }
    if std::env::args().any(|a| a == "--crowd-test") {
        crate::crowd_probe::run(world);
        return;
    }
    if std::env::args().any(|a| a == "--polish-test") {
        crate::polish_probe::run(world);
        return;
    }
    if std::env::args().any(|a| a == "--style-test") {
        crate::systems::biome_encounters::style_probe(world);
        return;
    }
    if std::env::args().any(|a| a == "--refinement-test") {
        crate::refinement_probe::run(world);
        return;
    }
    if std::env::args().any(|a| a == "--overhaul-test") {
        crate::overhaul_probe::advance_gallery(world);
        crate::overhaul_probe::run(world);
        return;
    }
    if std::env::args().any(|a| a == "--launcher-test") {
        crate::systems::launcher::smoke_probe(world);
        return;
    }
    if std::env::args().any(|a| a == "--torch-view-test") {
        torch_view_probe(world);
        return;
    }
    if std::env::args().any(|a| a == "--rt-flicker-test") {
        return;
    }
    if std::env::args().any(|a| a == "--forest-weather-rt-test") {
        forest_weather_rt_probe(world);
        return;
    }
    if std::env::args()
        .any(|a| a == "--crowd-test" || a == "--ecology-test" || a == "--forest-trees-test")
    {
        tree_probe(world);
        return;
    }
    if std::env::args().any(|a| a == "--biome-atmosphere-test") {
        biome_probe(world);
        return;
    }
    if std::env::args().any(|a| a == "--fire-impact-test") {
        fire_probe(world);
        return;
    }
    let t = world.resource::<Time<Real>>().elapsed_secs();
    let dt = world.resource::<Time<Real>>().delta_secs() * 1000.;
    let shadow_check = std::env::args().any(|a| a == "--shadow-check");
    if !shadow_check && (71. ..74.).contains(&t) {
        crate::systems::dragon_flyover::smoke_verify_turn_depth(world);
    }
    if shadow_check && t > 27. {
        eprintln!("[VISUAL SHADOW] completed");
        world.write_message(AppExit::Success);
        return;
    }
    if (30. ..38.).contains(&t) {
        world.resource_mut::<Probe>().clear.push(dt);
    }
    if (52. ..62.).contains(&t) {
        world.resource_mut::<Probe>().snow.push(dt);
    }
    for (i, (at, name)) in [
        (9.3, "steam"),
        (13., "sr"),
        (17., "no-ao"),
        (21., "no-shadows"),
        (25., "native"),
        (61., "snow"),
        (71.2, "dragon-turn-quarter"),
        (71.4, "dragon-turn-middle"),
        (72., "dragon-turn-right"),
    ]
    .iter()
    .enumerate()
    {
        let mask = 1 << i;
        if t > *at && world.resource::<Probe>().captures & mask == 0 {
            capture(world, name);
            world.resource_mut::<Probe>().captures |= mask;
        }
    }
    if t > 11. && !world.resource::<Probe>().night {
        let mut cycle = world.resource_mut::<crate::resources::day_night_cycle::DayNightCycle>();
        cycle.current_time = cycle.cycle_duration * 0.75;
        world.resource_mut::<Probe>().night = true;
    }
    if !shadow_check && t > 71. && !world.resource::<Probe>().turn_started {
        crate::systems::dragon_flyover::smoke_turn(world);
        world.resource_mut::<Probe>().turn_started = true;
    }
    let step = world.resource::<Probe>().step;
    match step {
        0 if t > 8. => {
            crate::smoke_test::liquid_gallery(world);
            crate::systems::graphics::smoke_torches(world);
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.frame_generation = false;
            g.ray_tracing = false;
            g.dlss = true;
            g.shadows = true;
            let mut settings = world.resource_mut::<GameSettings>();
            settings.fps_limit = 0;
            settings.vsync_enabled = false;
            let mut w = world.resource_mut::<ActiveWeather>();
            w.weather_type = WeatherType::Clear;
            w.duration_remaining = 999.;
            w.intensity = 0.;
            let mut cycle =
                world.resource_mut::<crate::resources::day_night_cycle::DayNightCycle>();
            cycle.current_time = cycle.cycle_duration * 0.25;
        }
        1 if t > 14. => {
            let mut g = world.resource_mut::<GraphicsSettings>();
            if shadow_check {
                g.shadows = false;
            } else {
                g.ambient_occlusion = false;
            }
        }
        2 if t > 18. => {
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.ambient_occlusion = true;
            g.shadows = shadow_check;
        }
        3 if t > 22. && !shadow_check => {
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.shadows = true;
            g.dlss = false;
        }
        4 if t > 26. => {
            world.resource_mut::<GraphicsSettings>().dlss = true;
        }
        5 if t > 38. => {
            let mut w = world.resource_mut::<ActiveWeather>();
            w.weather_type = WeatherType::Snow;
            w.duration_remaining = 999.;
            w.intensity = 1.;
            w.transition_timer = 0.;
        }
        6 if t > 61. => {}
        7 if t > 62. => {
            let mut p = world.resource_mut::<Probe>();
            report("clear", &mut p.clear);
            report("snow", &mut p.snow);
            let particles = world
                .query::<&crate::components::weather_particle::WeatherParticle>()
                .iter(world)
                .count();
            eprintln!(
                "[VISUAL PROFILE] particles={particles} meshes={} materials={}",
                world.resource::<Assets<Mesh>>().len(),
                world.resource::<Assets<StandardMaterial>>().len()
            );
            for d in world.resource::<DiagnosticsStore>().iter() {
                if format!("{:?}", d.path()).contains("render") {
                    eprintln!("[VISUAL GPU] {:?} {:?}", d.path(), d.smoothed());
                }
            }
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(1280., 900.);
            world.resource_mut::<ActiveWeather>().weather_type = WeatherType::Clear;
            crate::systems::dragon_flyover::smoke_spawn(world);
        }
        8 if t > 70. => {
            capture(world, "dragon");
        }
        9 if t > 75. => {
            crate::systems::dragon_flyover::smoke_breath(world);
            crate::systems::dragon_flyover::smoke_surface_gallery(world);
        }
        10 if t > 77. => {
            capture(world, "dragon-fire");
        }
        11 if t > 79. => {
            world.write_message(AppExit::Success);
        }
        3 if t > 22. && shadow_check => {}
        _ => return,
    }
    world.resource_mut::<Probe>().step += 1;
}

fn torch_view_probe(world: &mut World) {
    use crate::{
        components::{block::BiomeType, hero::Hero},
        systems::biome_teleport::BiomeTeleport,
    };
    let t = world.resource::<Time<Real>>().elapsed_secs();
    let step = world.resource::<Probe>().step;
    if let Some(mut anchor) = world.resource::<Probe>().anchor {
        if (4..=5).contains(&step) || step >= 10 {
            anchor.x += 350.;
        }
        for (mut tr, mut vel) in world
            .query_filtered::<(&mut Transform, &mut avian3d::prelude::LinearVelocity), With<Hero>>()
            .iter_mut(world)
        {
            tr.translation = anchor;
            vel.0 = Vec3::ZERO;
        }
    }
    match step {
        0 if t > 6. => {
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(2560., 1080.);
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.frame_generation = false;
            g.ray_tracing = false;
            g.dlss = false;
            g.local_lights = true;
            let mut w = world.resource_mut::<ActiveWeather>();
            w.weather_type = WeatherType::Clear;
            w.intensity = 0.;
            w.duration_remaining = 999.;
            world.resource_mut::<BiomeTeleport>().target = Some(Vec3::new(
                0.,
                crate::utils::biome_utils::nearest_biome_y(0., BiomeType::Underground),
                90.,
            ));
        }
        1 if t > 13. => {
            biome_anchor(world, BiomeType::Underground);
            let p = world.resource::<Probe>().anchor.unwrap();
            crate::systems::graphics::smoke_torch_gallery(world, p);
        }
        2 if t > 19. => {
            crate::systems::graphics::smoke_verify_torch_gallery(world);
            capture(world, "torch-wide-native");
        }
        3 if t > 20. => {}
        4 if t > 25. => {
            crate::systems::graphics::smoke_verify_torch_gallery(world);
            capture(world, "torch-wide-moved");
        }
        5 if t > 27. => {
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(1280., 900.);
        }
        6 if t > 33. => {
            crate::systems::graphics::smoke_verify_torch_gallery(world);
            capture(world, "torch-standard");
        }
        7 if t > 40. => {
            crate::systems::graphics::smoke_verify_torch_gallery(world);
            assert_eq!(
                world
                    .resource::<crate::systems::native_gpu::NativeStatus>()
                    .mode,
                1
            );
            capture(world, "torch-dlss45");
        }
        8 if t > 49. => {
            crate::systems::graphics::smoke_verify_torch_gallery(world);
            assert_eq!(
                world
                    .resource::<crate::systems::native_gpu::NativeStatus>()
                    .mode,
                2
            );
            capture(world, "torch-wide-rt");
        }
        9 if t > 51. => {}
        10 if t > 58. => {
            crate::systems::graphics::smoke_verify_torch_gallery(world);
            capture(world, "torch-wide-rt-moved");
        }
        11 if t > 61. => {
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(1360., 900.);
            world
                .resource_mut::<NextState<crate::states::GameState>>()
                .set(crate::states::GameState::Settings);
        }
        12 if t > 66. => {
            capture(world, "graphics-settings");
        }
        13 if t > 69. => {
            eprintln!(
                "[TORCH VIEW] completed: native, resize, camera movement, DLSS 4.5 Performance and RT"
            );
            world.write_message(AppExit::Success);
        }
        _ => {
            // Screenshot readback is asynchronous. Leave a full second before
            // resizing or changing the renderer so captures show settled output.
            if step == 7 && t > 34. && t < 40. {
                let mut window = world.query::<&mut Window>().single_mut(world).unwrap();
                if window.resolution.width() != 2560. {
                    window.resolution.set(2560., 1080.);
                }
                let mut g = world.resource_mut::<GraphicsSettings>();
                if !g.dlss {
                    g.dlss = true;
                }
                if !g.dlss_45_performance {
                    g.dlss_45_performance = true;
                }
            }
            if step == 8 && t > 41. && !world.resource::<GraphicsSettings>().ray_tracing {
                world.resource_mut::<GraphicsSettings>().ray_tracing = true;
            }
            if step == 12 && t > 62. {
                if let Some(mut nav) =
                    world.get_resource_mut::<crate::components::settings_menu::SettingsNavigation>()
                {
                    if nav.0 != crate::components::settings_menu::SettingsPage::Graphics {
                        nav.0 = crate::components::settings_menu::SettingsPage::Graphics;
                    }
                }
            }
            return;
        }
    }
    world.resource_mut::<Probe>().step += 1;
}

#[derive(Resource, Default)]
struct FlickerProbe {
    started: bool,
    anchor: Option<Vec3>,
    stage: usize,
    frame: u32,
    count: u32,
}
fn flicker_probe(world: &mut World) {
    use crate::{
        components::{block::BiomeType, hero::Hero},
        systems::biome_teleport::BiomeTeleport,
    };
    let t = world.resource::<Time<Real>>().elapsed_secs();
    if !world.resource::<FlickerProbe>().started && t > 6. {
        world
            .query::<&mut Window>()
            .single_mut(world)
            .unwrap()
            .resolution
            .set(1280., 800.);
        let mut g = world.resource_mut::<GraphicsSettings>();
        g.frame_generation = false;
        g.ray_tracing = true;
        g.dlss = true;
        g.lighting = true;
        g.local_lights = true;
        let y = crate::utils::biome_utils::nearest_biome_y(
            if std::env::args().any(|a| a == "--rt-negative-test") {
                -193408.
            } else {
                193408.
            },
            BiomeType::End,
        );
        world.resource_mut::<BiomeTeleport>().target = Some(Vec3::new(0., y, 90.));
        world.resource_mut::<FlickerProbe>().started = true;
    }
    if t < 14. {
        return;
    }
    let stage = ((t - 14.) / 8.) as usize;
    if stage >= 5 {
        if t > 56. {
            eprintln!("[RT FLICKER] completed: 5 sampled sequences");
            world.write_message(AppExit::Success);
        }
        return;
    }
    if world.resource::<FlickerProbe>().anchor.is_none() {
        let p = world
            .query_filtered::<&Transform, With<Hero>>()
            .single(world)
            .unwrap()
            .translation;
        world.resource_mut::<FlickerProbe>().anchor = Some(p);
    }
    let validation = std::env::args().any(|a| a == "--rt-validation-test");
    let mut anchor = world.resource::<FlickerProbe>().anchor.unwrap();
    if validation && stage == 1 {
        anchor.x += (t * 0.8).sin() * 350.;
        anchor.y += (t * 0.6).cos() * 250.;
    }
    for (mut tr, mut vel) in world
        .query_filtered::<(&mut Transform, &mut avian3d::prelude::LinearVelocity), With<Hero>>()
        .iter_mut(world)
    {
        tr.translation = anchor;
        vel.0 = Vec3::ZERO;
    }
    for mut tr in world
        .query_filtered::<&mut Transform, With<Camera3d>>()
        .iter_mut(world)
    {
        tr.translation = Vec3::new(anchor.x + 30., anchor.y + 80., 1500.);
    }
    let mut weather = world.resource_mut::<ActiveWeather>();
    weather.weather_type = WeatherType::Clear;
    weather.intensity = 0.;
    weather.duration_remaining = 999.;
    let mut cycle = world.resource_mut::<crate::resources::day_night_cycle::DayNightCycle>();
    cycle.current_time = cycle.cycle_duration * 0.25;
    let bias_check = std::env::args().any(|a| a == "--rt-bias-check");
    {
        let mut g = world.resource_mut::<GraphicsSettings>();
        let local_lights = validation || bias_check || stage != 1;
        if g.local_lights != local_lights {
            g.local_lights = local_lights;
        }
        if validation {
            if g.frame_generation != (stage == 3) {
                g.frame_generation = stage == 3;
            }
            if g.dlss != (stage != 2) {
                g.dlss = stage != 2;
            }
        }
    }
    let changed = world.resource::<FlickerProbe>().stage != stage;
    if bias_check && (changed || world.resource::<FlickerProbe>().frame == 0) {
        let mut shaders = world.resource_mut::<Assets<bevy::shader::Shader>>();
        let id = shaders
            .iter()
            .find(|(_, s)| {
                s.path
                    .replace('\\', "/")
                    .ends_with("bevy_solari/src/scene/bindings.wesl")
                    || s.source.as_str().contains("const RAY_T_MIN =")
            })
            .map(|(id, _)| id)
            .expect("Solari scene shader missing");
        let mut shader = shaders.get_mut(id).unwrap();
        let source = shader
            .source
            .as_str()
            .replace("const RAY_T_MIN = 0.001f;", "const RAY_T_MIN = 0.064f;");
        let source = if stage == 2 || stage == 4 {
            source
        } else {
            source.replace("const RAY_T_MIN = 0.064f;", "const RAY_T_MIN = 0.001f;")
        };
        shader.source = bevy::shader::Source::Wesl(source.into());
        eprintln!(
            "[RT BIAS] stage={stage} scaled={}",
            stage == 2 || stage == 4
        );
    }
    for mut solari in world
        .query::<&mut bevy::solari::prelude::SolariLighting>()
        .iter_mut(world)
    {
        solari.restir = !validation && !bias_check && stage == 2;
        solari.max_bounces = if !validation && !bias_check && stage == 3 {
            1
        } else {
            3
        };
        if changed {
            solari.reset = true;
        }
    }
    if !validation && !bias_check && stage == 4 {
        for mut sun in world.query::<&mut DirectionalLight>().iter_mut(world) {
            sun.illuminance = 0.;
        }
    }
    let mut p = world.resource_mut::<FlickerProbe>();
    if changed {
        p.stage = stage;
        p.count = 0;
    }
    p.frame += 1;
    // Sample different jitter phases instead of relying on one good frame.
    if p.frame % 7 == 0 && t - 14. - stage as f32 * 8. > 2. {
        let n = p.count;
        p.count += 1;
        drop(p);
        assert_eq!(
            world.resource::<GraphicsSettings>().frame_generation,
            validation && stage == 3
        );
        assert_eq!(
            world
                .resource::<crate::systems::native_gpu::NativeStatus>()
                .mode,
            2
        );
        capture(world, &format!("flicker-{stage}-{n:03}"));
    }
}

fn forest_weather_rt_probe(world: &mut World) {
    use crate::{
        components::{block::BiomeType, hero::Hero},
        systems::biome_teleport::BiomeTeleport,
    };
    let t = world.resource::<Time<Real>>().elapsed_secs();
    let step = world.resource::<Probe>().step;
    if let Some(mut anchor) = world.resource::<Probe>().anchor {
        if (10..=11).contains(&step) {
            anchor.x += (t * 0.8).sin() * 350.;
            anchor.y += (t * 0.6).cos() * 250.;
        }
        for (mut transform, mut velocity) in world
            .query_filtered::<(&mut Transform, &mut avian3d::prelude::LinearVelocity), With<Hero>>()
            .iter_mut(world)
        {
            transform.translation = anchor;
            velocity.0 = Vec3::ZERO;
        }
    }
    if step > 0 {
        let mut weather = world.resource_mut::<ActiveWeather>();
        weather.weather_type = match step {
            3 | 4 => WeatherType::Snow,
            5 | 6 => WeatherType::Rain,
            _ => WeatherType::Clear,
        };
        weather.duration_remaining = 999.;
        weather.intensity = if (3..=6).contains(&step) { 1. } else { 0. };
        weather.transition_timer = 0.;
        let mut cycle = world.resource_mut::<crate::resources::day_night_cycle::DayNightCycle>();
        cycle.current_time = cycle.cycle_duration * 0.25;
    }
    match step {
        0 if t > 6. => {
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(1600., 1000.);
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.frame_generation = false;
            g.ray_tracing = false;
            g.dlss = false;
            g.fog = true;
            g.lighting = true;
            g.weather_lighting = true;
            let y = crate::utils::biome_utils::nearest_biome_y(0., BiomeType::Forest);
            world.resource_mut::<BiomeTeleport>().target = Some(Vec3::new(0., y, 90.));
        }
        1 if t > 13. => {
            biome_anchor(world, BiomeType::Forest);
        }
        2 if t > 18. => {
            capture(world, "forest-density-clear");
        }
        3 if t > 24. => {
            capture(world, "forest-density-snow");
        }
        4 if t > 25. => {}
        5 if t > 31. => {
            capture(world, "forest-density-rain");
        }
        6 if t > 33. => {
            world.resource_mut::<Probe>().anchor = None;
            world.resource_mut::<BiomeTeleport>().target = Some(Vec3::new(
                0.,
                crate::utils::biome_utils::nearest_biome_y(193408., BiomeType::End),
                90.,
            ));
            world.resource_mut::<GraphicsSettings>().ray_tracing = true;
        }
        7 if t > 40. => {
            biome_anchor(world, BiomeType::End);
        }
        8 if t > 48. => {
            assert_eq!(
                world
                    .resource::<crate::systems::native_gpu::NativeStatus>()
                    .mode,
                2
            );
            capture(world, "end-rt-still");
        }
        9 if t > 50. => {}
        10 if t > 58. => {
            capture(world, "end-rt-moving");
        }
        11 if t > 63. => {
            capture(world, "end-rt-moving-later");
        }
        12 if t > 65. => {
            world.resource_mut::<GraphicsSettings>().local_lights = false;
        }
        13 if t > 73. => {
            capture(world, "end-rt-no-torches");
        }
        14 if t > 75. => {
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.ray_tracing = false;
            g.dlss = true;
            g.local_lights = true;
            g.shadows = true;
        }
        15 if t > 82. => {
            capture(world, "end-raster-shadows");
        }
        16 if t > 83. => {
            world.resource_mut::<GraphicsSettings>().shadows = false;
        }
        17 if t > 87. => {
            capture(world, "end-raster-no-shadows");
        }
        18 if t > 88. => {
            world.resource_mut::<GraphicsSettings>().ray_tracing = true;
        }
        19 if t > 96. => {
            capture(world, "end-rt-quality");
        }
        20 if t > 97. => {
            world.resource_mut::<GraphicsSettings>().frame_generation = true;
        }
        21 if t > 105. => {
            capture(world, "end-rt-quality-fg-input");
        }
        22 if t > 108. => {
            eprintln!(
                "[FOREST WEATHER RT] completed: forest, clear/snow/rain, End RT stationary/moving/local lights"
            );
            world.write_message(AppExit::Success);
        }
        _ => return,
    }
    world.resource_mut::<Probe>().step += 1;
}

fn biome_probe(world: &mut World) {
    use crate::{
        components::{block::BiomeType, hero::Hero},
        systems::biome_teleport::BiomeTeleport,
    };
    let t = world.resource::<Time<Real>>().elapsed_secs();
    let step = world.resource::<Probe>().step;
    if let Some(anchor) = world.resource::<Probe>().anchor {
        for (mut transform, mut velocity) in world
            .query_filtered::<(&mut Transform, &mut avian3d::prelude::LinearVelocity), With<Hero>>()
            .iter_mut(world)
        {
            transform.translation = anchor;
            velocity.0 = Vec3::ZERO;
        }
    }
    if step > 0 {
        let mut w = world.resource_mut::<ActiveWeather>();
        w.weather_type = WeatherType::Clear;
        w.duration_remaining = 999.;
        w.intensity = 0.;
        let mut cycle = world.resource_mut::<crate::resources::day_night_cycle::DayNightCycle>();
        cycle.current_time =
            cycle.cycle_duration * if (3..=4).contains(&step) { 0.75 } else { 0.25 };
    }
    match step {
        0 if t > 6. => {
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(1280., 900.);
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.frame_generation = false;
            g.ray_tracing = false;
            g.dlss = false;
            g.fog = true;
            g.lighting = true;
            let y = crate::utils::biome_utils::nearest_biome_y(0., BiomeType::Nether);
            world.resource_mut::<BiomeTeleport>().target = Some(Vec3::new(0., y, 90.));
        }
        1 if t > 13. => {
            biome_anchor(world, BiomeType::Nether);
        }
        2 if t > 17. => {
            capture(world, "nether-day");
        }
        3 if t > 23. => {
            capture(world, "nether-night");
        }
        4 if t > 24. => {
            world.resource_mut::<Probe>().anchor = None;
            let y = crate::utils::biome_utils::nearest_biome_y(0., BiomeType::Forest);
            world.resource_mut::<BiomeTeleport>().target = Some(Vec3::new(0., y, 90.));
        }
        5 if t > 31. => {
            biome_anchor(world, BiomeType::Forest);
            if let Some(p) = crate::systems::biome_mist::smoke_focus(world) {
                world.resource_mut::<Probe>().anchor = Some(p);
            }
        }
        // Screenshot readback is asynchronous: change render settings on a later step.
        6 if t > 35. => {
            crate::systems::biome_mist::smoke_verify(world);
            capture(world, "forest-mist");
        }
        7 if t > 36. => {
            world.resource_mut::<GraphicsSettings>().fog = false;
        }
        8 if t > 39. => {
            capture(world, "forest-clear");
        }
        9 if t > 40. => {
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.fog = true;
            g.ray_tracing = true;
        }
        10 if t > 45. => {
            capture(world, "forest-rt");
        }
        11 if t > 47. => {
            eprintln!(
                "[BIOME ATMOSPHERE] completed: Nether day/night, forest mist toggle, RT environment"
            );
            world.write_message(AppExit::Success);
        }
        _ => return,
    }
    world.resource_mut::<Probe>().step += 1;
}
fn biome_anchor(world: &mut World, expected: crate::components::block::BiomeType) {
    let p = world
        .query_filtered::<&Transform, With<crate::components::hero::Hero>>()
        .single(world)
        .unwrap()
        .translation;
    assert_eq!(
        crate::utils::biome_utils::determine_biome_at_position(p.x, p.y),
        expected,
        "Biome probe teleport failed"
    );
    world.resource_mut::<Probe>().anchor = Some(p);
}

fn tree_probe(world: &mut World) {
    use crate::{
        components::{block::BiomeType, hero::Hero},
        systems::{
            biome_teleport::BiomeTeleport,
            forest_trees::{TreeBlock, TreeSpecies},
        },
    };
    let t = world.resource::<Time<Real>>().elapsed_secs();
    let step = world.resource::<Probe>().step;
    if let Some(anchor) = world.resource::<Probe>().anchor {
        for (mut transform, mut velocity) in world
            .query_filtered::<(&mut Transform, &mut avian3d::prelude::LinearVelocity), With<Hero>>()
            .iter_mut(world)
        {
            transform.translation = anchor;
            velocity.0 = Vec3::ZERO;
        }
    }
    if step > 0 {
        let mut weather = world.resource_mut::<ActiveWeather>();
        weather.weather_type = WeatherType::Clear;
        weather.duration_remaining = 999.;
        weather.intensity = 0.;
        let mut cycle = world.resource_mut::<crate::resources::day_night_cycle::DayNightCycle>();
        cycle.current_time = cycle.cycle_duration * 0.25;
    }
    match step {
        0 if t > 6. => {
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(1600., 1000.);
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.frame_generation = false;
            g.ray_tracing = false;
            g.dlss = false;
            g.fog = true;
            g.lighting = true;
            let y = crate::utils::biome_utils::nearest_biome_y(0., BiomeType::Forest);
            world.resource_mut::<BiomeTeleport>().target = Some(Vec3::new(0., y, 90.));
        }
        1 if t > 14. => {
            biome_anchor(world, BiomeType::Forest);
            let hero = world.resource::<Probe>().anchor.unwrap();
            let root = world
                .query::<(&TreeBlock, &TreeSpecies)>()
                .iter(world)
                .map(|(tree, _)| tree.root)
                .min_by_key(|p| ((p.as_vec2() * 64.).distance_squared(hero.xy())) as i64)
                .expect("No natural forest trees generated");
            world.resource_mut::<Probe>().anchor =
                Some((root.as_vec2() * 64. + Vec2::new(-256., 200.)).extend(90.));
            eprintln!("[FOREST TREES] natural root {root:?}");
        }
        2 if t > 19. => {
            capture(world, "trees-natural");
        }
        3 if t > 20. => {
            tree_gallery(world);
        }
        4 if t > 25. => {
            capture(world, "trees-species");
        }
        5 if t > 27. => {
            world.resource_mut::<GraphicsSettings>().ray_tracing = true;
        }
        6 if t > 34. => {
            capture(world, "trees-rt");
        }
        7 if t > 36. => {
            let cache = world.resource::<crate::resources::material_cache::MaterialCache>();
            let expected = cache.tree_materials.clone();
            for (species, kind, material) in world
                .query::<(
                    &TreeSpecies,
                    &crate::components::block::BlockType,
                    &MeshMaterial3d<StandardMaterial>,
                )>()
                .iter(world)
            {
                assert_eq!(
                    material.0,
                    expected[&(
                        *species,
                        *kind == crate::components::block::BlockType::Leaves
                    )],
                    "Species material was overwritten"
                );
            }
            for species in [TreeSpecies::Oak, TreeSpecies::Birch, TreeSpecies::Spruce] {
                let count = world
                    .query::<&TreeSpecies>()
                    .iter(world)
                    .filter(|s| **s == species)
                    .count();
                assert!(count > 10, "Missing {species:?}");
            }
            eprintln!("[FOREST TREES] completed: natural placement, three species, native and RT");
            world.write_message(AppExit::Success);
        }
        _ => return,
    }
    world.resource_mut::<Probe>().step += 1;
}
fn tree_gallery(world: &mut World) {
    use crate::{
        components::{
            block::{Block, BlockType},
            chunk::ChunkMember,
        },
        resources::{material_cache::MaterialCache, mesh_cache::MeshCache},
        systems::forest_trees::{TreeBlock, TreeSpecies, shape},
    };
    let anchor = world.resource::<Probe>().anchor.unwrap();
    let base = (anchor.xy() / 64.).round().as_ivec2() - IVec2::new(0, 4);
    let remove: Vec<_> = world
        .query_filtered::<(Entity, &Transform), With<Block>>()
        .iter(world)
        .filter(|(_, t)| {
            let p = t.translation.xy() / 64. - base.as_vec2();
            p.x.abs() < 16. && p.y >= -1. && p.y < 14.
        })
        .map(|(e, _)| e)
        .collect();
    for e in remove {
        world.despawn(e);
    }
    let wood = world.resource::<MeshCache>().wood_mesh.clone();
    let leaves = world.resource::<MeshCache>().leaves_mesh.clone();
    let materials = world.resource::<MaterialCache>().tree_materials.clone();
    let floor = world
        .resource::<MaterialCache>()
        .forest_floor
        .clone()
        .unwrap();
    for x in -15..=15 {
        let p = base + IVec2::new(x, -1);
        world.spawn((
            Block {
                health: 3,
                max_health: 3,
            },
            BlockType::Dirt,
            ChunkMember {
                chunk_x: p.x.div_euclid(16),
                chunk_y: p.y.div_euclid(16),
            },
            Mesh3d(wood.clone()),
            MeshMaterial3d(floor.clone()),
            Transform::from_translation((p.as_vec2() * 64.).extend(-10.)),
        ));
    }
    for (x, species) in [
        (-8, TreeSpecies::Oak),
        (0, TreeSpecies::Birch),
        (8, TreeSpecies::Spruce),
    ] {
        let root = base + IVec2::new(x, 0);
        for (offset, leaf) in shape(species, 0) {
            let p = root + offset;
            let kind = if leaf {
                BlockType::Leaves
            } else {
                BlockType::Wood
            };
            world.spawn((
                Block {
                    health: kind.starting_health(),
                    max_health: kind.starting_health(),
                },
                kind,
                species,
                TreeBlock { root },
                ChunkMember {
                    chunk_x: p.x.div_euclid(16),
                    chunk_y: p.y.div_euclid(16),
                },
                Mesh3d(if leaf { leaves.clone() } else { wood.clone() }),
                MeshMaterial3d(materials[&(species, leaf)].clone()),
                Transform::from_translation((p.as_vec2() * 64.).extend(-10.)),
            ));
        }
    }
    world.resource_mut::<Probe>().anchor = Some(anchor + Vec3::new(220., -40., 0.));
}

fn fire_probe(world: &mut World) {
    let t = world.resource::<Time<Real>>().elapsed_secs();
    let step = world.resource::<Probe>().step;
    if step > 0 {
        crate::systems::dragon_flyover::smoke_verify_impact(world);
    }
    match step {
        0 if t > 8. => {
            crate::smoke_test::liquid_gallery(world);
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(1280., 900.);
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.frame_generation = false;
            g.ray_tracing = false;
            g.dlss = false;
            let mut w = world.resource_mut::<ActiveWeather>();
            w.weather_type = WeatherType::Clear;
            w.duration_remaining = 999.;
            w.intensity = 0.;
            crate::systems::dragon_flyover::smoke_impact_gallery(world);
        }
        1 if t > 11. => {
            capture(world, "fire-impact");
        }
        2 if t > 12.06 => {
            capture(world, "fire-release");
        }
        3 if t > 12.55 => {
            capture(world, "fire-off");
        }
        4 if t > 13.6 => {
            capture(world, "fire-settled");
        }
        5 if t > 14.2 => {
            eprintln!("[FIRE IMPACT] completed: exposed faces, burst lifetime, fixed scale");
            world.write_message(AppExit::Success);
        }
        _ => return,
    }
    world.resource_mut::<Probe>().step += 1;
}
