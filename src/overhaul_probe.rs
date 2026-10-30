//! Hidden, opt-in rendered regression gallery; never changes saved settings.
use crate::{
    components::{
        block::{BiomeType, Block, BlockType},
        enemy::Enemy,
        hero::Hero,
    },
    resources::{
        graphics_settings::GraphicsSettings, material_cache::MaterialCache, mesh_cache::MeshCache,
    },
    systems::biome_encounters::{self, Site},
};
use avian3d::prelude::*;
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
#[derive(Resource, Default)]
struct Probe {
    step: u8,
    since: f32,
    anchor: Option<Vec3>,
    parts: Vec<Entity>,
    damaged: Vec<Entity>,
}
const BIOMES: [BiomeType; 8] = [
    BiomeType::Plains,
    BiomeType::Forest,
    BiomeType::Desert,
    BiomeType::Mountains,
    BiomeType::Underground,
    BiomeType::Nether,
    BiomeType::End,
    BiomeType::Iceland,
];
fn capture(world: &mut World, name: &str) {
    world
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(format!("target/visual-overhaul-{name}.png")));
}
fn gallery(world: &mut World, index: usize) {
    let walls: Vec<_> = world
        .query_filtered::<Entity, With<biome_encounters::StructureBackdrop>>()
        .iter(world)
        .collect();
    for e in walls {
        world.despawn(e);
    }
    let purpur = world.resource::<MaterialCache>().materials[&BlockType::Purpur].clone();
    assert!(
        world
            .resource::<Assets<StandardMaterial>>()
            .get(&purpur)
            .unwrap()
            .base_color_texture
            .is_some(),
        "Purpur pillars must have a texture"
    );
    let anchor = world.resource::<Probe>().anchor.unwrap();
    let old = std::mem::take(&mut world.resource_mut::<Probe>().parts);
    for e in old {
        let _ = world.despawn(e);
    }
    let old: Vec<_> = world
        .query::<(Entity, &Transform, Option<&Block>, Option<&Enemy>)>()
        .iter(world)
        .filter(|(_, t, b, e)| {
            (b.is_some() || e.is_some()) && t.translation.xy().distance(anchor.xy()) < 750.
        })
        .map(|(e, _, _, _)| e)
        .collect();
    for e in old {
        world.despawn(e);
    }
    let before: std::collections::HashSet<_> = world.query::<Entity>().iter(world).collect();
    let site = Site {
        origin: (anchor.xy() / 64.).round().as_ivec2(),
        biome: BIOMES[index],
        seed: 921,
    };
    world.resource_scope(|world, meshes: Mut<MeshCache>| {
        world.resource_scope(|world, mats: Mut<MaterialCache>| {
            biome_encounters::spawn(&mut world.commands(), site, &meshes, &mats)
        })
    });
    world.flush();
    let parts: Vec<_> = world
        .query::<Entity>()
        .iter(world)
        .filter(|e| !before.contains(e))
        .collect();
    world.resource_mut::<Probe>().parts = parts;
    eprintln!(
        "[OVERHAUL TEST] gallery {}",
        biome_encounters::title(site.biome)
    );
}
pub fn run(world: &mut World) {
    if !world.contains_resource::<Probe>() {
        world.init_resource::<Probe>();
    }
    let t = world.resource::<Time<Real>>().elapsed_secs();
    let step = world.resource::<Probe>().step;
    let since = world.resource::<Probe>().since;
    assert!(t < 135., "Overhaul timeout at {step}");
    // Isolate the authored gallery from random raids and weather, not from the
    // real encounter systems. Dedicated unit tests check health/damage behavior.
    if step > 0 {
        let old: Vec<_> = world
            .query_filtered::<Entity, (
                Or<(With<Enemy>, With<crate::components::villager::Villager>)>,
                Without<biome_encounters::EncounterFoe>,
            )>()
            .iter(world)
            .collect();
        for e in old {
            let _ = world.despawn(e);
        }
        for mut hp in world.query_filtered::<&mut crate::components::enemy::Health,With<biome_encounters::EncounterFoe>>().iter_mut(world){hp.current=hp.max;}
        let mut weather = world.resource_mut::<crate::components::weather::ActiveWeather>();
        weather.weather_type = crate::components::weather::WeatherType::Clear;
        weather.intensity = 0.;
        weather.duration_remaining = 999.;
    }
    if let Some(anchor) = world.resource::<Probe>().anchor {
        for (mut tr, mut pos, mut vel) in world
            .query_filtered::<(&mut Transform, &mut Position, &mut LinearVelocity), With<Hero>>()
            .iter_mut(world)
        {
            let p = anchor + Vec3::new(-128., 0., 90.);
            tr.translation = p;
            pos.0 = p;
            vel.0 = Vec3::ZERO;
        }
    }
    match step {
        0 if t > 8. => {
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(1360., 900.);
            let p = world
                .query_filtered::<&Transform, With<Hero>>()
                .single(world)
                .unwrap()
                .translation;
            world.resource_mut::<Probe>().anchor = Some((p.xy() / 64.).round().extend(0.) * 64.);
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.frame_generation = false;
            g.ray_tracing = false;
            g.dlss = false;
            gallery(world, 0);
        }
        1..=8 if t > since + 3.5 => {
            if step == 1 {
                biome_encounters::verify_structure_depth(world, false);
            }
            capture(world, &format!("biome-{}", step - 1));
        }
        9 if t > since + 2. => {
            let anchor = world.resource::<Probe>().anchor.unwrap();
            let mesh = world.resource::<MeshCache>().block_mesh.clone();
            let material = world.resource::<MaterialCache>().materials[&BlockType::Stone].clone();
            let mut damaged = vec![];
            for i in 0..3 {
                damaged.push(
                    world
                        .spawn((
                            Block {
                                health: (1 + i) * 1000,
                                max_health: 5000,
                            },
                            BlockType::Stone,
                            Mesh3d(mesh.clone()),
                            MeshMaterial3d(material.clone()),
                            Transform::from_translation(
                                anchor + Vec3::new((i as f32 - 1.) * 96., 128., -10.),
                            )
                            .with_scale(Vec3::splat(64.)),
                        ))
                        .id(),
                );
            }
            for e in &damaged {
                world.write_message(crate::events::block_events::BlockDamaged {
                    entity: *e,
                    damage: 0,
                    remaining_health: 1000,
                });
            }
            world.resource_mut::<Probe>().damaged = damaged;
        }
        10 if t > since + 3. => {
            capture(world, "cracks-native");
        }
        11 if t > since + 1. => {
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.ray_tracing = true;
            g.dlss = true;
            g.set_quality_step(0);
        }
        12 if t > since + 7. => {
            assert_eq!(
                world
                    .resource::<crate::systems::native_gpu::NativeStatus>()
                    .mode,
                2
            );
            let e = world.resource::<Probe>().damaged.clone();
            crate::systems::terrain_cracks::verify(world, &e);
            capture(world, "cracks-rt");
        }
        13 if t > since + 1. => {
            let e = world.resource::<Probe>().damaged[1];
            world.despawn(e);
        }
        14 if t > since + 3. => {
            capture(world, "destruction-rt");
        }
        15 if t > since + 1. => {
            world.resource_mut::<GraphicsSettings>().set_quality_step(1);
        }
        16 if t > since + 4. => {
            use bevy::anti_alias::dlss::*;
            assert_eq!(
                world
                    .query::<&Dlss<DlssRayReconstructionFeature>>()
                    .single(world)
                    .unwrap()
                    .perf_quality_mode,
                DlssPerfQualityMode::Balanced
            );
            capture(world, "balanced-rt");
        }
        17 if t > since + 1. => {
            world.resource_mut::<GraphicsSettings>().set_quality_step(0);
            gallery(world, 0);
        }
        18 if t > since + 7. => {
            biome_encounters::verify_structure_depth(world, true);
            capture(world, "structure-depth-rt");
        }
        19 if t > since + 1. => {
            let mut g = world.resource_mut::<GraphicsSettings>();
            g.ray_tracing = false;
            g.set_quality_step(1);
        }
        20 if t > since + 4. => {
            capture(world, "balanced-sr");
        }
        21 if t > since + 1. => {
            world
                .resource_mut::<NextState<crate::states::GameState>>()
                .set(crate::states::GameState::Settings);
        }
        22 if t > since + 1. => {
            world
                .resource_mut::<crate::components::settings_menu::SettingsNavigation>()
                .0 = crate::components::settings_menu::SettingsPage::Graphics;
        }
        23 if t > since + 2. => {
            crate::systems::graphics::verify_quality_slider(world);
        }
        24 if t > since + 3. => {
            capture(world, "settings");
        }
        25 if t > since + 2. => {
            eprintln!(
                "[OVERHAUL TEST] completed: eight landmarks, solid recessed walls in raster/RT, RT cracks/destruction, Balanced SR/RR, real slider input"
            );
            world.write_message(AppExit::Success);
        }
        _ => return,
    }
    // Leave each screenshot a separate frame before changing the displayed room.
    world.resource_mut::<Probe>().step += 1;
    world.resource_mut::<Probe>().since = t;
}
pub fn advance_gallery(world: &mut World) {
    if !world.contains_resource::<Probe>() {
        return;
    }
    let p = world.resource::<Probe>();
    let step = p.step;
    if (2..=8).contains(&step) && world.resource::<Time<Real>>().elapsed_secs() - p.since > 0.6 {
        // Capture filename is used only as a stage marker; entities are replaced once.
        let marker = format!("gallery-{step}");
        if world
            .get_resource::<GalleryIndex>()
            .is_none_or(|g| g.0 != step)
        {
            gallery(world, (step - 1) as usize);
            world.insert_resource(GalleryIndex(step));
            eprintln!("[OVERHAUL TEST] {marker}");
        }
    }
}
#[derive(Resource)]
struct GalleryIndex(u8);
