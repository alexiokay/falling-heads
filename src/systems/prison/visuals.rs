//! The prison scene: a spinning bedrock cell with trapdoors and buttons, four
//! fixed electromagnets around it (iron core, copper coils, steel mount, cable;
//! the active one glows and throws electric arcs), heads, guards with batons,
//! blast flashes and debris, Earth below, a starfield, the escape rocket, the HUD.
//! Everything is unlit and on render layer 1, which the camera alone shows.
use super::sim::{self, Cell, Event, Kind};
use super::*;

/// Earth, far below and behind the prison (relative to the scene origin).
pub(super) const EARTH_CENTER: Vec3 = Vec3::new(500., -2700., -1800.);
pub(super) const EARTH_RADIUS: f32 = 1750.;
/// Pole face distance of each electromagnet from the cell centre; the spinning
/// cell's corners reach about 543.
const MAGNET_POLE: f32 = 600.;

#[derive(Resource, Clone)]
pub(super) struct Art {
    cube: Handle<Mesh>,
    quad: Handle<Mesh>,
    cylinder: Handle<Mesh>,
    cone: Handle<Mesh>,
    sphere: Handle<Mesh>,
    bedrock: Handle<StandardMaterial>,
    backwall: Handle<StandardMaterial>,
    trapdoor: Handle<StandardMaterial>,
    button: Handle<StandardMaterial>,
    button_pressed: Handle<StandardMaterial>,
    guard: Handle<StandardMaterial>,
    villager: Handle<StandardMaterial>,
    stars: Handle<StandardMaterial>,
    earth: Handle<StandardMaterial>,
    air: Handle<StandardMaterial>,
    flash: Handle<StandardMaterial>,
    debris: Handle<StandardMaterial>,
    steel: Handle<StandardMaterial>,
    bolt: Handle<StandardMaterial>,
    iron: Handle<StandardMaterial>,
    copper: Handle<StandardMaterial>,
    copper_dark: Handle<StandardMaterial>,
    pole: Handle<StandardMaterial>,
    pole_hot: Handle<StandardMaterial>,
    spark: Handle<StandardMaterial>,
    cable: Handle<StandardMaterial>,
    baton: Handle<StandardMaterial>,
    body: Handle<StandardMaterial>,
    hull: Handle<StandardMaterial>,
    red: Handle<StandardMaterial>,
    flame: Handle<StandardMaterial>,
    heat: Handle<StandardMaterial>,
    boom: [Handle<AudioSource>; 4],
    click: Handle<AudioSource>,
    bell: Handle<AudioSource>,
    hurt: Handle<AudioSource>,
    zap: Handle<AudioSource>,
}

/// The cell, turned with the simulation's angle.
#[derive(Component)]
struct CellRoot;
#[derive(Component)]
struct DoorTile(usize);
#[derive(Component)]
struct ButtonTile(usize);
/// The pole face of electromagnet `0` (glows while it pulls).
#[derive(Component)]
struct MagnetPole(usize);
/// One straight piece of an electric arc: magnet, arc, segment.
#[derive(Component)]
struct ArcSeg(usize, usize, usize);
#[derive(Component)]
struct HeadVisual(usize);
#[derive(Component)]
struct Baton;
#[derive(Component)]
struct Flash(f32);
#[derive(Component)]
struct Debris {
    v: Vec3,
    age: f32,
}
#[derive(Component)]
struct Earth;
/// An electromagnet's group (hidden once the escape rocket has left).
#[derive(Component)]
struct MagnetRoot;
#[derive(Component)]
struct Rocket;
#[derive(Component)]
struct RocketFlame;
#[derive(Component)]
struct RocketHeat;
#[derive(Component)]
struct Hud;
#[derive(Component)]
struct Veil;

/// The hero's face material for this visit.
#[derive(Resource)]
struct HeroFace(Handle<StandardMaterial>);

const ARCS: usize = 3;
const ARC_SEGMENTS: usize = 5;

pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    server: Res<AssetServer>,
    textures: Option<Res<crate::resources::asset_handles::GameTextures>>,
) {
    let pixel = |path: &str| -> Handle<Image> {
        server
            .load_builder()
            .with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
                s.sampler = bevy::image::ImageSampler::nearest();
            })
            .load(path.to_string())
    };
    let mut flat = |texture: Option<Handle<Image>>, color: Color| {
        mats.add(StandardMaterial {
            base_color_texture: texture,
            base_color: color,
            unlit: true,
            alpha_mode: AlphaMode::Mask(0.4),
            ..default()
        })
    };
    let bedrock = flat(Some(pixel("textures/prison/bedrock.png")), Color::WHITE);
    let backwall = flat(
        Some(pixel("textures/prison/bedrock.png")),
        Color::srgb(0.42, 0.42, 0.48),
    );
    let trapdoor = flat(Some(pixel("textures/prison/trapdoor.png")), Color::WHITE);
    let button = flat(Some(pixel("textures/prison/button.png")), Color::WHITE);
    let button_pressed = flat(
        Some(pixel("textures/prison/button_pressed.png")),
        Color::WHITE,
    );
    let guard = flat(Some(pixel("textures/prison/guard.png")), Color::WHITE);
    let villager = flat(
        textures
            .as_ref()
            .map(|t| t.villager.clone())
            .or_else(|| Some(server.load("textures/entities/villager.png"))),
        Color::WHITE,
    );
    let stars = flat(Some(server.load("textures/prison/stars.png")), Color::WHITE);
    let earth = flat(Some(server.load("textures/prison/earth.png")), Color::WHITE);
    let steel = flat(None, Color::srgb(0.26, 0.28, 0.32));
    let bolt = flat(None, Color::srgb(0.6, 0.62, 0.66));
    let iron = flat(None, Color::srgb(0.45, 0.46, 0.5));
    let copper = flat(None, Color::srgb(0.85, 0.5, 0.22));
    let copper_dark = flat(None, Color::srgb(0.55, 0.29, 0.12));
    let pole = flat(None, Color::srgb(0.7, 0.72, 0.78));
    let cable = flat(None, Color::srgb(0.08, 0.08, 0.09));
    let baton = flat(None, Color::srgb(0.12, 0.1, 0.1));
    let body = flat(None, Color::srgb(0.2, 0.2, 0.24));
    let debris = flat(None, Color::srgb(0.3, 0.3, 0.33));
    let hull = flat(None, Color::srgb(0.93, 0.93, 0.95));
    let red = flat(None, Color::srgb(0.85, 0.15, 0.12));
    let mut glow = |color: Color, emissive: LinearRgba| {
        mats.add(StandardMaterial {
            base_color: color,
            emissive,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })
    };
    let air = glow(Color::srgba(0.45, 0.7, 1., 0.18), LinearRgba::BLACK);
    let flash = glow(
        Color::srgba(1., 0.75, 0.3, 0.75),
        LinearRgba::rgb(4., 2.2, 0.6),
    );
    let pole_hot = glow(Color::srgb(0.7, 0.95, 1.), LinearRgba::rgb(2., 4., 6.));
    let spark = glow(Color::srgb(0.75, 0.95, 1.), LinearRgba::rgb(3., 6., 9.));
    let flame = glow(
        Color::srgba(1., 0.6, 0.15, 0.85),
        LinearRgba::rgb(5., 2.2, 0.3),
    );
    let heat = glow(
        Color::srgba(1., 0.45, 0.1, 0.5),
        LinearRgba::rgb(4., 1.4, 0.2),
    );
    commands.insert_resource(Art {
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        quad: meshes.add(Rectangle::new(1., 1.)),
        cylinder: meshes.add(Cylinder::new(0.5, 1.)),
        cone: meshes.add(Cone {
            radius: 0.5,
            height: 1.,
        }),
        sphere: meshes.add(Sphere::new(1.).mesh().uv(48, 24)),
        bedrock,
        backwall,
        trapdoor,
        button,
        button_pressed,
        guard,
        villager,
        stars,
        earth,
        air,
        flash,
        debris,
        steel,
        bolt,
        iron,
        copper,
        copper_dark,
        pole,
        pole_hot,
        spark,
        cable,
        baton,
        body,
        hull,
        red,
        flame,
        heat,
        boom: std::array::from_fn(|i| {
            server.load(format!("audio/sfx/explode/explode{}.ogg", i + 1))
        }),
        click: server.load("audio/mechanisms/click.ogg"),
        bell: server.load("audio/mechanisms/bell_use1.ogg"),
        hurt: server.load("audio/sfx/mobs/zombie/hurt1.ogg"),
        zap: server.load("audio/mechanisms/bell_resonate.ogg"),
    });
}

fn mesh(
    w: &mut World,
    parent: Option<Entity>,
    mesh: &Handle<Mesh>,
    mat: &Handle<StandardMaterial>,
    t: Transform,
    extra: impl Bundle,
) -> Entity {
    let mut e = w.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(mat.clone()),
        t,
        RenderLayers::layer(1),
        extra,
    ));
    if let Some(parent) = parent {
        e.insert(ChildOf(parent));
    } else {
        e.insert(Scene);
    }
    e.id()
}

fn group(w: &mut World, t: Transform, extra: impl Bundle) -> Entity {
    w.spawn((Scene, t, Visibility::Inherited, extra)).id()
}

/// An electromagnet seen from the side, pointing at the cell along `dir`:
/// steel mount with bolts and a cable, iron core wrapped in copper windings,
/// a polished pole face. Built along local +X, then turned to `dir`.
fn electromagnet(w: &mut World, a: &Art, origin: Vec3, index: usize, dir: Vec2) {
    let root = group(
        w,
        Transform::from_translation(origin)
            .with_rotation(Quat::from_rotation_z(dir.y.atan2(dir.x))),
        MagnetRoot,
    );
    let at = |d: f32, size: Vec3| Transform::from_xyz(d, 0., 0.).with_scale(size);
    // Mount plate and its bolts.
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.steel,
        at(MAGNET_POLE + 200., Vec3::new(28., 170., 90.)),
        (),
    );
    for y in [-62., 62.] {
        mesh(
            w,
            Some(root),
            &a.cube,
            &a.bolt,
            Transform::from_xyz(MAGNET_POLE + 200., y, 46.).with_scale(Vec3::new(12., 12., 4.)),
            (),
        );
    }
    // Cable out of the back, with a kink.
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.cable,
        at(MAGNET_POLE + 260., Vec3::new(100., 12., 12.)),
        (),
    );
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.cable,
        Transform::from_xyz(MAGNET_POLE + 310., 40., 0.).with_scale(Vec3::new(12., 92., 12.)),
        (),
    );
    // Iron core and the copper windings around it.
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.iron,
        at(MAGNET_POLE + 95., Vec3::new(190., 46., 60.)),
        (),
    );
    for i in 0..10 {
        let mat = if i % 2 == 0 {
            &a.copper
        } else {
            &a.copper_dark
        };
        mesh(
            w,
            Some(root),
            &a.cube,
            mat,
            at(MAGNET_POLE + 34. + i as f32 * 14., Vec3::new(12., 92., 96.)),
            (),
        );
    }
    // Pole face toward the cell.
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.pole,
        at(MAGNET_POLE + 10., Vec3::new(20., 74., 80.)),
        MagnetPole(index),
    );
    for arc in 0..ARCS {
        for seg in 0..ARC_SEGMENTS {
            mesh(
                w,
                None,
                &a.cube,
                &a.spark,
                Transform::from_translation(origin),
                (ArcSeg(index, arc, seg), Visibility::Hidden),
            );
        }
    }
}

pub(super) fn scene(w: &mut World, origin: Vec3, hero: Entity) {
    let a = w.resource::<Art>().clone();
    let root = group(w, Transform::from_translation(origin), CellRoot);
    let step = sim::WALL;
    let ring = sim::HALF + step / 2.;
    // Bedrock ring with the three trapdoors left out.
    let tiles = ((sim::HALF * 2. + step * 2.) / step) as i32;
    for i in 0..tiles {
        let along = -sim::HALF - step / 2. + i as f32 * step;
        for normal in [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y] {
            let side = Vec2::new(-normal.y, normal.x);
            let p = normal * ring + side * along;
            let door = [Vec2::Y, Vec2::NEG_X, Vec2::X]
                .iter()
                .position(|n| *n == normal)
                .filter(|_| along.abs() < sim::DOOR_HALF);
            let t = Transform::from_translation(p.extend(0.)).with_scale(Vec3::splat(step));
            match door {
                Some(d) => mesh(w, Some(root), &a.cube, &a.trapdoor, t, DoorTile(d)),
                None => mesh(w, Some(root), &a.cube, &a.bedrock, t, ()),
            };
        }
    }
    // Back wall, a shade darker, so the room reads as a room.
    let n = (sim::HALF * 2. / step) as i32;
    for x in 0..n {
        for y in 0..n {
            let p = Vec2::new(
                -sim::HALF + step / 2. + x as f32 * step,
                -sim::HALF + step / 2. + y as f32 * step,
            );
            mesh(
                w,
                Some(root),
                &a.quad,
                &a.backwall,
                Transform::from_translation(p.extend(-step / 2.)).with_scale(Vec3::splat(step)),
                (),
            );
        }
    }
    // Buttons: chunky bumps on the walls' inner faces, facing the camera.
    let buttons = sim::Cell::new(1, 0).buttons;
    for (i, b) in buttons.iter().enumerate() {
        let size = if b.normal.x != 0. {
            Vec3::new(26., 56., 56.)
        } else {
            Vec3::new(56., 26., 56.)
        };
        mesh(
            w,
            Some(root),
            &a.cube,
            &a.button,
            Transform::from_translation((b.at - b.normal * 12.).extend(0.)).with_scale(size),
            ButtonTile(i),
        );
    }
    // Four fixed electromagnets around the spinning cell.
    for (i, dir) in sim::MAGNETS.iter().enumerate() {
        electromagnet(w, &a, origin, i, *dir);
    }
    // Earth below, with a thin atmosphere, turning slowly.
    let earth = group(
        w,
        Transform::from_translation(origin + EARTH_CENTER)
            .with_rotation(Quat::from_rotation_z(0.35)),
        Earth,
    );
    mesh(
        w,
        Some(earth),
        &a.sphere,
        &a.earth,
        Transform::from_scale(Vec3::splat(EARTH_RADIUS)),
        (),
    );
    mesh(
        w,
        Some(earth),
        &a.sphere,
        &a.air,
        Transform::from_scale(Vec3::splat(EARTH_RADIUS * 1.04)),
        (),
    );
    // Starfield far behind.
    for x in -5..5 {
        for y in -5..5 {
            mesh(
                w,
                None,
                &a.quad,
                &a.stars,
                Transform::from_translation(
                    origin + Vec3::new(x as f32 * 1800. + 900., y as f32 * 1800. + 900., -4200.),
                )
                .with_scale(Vec3::splat(1800.)),
                (),
            );
        }
    }
    let face = {
        let texture = w
            .get::<MeshMaterial3d<StandardMaterial>>(hero)
            .and_then(|m| w.resource::<Assets<StandardMaterial>>().get(&m.0))
            .and_then(|m| m.base_color_texture.clone());
        w.resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color_texture: texture,
                unlit: true,
                alpha_mode: AlphaMode::Mask(0.4),
                ..default()
            })
    };
    w.insert_resource(HeroFace(face));
    w.spawn((
        Scene,
        Hud,
        Text::new("SPACE PRISON"),
        TextFont {
            font_size: FontSize::Px(20.),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.9, 1.)),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.),
            left: Val::Percent(50.),
            margin: UiRect::left(Val::Px(-260.)),
            width: Val::Px(520.),
            padding: UiRect::all(Val::Px(10.)),
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.08, 0.82)),
        GlobalZIndex(150),
    ));
    w.spawn((
        Scene,
        Veil,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        GlobalZIndex(200),
        bevy::picking::Pickable::IGNORE,
    ));
}

fn spawn_head(w: &mut World, root: Entity, index: usize, kind: Kind) {
    let a = w.resource::<Art>().clone();
    let face = match kind {
        Kind::Hero => w.resource::<HeroFace>().0.clone(),
        Kind::Villager => a.villager.clone(),
        Kind::Guard => a.guard.clone(),
    };
    let head = w
        .spawn((
            HeadVisual(index),
            Transform::from_xyz(0., 0., 30.),
            Visibility::Inherited,
            ChildOf(root),
        ))
        .id();
    mesh(
        w,
        Some(head),
        &a.cube,
        &a.body,
        Transform::from_scale(Vec3::splat(50.)),
        (),
    );
    mesh(
        w,
        Some(head),
        &a.quad,
        &face,
        Transform::from_xyz(0., 0., 25.5).with_scale(Vec3::new(52., 52., 1.)),
        (),
    );
    if kind == Kind::Guard {
        mesh(
            w,
            Some(head),
            &a.cube,
            &a.baton,
            Transform::from_xyz(38., 0., 20.).with_scale(Vec3::new(8., 56., 8.)),
            Baton,
        );
    }
}

/// The escape rocket: white hull, red nose and fins, the hero's face in the
/// window (saved villagers in the small ones), a flame and a re-entry glow.
/// Built pointing up (+Y); returns its root.
pub(super) fn spawn_rocket(w: &mut World, at: Vec3, saved: u32) -> Entity {
    let a = w.resource::<Art>().clone();
    let face = w.resource::<HeroFace>().0.clone();
    let root = group(w, Transform::from_translation(at), Rocket);
    mesh(
        w,
        Some(root),
        &a.cylinder,
        &a.hull,
        Transform::from_scale(Vec3::new(64., 140., 64.)),
        (),
    );
    mesh(
        w,
        Some(root),
        &a.cone,
        &a.red,
        Transform::from_xyz(0., 100., 0.).with_scale(Vec3::new(64., 60., 64.)),
        (),
    );
    for side in [-1., 1.] {
        mesh(
            w,
            Some(root),
            &a.cube,
            &a.red,
            Transform::from_xyz(side * 38., -52., 0.)
                .with_rotation(Quat::from_rotation_z(side * 0.35))
                .with_scale(Vec3::new(16., 52., 20.)),
            (),
        );
    }
    // Window frame and the hero looking out.
    mesh(
        w,
        Some(root),
        &a.quad,
        &a.steel,
        Transform::from_xyz(0., 28., 32.5).with_scale(Vec3::splat(46.)),
        (),
    );
    mesh(
        w,
        Some(root),
        &a.quad,
        &face,
        Transform::from_xyz(0., 28., 33.).with_scale(Vec3::splat(38.)),
        (),
    );
    for i in 0..saved.min(2) {
        let x = if i == 0 { -14. } else { 14. };
        mesh(
            w,
            Some(root),
            &a.quad,
            &a.villager,
            Transform::from_xyz(x, -18., 33.).with_scale(Vec3::splat(22.)),
            (),
        );
    }
    // Exhaust flame pointing down, and the re-entry glow around the nose.
    mesh(
        w,
        Some(root),
        &a.cone,
        &a.flame,
        Transform::from_xyz(0., -110., 0.)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::PI))
            .with_scale(Vec3::new(44., 80., 44.)),
        RocketFlame,
    );
    mesh(
        w,
        Some(root),
        &a.sphere,
        &a.heat,
        Transform::from_xyz(0., 90., 0.).with_scale(Vec3::ZERO),
        RocketHeat,
    );
    root
}

/// Poses the escape rocket: position, heading (direction of travel), flame
/// length and re-entry heat (0..1).
pub(super) fn pose_rocket(w: &mut World, at: Vec3, heading: Vec3, flame: f32, heat: f32) {
    let up = heading.normalize_or(Vec3::Y);
    for mut t in w
        .query_filtered::<&mut Transform, With<Rocket>>()
        .iter_mut(w)
    {
        t.translation = at;
        t.rotation = Quat::from_rotation_arc(Vec3::Y, up);
    }
    let flicker = 1. + (w.resource::<Time<Real>>().elapsed_secs() * 40.).sin() * 0.15;
    for mut t in w
        .query_filtered::<&mut Transform, With<RocketFlame>>()
        .iter_mut(w)
    {
        let length = 30. + 140. * flame * flicker;
        t.translation.y = -70. - length / 2.;
        t.scale = Vec3::new(44. * (0.6 + flame * 0.4), length, 44.);
    }
    for mut t in w
        .query_filtered::<&mut Transform, With<RocketHeat>>()
        .iter_mut(w)
    {
        t.scale = Vec3::splat(heat * 110. * flicker);
    }
}

fn play(w: &mut World, clip: Handle<AudioSource>, volume: f32) {
    w.spawn((
        Scene,
        AudioPlayer::new(clip),
        PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(volume)),
    ));
}

/// A cheap, stable pseudo-random value in -1..1 for arc jitter.
fn jitter(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9e37_79b9) ^ b.wrapping_mul(0x85eb_ca6b);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2c1b_3c6d);
    x ^= x >> 12;
    (x & 0xffff) as f32 / 32767.5 - 1.
}

#[allow(clippy::too_many_arguments)] // One call per frame from the prison update.
pub(super) fn animate(
    w: &mut World,
    origin: Vec3,
    cell: &Cell,
    events: &[Event],
    villagers: usize,
    veil: Color,
    ride: Option<f32>,
    magnet_was: Option<usize>,
) {
    let escaping = ride.is_some();
    // Once the rocket dives, the camera follows it past the prison: hide the
    // cell and the magnets so they do not fill the view up close.
    let gone = ride.is_some_and(|t| t > 1.6);
    for mut v in w
        .query_filtered::<&mut Visibility, Or<(With<CellRoot>, With<MagnetRoot>)>>()
        .iter_mut(w)
    {
        *v = if gone {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    let a = w.resource::<Art>().clone();
    let Some(root) = w.query_filtered::<Entity, With<CellRoot>>().iter(w).next() else {
        return;
    };
    let turn = Quat::from_rotation_z(cell.angle);
    if let Some(mut t) = w.get_mut::<Transform>(root) {
        t.translation = origin;
        t.rotation = turn;
    }
    // Trapdoors vanish while open; buttons light up when pressed.
    for (tile, mut v) in w.query::<(&DoorTile, &mut Visibility)>().iter_mut(w) {
        let open = cell.doors[tile.0].open > 0.;
        *v = if open {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for (tile, mut m) in w
        .query::<(&ButtonTile, &mut MeshMaterial3d<StandardMaterial>)>()
        .iter_mut(w)
    {
        let mat = if cell.buttons[tile.0].lit > 0. {
            &a.button_pressed
        } else {
            &a.button
        };
        if m.0 != *mat {
            m.0 = mat.clone();
        }
    }
    // The pulling magnet: hot pole face and flickering arcs to the cell.
    if cell.magnet.is_some() && cell.magnet != magnet_was {
        play(w, a.zap.clone(), 0.35);
    }
    for (pole, mut m) in w
        .query::<(&MagnetPole, &mut MeshMaterial3d<StandardMaterial>)>()
        .iter_mut(w)
    {
        let mat = if cell.magnet == Some(pole.0) {
            &a.pole_hot
        } else {
            &a.pole
        };
        if m.0 != *mat {
            m.0 = mat.clone();
        }
    }
    let frame = (cell.time * 24.) as u32;
    for (seg, mut t, mut v) in w
        .query::<(&ArcSeg, &mut Transform, &mut Visibility)>()
        .iter_mut(w)
    {
        let ArcSeg(m, arc, i) = *seg;
        if cell.magnet != Some(m) || gone {
            *v = Visibility::Hidden;
            continue;
        }
        *v = Visibility::Inherited;
        let dir = sim::MAGNETS[m];
        let side = Vec2::new(-dir.y, dir.x);
        let start = dir * MAGNET_POLE + side * (arc as f32 - 1.) * 22.;
        let end = dir * 500. + side * jitter(frame, arc as u32 * 31) * 160.;
        let point = |k: usize| {
            let f = k as f32 / ARC_SEGMENTS as f32;
            let wobble = if k == 0 || k == ARC_SEGMENTS {
                0.
            } else {
                jitter(frame + k as u32 * 7, (arc * 13 + m * 101) as u32) * 30.
            };
            start.lerp(end, f) + side * wobble
        };
        let (p0, p1) = (point(i), point(i + 1));
        let d = p1 - p0;
        t.translation = origin + ((p0 + p1) / 2.).extend(20.);
        t.rotation = Quat::from_rotation_z(d.y.atan2(d.x));
        t.scale = Vec3::new(d.length() + 4., 5., 5.);
    }
    // One visual per head, spawned when a guard first appears.
    let existing: Vec<usize> = w.query::<&HeadVisual>().iter(w).map(|h| h.0).collect();
    for (i, b) in cell.bodies.iter().enumerate() {
        if !existing.contains(&i) {
            spawn_head(w, root, i, b.kind);
        }
    }
    let spin = cell.time;
    for (h, mut t, mut v) in w
        .query::<(&HeadVisual, &mut Transform, &mut Visibility)>()
        .iter_mut(w)
    {
        let Some(b) = cell.bodies.get(h.0) else {
            continue;
        };
        t.translation = b.p.extend(30.);
        // Heads tumble with their speed but always show their face to the camera.
        t.rotation =
            turn.inverse() * Quat::from_rotation_z((b.v.x - b.v.y) * 0.0004 * (spin * 3.).sin());
        *v = if b.escaped {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for mut t in w
        .query_filtered::<&mut Transform, With<Baton>>()
        .iter_mut(w)
    {
        t.rotation = Quat::from_rotation_z((cell.time * 9.).sin() * 0.9);
    }
    for mut t in w
        .query_filtered::<&mut Transform, With<Earth>>()
        .iter_mut(w)
    {
        t.rotation = Quat::from_rotation_z(0.35) * Quat::from_rotation_y(cell.time * 0.05);
    }
    // Blasts: a swelling flash and a spray of bedrock chips.
    for e in events {
        match *e {
            Event::Blast(at) => {
                let i = (cell.time * 7.) as usize % 4;
                play(w, a.boom[i].clone(), 0.5);
                let world = origin + (turn * at.extend(40.));
                mesh(
                    w,
                    None,
                    &a.cube,
                    &a.flash,
                    Transform::from_translation(world).with_scale(Vec3::splat(40.)),
                    Flash(0.),
                );
                for k in 0..12 {
                    let angle = k as f32 / 12. * std::f32::consts::TAU + jitter(frame, k) * 0.3;
                    let speed = 500. + (jitter(k, frame) + 1.) * 350.;
                    mesh(
                        w,
                        None,
                        &a.cube,
                        &a.debris,
                        Transform::from_translation(world)
                            .with_scale(Vec3::splat(10. + (k % 3) as f32 * 4.)),
                        Debris {
                            v: Vec3::new(angle.cos(), angle.sin(), 0.) * speed,
                            age: 0.,
                        },
                    );
                }
            }
            Event::Button(_) => play(w, a.click.clone(), 0.8),
            Event::GuardIn(_) => play(w, a.bell.clone(), 0.6),
            Event::HeroHit => play(w, a.hurt.clone(), 0.7),
            Event::VillagerOut => play(w, a.bell.clone(), 0.4),
        }
    }
    let dt = w.resource::<Time<Real>>().delta_secs();
    let mut done = Vec::new();
    for (e, mut f, mut t) in w
        .query::<(Entity, &mut Flash, &mut Transform)>()
        .iter_mut(w)
    {
        f.0 += dt;
        t.scale = Vec3::splat(40. + f.0 * 900.);
        if f.0 > 0.3 {
            done.push(e);
        }
    }
    for (e, mut d, mut t) in w
        .query::<(Entity, &mut Debris, &mut Transform)>()
        .iter_mut(w)
    {
        d.age += dt;
        t.translation += d.v * dt;
        t.rotate_z(dt * 9.);
        t.scale *= 1. - dt * 1.5;
        if d.age > 0.7 {
            done.push(e);
        }
    }
    for e in done {
        w.despawn(e);
    }
    let left = (sim::TIME_LIMIT - cell.time).max(0.);
    let doors_open = cell.doors.iter().filter(|d| d.open > 0.).count();
    for mut text in w.query_filtered::<&mut Text, With<Hud>>().iter_mut(w) {
        **text = if escaping {
            "ESCAPED! Heading home to Earth...".into()
        } else {
            format!(
                "SPACE PRISON  {left:.0}s  |  villagers out {}/{villagers}  |  guards {}\n{}\nWASD / arrows: thrust  |  Esc: give up",
                cell.saved,
                cell.guards(),
                if doors_open > 0 {
                    "TRAPDOOR OPEN - GO GO GO!"
                } else {
                    "Hit a red button: it opens the trapdoor across the room"
                }
            )
        };
    }
    for mut c in w
        .query_filtered::<&mut BackgroundColor, With<Veil>>()
        .iter_mut(w)
    {
        c.0 = veil;
    }
}
