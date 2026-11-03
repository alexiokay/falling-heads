use super::*;

pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut flames: ResMut<Assets<crate::systems::graphics::FlameMaterial>>,
    _server: Res<AssetServer>,
) {
    let mut material = |color: Color, metallic: f32| {
        materials.add(StandardMaterial {
            base_color: color,
            metallic,
            perceptual_roughness: 0.45,
            ..default()
        })
    };
    let metal = material(Color::srgb(0.72, 0.78, 0.83), 0.65);
    let red = material(Color::srgb(0.7, 0.12, 0.055), 0.25);
    let dark = material(Color::srgb(0.065, 0.08, 0.105), 0.5);
    let hull = material(Color::srgb(0.93, 0.93, 0.95), 0.3);
    let panel = material(Color::srgb(0.55, 0.57, 0.62), 0.6);
    // Porthole glass: a faint blue tint with a sharp shine.
    let porthole = materials.add(StandardMaterial {
        base_color: Color::srgba(0.6, 0.8, 1., 0.22),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.05,
        reflectance: 0.9,
        ..default()
    });
    let mut light = |color: Color, emissive: LinearRgba| {
        materials.add(StandardMaterial {
            base_color: color,
            emissive,
            unlit: true,
            ..default()
        })
    };
    let nav_red = light(Color::srgb(1., 0.2, 0.15), LinearRgba::rgb(6., 0.5, 0.4));
    let nav_green = light(Color::srgb(0.3, 1., 0.4), LinearRgba::rgb(0.5, 6., 0.8));
    let flame = flames.add(crate::systems::graphics::FlameMaterial { effect: Vec4::ZERO });
    let villager = materials.add(StandardMaterial {
        unlit: true,
        alpha_mode: AlphaMode::Mask(0.5),
        ..default()
    });
    commands.insert_resource(RocketAssets {
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        quad: meshes.add(Rectangle::new(64., 64.)),
        cylinder: meshes.add(Cylinder::new(0.5, 1.)),
        cone: meshes.add(Cone {
            radius: 0.5,
            height: 1.,
        }),
        metal,
        red,
        dark,
        hull,
        panel,
        porthole,
        nav_red,
        nav_green,
        flame,
        villager,
    });
}

/// A head looking out of a porthole; kept upright and facing the camera by the
/// flight update whichever way the rocket points.
#[derive(Component)]
pub(super) struct RocketHead;

/// One part of the rocket: `mesh` at `p`, turned by `rotation`, scaled by `size`.
fn piece(
    world: &mut World,
    root: Entity,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    p: Vec3,
    rotation: Quat,
    size: Vec3,
) {
    world.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(p)
            .with_rotation(rotation)
            .with_scale(size),
        ChildOf(root),
    ));
}

/// The LAUNCHER rocket, nose toward +X: a round white hull with panel lines and
/// a red band, red nose with a metal tip, swept fins with navigation lights, an
/// engine bell, and portholes on the camera side with the hero's head in the big
/// one and up to three passengers' heads in the small ones.
pub(super) fn ship(world: &mut World, position: Vec3, hero: Entity, passengers: u8) -> Entity {
    let root = world
        .spawn((Transform::from_translation(position), Visibility::Visible))
        .id();
    let a = world.resource::<RocketAssets>();
    let (cube, cylinder, cone) = (a.cube.clone(), a.cylinder.clone(), a.cone.clone());
    let (hull, metal, red, dark, panel, glass) = (
        a.hull.clone(),
        a.metal.clone(),
        a.red.clone(),
        a.dark.clone(),
        a.panel.clone(),
        a.porthole.clone(),
    );
    let (nav_red, nav_green, villager) =
        (a.nav_red.clone(), a.nav_green.clone(), a.villager.clone());
    // Cylinders and cones are built along +Y; this lays them along +X (the nose).
    let along = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    let facing = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    piece(
        world,
        root,
        cylinder.clone(),
        hull,
        Vec3::ZERO,
        along,
        Vec3::new(68., 150., 68.),
    );
    for x in [-40., 8., 46.] {
        piece(
            world,
            root,
            cylinder.clone(),
            panel.clone(),
            Vec3::X * x,
            along,
            Vec3::new(70., 3., 70.),
        );
    }
    piece(
        world,
        root,
        cylinder.clone(),
        red.clone(),
        Vec3::X * -60.,
        along,
        Vec3::new(71., 14., 71.),
    );
    // Nose cone and metal tip.
    piece(
        world,
        root,
        cone.clone(),
        red.clone(),
        Vec3::X * 109.,
        along,
        Vec3::new(68., 68., 68.),
    );
    piece(
        world,
        root,
        cone.clone(),
        metal.clone(),
        Vec3::X * 148.,
        along,
        Vec3::new(14., 14., 14.),
    );
    // Swept fins above and below the tail, with lights on their tips.
    for s in [-1., 1.] {
        piece(
            world,
            root,
            cube.clone(),
            red.clone(),
            Vec3::new(-62., s * 42., 0.),
            Quat::from_rotation_z(s * 0.5),
            Vec3::new(52., 20., 10.),
        );
        piece(
            world,
            root,
            cube.clone(),
            panel.clone(),
            Vec3::new(-50., s * 36., 0.),
            Quat::from_rotation_z(s * 0.5),
            Vec3::new(30., 4., 12.),
        );
        world.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(if s < 0. {
                nav_red.clone()
            } else {
                nav_green.clone()
            }),
            Transform::from_xyz(-84., s * 58., 6.).with_scale(Vec3::splat(9.)),
            NavLight(if s < 0. { 0. } else { 0.5 }),
            ChildOf(root),
        ));
    }
    // Engine bell at the tail.
    piece(
        world,
        root,
        cone.clone(),
        dark.clone(),
        Vec3::X * -88.,
        along,
        Vec3::new(54., 30., 54.),
    );
    // Portholes on the camera side (+Z): the hero's big one, passengers' small.
    let hero_face = world
        .get::<MeshMaterial3d<StandardMaterial>>(hero)
        .and_then(|m| world.resource::<Assets<StandardMaterial>>().get(&m.0))
        .and_then(|m| m.base_color_texture.clone());
    let hero_face = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color_texture: hero_face,
            unlit: true,
            alpha_mode: AlphaMode::Mask(0.4),
            ..default()
        });
    let shown = passengers.min(3);
    let mut windows = vec![(Vec3::new(34., 0., 0.), 40., Some(hero_face))];
    for i in 0..3u8 {
        let face = (i < shown).then(|| villager.clone());
        windows.push((Vec3::new(-2. - i as f32 * 22., 0., 0.), 22., face));
    }
    for (at, size, face) in windows {
        piece(
            world,
            root,
            cylinder.clone(),
            metal.clone(),
            at + Vec3::Z * 32.,
            facing,
            Vec3::new(size + 8., 4., size + 8.),
        );
        piece(
            world,
            root,
            cylinder.clone(),
            dark.clone(),
            at + Vec3::Z * 33.,
            facing,
            Vec3::new(size, 4., size),
        );
        if let Some(face) = face {
            let head = world
                .spawn((
                    Transform::from_translation(at + Vec3::Z * 36.),
                    Visibility::Inherited,
                    RocketHead,
                    ChildOf(root),
                ))
                .id();
            world.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(dark.clone()),
                Transform::from_scale(Vec3::new(size * 0.8, size * 0.8, 3.)),
                ChildOf(head),
            ));
            world.spawn((
                Mesh3d(a_quad(world)),
                MeshMaterial3d(face),
                Transform::from_xyz(0., 0., 2.).with_scale(Vec3::splat(size * 0.8 / 64.)),
                ChildOf(head),
            ));
        }
        piece(
            world,
            root,
            cylinder.clone(),
            glass.clone(),
            at + Vec3::Z * 40.,
            facing,
            Vec3::new(size, 2., size),
        );
    }
    root
}

fn a_quad(world: &World) -> Handle<Mesh> {
    world.resource::<RocketAssets>().quad.clone()
}

/// A blinking navigation light on the rocket, with its phase.
#[derive(Component)]
pub(super) struct NavLight(pub f32);

pub(super) fn fragment(
    world: &mut World,
    position: Vec3,
    scale: Vec3,
    source: Handle<StandardMaterial>,
    velocity: Vec3,
    life: f32,
    seed: u32,
) -> Entity {
    // Individual alpha fades keep the debris shape intact, instead of shrinking an entire effect.
    let mut m = world
        .resource::<Assets<StandardMaterial>>()
        .get(&source)
        .cloned()
        .unwrap_or_default();
    m.alpha_mode = AlphaMode::Blend;
    let material = world.resource_mut::<Assets<StandardMaterial>>().add(m);
    let cube = world.resource::<RocketAssets>().cube.clone();
    world
        .spawn((
            Mesh3d(cube),
            MeshMaterial3d(material),
            Transform::from_translation(position).with_scale(scale),
            bevy::light::NotShadowCaster,
            Fragment {
                velocity,
                spin: Vec3::new(0.8, 1.4, (hash(seed) % 11) as f32 - 5.),
                life,
                duration: life,
            },
        ))
        .id()
}

pub(super) fn crash_burst(world: &mut World, p: Vec3, seed: u32) {
    // Use the same animated blast and explosion audio path as TNT, without
    // invoking TNT damage (the launcher owns its irregular landing crater).
    let frames = world
        .resource::<crate::resources::asset_handles::ExplosionTextures>()
        .frames
        .clone();
    if frames.is_empty() {
        return;
    }
    let quad = world.resource::<RocketAssets>().quad.clone();
    for i in 0..7u32 {
        let h = hash(seed.wrapping_add(i * 73));
        let angle = i as f32 * std::f32::consts::TAU / 7.;
        let radius = if i == 0 { 0. } else { 45. + (h % 65) as f32 };
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color_texture: Some(frames[0].clone()),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            });
        world.spawn((
            crate::components::explosive::ExplosionAnimation {
                timer: Timer::from_seconds(0.035, TimerMode::Repeating),
                current_frame: 0,
                total_frames: frames.len(),
                delay_timer: Timer::from_seconds(i as f32 * 0.055, TimerMode::Once),
                started: false,
            },
            Mesh3d(quad.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(
                p + Vec3::new(angle.cos() * radius, angle.sin() * radius, 230.),
            )
            .with_scale(Vec3::splat((130. + (h % 85) as f32) / 64.)),
            Visibility::Hidden,
            bevy::light::NotShadowCaster,
        ));
    }
    let metal = world.resource::<RocketAssets>().metal.clone();
    let red = world.resource::<RocketAssets>().red.clone();
    for i in 0..28u32 {
        let h = hash(seed.wrapping_add(i * 191));
        let angle = i as f32 * 2.39996;
        let speed = 180. + (h % 410) as f32;
        fragment(
            world,
            p + Vec3::Z * 200.,
            Vec3::new(3. + (h % 9) as f32, 2. + ((h >> 5) % 6) as f32, 3.),
            if i % 3 == 0 {
                red.clone()
            } else {
                metal.clone()
            },
            Vec3::new(angle.cos() * speed, angle.sin() * speed + 180., 35.),
            1.4 + (h % 90) as f32 * 0.01,
            h,
        );
    }
}
