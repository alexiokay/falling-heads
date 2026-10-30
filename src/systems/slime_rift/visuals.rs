use super::*;
#[derive(Component)]
struct TransitionVeil;
#[derive(Resource)]
pub(super) struct Art {
    cube: Handle<Mesh>,
    quad: Handle<Mesh>,
    slime: Handle<StandardMaterial>,
    stone: Handle<StandardMaterial>,
    moss: Handle<StandardMaterial>,
    glow: Handle<StandardMaterial>,
    crystal: Handle<StandardMaterial>,
    bark: Handle<StandardMaterial>,
    sound: Handle<AudioSource>,
    last_support: bool,
}
pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    server: Res<AssetServer>,
) {
    let mut textured = |path: &str, color: Color| {
        mats.add(StandardMaterial {
            base_color_texture: Some(
                server
                    .load_builder()
                    .with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
                        s.sampler = bevy::image::ImageSampler::nearest();
                    })
                    .load(path.to_string()),
            ),
            base_color: color,
            perceptual_roughness: 0.8,
            ..default()
        })
    };
    let slime = textured(
        "textures/biome_content/slime_block.png",
        Color::srgb(0.65, 1., 0.48),
    );
    let stone = textured(
        "textures/biome_content/mossy_stone_bricks.png",
        Color::srgb(0.58, 0.65, 0.64),
    );
    let moss = textured("textures/biome_content/moss_block.png", Color::WHITE);
    let crystal = textured(
        "textures/biome_content/amethyst_block.png",
        Color::srgb(0.8, 0.65, 1.),
    );
    let glow = mats.add(StandardMaterial {
        base_color: Color::srgb(0.55, 1., 0.38),
        emissive: LinearRgba::rgb(1.5, 3., 0.6),
        unlit: true,
        ..default()
    });
    let bark = mats.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.075, 0.04),
        perceptual_roughness: 1.,
        ..default()
    });
    commands.insert_resource(Art {
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        quad: meshes.add(Rectangle::new(1., 1.)),
        slime,
        stone,
        moss,
        glow,
        crystal,
        bark,
        sound: server.load("audio/biome_content/slime_jump.ogg"),
        last_support: true,
    });
}
#[allow(clippy::too_many_arguments)] // One explicit primitive constructor keeps scene ownership and layer assignment visible.
fn cube(
    w: &mut World,
    parent: Entity,
    p: Vec3,
    size: Vec3,
    mesh: Handle<Mesh>,
    mat: Handle<StandardMaterial>,
    layer: usize,
) {
    w.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(p).with_scale(size),
        RenderLayers::layer(layer),
        ChildOf(parent),
    ));
}
pub(super) fn gate(w: &mut World, p: Vec3, mini: bool) -> Entity {
    let layer = usize::from(mini);
    let art = w.resource::<Art>();
    let mesh = art.cube.clone();
    let stone = art.stone.clone();
    let glow = art.glow.clone();
    let root = w
        .spawn((Transform::from_translation(p), Visibility::Inherited, Gate))
        .id();
    if mini {
        w.entity_mut(root).insert(Scene);
    }
    // Segmented stone arch with an open centre, no rectangular screen overlay.
    for i in 0..20 {
        let angle = i as f32 / 20. * std::f32::consts::TAU;
        let c = Vec3::new(angle.cos() * 86., angle.sin() * 106., 0.);
        cube(
            w,
            root,
            c,
            Vec3::new(23., 23., 32.),
            mesh.clone(),
            stone.clone(),
            layer,
        );
        cube(
            w,
            root,
            c * 0.82 + Vec3::Z * 18.,
            Vec3::splat(8.),
            mesh.clone(),
            glow.clone(),
            layer,
        );
    }
    root
}
pub(super) fn scene(w: &mut World, origin: Vec3, hero_start: Vec3) {
    w.resource_mut::<Art>().last_support = true;
    let art = w.resource::<Art>();
    let mesh = art.cube.clone();
    let quad = art.quad.clone();
    let stone = art.stone.clone();
    let slime = art.slime.clone();
    let moss = art.moss.clone();
    let gem = art.crystal.clone();
    let bark = art.bark.clone();
    let glow = art.glow.clone();
    for i in 0..COUNT {
        let pad = course::pad(i, 0.);
        let rim = if pad.moving {
            gem.clone()
        } else {
            glow.clone()
        };
        let root = w
            .spawn((
                Scene,
                PadVisual(i),
                Transform::from_translation(origin + pad.center),
                Visibility::Inherited,
            ))
            .id();
        cube(
            w,
            root,
            Vec3::new(0., -30., 0.),
            Vec3::new(pad.half.x * 2., 52., pad.half.y * 2.),
            mesh.clone(),
            stone.clone(),
            1,
        );
        cube(
            w,
            root,
            Vec3::new(0., -5., 0.),
            Vec3::new(pad.half.x * 2. - 8., 12., pad.half.y * 2. - 8.),
            mesh.clone(),
            slime.clone(),
            1,
        );
        cube(
            w,
            root,
            Vec3::new(0., -74., 0.),
            Vec3::new(pad.half.x * 1.5, 40., pad.half.y * 1.45),
            mesh.clone(),
            moss.clone(),
            1,
        );
        cube(
            w,
            root,
            Vec3::new(13., -112., -7.),
            Vec3::new(pad.half.x, 36., pad.half.y),
            mesh.clone(),
            stone.clone(),
            1,
        );
        for x in [-1., 1.] {
            for z in [-1., 1.] {
                cube(
                    w,
                    root,
                    Vec3::new(x * (pad.half.x - 12.), 5., z * (pad.half.y - 12.)),
                    Vec3::new(12., 8., 12.),
                    mesh.clone(),
                    rim.clone(),
                    1,
                );
            }
        }
        if i > 0 {
            w.spawn((
                Scene,
                Gem(i),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(gem.clone()),
                Transform::from_translation(origin + pad.center + Vec3::Y * 78.)
                    .with_scale(Vec3::splat(26.)),
                RenderLayers::layer(1),
            ));
        }
        // Dotted route ribbons describe the depth change between landings.
        if i < COUNT - 1 {
            for j in 1..7 {
                let t = j as f32 / 7.;
                let next = course::pad(i + 1, 0.).center;
                let p = origin
                    + pad.center.lerp(next, t)
                    + Vec3::Y * (70. + (t * std::f32::consts::PI).sin() * 210.);
                w.spawn((
                    Scene,
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(glow.clone()),
                    Transform::from_translation(p).with_scale(Vec3::splat(4.)),
                    RenderLayers::layer(1),
                ));
            }
        }
    }
    for i in COUNT..COUNT + 4 {
        w.spawn((
            Scene,
            Gem(i),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(gem.clone()),
            Transform::from_translation(origin + course::gem_position(i, 0.))
                .with_scale(Vec3::splat(32.)),
            RenderLayers::layer(1),
        ));
    }
    gate(
        w,
        origin + course::pad(COUNT - 1, 0.).center + Vec3::new(60., 105., -70.),
        true,
    );
    let runner = w
        .spawn((
            Scene,
            Runner,
            Transform::from_translation(hero_start),
            Visibility::Inherited,
        ))
        .id();
    cube(
        w,
        runner,
        Vec3::ZERO,
        Vec3::splat(52.),
        mesh.clone(),
        bark,
        1,
    );
    let texture = w
        .resource::<crate::resources::asset_handles::GameTextures>()
        .hero
        .clone();
    let face = w
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color_texture: Some(texture),
            unlit: true,
            alpha_mode: AlphaMode::Mask(0.4),
            ..default()
        });
    w.spawn((
        Mesh3d(quad),
        MeshMaterial3d(face),
        Transform::from_xyz(0., 0., 26.2).with_scale(Vec3::new(52., 52., 1.)),
        RenderLayers::layer(1),
        ChildOf(runner),
    ));
    w.spawn((
        Scene,
        DirectionalLight {
            illuminance: 2600.,
            color: Color::srgb(0.9, 1., 0.85),
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.45, 0.)),
        RenderLayers::layer(1),
    ));
    // Dedicated lights don't illuminate the paused main world.
    for (offset, color, intensity) in [
        (
            Vec3::new(600., 1000., 800.),
            Color::srgb(0.8, 1., 0.8),
            700_000_000.,
        ),
        (
            Vec3::new(1700., 650., -600.),
            Color::srgb(0.45, 0.65, 1.),
            440_000_000.,
        ),
    ] {
        w.spawn((
            Scene,
            PointLight {
                color,
                intensity,
                range: 4500.,
                radius: 90.,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(origin + offset),
            RenderLayers::layer(1),
        ));
    }
    // No artificial backdrop: the camera also renders the real world (layer 0)
    // behind the course.
    w.spawn((
        Scene,
        TransitionVeil,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..default()
        },
        BackgroundColor(Color::NONE),
        GlobalZIndex(200),
        bevy::picking::Pickable::IGNORE,
    ));
    w.spawn((
        Scene,
        Hud,
        Text::new("SLIME RIFT"),
        TextFont {
            font_size: FontSize::Px(22.),
            ..default()
        },
        TextColor(Color::srgb(0.82, 1., 0.73)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(28.),
            top: Val::Px(24.),
            padding: UiRect::all(Val::Px(16.)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.045, 0.04, 0.88)),
        GlobalZIndex(150),
    ));
}
pub(super) fn animate(
    w: &mut World,
    origin: Vec3,
    c: &Course,
    p: Vec3,
    paused: bool,
    returning: bool,
) {
    for (pad, mut t) in w.query::<(&PadVisual, &mut Transform)>().iter_mut(w) {
        t.translation = origin + course::pad(pad.0, c.frame.time).center;
    }
    for (gem, mut t, mut v) in w
        .query::<(&Gem, &mut Transform, &mut Visibility)>()
        .iter_mut(w)
    {
        t.translation = origin + course::gem_position(gem.0, c.frame.time);
        t.rotation = Quat::from_rotation_y(c.frame.time) * Quat::from_rotation_z(0.6);
        *v = if c.frame.gems & (1 << gem.0) != 0 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for mut t in w
        .query_filtered::<&mut Transform, With<Runner>>()
        .iter_mut(w)
    {
        t.translation = p;
        let squash = if c.frame.support.is_some() && !returning {
            (c.frame.hold / 0.32).clamp(0., 1.)
        } else {
            0.
        };
        t.scale = Vec3::new(1. + squash * 0.12, 1. - squash * 0.22, 1. + squash * 0.12);
        t.translation.y -= course::RADIUS * squash * 0.22;
        t.rotation = Quat::from_rotation_y(-0.2 + c.frame.v.z * 0.0005);
    }
    let held = c.frame.support.is_some();
    if w.resource::<Art>().last_support && !held && !c.rewinding && !returning {
        let sound = w.resource::<Art>().sound.clone();
        w.spawn((
            AudioPlayer::new(sound),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.25)),
        ));
    }
    w.resource_mut::<Art>().last_support = held;
    for mut text in w.query_filtered::<&mut Text, With<Hud>>().iter_mut(w) {
        **text = format!(
            "SLIME RIFT  |  {}/12 crystals\n{}\nA/D: steer across  |  W/S: depth  |  R: rewind  |  P: pause  |  Esc: leave",
            c.frame.gems.count_ones(),
            if returning {
                "Returning to your world".into()
            } else if paused {
                "Paused - press P to continue".into()
            } else if c.rewinding {
                "REWINDING - movement, platforms and crystals".into()
            } else {
                format!(
                    "Steer for 4 bonus crystals  |  {}  |  checkpoint {}/9",
                    if c.used_rewind {
                        "rewind used"
                    } else {
                        "5-second rewind ready"
                    },
                    c.frame.checkpoint + 1
                )
            }
        );
    }
}

pub(super) fn transition(w: &mut World, opacity: f32) {
    for mut color in w
        .query_filtered::<&mut BackgroundColor, With<TransitionVeil>>()
        .iter_mut(w)
    {
        color.0 = Color::srgba(0.02, 0.025, 0.04, opacity.clamp(0., 1.));
    }
}
