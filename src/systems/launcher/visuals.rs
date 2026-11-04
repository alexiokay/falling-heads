use super::*;

pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut flames: ResMut<Assets<crate::systems::graphics::FlameMaterial>>,
    server: Res<AssetServer>,
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
        rocket: crate::systems::block_rocket::RocketArt::new(
            &mut meshes,
            &mut materials,
            &server,
            &mut Vec::new(),
        ),
        flame,
        villager,
    });
}

/// How big the rocket flies in the world (it is built about 360 long).
pub(super) const ROCKET_SCALE: f32 = 0.62;

/// The LAUNCHER rocket (nose toward +X): the voxel block rocket shared with the
/// space prison, with the hero's head in the glass cockpit and up to three
/// passengers' heads in the portholes.
pub(super) fn ship(world: &mut World, position: Vec3, hero: Entity, passengers: u8) -> Entity {
    let root = world
        .spawn((
            Transform::from_translation(position).with_scale(Vec3::splat(ROCKET_SCALE)),
            Visibility::Visible,
        ))
        .id();
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
    // The passengers' face (set when they land, too).
    let texture = world
        .resource::<crate::resources::asset_handles::GameTextures>()
        .villager
        .clone();
    let villager = world.resource::<RocketAssets>().villager.clone();
    if let Some(mut m) = world
        .resource_mut::<Assets<StandardMaterial>>()
        .get_mut(&villager)
    {
        m.base_color_texture = Some(texture);
    }
    let art = world.resource::<RocketAssets>().rocket.clone();
    let faces = vec![villager; passengers.min(3) as usize];
    crate::systems::block_rocket::build(world, root, &art, None, hero_face, &faces);
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
