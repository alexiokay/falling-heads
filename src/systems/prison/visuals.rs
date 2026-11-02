//! The prison scene: a spinning bedrock cell with trapdoors and buttons, held
//! by four electromagnets on a space-station truss ring (solar wings, antenna,
//! blinking beacons); the active magnet glows, heats its coil, throws arcs and
//! a shader field beam. Heads, guards with batons, blast flashes and debris,
//! a lit Earth with a shader atmosphere, a starfield, the escape rocket, the HUD.
//! The station, cell and Earth are lit by the scene's own lights (they stay lit
//! whatever Graphics > Lighting says); faces and effects are unlit. Everything
//! is on render layer 1, which the camera alone shows.
use super::sim::{self, Cell, Event, Kind};
use super::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Earth, far below and behind the prison (relative to the scene origin).
pub(super) const EARTH_CENTER: Vec3 = Vec3::new(500., -2700., -1800.);
pub(super) const EARTH_RADIUS: f32 = 1750.;
/// Pole face distance of each electromagnet from the cell centre; the spinning
/// cell's corners reach about 543.
const MAGNET_POLE: f32 = 600.;

/// The prison's shader effects (`shaders/prison_field.wesl`): a magnet's field
/// beam, Earth's atmosphere rim, the containment band around the cell.
/// `params`: x time, y strength 0..1, z mode (0 beam, 1 atmosphere, 2 band).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub(crate) struct PrisonField {
    #[uniform(0)]
    pub params: Vec4,
}
impl Material for PrisonField {
    fn fragment_shader() -> ShaderRef {
        "shaders/prison_field.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

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
    flash: Handle<StandardMaterial>,
    debris: Handle<StandardMaterial>,
    steel: Handle<StandardMaterial>,
    bolt: Handle<StandardMaterial>,
    iron: Handle<StandardMaterial>,
    copper: Handle<StandardMaterial>,
    copper_dark: Handle<StandardMaterial>,
    copper_hot: Handle<StandardMaterial>,
    pole: Handle<StandardMaterial>,
    pole_hot: Handle<StandardMaterial>,
    yoke: Handle<StandardMaterial>,
    fin: Handle<StandardMaterial>,
    hazard: Handle<StandardMaterial>,
    truss: Handle<StandardMaterial>,
    node: Handle<StandardMaterial>,
    solar: Handle<StandardMaterial>,
    beacon: Handle<StandardMaterial>,
    lamp_idle: Handle<StandardMaterial>,
    lamp_on: Handle<StandardMaterial>,
    spark: Handle<StandardMaterial>,
    cable: Handle<StandardMaterial>,
    baton: Handle<StandardMaterial>,
    body: Handle<StandardMaterial>,
    hull: Handle<StandardMaterial>,
    red: Handle<StandardMaterial>,
    flame: Handle<StandardMaterial>,
    heat: Handle<StandardMaterial>,
    fields: [Handle<PrisonField>; 4],
    atmosphere: Handle<PrisonField>,
    containment: Handle<PrisonField>,
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
/// The pole shoe of electromagnet `0` (glows while it pulls).
#[derive(Component)]
struct MagnetPole(usize);
/// A coil winding of magnet `0` (`1`: the light-copper ones); glows hot while
/// the magnet pulls.
#[derive(Component)]
struct CoilWinding(usize, bool);
/// The status lamp of magnet `0`.
#[derive(Component)]
struct StatusLamp(usize);
/// A beacon on the station that blinks, with its phase in seconds.
#[derive(Component)]
struct Beacon(f32);
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
/// The station and its magnets (hidden once the escape rocket has left).
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

#[allow(clippy::too_many_arguments)] // Bevy system parameters
pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut fields: ResMut<Assets<PrisonField>>,
    mut keep: ResMut<crate::systems::graphics::KeepLit>,
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
    let baton = flat(None, Color::srgb(0.12, 0.1, 0.1));
    let body = flat(None, Color::srgb(0.2, 0.2, 0.24));
    // Lit surfaces: metal shines and shades under the scene's lights.
    let mut lit = |texture: Option<Handle<Image>>, color: Color, metal: f32, rough: f32| {
        let h = mats.add(StandardMaterial {
            base_color_texture: texture,
            base_color: color,
            metallic: metal,
            perceptual_roughness: rough,
            reflectance: 0.5,
            ..default()
        });
        keep.0.insert(h.id());
        h
    };
    let bedrock = lit(
        Some(pixel("textures/prison/bedrock.png")),
        Color::WHITE,
        0.,
        0.9,
    );
    let backwall = lit(
        Some(pixel("textures/prison/bedrock.png")),
        Color::srgb(0.5, 0.5, 0.56),
        0.,
        0.95,
    );
    let trapdoor = lit(
        Some(pixel("textures/prison/trapdoor.png")),
        Color::WHITE,
        0.6,
        0.45,
    );
    let earth = lit(
        Some(server.load("textures/prison/earth.png")),
        Color::WHITE,
        0.,
        0.7,
    );
    let hazard = lit(
        Some(pixel("textures/prison/hazard.png")),
        Color::WHITE,
        0.3,
        0.5,
    );
    let solar = lit(
        Some(pixel("textures/prison/solar.png")),
        Color::WHITE,
        0.4,
        0.25,
    );
    let steel = lit(None, Color::srgb(0.3, 0.32, 0.36), 0.9, 0.35);
    let bolt = lit(None, Color::srgb(0.75, 0.77, 0.8), 1., 0.25);
    let iron = lit(None, Color::srgb(0.5, 0.51, 0.55), 0.85, 0.4);
    let copper = lit(None, Color::srgb(0.95, 0.55, 0.3), 1., 0.28);
    let copper_dark = lit(None, Color::srgb(0.62, 0.32, 0.15), 1., 0.38);
    let pole = lit(None, Color::srgb(0.82, 0.84, 0.9), 1., 0.15);
    let yoke = lit(None, Color::srgb(0.2, 0.21, 0.24), 0.8, 0.45);
    let fin = lit(None, Color::srgb(0.6, 0.62, 0.66), 0.9, 0.3);
    let truss = lit(None, Color::srgb(0.66, 0.68, 0.72), 0.85, 0.35);
    let node = lit(None, Color::srgb(0.34, 0.36, 0.4), 0.8, 0.4);
    let cable = lit(None, Color::srgb(0.06, 0.06, 0.07), 0., 0.6);
    let debris = lit(None, Color::srgb(0.32, 0.32, 0.35), 0., 0.9);
    let hull = lit(None, Color::srgb(0.93, 0.93, 0.95), 0.3, 0.3);
    let red = lit(None, Color::srgb(0.85, 0.15, 0.12), 0.2, 0.4);
    let mut glow = |color: Color, emissive: LinearRgba, blend: bool| {
        mats.add(StandardMaterial {
            base_color: color,
            emissive,
            unlit: true,
            alpha_mode: if blend {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            ..default()
        })
    };
    let flash = glow(
        Color::srgba(1., 0.75, 0.3, 0.75),
        LinearRgba::rgb(4., 2.2, 0.6),
        true,
    );
    let pole_hot = glow(
        Color::srgb(0.7, 0.95, 1.),
        LinearRgba::rgb(2., 4., 6.),
        false,
    );
    let copper_hot = glow(
        Color::srgb(1., 0.6, 0.3),
        LinearRgba::rgb(3., 1.2, 0.3),
        false,
    );
    let spark = glow(
        Color::srgb(0.75, 0.95, 1.),
        LinearRgba::rgb(3., 6., 9.),
        true,
    );
    let beacon = glow(
        Color::srgb(1., 0.2, 0.15),
        LinearRgba::rgb(6., 0.6, 0.4),
        false,
    );
    let lamp_idle = glow(
        Color::srgb(0.3, 1., 0.4),
        LinearRgba::rgb(0.5, 2.5, 0.6),
        false,
    );
    let lamp_on = glow(
        Color::srgb(1., 0.25, 0.2),
        LinearRgba::rgb(6., 0.8, 0.5),
        false,
    );
    let flame = glow(
        Color::srgba(1., 0.6, 0.15, 0.85),
        LinearRgba::rgb(5., 2.2, 0.3),
        true,
    );
    let heat = glow(
        Color::srgba(1., 0.45, 0.1, 0.5),
        LinearRgba::rgb(4., 1.4, 0.2),
        true,
    );
    let field = |mode: f32| PrisonField {
        params: Vec4::new(0., 0., mode, 0.),
    };
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
        flash,
        debris,
        steel,
        bolt,
        iron,
        copper,
        copper_dark,
        copper_hot,
        pole,
        pole_hot,
        yoke,
        fin,
        hazard,
        truss,
        node,
        solar,
        beacon,
        lamp_idle,
        lamp_on,
        spark,
        cable,
        baton,
        body,
        hull,
        red,
        flame,
        heat,
        fields: std::array::from_fn(|_| fields.add(field(0.))),
        atmosphere: fields.add(field(1.)),
        containment: fields.add(field(2.)),
        boom: std::array::from_fn(|i| {
            server.load(format!("audio/sfx/explode/explode{}.ogg", i + 1))
        }),
        click: server.load("audio/mechanisms/click.ogg"),
        bell: server.load("audio/mechanisms/bell_use1.ogg"),
        hurt: server.load("audio/sfx/mobs/zombie/hurt1.ogg"),
        zap: server.load("audio/mechanisms/bell_resonate.ogg"),
    });
}

fn mesh<M: Material>(
    w: &mut World,
    parent: Option<Entity>,
    mesh: &Handle<Mesh>,
    mat: &Handle<M>,
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

/// Middle radius of the station's truss ring that holds the magnets.
const RING: f32 = 960.;

/// An electromagnet seen from the side, pointing at the cell along `dir`:
/// a polished pole shoe, an iron core wound with copper between two flanges,
/// a horseshoe yoke with bolts and cooling fins, a status lamp, a hazard-striped
/// strut bolted to the station ring and a cable. Built along local +X.
fn electromagnet(w: &mut World, a: &Art, origin: Vec3, index: usize, dir: Vec2) {
    let root = group(
        w,
        Transform::from_translation(origin)
            .with_rotation(Quat::from_rotation_z(dir.y.atan2(dir.x))),
        MagnetRoot,
    );
    let at = |x: f32, y: f32, size: Vec3| Transform::from_xyz(x, y, 0.).with_scale(size);
    let p = MAGNET_POLE;
    // The field beam from the pole toward the cell (drawn by the shader).
    mesh(
        w,
        Some(root),
        &a.quad,
        &a.fields[index],
        Transform::from_xyz(p - 100., 0., 25.)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::PI))
            .with_scale(Vec3::new(200., 150., 1.)),
        (),
    );
    // Pole shoe and core.
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.pole,
        at(p + 12., 0., Vec3::new(24., 100., 92.)),
        MagnetPole(index),
    );
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.iron,
        at(p + 115., 0., Vec3::new(190., 46., 60.)),
        (),
    );
    // Coil: end flanges and the windings between them.
    for x in [p + 36., p + 190.] {
        mesh(
            w,
            Some(root),
            &a.cube,
            &a.yoke,
            at(x, 0., Vec3::new(8., 118., 106.)),
            (),
        );
    }
    for i in 0..11 {
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
            at(p + 46. + i as f32 * 13.5, 0., Vec3::new(12.5, 102., 98.)),
            CoilWinding(index, i % 2 == 0),
        );
    }
    // Horseshoe yoke: back plate and two arms reaching toward the cell.
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.yoke,
        at(p + 225., 0., Vec3::new(30., 176., 96.)),
        (),
    );
    for y in [-80., 80.] {
        mesh(
            w,
            Some(root),
            &a.cube,
            &a.yoke,
            at(p + 165., y, Vec3::new(150., 18., 72.)),
            (),
        );
        mesh(
            w,
            Some(root),
            &a.cube,
            &a.pole,
            at(p + 92., y * 0.94, Vec3::new(14., 30., 74.)),
            (),
        );
        for x in [p + 140., p + 200.] {
            mesh(
                w,
                Some(root),
                &a.cube,
                &a.bolt,
                Transform::from_xyz(x, y, 37.).with_scale(Vec3::new(9., 9., 4.)),
                (),
            );
        }
    }
    // Cooling fins behind the yoke.
    for i in 0..5 {
        mesh(
            w,
            Some(root),
            &a.cube,
            &a.fin,
            at(p + 248. + i as f32 * 10., 0., Vec3::new(4., 136., 84.)),
            (),
        );
    }
    // Status lamp on the yoke: green idle, red while pulling.
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.lamp_idle,
        Transform::from_xyz(p + 225., 0., 50.).with_scale(Vec3::splat(16.)),
        StatusLamp(index),
    );
    // Hazard-striped strut to the ring, and a cable running beside it.
    let strut = RING - 22. - (p + 296.);
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.hazard,
        at(p + 296. + strut / 2., 0., Vec3::new(strut, 36., 44.)),
        (),
    );
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.cable,
        at(p + 225., 96., Vec3::new(10., 22., 10.)),
        (),
    );
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.cable,
        at(
            (p + 225. + RING) / 2.,
            104.,
            Vec3::new(RING - p - 225., 10., 10.),
        ),
        (),
    );
    // Arc pieces, posed each frame while this magnet pulls.
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

/// The station that holds the magnets: an octagonal truss ring (two rails and
/// zig-zag bracing) with corner nodes and blinking beacons, two solar wings on
/// booms and an antenna mast with a dish.
fn station(w: &mut World, a: &Art, origin: Vec3) {
    let root = group(w, Transform::from_translation(origin), MagnetRoot);
    let eighth = std::f32::consts::PI / 8.;
    let side = 2. * RING * eighth.tan();
    for k in 0..8 {
        let angle = k as f32 * std::f32::consts::FRAC_PI_4;
        let segment = w
            .spawn((
                Transform::from_rotation(Quat::from_rotation_z(angle)),
                Visibility::Inherited,
                ChildOf(root),
            ))
            .id();
        // Local X points out from the centre; the segment runs along local Y.
        for x in [RING - 22., RING + 22.] {
            mesh(
                w,
                Some(segment),
                &a.cube,
                &a.truss,
                Transform::from_xyz(x, 0., 0.).with_scale(Vec3::new(9., side, 30.)),
                (),
            );
        }
        let bays = 12;
        let bay = side / bays as f32;
        for i in 0..bays {
            let y0 = -side / 2. + i as f32 * bay;
            let (from, to) = if i % 2 == 0 {
                (Vec2::new(RING - 22., y0), Vec2::new(RING + 22., y0 + bay))
            } else {
                (Vec2::new(RING + 22., y0), Vec2::new(RING - 22., y0 + bay))
            };
            let d = to - from;
            mesh(
                w,
                Some(segment),
                &a.cube,
                &a.truss,
                Transform::from_translation(((from + to) / 2.).extend(-4.))
                    .with_rotation(Quat::from_rotation_z(d.y.atan2(d.x)))
                    .with_scale(Vec3::new(d.length(), 5., 18.)),
                (),
            );
        }
        // Corner node, with a blinking beacon on every other one.
        let corner = angle + eighth;
        let at = Vec2::new(corner.cos(), corner.sin()) * (RING / eighth.cos());
        mesh(
            w,
            Some(root),
            &a.cube,
            &a.node,
            Transform::from_translation(at.extend(0.))
                .with_rotation(Quat::from_rotation_z(corner))
                .with_scale(Vec3::new(64., 64., 44.)),
            (),
        );
        if k % 2 == 0 {
            mesh(
                w,
                Some(root),
                &a.cube,
                &a.beacon,
                Transform::from_translation(at.extend(26.)).with_scale(Vec3::splat(14.)),
                Beacon(k as f32 * 0.37),
            );
        }
    }
    // Solar wings left and right, on booms out of the ring.
    for s in [-1., 1.] {
        mesh(
            w,
            Some(root),
            &a.cube,
            &a.truss,
            Transform::from_xyz(s * (RING + 290.), 0., -10.).with_scale(Vec3::new(520., 14., 14.)),
            (),
        );
        for y in [-1., 1.] {
            mesh(
                w,
                Some(root),
                &a.cube,
                &a.solar,
                Transform::from_xyz(s * (RING + 330.), y * 96., -12.)
                    .with_scale(Vec3::new(400., 150., 6.)),
                (),
            );
        }
    }
    // Antenna mast and dish on top, beside the upper magnet.
    let top = RING + 22.;
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.truss,
        Transform::from_xyz(-200., top + 120., -6.).with_scale(Vec3::new(12., 240., 12.)),
        (),
    );
    mesh(
        w,
        Some(root),
        &a.cone,
        &a.hull,
        Transform::from_xyz(-200., top + 250., -6.)
            .with_rotation(Quat::from_rotation_z(0.6))
            .with_scale(Vec3::new(110., 40., 110.)),
        (),
    );
    mesh(
        w,
        Some(root),
        &a.cube,
        &a.beacon,
        Transform::from_xyz(-200., top + 245., 20.).with_scale(Vec3::splat(12.)),
        Beacon(0.5),
    );
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
    // The station ring and its four electromagnets around the spinning cell.
    station(w, &a, origin);
    for (i, dir) in sim::MAGNETS.iter().enumerate() {
        electromagnet(w, &a, origin, i, *dir);
    }
    // The containment field band around the cell (shader).
    mesh(
        w,
        None,
        &a.quad,
        &a.containment,
        Transform::from_translation(origin + Vec3::Z * 70.).with_scale(Vec3::splat(1250.)),
        (),
    );
    // The scene's own lights: a white sun from the upper left, blue earthshine
    // from below and a warm rim from behind on the right.
    for (offset, color, intensity) in [
        (
            Vec3::new(-1500., 1300., 1600.),
            Color::srgb(1., 0.97, 0.92),
            1_400_000_000.,
        ),
        (
            Vec3::new(400., -2200., 900.),
            Color::srgb(0.45, 0.65, 1.),
            600_000_000.,
        ),
        (
            Vec3::new(1600., 700., -700.),
            Color::srgb(1., 0.75, 0.5),
            450_000_000.,
        ),
    ] {
        w.spawn((
            Scene,
            PointLight {
                color,
                intensity,
                range: 9000.,
                radius: 120.,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(origin + offset),
            RenderLayers::layer(1),
        ));
    }
    // Earth below, lit by the sun, with a glowing atmosphere rim, turning slowly.
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
        &a.atmosphere,
        Transform::from_scale(Vec3::splat(EARTH_RADIUS * 1.05)),
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
    // Coils glow hot and the lamp turns red on the pulling magnet.
    for (coil, mut m) in w
        .query::<(&CoilWinding, &mut MeshMaterial3d<StandardMaterial>)>()
        .iter_mut(w)
    {
        let mat = if cell.magnet == Some(coil.0) && coil.1 {
            &a.copper_hot
        } else if coil.1 {
            &a.copper
        } else {
            &a.copper_dark
        };
        if m.0 != *mat {
            m.0 = mat.clone();
        }
    }
    for (lamp, mut m) in w
        .query::<(&StatusLamp, &mut MeshMaterial3d<StandardMaterial>)>()
        .iter_mut(w)
    {
        let mat = if cell.magnet == Some(lamp.0) {
            &a.lamp_on
        } else {
            &a.lamp_idle
        };
        if m.0 != *mat {
            m.0 = mat.clone();
        }
    }
    for (beacon, mut v) in w.query::<(&Beacon, &mut Visibility)>().iter_mut(w) {
        let on = (cell.time + beacon.0) % 1.2 < 0.25 && !gone;
        *v = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    // Shader effects: field beams, the atmosphere and the containment band.
    {
        let now = w.resource::<Time<Real>>().elapsed_secs();
        let mut fields = w.resource_mut::<Assets<PrisonField>>();
        for (i, h) in a.fields.iter().enumerate() {
            if let Some(mut f) = fields.get_mut(h) {
                let on = if cell.magnet == Some(i) && !gone {
                    1.
                } else {
                    0.
                };
                f.params = Vec4::new(now, on, 0., 0.);
            }
        }
        if let Some(mut f) = fields.get_mut(&a.atmosphere) {
            f.params = Vec4::new(now, 1., 1., 0.);
        }
        if let Some(mut f) = fields.get_mut(&a.containment) {
            let strength = if gone {
                0.
            } else if cell.magnet.is_some() {
                1.
            } else {
                0.3
            };
            f.params = Vec4::new(now, strength, 2., if gone { 0. } else { 1. });
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
