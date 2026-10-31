//! Opt-in hidden native regression: drives real picking inputs, not button components.
use crate::{
    components::{dev_panel::*, pause_menu::*, settings_menu::*, weather::*},
    resources::settings::GameSettings,
    states::GameState,
};
use bevy::{
    picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
pub struct UiSmokePlugin;
#[derive(Resource, Default)]
struct Probe {
    step: usize,
    since: f64,
    location: Option<Location>,
}
impl Plugin for UiSmokePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Probe>()
            .add_systems(PreUpdate, probe.after(bevy::input::InputSystems));
    }
}
fn key(world: &mut World, key: KeyCode) {
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.reset(key);
    keys.press(key);
}
fn state(world: &World, state: GameState) {
    assert_eq!(*world.resource::<State<GameState>>().get(), state);
}
fn hover<T: Component>(world: &mut World) {
    let (transform, node) = world
        .query_filtered::<(&UiGlobalTransform, &ComputedNode), With<T>>()
        .single(world)
        .unwrap();
    let position = transform.translation * node.inverse_scale_factor();
    move_pointer(world, position);
}
fn move_pointer(world: &mut World, position: Vec2) {
    let window = world
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(world)
        .unwrap();
    let location = Location {
        position,
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
    };
    world.resource_mut::<Probe>().location = Some(location.clone());
    world.write_message(PointerInput::new(
        PointerId::Mouse,
        location,
        PointerAction::Move { delta: Vec2::ZERO },
    ));
}
fn probe(world: &mut World) {
    let elapsed = world.resource::<Time<Real>>().elapsed_secs_f64();
    let step = world.resource::<Probe>().step;
    assert!(elapsed < 150., "UI smoke timed out at {step}");
    if elapsed < 45.
        || elapsed - world.resource::<Probe>().since < if step == 2 { 15. } else { 0.7 }
    {
        return;
    }
    eprintln!("UI SMOKE step {step}");
    match step {
        0 => {
            state(world, GameState::Playing);
            damage_gallery(world);
            key(world, KeyCode::Escape);
            for mut window in world.query::<&mut Window>().iter_mut(world) {
                window.resolution.set(720., 1180.);
            }
        }
        1 => {
            state(world, GameState::Paused);
            world
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("falling-heads-ui-pause.png"));
            move_pointer(world, Vec2::new(15., 100.));
        }
        2 => {
            let backdrop = world
                .query_filtered::<&BackgroundColor, With<PauseMenuRoot>>()
                .single(world)
                .unwrap();
            assert!(
                backdrop.0.alpha() < 0.5,
                "hovering empty menu space made the backdrop opaque"
            );
            world
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("falling-heads-ui-long-pause.png"));
            hover::<ResumeButton>(world);
        }
        3 | 7 | 11 | 14 | 17 | 20 | 23 => {
            let location = world.resource::<Probe>().location.clone().unwrap();
            world.write_message(PointerInput::new(
                PointerId::Mouse,
                location,
                PointerAction::Press(PointerButton::Primary),
            ));
        }
        4 | 8 | 12 | 15 | 18 | 21 | 24 => {
            let location = world.resource::<Probe>().location.clone().unwrap();
            world.write_message(PointerInput::new(
                PointerId::Mouse,
                location,
                PointerAction::Release(PointerButton::Primary),
            ));
        }
        5 => {
            state(world, GameState::Playing);
            key(world, KeyCode::Tab);
        }
        6 => {
            assert!(world.resource::<GameSettings>().dev_mode);
            assert_eq!(
                world
                    .query_filtered::<&Node, With<DevPanelRoot>>()
                    .single(world)
                    .unwrap()
                    .display,
                Display::Flex
            );
            world
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("falling-heads-ui-dev.png"));
            // First weather button is Clear; select by marker + type through a temporary marker.
            let entity = world
                .query::<(Entity, &WeatherControlButton)>()
                .iter(world)
                .find(|(_, b)| b.weather_type == WeatherType::Rain)
                .unwrap()
                .0;
            world.entity_mut(entity).insert(RainTarget);
            hover::<RainTarget>(world);
        }
        9 => {
            assert_eq!(
                world.resource::<ActiveWeather>().weather_type,
                WeatherType::Rain
            );
            key(world, KeyCode::Escape);
        }
        10 => {
            state(world, GameState::Paused);
            hover::<SettingsButton>(world);
        }
        13 => {
            state(world, GameState::Settings);
            let tab = world
                .query::<(Entity, &SettingsTab)>()
                .iter(world)
                .find(|(_, t)| t.0 == SettingsPage::Graphics)
                .unwrap()
                .0;
            world.entity_mut(tab).insert(GraphicsTarget);
            hover::<GraphicsTarget>(world);
        }
        16 => {
            assert!(world.resource::<SettingsNavigation>().0 == SettingsPage::Graphics);
            world
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("falling-heads-ui-graphics.png"));
            let tab = world
                .query::<(Entity, &SettingsTab)>()
                .iter(world)
                .find(|(_, t)| t.0 == SettingsPage::General)
                .unwrap()
                .0;
            world.entity_mut(tab).insert(GeneralTarget);
            hover::<GeneralTarget>(world);
        }
        19 => {
            assert!(world.resource::<SettingsNavigation>().0 == SettingsPage::General);
            hover::<DevModeToggleButton>(world);
        }
        22 => {
            assert!(!world.resource::<GameSettings>().dev_mode);
            hover::<BackToMenuButton>(world);
        }
        25 => {
            state(world, GameState::Paused);
            key(world, KeyCode::Escape);
        }
        26 => {
            state(world, GameState::Playing);
            assert_eq!(
                world
                    .query_filtered::<&Node, With<DevPanelRoot>>()
                    .single(world)
                    .unwrap()
                    .display,
                Display::None
            );
            eprintln!(
                "UI SMOKE PASSED: pause, resize, Escape, Resume, Tab, weather, Settings, Graphics/General navigation, dev toggle, Back"
            );
            world.write_message(AppExit::Success);
        }
        _ => return,
    }
    let mut probe = world.resource_mut::<Probe>();
    probe.step += 1;
    probe.since = elapsed;
}
#[derive(Component)]
struct RainTarget;

fn damage_gallery(world: &mut World) {
    use crate::{
        components::block::{Block, BlockType},
        events::block_events::BlockDamaged,
        resources::{material_cache::MaterialCache, mesh_cache::MeshCache},
    };
    let camera = *world
        .query_filtered::<&Transform, With<Camera3d>>()
        .single(world)
        .unwrap();
    let origin = camera.translation - Vec3::new(30., 80., 1100.);
    let mesh = world.resource::<MeshCache>().block_mesh.clone();
    for (column, kind) in [BlockType::Netherrack, BlockType::Stone, BlockType::Sand]
        .into_iter()
        .enumerate()
    {
        for row in 0..2 {
            let material = world.resource::<MaterialCache>().get(kind).unwrap();
            let block = world
                .spawn((
                    Block {
                        health: 2 - row,
                        max_health: 3,
                    },
                    kind,
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(
                        origin
                            + Vec3::new(
                                (column as f32 - 1.) * 150.,
                                if row == 0 { 260. } else { -260. },
                                0.,
                            ),
                    )
                    .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.25, -0.4, 0.))
                    .with_scale(Vec3::splat(64.)),
                ))
                .id();
            world.write_message(BlockDamaged {
                entity: block,
                damage: 0,
                remaining_health: 2 - row,
            });
        }
    }
}

#[derive(Component)]
struct GraphicsTarget;
#[derive(Component)]
struct GeneralTarget;
