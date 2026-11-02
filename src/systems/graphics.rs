use super::biome_climate::{Atmosphere, atmosphere};
use crate::{
    components::{
        block::{BiomeType, Block, BlockType},
        hero::Hero,
        weather::{ActiveWeather, WeatherType},
        weather_particle::WeatherParticle,
    },
    resources::{
        day_night_cycle::DayNightCycle, graphics_settings::GraphicsSettings,
        material_cache::MaterialCache,
    },
    states::GameState,
};
use bevy::{
    anti_alias::fxaa::Fxaa,
    camera::Hdr,
    core_pipeline::prepass::{DepthPrepass, NormalPrepass},
    pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel},
    picking::hover::PickingInteraction,
    post_process::bloom::Bloom,
    prelude::*,
    ui_widgets::Button,
};
use std::collections::{HashMap, HashSet};
pub struct GraphicsPlugin;
impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GraphicsSettings::load())
            .add_plugins(MaterialPlugin::<FlameMaterial>::default())
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    camera_settings,
                    lights_off_everywhere,
                    night_shade,
                    lighting,
                    torches,
                    local_lights,
                    animate_flames,
                    particles,
                ),
            )
            .add_systems(
                OnEnter(GameState::Settings),
                settings_panel.after(super::settings_menu_system::setup_settings_menu),
            )
            .add_systems(
                Update,
                (buttons, quality_slider).run_if(in_state(GameState::Settings)),
            );
    }
}

/// With Lighting off nothing is lit, so night would look like day. This even
/// dark-blue shade over the world (under the HUD) dims blocks, heads and
/// items together at night, fading in at dusk and out at dawn. With Lighting
/// on, the sun, moon and torches do this instead. Follows "Weather / moon
/// lighting" like the backdrop does.
#[derive(Component)]
pub(crate) struct NightShade;
const NIGHT_SHADE: f32 = 0.42;
fn night_shade(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<GraphicsSettings>,
    cycle: Option<Res<DayNightCycle>>,
    mut shade: Query<&mut BackgroundColor, With<NightShade>>,
    mut level: Local<f32>,
) {
    let Ok(mut color) = shade.single_mut() else {
        commands.spawn((
            NightShade,
            bevy::picking::Pickable::IGNORE,
            GlobalZIndex(-50),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ));
        return;
    };
    let night = cycle.is_some_and(|c| !c.is_day());
    let target = if night && !settings.lighting && settings.weather_lighting {
        NIGHT_SHADE
    } else {
        0.
    };
    *level += (target - *level) * (time.delta_secs() * 0.8).min(1.);
    let next = Color::srgba(0.01, 0.02, 0.07, *level);
    if color.0 != next {
        color.0 = next;
    }
}

/// Graphics > Lighting for every material in the game, whoever made it
/// (blocks, variant tiles, landmarks, mechanisms, mobs...): with lighting off
/// each material that was made lit renders flat at full colour, so nothing
/// turns near-black at night or underground; turning lighting on restores them.
/// Materials made unlit on purpose (heads, items, particles) are left alone.
fn lights_off_everywhere(
    settings: Res<GraphicsSettings>,
    mut events: MessageReader<AssetEvent<StandardMaterial>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut made_lit: Local<HashSet<AssetId<StandardMaterial>>>,
) {
    let mut fresh = Vec::new();
    for event in events.read() {
        match *event {
            AssetEvent::Added { id } => fresh.push(id),
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                made_lit.remove(&id);
            }
            _ => {}
        }
    }
    for id in fresh {
        if mats.get(id).is_some_and(|m| !m.unlit) {
            made_lit.insert(id);
            if !settings.lighting
                && let Some(mut m) = mats.get_mut(id)
            {
                m.unlit = true;
            }
        }
    }
    if settings.is_changed() {
        for &id in made_lit.iter() {
            if mats.get(id).is_some_and(|m| m.unlit == settings.lighting)
                && let Some(mut m) = mats.get_mut(id)
            {
                m.unlit = !settings.lighting;
            }
        }
    }
}

#[derive(Component)]
struct GraphicsButton(usize);
#[derive(Component)]
struct QualityBar;
#[derive(Component)]
struct QualityFill;
#[derive(Component)]
struct QualityText;
fn quality_control(p: &mut ChildSpawnerCommands, settings: &GraphicsSettings) {
    p.spawn((
        QualityText,
        Text::new(format!("DLSS quality: {}", settings.quality_name())),
        TextFont {
            font_size: FontSize::Px(14.),
            ..default()
        },
        Node {
            width: Val::Percent(100.),
            margin: UiRect::top(Val::Px(8.)),
            ..default()
        },
        Pickable::IGNORE,
    ));
    p.spawn((
        QualityBar,
        crate::components::settings_menu::SettingsControl,
        PickingInteraction::default(),
        bevy::ui::RelativeCursorPosition::default(),
        Node {
            width: Val::Percent(100.),
            height: Val::Px(28.),
            margin: UiRect::bottom(Val::Px(10.)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.08, 0.12, 0.14)),
    ))
    .with_children(|p| {
        p.spawn((
            QualityFill,
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(settings.quality_step() as f32 * 48.),
                width: Val::Percent(4.),
                height: Val::Percent(100.),
                ..default()
            },
            BackgroundColor(Color::srgb(0.45, 0.85, 0.9)),
        ));
    });
    p.spawn((
        Node {
            width: Val::Percent(100.),
            justify_content: JustifyContent::SpaceBetween,
            margin: UiRect::bottom(Val::Px(8.)),
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|p| {
        for name in ["Quality", "Balanced", "Performance"] {
            p.spawn((
                Text::new(name),
                TextFont {
                    font_size: FontSize::Px(12.),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        }
    });
}
fn quality_slider(
    mut settings: ResMut<GraphicsSettings>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    bars: Query<
        (
            Entity,
            &PickingInteraction,
            &bevy::ui::RelativeCursorPosition,
        ),
        With<QualityBar>,
    >,
    mut fill: Query<&mut Node, With<QualityFill>>,
    mut text: Query<&mut Text, With<QualityText>>,
    mut drag: Local<Option<u8>>,
    mut dragging: Local<bool>,
    mut presses: Local<crate::systems::settings_menu_system::PressTracker>,
) {
    for (entity, interaction, cursor) in &bars {
        // A drag starts only from a click on the bar, then follows the mouse
        // until release. Holding the button while passing over it (after
        // clicking a tab) does nothing.
        if presses.pressed(entity, *interaction) {
            *dragging = true;
        }
        if *dragging && mouse.pressed(MouseButton::Left) {
            if let Some(p) = cursor.normalized {
                *drag = Some(((p.x + 0.5).clamp(0., 1.) * 2.).round() as u8);
            }
        }
        if *interaction == PickingInteraction::Hovered {
            if keys.just_pressed(KeyCode::ArrowLeft) {
                *drag = Some(settings.quality_step().saturating_sub(1));
            }
            if keys.just_pressed(KeyCode::ArrowRight) {
                *drag = Some((settings.quality_step() + 1).min(2));
            }
        }
    }
    // Apply on release: dragging must not rebuild the GPU context every frame.
    if !mouse.pressed(MouseButton::Left) {
        *dragging = false;
        if let Some(step) = drag.take() {
            settings.set_quality_step(step);
            settings.dlss_model_45 = true;
            if !std::env::args().any(|a| a == "--visual-smoke-test") {
                settings.save();
            }
        }
    }
    let step = drag.unwrap_or(settings.quality_step());
    for mut node in &mut fill {
        node.left = Val::Percent(step as f32 * 48.);
    }
    let name = match step {
        0 => "Quality",
        1 => "Balanced",
        _ => "Performance",
    };
    for mut label in &mut text {
        **label = format!("DLSS quality: {name}");
    }
}
fn set_quality_bar(world: &mut World, interaction: PickingInteraction, x: Option<f32>) {
    for (mut i, mut cursor) in world
        .query_filtered::<(
            &mut PickingInteraction,
            &mut bevy::ui::RelativeCursorPosition,
        ), With<QualityBar>>()
        .iter_mut(world)
    {
        *i = interaction;
        if let Some(x) = x {
            cursor.normalized = Some(Vec2::new(x, 0.));
        }
    }
}

fn set_graphics_buttons(world: &mut World, interaction: PickingInteraction) {
    for mut i in world
        .query_filtered::<&mut PickingInteraction, With<GraphicsButton>>()
        .iter_mut(world)
    {
        *i = interaction;
    }
}

/// Switching to the Graphics tab with the mouse still held puts the pointer on
/// a toggle or the quality bar without a click on them. Nothing may change;
/// a real click on a toggle still works.
pub(crate) fn verify_held_press_is_ignored(world: &mut World) {
    let before = world.resource::<GraphicsSettings>().clone();
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    set_graphics_buttons(world, PickingInteraction::Pressed);
    set_quality_bar(world, PickingInteraction::Pressed, Some(0.5));
    world.run_system_cached(buttons).unwrap();
    world.run_system_cached(quality_slider).unwrap();
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    set_graphics_buttons(world, PickingInteraction::None);
    set_quality_bar(world, PickingInteraction::None, None);
    world.run_system_cached(buttons).unwrap();
    world.run_system_cached(quality_slider).unwrap();
    assert!(
        *world.resource::<GraphicsSettings>() == before,
        "a held press landing on graphics controls changed the settings"
    );
    // One genuine click on the first toggle: hover, then press.
    let Some(first) = world
        .query_filtered::<Entity, With<GraphicsButton>>()
        .iter(world)
        .next()
    else {
        return;
    };
    for interaction in [PickingInteraction::Hovered, PickingInteraction::Pressed] {
        world.entity_mut(first).insert(interaction);
        world.run_system_cached(buttons).unwrap();
    }
    assert!(
        *world.resource::<GraphicsSettings>() != before,
        "a real click on a graphics toggle did nothing"
    );
    // Click it again to put the setting back.
    for interaction in [
        PickingInteraction::None,
        PickingInteraction::Hovered,
        PickingInteraction::Pressed,
    ] {
        world.entity_mut(first).insert(interaction);
        world.run_system_cached(buttons).unwrap();
    }
    world.entity_mut(first).insert(PickingInteraction::None);
}

pub(crate) fn verify_quality_slider(world: &mut World) {
    // Holding the button while passing over the bar (after a tab click) must
    // not move it: the press did not start on the bar.
    let before = world.resource::<GraphicsSettings>().quality_step();
    let far = if before == 2 { -0.5 } else { 0.5 };
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    set_quality_bar(world, PickingInteraction::Pressed, Some(far));
    world.run_system_cached(quality_slider).unwrap();
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    set_quality_bar(world, PickingInteraction::None, None);
    world.run_system_cached(quality_slider).unwrap();
    assert_eq!(
        world.resource::<GraphicsSettings>().quality_step(),
        before,
        "A held drag passing over the bar changed the quality"
    );
    for step in [0u8, 1, 2, 0] {
        let before = world.resource::<GraphicsSettings>().quality_step();
        // A real click: the pointer hovers the bar before the button goes down.
        set_quality_bar(world, PickingInteraction::Hovered, None);
        world.run_system_cached(quality_slider).unwrap();
        set_quality_bar(
            world,
            PickingInteraction::Pressed,
            Some(step as f32 / 2. - 0.5),
        );
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        world.run_system_cached(quality_slider).unwrap();
        assert_eq!(
            world.resource::<GraphicsSettings>().quality_step(),
            before,
            "Apply only on release"
        );
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        world.run_system_cached(quality_slider).unwrap();
        assert_eq!(world.resource::<GraphicsSettings>().quality_step(), step);
    }
    for mut interaction in world
        .query_filtered::<&mut PickingInteraction, With<QualityBar>>()
        .iter_mut(world)
    {
        *interaction = PickingInteraction::None;
    }
}
#[derive(Component)]
pub(crate) struct GraphicsCamera;
#[derive(Component)]
pub(crate) struct Torch {
    owner: Entity,
}
#[derive(Resource)]
pub(crate) struct CaveEnvironment(std::collections::HashMap<BiomeType, EnvironmentMapLight>);
#[derive(Component)]
struct LocalLight(usize);
const LOCAL_LIGHT_BUDGET: usize = 128;
const LOCAL_LIGHT_RANGE: f32 = 440.;
const LIGHT_FADE_MARGIN: f32 = 256.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum LightKey {
    Torch(Entity),
    Glowstone(Entity),
    Lava(i32, i32),
}

// Use the actual camera footprint, including ultrawide/fullscreen and zoom.
// The extra range keeps offscreen lights that still illuminate visible surfaces.
pub(crate) fn light_view(
    cameras: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    fallback: Vec2,
) -> Rect {
    if let Ok((camera, transform)) = cameras.single() {
        if let Some(size) = camera.logical_viewport_size() {
            let mut min = Vec2::splat(f32::INFINITY);
            let mut max = Vec2::splat(f32::NEG_INFINITY);
            let mut corners = 0;
            for corner in [
                Vec2::ZERO,
                Vec2::new(size.x, 0.),
                size,
                Vec2::new(0., size.y),
            ] {
                let Ok(ray) = camera.viewport_to_world(transform, corner) else {
                    break;
                };
                let Some(distance) = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Z))
                else {
                    break;
                };
                let point = ray.get_point(distance).xy();
                min = min.min(point);
                max = max.max(point);
                corners += 1;
            }
            if corners == 4 && min.is_finite() && max.is_finite() && min.cmplt(max).all() {
                return Rect { min, max };
            }
        }
    }
    Rect::from_center_size(fallback, Vec2::new(2400., 1600.))
}
fn distance_to_view(view: Rect, position: Vec2) -> f32 {
    (position - position.clamp(view.min, view.max)).length()
}
fn light_visibility(view: Rect, position: Vec2) -> f32 {
    let t =
        ((distance_to_view(view, position) - LOCAL_LIGHT_RANGE) / LIGHT_FADE_MARGIN).clamp(0., 1.);
    1. - t * t * (3. - 2. * t)
}
#[derive(Resource)]
struct GraphicsAssets {
    cube: Handle<Mesh>,
    quad: Handle<Mesh>,
    wood: Handle<StandardMaterial>,
    flame: Handle<FlameMaterial>,
}
#[derive(
    Asset, bevy::reflect::TypePath, bevy::render::render_resource::AsBindGroup, Clone, Debug,
)]
pub(crate) struct FlameMaterial {
    #[uniform(0)]
    pub effect: Vec4,
}
impl Material for FlameMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/flame.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut flames: ResMut<Assets<FlameMaterial>>,
    mut images: ResMut<Assets<Image>>,
    server: Res<AssetServer>,
) {
    let mut environments = std::collections::HashMap::new();
    for biome in [
        BiomeType::Plains,
        BiomeType::Forest,
        BiomeType::Desert,
        BiomeType::Mountains,
        BiomeType::Underground,
        BiomeType::Nether,
        BiomeType::End,
        BiomeType::Iceland,
    ] {
        let p = atmosphere(biome, 1.);
        let color = |v: Vec3| Color::srgb(v.x, v.y, v.z);
        environments.insert(
            biome,
            EnvironmentMapLight::hemispherical_gradient(
                &mut images,
                color(p.fill),
                color(p.fill * 0.65),
                color(p.fill * 0.3),
            )
            .with_intensity(p.environment),
        );
    }
    commands.insert_resource(CaveEnvironment(environments));
    commands.insert_resource(GraphicsAssets {
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        quad: meshes.add(Rectangle::new(1., 1.)),
        wood: materials.add(StandardMaterial {
            base_color_texture: Some(server.load("textures/blocks/forest/oak_log/oak_log.png")),
            base_color: Color::WHITE,
            perceptual_roughness: 0.9,
            // Lit by its own flame: never black in a dark cave or with lighting off.
            unlit: true,
            ..default()
        }),
        flame: flames.add(FlameMaterial { effect: Vec4::ZERO }),
    });
    for i in 0..LOCAL_LIGHT_BUDGET {
        commands.spawn((
            LocalLight(i),
            PointLight {
                intensity: 0.,
                range: LOCAL_LIGHT_RANGE,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::IDENTITY,
        ));
    }
}
pub(crate) fn camera_settings(
    mut commands: Commands,
    settings: Res<GraphicsSettings>,
    cameras: Query<(Entity, Has<GraphicsCamera>), With<Camera3d>>,
    environment: Option<Res<CaveEnvironment>>,
) {
    for (e, done) in &cameras {
        if done && !settings.is_changed() {
            continue;
        }
        let mut camera = commands.entity(e);
        camera.insert((GraphicsCamera, Hdr, Msaa::Off, DepthPrepass, NormalPrepass));
        if let Some(environment) = environment.as_ref() {
            if settings.ray_tracing && settings.lighting {
                camera.insert(environment.0[&BiomeType::Plains].clone());
            } else {
                camera.remove::<EnvironmentMapLight>();
            }
        }
        if settings.bloom {
            camera.insert(Bloom {
                intensity: 0.035,
                prefilter: bevy::post_process::bloom::BloomPrefilter {
                    threshold: 1.5,
                    threshold_softness: 0.25,
                },
                composite_mode: bevy::post_process::bloom::BloomCompositeMode::Additive,
                ..default()
            });
        } else {
            camera.remove::<Bloom>();
        }
        if settings.ambient_occlusion && settings.lighting && !settings.ray_tracing {
            camera.insert(ScreenSpaceAmbientOcclusion {
                quality_level: ScreenSpaceAmbientOcclusionQualityLevel::High,
                radius: 38.,
                constant_object_thickness: 28.,
            });
        } else {
            camera.remove::<ScreenSpaceAmbientOcclusion>();
        }
        if settings.anti_aliasing && !settings.dlss && !settings.ray_tracing {
            camera.insert(Fxaa::default());
        } else {
            camera.remove::<Fxaa>();
        }
    }
}
fn lighting(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<GraphicsSettings>,
    cycle: Res<DayNightCycle>,
    weather: Res<ActiveWeather>,
    hero: Query<&Transform, With<Hero>>,
    cache: Option<Res<MaterialCache>>,
    backgrounds: Option<Res<crate::components::background::BackgroundCache>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut sun: Query<
        (
            Entity,
            &mut DirectionalLight,
            &mut Transform,
            &bevy::light::CascadeShadowConfig,
        ),
        Without<Hero>,
    >,
    mut cameras: Query<(Entity, Option<&mut EnvironmentMapLight>), With<Camera3d>>,
    mut blend: Local<f32>,
    mut mood: Local<Option<Atmosphere>>,
    environments: Option<Res<CaveEnvironment>>,
) {
    let Ok(hero) = hero.single() else {
        return;
    };
    let biome = crate::utils::biome_utils::determine_biome_at_position(
        hero.translation.x,
        hero.translation.y,
    );
    let daylight = if cycle.is_day() { 1. } else { 0. };
    *blend += (daylight - *blend) * (time.delta_secs() * 0.8).min(1.);
    let storm = if settings.weather_lighting && weather.duration_remaining > 0. {
        weather.intensity.clamp(0., 1.)
    } else {
        0.
    };
    let wet = if weather.weather_type == WeatherType::Rain {
        storm
    } else {
        0.
    };
    let overcast = storm
        * match weather.weather_type {
            WeatherType::Rain => 0.16,
            WeatherType::Snow => 0.08,
            WeatherType::Sandstorm => 0.20,
            _ => 0.,
        };
    if let Some(cache) = cache.as_ref() {
        for h in cache
            .materials
            .values()
            .chain(cache.resource_materials.values())
            .chain(cache.tree_materials.values())
            .chain(cache.forest_floor.iter())
        {
            if let Some(mut m) = mats.get_mut(h) {
                if m.unlit != !settings.lighting {
                    m.unlit = !settings.lighting;
                }
                let roughness = 0.86 - wet * 0.22;
                if m.perceptual_roughness != roughness {
                    m.perceptual_roughness = roughness;
                }
                if m.reflectance != 0.08 {
                    m.reflectance = 0.08;
                }
            }
        }
    }
    if let Some(cache) = cache.as_ref() {
        if let Some(handle) = cache.get(BlockType::Glowstone) {
            if let Some(mut material) = mats.get_mut(&handle) {
                let emission = if settings.local_lights && settings.lighting {
                    LinearRgba::rgb(1.8, 1.2, 0.35)
                } else {
                    LinearRgba::BLACK
                };
                if material.emissive != emission {
                    material.emissive = emission;
                }
                if material.emissive_texture != material.base_color_texture {
                    material.emissive_texture = material.base_color_texture.clone();
                }
            }
        }
    }
    let daylight = if settings.weather_lighting {
        *blend
    } else {
        1.
    };
    let target_mood = atmosphere(biome, daylight);
    let p = mood.map_or(target_mood, |old| {
        old.blend(target_mood, 1. - (-time.delta_secs() * 1.8).exp())
    });
    *mood = Some(p);
    // The backdrop fills most of this side-view game: it must share the weather mood.
    if let Some(backgrounds) = backgrounds {
        for (biome, handle) in &backgrounds.materials {
            if let Some(mut material) = mats.get_mut(handle) {
                let backdrop = atmosphere(*biome, daylight).backdrop;
                let base_color = Color::srgb(backdrop.x, backdrop.y, backdrop.z);
                // The backdrop must provide a valid deferred surface for RR and
                // receive torch light. A small textured emission supplies cave fill.
                let emissive = base_color.to_linear() * 0.45;
                if material.base_color != base_color {
                    material.base_color = base_color;
                }
                if material.unlit != !settings.lighting {
                    material.unlit = !settings.lighting;
                }
                if material.emissive_texture != material.base_color_texture {
                    material.emissive_texture = material.base_color_texture.clone();
                }
                if material.emissive != emissive {
                    material.emissive = emissive;
                }
                // Preserve the upper-left directional glow at reduced strength,
                // with a wider, softer falloff than the original reflective wall.
                let roughness = super::background_system::BACKDROP_ROUGHNESS;
                let reflectance = super::background_system::BACKDROP_REFLECTANCE;
                if material.perceptual_roughness != roughness {
                    material.perceptual_roughness = roughness;
                }
                if material.reflectance != reflectance {
                    material.reflectance = reflectance;
                }
            }
        }
    }
    ambient.color = Color::srgb(p.fill.x, p.fill.y, p.fill.z);
    // Clouds soften the key light while diffuse fill preserves readability.
    ambient.brightness = p.ambient * (1. + overcast * 0.5);
    for (entity, mut light, mut transform, cascades) in &mut sun {
        light.illuminance = if settings.lighting {
            p.sun * (1. - overcast)
        } else {
            0.
        };
        light.color = Color::srgb(p.key.x, p.key.y, p.key.z);
        let shadows = settings.shadows && settings.lighting && !settings.ray_tracing;
        let reenable = shadows && !light.shadow_maps_enabled;
        light.shadow_maps_enabled = shadows;
        transform.rotation = Quat::from_euler(EulerRot::XYZ, -0.45, -0.50, 0.);
        if reenable {
            // Rebuild the owned sun's render-side shadow views on re-enable.
            // Preserve the custom cascade distances: default Bevy cascades do
            // not reach this game's play plane, 1500 units from the camera.
            commands.spawn((light.clone(), *transform, cascades.clone()));
            commands.entity(entity).despawn();
        }
    }
    for (camera, environment) in &mut cameras {
        if let Some(mut environment) = environment {
            if let Some(maps) = environments.as_ref() {
                let target = &maps.0[&biome];
                if environment.diffuse_map != target.diffuse_map {
                    environment.diffuse_map = target.diffuse_map.clone();
                    environment.specular_map = target.specular_map.clone();
                }
            }
            environment.intensity = p.environment * (1. + overcast * 0.25);
        }
        // Almost all geometry shares the same camera distance in this side view.
        // Distance fog therefore washes the entire scene; use terrain-bound banks.
        commands.entity(camera).remove::<DistanceFog>();
    }
}
fn torches(
    mut commands: Commands,
    time: Res<Time>,
    mut clock: Local<f32>,
    hero: Query<&Transform, With<Hero>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    blocks: Query<(Entity, &Transform, &BlockType), With<Block>>,
    torches: Query<(Entity, &Torch, &GlobalTransform)>,
    assets: Res<GraphicsAssets>,
) {
    *clock += time.delta_secs();
    if *clock < 0.7 {
        return;
    }
    *clock = 0.;
    let Ok(hero) = hero.single() else {
        return;
    };
    let view = light_view(&cameras, hero.translation.xy());
    let mut owners = HashSet::new();
    for (e, t, p) in &torches {
        if !blocks
            .get(t.owner)
            .is_ok_and(|(_, _, kind)| kind.supports_fixture())
            || distance_to_view(view, p.translation().xy())
                > LOCAL_LIGHT_RANGE + LIGHT_FADE_MARGIN + 256.
        {
            commands.entity(e).try_despawn();
        } else {
            owners.insert(t.owner);
        }
    }
    let cells: HashSet<_> = blocks
        .iter()
        .map(|(_, t, _)| {
            (
                (t.translation.x / 64.).round() as i32,
                (t.translation.y / 64.).round() as i32,
            )
        })
        .collect();
    for (e, t, kind) in &blocks {
        if owners.len() >= LOCAL_LIGHT_BUDGET {
            break;
        }
        if owners.contains(&e)
            || !kind.supports_fixture()
            || distance_to_view(view, t.translation.xy()) > LOCAL_LIGHT_RANGE + LIGHT_FADE_MARGIN
        {
            continue;
        }
        let x = (t.translation.x / 64.).round() as i32;
        let y = (t.translation.y / 64.).round() as i32;
        let h = (x as u32).wrapping_mul(73856093) ^ (y as u32).wrapping_mul(19349663);
        if h % 71 != 0 {
            continue;
        }
        let side = if !cells.contains(&(x + 1, y)) {
            1.
        } else if !cells.contains(&(x - 1, y)) {
            -1.
        } else {
            0.
        };
        spawn_torch(&mut commands, &assets, e, t.scale, side);
        owners.insert(e);
    }
}
#[derive(Component)]
struct TorchFlame;
fn spawn_torch(
    commands: &mut Commands,
    assets: &GraphicsAssets,
    e: Entity,
    scale: Vec3,
    side: f32,
) {
    let anchor = torch_anchor(side);
    commands.entity(e).with_children(|owner| {
        owner
            .spawn((
                Torch { owner: e },
                Transform::from_translation(anchor / scale).with_scale(scale.recip()),
                Visibility::Inherited,
            ))
            .with_children(|p| {
                p.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(assets.wood.clone()),
                    Transform::from_xyz(side, 0., 0.).with_scale(if side == 0. {
                        Vec3::new(12., 9., 10.)
                    } else {
                        Vec3::new(10., 9., 12.)
                    }),
                ));
                p.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(assets.wood.clone()),
                    Transform::from_xyz(side * 8., 12., if side == 0. { 7. } else { 0. })
                        .with_rotation(Quat::from_rotation_z(-side * 0.5))
                        .with_scale(Vec3::new(7., 29., 7.)),
                ));
                p.spawn((
                    TorchFlame,
                    bevy::light::NotShadowCaster,
                    Mesh3d(assets.quad.clone()),
                    MeshMaterial3d(assets.flame.clone()),
                    Transform::from_xyz(side * 15., 29., if side == 0. { 12. } else { 4. })
                        .with_scale(Vec3::new(16., 23., 1.)),
                ));
            });
    });
}

pub(crate) fn smoke_torches(world: &mut World) {
    let center = world
        .query_filtered::<&Transform, With<Hero>>()
        .single(world)
        .unwrap()
        .translation;
    let mesh = world
        .resource::<crate::resources::mesh_cache::MeshCache>()
        .block_mesh
        .clone();
    let material = world
        .resource::<MaterialCache>()
        .get(BlockType::Stone)
        .unwrap();
    let mut owners = Vec::new();
    for side in [-1., 0., 1.] {
        let e = world
            .spawn((
                Block {
                    health: 100,
                    max_health: 100,
                },
                BlockType::Stone,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(center + Vec3::new(side * 200., 330., -10.))
                    .with_scale(Vec3::splat(64.)),
            ))
            .id();
        owners.push((e, side));
    }
    let assets = world.resource::<GraphicsAssets>();
    let assets = GraphicsAssets {
        cube: assets.cube.clone(),
        quad: assets.quad.clone(),
        wood: assets.wood.clone(),
        flame: assets.flame.clone(),
    };
    for (e, side) in owners {
        spawn_torch(&mut world.commands(), &assets, e, Vec3::splat(64.), side);
    }
}

#[derive(Component)]
struct TorchGallery;
pub(crate) fn smoke_torch_gallery(world: &mut World, center: Vec3) {
    let old: Vec<_> = world
        .query_filtered::<(Entity, &Transform), With<Block>>()
        .iter(world)
        .filter(|(_, t)| {
            (t.translation.xy() - center.xy())
                .abs()
                .cmplt(Vec2::new(1900., 800.))
                .all()
        })
        .map(|(e, _)| e)
        .collect();
    for e in old {
        world.despawn(e);
    }
    let mesh = world
        .resource::<crate::resources::mesh_cache::MeshCache>()
        .block_mesh
        .clone();
    let material = world
        .resource::<MaterialCache>()
        .get(BlockType::Stone)
        .unwrap();
    let a = world.resource::<GraphicsAssets>();
    let assets = GraphicsAssets {
        cube: a.cube.clone(),
        quad: a.quad.clone(),
        wood: a.wood.clone(),
        flame: a.flame.clone(),
    };
    for row in [-1., 1.] {
        for column in 0..12 {
            let e = world
                .spawn((
                    TorchGallery,
                    Block {
                        health: 10000,
                        max_health: 10000,
                    },
                    BlockType::Stone,
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(
                        center
                            + Vec3::new(-1375. + column as f32 * 250., row * 270., -center.z + 20.),
                    )
                    .with_scale(Vec3::splat(64.)),
                ))
                .id();
            spawn_torch(&mut world.commands(), &assets, e, Vec3::splat(64.), 0.);
        }
    }
    world.flush();
}
pub(crate) fn smoke_verify_torch_gallery(world: &mut World) {
    let (camera, transform) = world
        .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>()
        .single(world)
        .unwrap();
    let size = camera.logical_viewport_size().unwrap();
    let camera = camera.clone();
    let transform = *transform;
    let owners: HashSet<_> = world
        .query_filtered::<Entity, With<TorchGallery>>()
        .iter(world)
        .collect();
    assert_eq!(owners.len(), 24);
    let roots: HashSet<_> = world
        .query::<(Entity, &Torch)>()
        .iter(world)
        .filter(|(_, t)| owners.contains(&t.owner))
        .map(|(e, _)| e)
        .collect();
    let visible: Vec<_> = world
        .query_filtered::<(&ChildOf, &GlobalTransform), With<TorchFlame>>()
        .iter(world)
        .filter(|(parent, _)| roots.contains(&parent.parent()))
        .filter_map(|(_, t)| {
            let p = t.translation();
            let screen = camera.world_to_viewport(&transform, p).ok()?;
            (screen.cmpge(Vec2::ZERO).all() && screen.cmple(size).all())
                .then_some(p + Vec3::new(0., -4., 8.))
        })
        .collect();
    assert!(
        visible.len() >= 12,
        "Gallery must exercise screen edges, visible={}",
        visible.len()
    );
    let lights: Vec<_> = world
        .query_filtered::<(&PointLight, &Transform), With<LocalLight>>()
        .iter(world)
        .map(|(l, t)| (l.intensity, t.translation))
        .collect();
    for p in &visible {
        assert!(
            lights
                .iter()
                .any(|(power, q)| *power > 30_000_000. && p.distance(*q) < 1.),
            "Visible torch has no light at {p:?}"
        );
    }
    eprintln!(
        "[TORCH VIEW] {} visible gallery torches all illuminated, viewport={size:?}",
        visible.len()
    );
}

fn torch_anchor(side: f32) -> Vec3 {
    if side == 0. {
        Vec3::new(0., -8., 32.)
    } else {
        Vec3::new(side * 32., -8., 24.)
    }
}
#[allow(clippy::too_many_arguments)] // Bevy system parameters
fn local_lights(
    settings: Res<GraphicsSettings>,
    time: Res<Time>,
    hero: Query<&Transform, With<Hero>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    torches: Query<(Entity, &GlobalTransform), With<TorchFlame>>,
    fluids: Res<super::fluids::FluidWorld>,
    blocks: Query<(Entity, &Transform, &BlockType), (With<Block>, Without<LocalLight>)>,
    grid: Res<crate::resources::spatial_hash::SpatialHash>,
    mut slots: Local<Vec<Option<LightKey>>>,
    mut lights: Query<
        (&LocalLight, &mut PointLight, &mut Transform),
        (Without<Hero>, Without<Torch>),
    >,
) {
    let Ok(hero) = hero.single() else {
        return;
    };
    let view = light_view(&cameras, hero.translation.xy());
    let mut sources: Vec<_> = torches
        .iter()
        .map(|(e, t)| {
            (
                LightKey::Torch(e),
                t.translation() + Vec3::new(0., -4., 8.),
                0u8,
            )
        })
        .collect();
    sources.extend(
        fluids
            .lava_light_positions(
                view.min - Vec2::splat(LOCAL_LIGHT_RANGE + LIGHT_FADE_MARGIN),
                view.max + Vec2::splat(LOCAL_LIGHT_RANGE + LIGHT_FADE_MARGIN),
            )
            .into_iter()
            .map(|p| {
                (
                    LightKey::Lava((p.x / 64.).round() as i32, (p.y / 64.).round() as i32),
                    p.extend(12.),
                    1u8,
                )
            }),
    );
    // Glowstone near the view only (spatial hash), not a scan of every block
    let reach = Vec2::splat(LOCAL_LIGHT_RANGE + LIGHT_FADE_MARGIN + 64.);
    sources.extend(
        grid.rect_iter(view.min - reach, view.max + reach)
            .filter_map(|e| blocks.get(e).ok())
            .filter(|(_, _, kind)| **kind == BlockType::Glowstone)
            .map(|(e, t, _)| {
                (
                    LightKey::Glowstone(e),
                    t.translation + Vec3::new(0., 0., 48.),
                    2u8,
                )
            }),
    );
    sources.retain(|(_, p, _)| light_visibility(view, p.xy()) > 0.);
    sources.sort_by(|a, b| {
        distance_to_view(view, a.1.xy())
            .total_cmp(&distance_to_view(view, b.1.xy()))
            .then(a.2.cmp(&b.2))
            .then(a.1.x.total_cmp(&b.1.x))
            .then(a.1.y.total_cmp(&b.1.y))
    });
    sources.truncate(LOCAL_LIGHT_BUDGET);
    let selected: HashMap<_, _> = sources
        .iter()
        .map(|(key, p, kind)| (*key, (*p, *kind)))
        .collect();
    // Do not move a light entity to another source when the hero passes it:
    // Solari's temporal lighting history needs a stable emitter identity.
    slots.resize(LOCAL_LIGHT_BUDGET, None);
    for slot in slots.iter_mut() {
        if slot.is_some_and(|key| !selected.contains_key(&key)) {
            *slot = None;
        }
    }
    let assigned: HashSet<_> = slots.iter().flatten().copied().collect();
    let mut unassigned = sources.iter().filter(|(key, _, _)| !assigned.contains(key));
    for slot in slots.iter_mut().filter(|s| s.is_none()) {
        *slot = unassigned.next().map(|(key, _, _)| *key);
    }
    for (i, mut light, mut t) in &mut lights {
        if settings.local_lights && settings.lighting {
            if let Some((p, kind)) = slots
                .get(i.0)
                .and_then(|key| key.and_then(|key| selected.get(&key)))
            {
                t.translation = *p;
                let flicker = if *kind == 2 {
                    1.
                } else {
                    0.94 + 0.06 * (time.elapsed_secs() * 7. + p.x * 0.037 + p.y * 0.021).sin()
                };
                light.intensity = (match *kind {
                    1 => 65_000_000.,
                    2 => 48_000_000.,
                    _ => 40_000_000.,
                }) * flicker
                    * light_visibility(view, p.xy());
                light.color = match *kind {
                    1 => Color::srgb(1., 0.23, 0.035),
                    2 => Color::srgb(1., 0.78, 0.32),
                    _ => Color::srgb(1., 0.57, 0.18),
                };
                continue;
            }
        }
        light.intensity = 0.;
    }
}
fn animate_flames(time: Res<Time>, mut mats: ResMut<Assets<FlameMaterial>>) {
    for (_, m) in mats.iter_mut() {
        m.effect.x = time.elapsed_secs();
    }
}
fn particles(
    settings: Res<GraphicsSettings>,
    mut particles: Query<&mut Visibility, With<WeatherParticle>>,
) {
    for mut v in &mut particles {
        *v = if settings.weather_particles {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
fn settings_panel(
    mut commands: Commands,
    settings: Res<GraphicsSettings>,
    pages: Query<(Entity, &crate::components::settings_menu::SettingsPageRoot)>,
) {
    let Some((page, _)) = pages
        .iter()
        .find(|(_, p)| p.0 == crate::components::settings_menu::SettingsPage::Graphics)
    else {
        return;
    };
    commands.entity(page).with_children(|p| {
        p.spawn((
            Node {
                width: Val::Percent(100.),
                ..default()
            },
            Text::new("GRAPHICS"),
            TextFont {
                font_size: FontSize::Px(19.),
                ..default()
            },
            Pickable::IGNORE,
        ));
        for n in 0..14 {
            p.spawn((
                GraphicsButton(n),
                crate::components::settings_menu::SettingsControl,
                Button,
                PickingInteraction::default(),
                Node {
                    width: Val::Percent(49.),
                    min_height: Val::Px(48.),
                    padding: UiRect::all(Val::Px(6.)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.10, 0.16, 0.19)),
            ))
            .with_child((
                Text::new(settings.label(n)),
                TextFont {
                    font_size: FontSize::Px(14.),
                    ..default()
                },
                TextColor(label_color(&settings, n)),
                Pickable::IGNORE,
            ));
        }
        quality_control(p, &settings);
        p.spawn((
            super::native_gpu::StatusLabel,
            Node {
                width: Val::Percent(100.),
                ..default()
            },
            Text::new("Checking native renderer..."),
            TextFont {
                font_size: FontSize::Px(12.),
                ..default()
            },
            TextColor(Color::srgb(0.6, 0.64, 0.7)),
            Pickable::IGNORE,
        ));
    });
}
/// Grey for an option that does nothing right now because Lighting is off.
fn label_color(settings: &GraphicsSettings, n: usize) -> Color {
    if settings.inactive(n) {
        Color::srgb(0.5, 0.52, 0.55)
    } else {
        Color::WHITE
    }
}
fn buttons(
    mut settings: ResMut<GraphicsSettings>,
    q: Query<(Entity, &PickingInteraction, &GraphicsButton), Changed<PickingInteraction>>,
    all: Query<(&GraphicsButton, &Children)>,
    mut text: Query<(&mut Text, &mut TextColor)>,
    mut presses: Local<crate::systems::settings_menu_system::PressTracker>,
) {
    for (entity, i, b) in &q {
        if !presses.pressed(entity, *i) {
            continue;
        }
        settings.toggle(b.0);
        settings.save();
    }
    // Every label, not just the clicked one: Lighting changes whether the
    // options that depend on it are active.
    if !settings.is_changed() {
        return;
    }
    for (b, children) in &all {
        for child in children {
            if let Ok((mut t, mut color)) = text.get_mut(*child) {
                let label = settings.label(b.0);
                if **t != label {
                    **t = label;
                }
                let c = label_color(&settings, b.0);
                if color.0 != c {
                    color.0 = c;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lighting_off_flattens_every_lit_material_and_on_restores_it() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<StandardMaterial>()
            .insert_resource(GraphicsSettings {
                lighting: false,
                ..default()
            })
            .add_systems(Update, lights_off_everywhere);
        let mut mats = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
        let lit = mats.add(StandardMaterial::default());
        let flat = mats.add(StandardMaterial {
            unlit: true,
            ..default()
        });
        app.update();
        app.update();
        let unlit = |app: &App, h: &Handle<StandardMaterial>| {
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(h)
                .unwrap()
                .unlit
        };
        assert!(
            unlit(&app, &lit),
            "lit material not flattened with lighting off"
        );
        app.world_mut().resource_mut::<GraphicsSettings>().lighting = true;
        app.update();
        assert!(!unlit(&app, &lit), "lighting on did not restore it");
        assert!(
            unlit(&app, &flat),
            "an unlit-by-design material was changed"
        );
    }
    #[test]
    fn lights_cover_wide_views_and_fade_only_beyond_their_visible_reach() {
        let view = Rect::from_center_size(Vec2::new(300., -200_000.), Vec2::new(3600., 1400.));
        for p in [
            view.min,
            view.max,
            view.center(),
            view.max + Vec2::new(LOCAL_LIGHT_RANGE, 0.),
        ] {
            assert_eq!(light_visibility(view, p), 1.);
        }
        let outer = view.max + Vec2::new(LOCAL_LIGHT_RANGE + LIGHT_FADE_MARGIN, 0.);
        assert_eq!(light_visibility(view, outer), 0.);
        assert!(
            (light_visibility(view, outer - Vec2::new(LIGHT_FADE_MARGIN * 0.5, 0.)) - 0.5).abs()
                < 0.001
        );
    }
    #[test]
    fn more_than_twelve_torches_keep_their_light_entities_when_the_hero_moves() {
        let mut app = App::new();
        app.init_resource::<GraphicsSettings>()
            .insert_resource(Time::<()>::default())
            .init_resource::<super::super::fluids::FluidWorld>()
            .insert_resource(crate::resources::spatial_hash::SpatialHash::new(256.))
            .add_systems(Update, local_lights);
        let hero = app
            .world_mut()
            .spawn((
                Hero {
                    move_direction: 1.,
                    direction_change_timer: Timer::default(),
                },
                Transform::IDENTITY,
            ))
            .id();
        for i in 0..24 {
            app.world_mut().spawn((
                TorchFlame,
                GlobalTransform::from_translation(Vec3::new(-1100. + i as f32 * 95., 0., 40.)),
            ));
        }
        let lights: Vec<_> = (0..LOCAL_LIGHT_BUDGET)
            .map(|i| {
                app.world_mut()
                    .spawn((LocalLight(i), PointLight::default(), Transform::IDENTITY))
                    .id()
            })
            .collect();
        app.update();
        let before: Vec<_> = lights
            .iter()
            .filter(|e| app.world().get::<PointLight>(**e).unwrap().intensity > 0.)
            .map(|e| (*e, app.world().get::<Transform>(*e).unwrap().translation))
            .collect();
        assert_eq!(before.len(), 24);
        app.world_mut()
            .get_mut::<Transform>(hero)
            .unwrap()
            .translation
            .x = 320.;
        app.update();
        for (e, p) in before {
            assert_eq!(app.world().get::<Transform>(e).unwrap().translation, p);
            assert!(app.world().get::<PointLight>(e).unwrap().intensity > 0.);
        }
    }
    #[test]
    fn precipitation_preserves_fill_and_snow_does_not_wet_surfaces() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GraphicsSettings>()
            .init_resource::<DayNightCycle>()
            .init_resource::<ActiveWeather>()
            .init_resource::<GlobalAmbientLight>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<MaterialCache>()
            .add_systems(Update, lighting);
        app.world_mut().resource_mut::<GraphicsSettings>().shadows = false;
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        app.world_mut()
            .resource_mut::<MaterialCache>()
            .materials
            .insert(BlockType::Stone, material.clone());
        app.world_mut().spawn((
            Hero {
                move_direction: 1.,
                direction_change_timer: Timer::default(),
            },
            Transform::IDENTITY,
        ));
        let sun = app
            .world_mut()
            .spawn((
                DirectionalLight::default(),
                bevy::light::CascadeShadowConfig::default(),
                Transform::IDENTITY,
            ))
            .id();
        let camera = app
            .world_mut()
            .spawn((Camera3d::default(), EnvironmentMapLight::default()))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(2));
        app.update();
        let key = app
            .world()
            .get::<DirectionalLight>(sun)
            .unwrap()
            .illuminance;
        let fill = app.world().resource::<GlobalAmbientLight>().brightness;
        let env = app
            .world()
            .get::<EnvironmentMapLight>(camera)
            .unwrap()
            .intensity;
        let dry = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&material)
            .unwrap()
            .perceptual_roughness;
        for kind in [WeatherType::Snow, WeatherType::Rain] {
            let mut w = app.world_mut().resource_mut::<ActiveWeather>();
            w.weather_type = kind;
            w.intensity = 1.;
            w.duration_remaining = 60.;
            app.update();
            assert!(
                app.world()
                    .get::<DirectionalLight>(sun)
                    .unwrap()
                    .illuminance
                    >= key * 0.80
            );
            assert!(app.world().resource::<GlobalAmbientLight>().brightness >= fill);
            assert!(
                app.world()
                    .get::<EnvironmentMapLight>(camera)
                    .unwrap()
                    .intensity
                    >= env
            );
            if kind == WeatherType::Snow {
                assert_eq!(
                    app.world()
                        .resource::<Assets<StandardMaterial>>()
                        .get(&material)
                        .unwrap()
                        .perceptual_roughness,
                    dry
                );
            }
        }
    }
    #[test]
    fn glowstone_emits_a_stable_light_and_obeys_the_local_light_toggle() {
        let mut app = App::new();
        app.init_resource::<GraphicsSettings>()
            .insert_resource(Time::<()>::default())
            .init_resource::<super::super::fluids::FluidWorld>()
            .insert_resource(crate::resources::spatial_hash::SpatialHash::new(256.))
            .add_systems(Update, local_lights);
        app.world_mut().spawn((
            Hero {
                move_direction: 1.,
                direction_change_timer: Timer::from_seconds(1., TimerMode::Repeating),
            },
            Transform::IDENTITY,
        ));
        let glow = app
            .world_mut()
            .spawn((
                Block {
                    health: 2,
                    max_health: 2,
                },
                BlockType::Glowstone,
                Transform::from_xyz(64., 0., 0.),
            ))
            .id();
        app.world_mut()
            .resource_mut::<crate::resources::spatial_hash::SpatialHash>()
            .insert(glow, 64., 0.);
        let light = app
            .world_mut()
            .spawn((LocalLight(0), PointLight::default(), Transform::IDENTITY))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Transform>(light).unwrap().translation,
            Vec3::new(64., 0., 48.)
        );
        assert_eq!(
            app.world().get::<PointLight>(light).unwrap().intensity,
            48_000_000.
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(3));
        app.update();
        assert_eq!(
            app.world().get::<PointLight>(light).unwrap().intensity,
            48_000_000.
        );
        app.world_mut()
            .resource_mut::<GraphicsSettings>()
            .local_lights = false;
        app.update();
        assert_eq!(app.world().get::<PointLight>(light).unwrap().intensity, 0.);
    }
    #[test]
    fn mushroom_never_gets_a_torch_and_changed_support_removes_existing_torch() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(GraphicsAssets {
                cube: default(),
                quad: default(),
                wood: default(),
                flame: default(),
            })
            .add_systems(Update, torches);
        app.world_mut().spawn((
            Hero {
                move_direction: 1.,
                direction_change_timer: Timer::default(),
            },
            Transform::default(),
        ));
        let support = app
            .world_mut()
            .spawn((
                Block {
                    health: 4,
                    max_health: 4,
                },
                BlockType::Mushroom,
                Transform::default(),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.update();
        assert_eq!(
            app.world_mut().query::<&Torch>().iter(app.world()).count(),
            0
        );
        app.world_mut().entity_mut(support).insert(BlockType::Stone);
        app.update();
        assert_eq!(
            app.world_mut().query::<&Torch>().iter(app.world()).count(),
            1
        );
        app.world_mut()
            .entity_mut(support)
            .insert(BlockType::Mushroom);
        app.update();
        assert_eq!(
            app.world_mut().query::<&Torch>().iter(app.world()).count(),
            0
        );
    }
    #[test]
    fn torch_mount_is_owned_and_touches_scaled_block_face() {
        let mut world = World::new();
        let assets = GraphicsAssets {
            cube: default(),
            quad: default(),
            wood: default(),
            flame: default(),
        };
        for side in [-1., 0., 1.] {
            let owner = world.spawn(Transform::from_scale(Vec3::splat(64.))).id();
            spawn_torch(
                &mut world.commands(),
                &assets,
                owner,
                Vec3::splat(64.),
                side,
            );
            world.flush();
            let torch = world.get::<Children>(owner).unwrap()[0];
            let t = world.get::<Transform>(torch).unwrap();
            let anchor = t.translation * 64.;
            assert!(
                (if side == 0. {
                    anchor.z.abs()
                } else {
                    anchor.x.abs()
                } - 32.)
                    .abs()
                    < 0.001
            );
            assert_eq!(t.scale * 64., Vec3::ONE);
            world.despawn(owner);
            assert!(
                world.get_entity(torch).is_err(),
                "Destroyed support must remove torch immediately"
            );
        }
    }
    #[test]
    fn world_hdr_and_native_overlay_graphics_toggles_apply() {
        let mut app = App::new();
        let mut initial = GraphicsSettings::default();
        initial.dlss = false;
        app.insert_resource(initial)
            .add_systems(Startup, crate::setup)
            .add_systems(Update, camera_settings);
        app.update();
        let world = app.world_mut();
        let camera = world
            .query_filtered::<Entity, With<Camera3d>>()
            .single(world)
            .unwrap();
        let overlay = world
            .query_filtered::<Entity, With<Camera2d>>()
            .single(world)
            .unwrap();
        assert!(world.get::<Hdr>(camera).is_some());
        assert!(
            world.get::<Hdr>(overlay).is_none(),
            "Resolved world image is composed in the native LDR overlay"
        );
        assert!(world.get::<Bloom>(camera).is_some());
        assert!(world.get::<ScreenSpaceAmbientOcclusion>(camera).is_some());
        assert!(world.get::<Fxaa>(camera).is_some());
        {
            let mut settings = world.resource_mut::<GraphicsSettings>();
            settings.bloom = false;
            settings.ambient_occlusion = false;
            settings.anti_aliasing = false;
        }
        app.update();
        assert!(app.world().get::<Bloom>(camera).is_none());
        assert!(
            app.world()
                .get::<ScreenSpaceAmbientOcclusion>(camera)
                .is_none()
        );
        assert!(app.world().get::<Fxaa>(camera).is_none());
    }
}
