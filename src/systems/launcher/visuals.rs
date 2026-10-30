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
    let glass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.06, 0.42, 0.65),
        emissive: LinearRgba::rgb(0.08, 0.6, 0.9),
        ..default()
    });
    let flame = flames.add(crate::systems::graphics::FlameMaterial { effect: Vec4::ZERO });
    let villager = materials.add(StandardMaterial {
        unlit: true,
        alpha_mode: AlphaMode::Mask(0.5),
        ..default()
    });
    commands.insert_resource(RocketAssets {
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        quad: meshes.add(Rectangle::new(64., 64.)),
        metal,
        red,
        dark,
        glass,
        flame,
        villager,
    });
}

pub(super) fn part(
    world: &mut World,
    root: Entity,
    p: Vec3,
    size: Vec3,
    material: Handle<StandardMaterial>,
) {
    let mesh = world.resource::<RocketAssets>().cube.clone();
    world.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(p).with_scale(size),
        ChildOf(root),
    ));
}

pub(super) fn ship(world: &mut World, position: Vec3) -> Entity {
    let root = world
        .spawn((Transform::from_translation(position), Visibility::Visible))
        .id();
    let a = world.resource::<RocketAssets>();
    let (metal, red, dark, glass) = (
        a.metal.clone(),
        a.red.clone(),
        a.dark.clone(),
        a.glass.clone(),
    );
    part(
        world,
        root,
        Vec3::ZERO,
        Vec3::new(130., 58., 52.),
        metal.clone(),
    );
    part(
        world,
        root,
        Vec3::new(66., 0., 0.),
        Vec3::new(32., 46., 42.),
        red.clone(),
    );
    part(
        world,
        root,
        Vec3::new(88., 0., 0.),
        Vec3::new(18., 30., 28.),
        metal.clone(),
    );
    part(
        world,
        root,
        Vec3::new(102., 0., 0.),
        Vec3::new(12., 14., 12.),
        red.clone(),
    );
    part(
        world,
        root,
        Vec3::new(-73., 0., 0.),
        Vec3::new(22., 42., 42.),
        dark.clone(),
    );
    for x in [-48., 45.] {
        part(
            world,
            root,
            Vec3::new(x, 0., 0.),
            Vec3::new(8., 61., 55.),
            dark.clone(),
        );
    }
    for angle in [
        0.,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        std::f32::consts::PI * 1.5,
    ] {
        let rotation = Quat::from_rotation_x(angle);
        let p = rotation * Vec3::new(-48., 40., 0.);
        let mesh = world.resource::<RocketAssets>().cube.clone();
        world.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(red.clone()),
            Transform::from_translation(p)
                .with_rotation(rotation * Quat::from_rotation_z(-0.22))
                .with_scale(Vec3::new(48., 35., 8.)),
            ChildOf(root),
        ));
    }
    for x in [-22., 18.] {
        for z in [-1., 1.] {
            part(
                world,
                root,
                Vec3::new(x, 0., z * 28.),
                Vec3::new(29., 32., 4.),
                dark.clone(),
            );
            part(
                world,
                root,
                Vec3::new(x, 0., z * 31.),
                Vec3::new(23., 25., 3.),
                glass.clone(),
            );
        }
    }
    for x in [-34., 34.] {
        for y in [-22., 22.] {
            part(
                world,
                root,
                Vec3::new(x, y, 28.),
                Vec3::splat(5.),
                dark.clone(),
            );
        }
    }
    root
}

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
