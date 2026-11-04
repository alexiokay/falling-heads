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
    cone: Handle<Mesh>,
    sphere: Handle<Mesh>,
    bedrock: Handle<StandardMaterial>,
    backwall: Handle<StandardMaterial>,
    trapdoor: Handle<StandardMaterial>,
    button: Handle<StandardMaterial>,
    button_pressed: Handle<StandardMaterial>,
    guard: Handle<StandardMaterial>,
    captain: Handle<StandardMaterial>,
    item_fuel: Handle<StandardMaterial>,
    item_shield: Handle<StandardMaterial>,
    item_key: Handle<StandardMaterial>,
    aura_fuel: Handle<StandardMaterial>,
    aura_shield: Handle<StandardMaterial>,
    aura_key: Handle<StandardMaterial>,
    sparkle: Handle<StandardMaterial>,
    bubble: Handle<StandardMaterial>,
    star: Handle<StandardMaterial>,
    villager: Handle<StandardMaterial>,
    stars: Handle<StandardMaterial>,
    earth: Handle<StandardMaterial>,
    debris: Handle<StandardMaterial>,
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
    puff: Handle<StandardMaterial>,
    plume: Handle<PrisonField>,
    shock: Handle<PrisonField>,
    /// The escape rocket (the voxel block rocket shared with the LAUNCHER), kept lit.
    rocket: crate::systems::block_rocket::RocketArt,
    fields: [Handle<PrisonField>; 4],
    atmosphere: Handle<PrisonField>,
    containment: Handle<PrisonField>,
    boom: [Handle<AudioSource>; 4],
    click: Handle<AudioSource>,
    bell: Handle<AudioSource>,
    hurt: Handle<AudioSource>,
    zap: Handle<AudioSource>,
    alarm: Handle<AudioSource>,
    pickup: Handle<AudioSource>,
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
/// A red alarm light around the cell, pulsing with the alarm level.
#[derive(Component)]
struct AlarmLight(f32);
/// A floating power-up (by its id).
#[derive(Component)]
struct PowerUpVisual(u32);
/// The glow behind a power-up.
#[derive(Component)]
struct ItemAura;
/// A twinkling sparkle by a power-up, with its phase.
#[derive(Component)]
struct Sparkle(f32);
/// The bubble around the hero while the shield lasts.
#[derive(Component)]
struct ShieldBubble;
/// A star circling the stunned hero.
#[derive(Component)]
struct StunStar(usize);
#[derive(Component)]
struct HeadVisual(usize);
#[derive(Component)]
struct Baton;
/// A blast fireball: its own shader material (progress differs per blast)
/// and the light it throws on the station.
#[derive(Component)]
struct Fireball {
    age: f32,
    mat: Handle<PrisonField>,
    light: Entity,
}
/// How long a blast fireball lasts, seconds.
const FIREBALL: f32 = 0.7;
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
/// An engine plume quad, by the x of its nozzle (in the rocket's space).
#[derive(Component)]
struct Plume(f32);
#[derive(Component)]
struct Shock;
/// A puff of exhaust smoke drifting away and fading.
#[derive(Component)]
struct Puff {
    v: Vec3,
    age: f32,
}
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
    let captain = flat(Some(pixel("textures/prison/captain.png")), Color::WHITE);
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
    // Power-up sprites (pixel art, unlit so their colours stay true).
    let item_fuel = flat(Some(pixel("textures/prison/item_fuel.png")), Color::WHITE);
    let item_shield = flat(Some(pixel("textures/prison/item_shield.png")), Color::WHITE);
    let item_key = flat(Some(pixel("textures/prison/item_key.png")), Color::WHITE);
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
    let puff = glow(Color::srgba(0.7, 0.72, 0.78, 0.16), LinearRgba::BLACK, true);
    let bubble = glow(
        Color::srgba(0.4, 0.85, 1., 0.22),
        LinearRgba::rgb(0.3, 0.8, 1.2),
        true,
    );
    let star = glow(
        Color::srgb(1., 0.95, 0.3),
        LinearRgba::rgb(4., 3.5, 0.6),
        false,
    );
    let rocket = {
        let mut ids = Vec::new();
        let art =
            crate::systems::block_rocket::RocketArt::new(&mut meshes, &mut mats, &server, &mut ids);
        keep.0.extend(ids);
        art
    };
    // Soft glows behind the power-ups (tinted per item) and their sparkles.
    let mut aura = |color: Color, emissive: LinearRgba, path: &str| {
        mats.add(StandardMaterial {
            base_color_texture: Some(server.load(path.to_string())),
            base_color: color,
            emissive,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })
    };
    let aura_fuel = aura(
        Color::srgba(1., 0.55, 0.15, 0.7),
        LinearRgba::rgb(1.6, 0.6, 0.1),
        "textures/prison/aura.png",
    );
    let aura_shield = aura(
        Color::srgba(0.3, 0.85, 1., 0.7),
        LinearRgba::rgb(0.3, 1.2, 1.8),
        "textures/prison/aura.png",
    );
    let aura_key = aura(
        Color::srgba(1., 0.85, 0.25, 0.7),
        LinearRgba::rgb(1.6, 1.3, 0.2),
        "textures/prison/aura.png",
    );
    let sparkle = aura(
        Color::WHITE,
        LinearRgba::rgb(3., 3., 3.),
        "textures/prison/sparkle.png",
    );
    let field = |mode: f32| PrisonField {
        params: Vec4::new(0., 0., mode, 0.),
    };
    commands.insert_resource(Art {
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        quad: meshes.add(Rectangle::new(1., 1.)),
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
        captain,
        item_fuel,
        item_shield,
        item_key,
        aura_fuel,
        aura_shield,
        aura_key,
        sparkle,
        bubble,
        star,
        villager,
        stars,
        earth,
        debris,
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
        puff,
        plume: fields.add(field(3.)),
        shock: fields.add(field(4.)),
        rocket,
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
        alarm: server.load("audio/mechanisms/bell_use2.ogg"),
        pickup: server.load("audio/mechanisms/bubble_pop.ogg"),
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
    // Red alarm lights on two sides of the cell, dark until the alarm rises.
    for (i, x) in [-1., 1.].into_iter().enumerate() {
        w.spawn((
            Scene,
            AlarmLight(i as f32 * std::f32::consts::PI),
            PointLight {
                color: Color::srgb(1., 0.12, 0.08),
                intensity: 0.,
                range: 2200.,
                radius: 80.,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(origin + Vec3::new(x * 700., 300., 500.)),
            RenderLayers::layer(1),
        ));
    }
    // The hero's shield bubble and stun stars, shown when needed.
    mesh(
        w,
        Some(root),
        &a.sphere,
        &a.bubble,
        Transform::from_scale(Vec3::splat(46.)),
        (ShieldBubble, Visibility::Hidden),
    );
    for i in 0..3 {
        mesh(
            w,
            Some(root),
            &a.cube,
            &a.star,
            Transform::from_scale(Vec3::splat(10.)),
            (StunStar(i), Visibility::Hidden),
        );
    }
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
            font_size: FontSize::Px(15.),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.9, 1.)),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.),
            left: Val::Percent(50.),
            margin: UiRect::left(Val::Px(-320.)),
            width: Val::Px(640.),
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
        Kind::Captain => a.captain.clone(),
    };
    // The captain is a size bigger.
    let scale = if kind == Kind::Captain { 1.3 } else { 1. };
    let head = w
        .spawn((
            HeadVisual(index),
            Transform::from_xyz(0., 0., 30.).with_scale(Vec3::splat(scale)),
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
    if kind.is_guard() {
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

/// The block rocket inside the escape rocket's root: turned so its nose (+X in the
/// rocket's own space) points up (+Y), the way the ride poses the root.
#[derive(Component)]
struct RocketBase;
const NOSE_UP: f32 = std::f32::consts::FRAC_PI_2;

/// The escape ship: the voxel block rocket shared with the LAUNCHER (hero in the
/// cockpit once picked up, saved villagers in the side windows), with shader
/// plumes on its three engines and a re-entry shock over the nose.
pub(super) fn spawn_rocket(w: &mut World, at: Vec3, saved: u32) -> Entity {
    let a = w.resource::<Art>().clone();
    let face = w.resource::<HeroFace>().0.clone();
    let root = group(w, Transform::from_translation(at), Rocket);
    let base = w
        .spawn((
            RocketBase,
            Transform::from_rotation(Quat::from_rotation_z(NOSE_UP)),
            Visibility::Inherited,
            ChildOf(root),
        ))
        .id();
    let faces = vec![a.villager.clone(); saved.min(3) as usize];
    let built = crate::systems::block_rocket::build(
        w,
        base,
        &a.rocket,
        Some(RenderLayers::layer(1)),
        face,
        &faces,
    );
    // Plumes stream back (-X) from each nozzle; the quad's top (+Y, where the
    // shader starts the flame) is turned onto +X, toward the nozzle.
    let toward_nozzle = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    for e in &built.engines {
        mesh(
            w,
            Some(base),
            &a.quad,
            &a.plume,
            Transform::from_translation(*e + Vec3::Z * 3.)
                .with_rotation(toward_nozzle)
                .with_scale(Vec3::new(34., 120., 1.)),
            Plume(e.x),
        );
    }
    mesh(
        w,
        Some(base),
        &a.quad,
        &a.shock,
        Transform::from_translation(built.nose + Vec3::new(80., 0., 70.))
            .with_rotation(toward_nozzle)
            .with_scale(Vec3::new(320., 240., 1.)),
        Shock,
    );
    root
}

/// Poses the escape ship: position, heading (direction of travel), flame
/// (0..1) and re-entry heat (0..1). The hero shows in the cockpit once boarded;
/// heads stay upright, lights blink, the engines leave a smoke trail.
#[allow(clippy::too_many_arguments)] // One call per frame from the prison update.
pub(super) fn pose_rocket(
    w: &mut World,
    at: Vec3,
    heading: Vec3,
    flame: f32,
    heat: f32,
    boarded: bool,
    dt: f32,
) {
    // Turn only within the screen, so the side with the windows always shows.
    let up = Vec3::new(heading.x, heading.y, 0.).normalize_or(Vec3::NEG_Y);
    let turn = Quat::from_rotation_arc(Vec3::Y, up);
    for mut t in w
        .query_filtered::<&mut Transform, With<Rocket>>()
        .iter_mut(w)
    {
        t.translation = at;
        t.rotation = turn;
    }
    let now = w.resource::<Time<Real>>().elapsed_secs();
    let flicker = 1. + (now * 40.).sin() * 0.12;
    for (plume, mut t) in w.query::<(&Plume, &mut Transform)>().iter_mut(w) {
        let length = 50. + 180. * flame * flicker;
        t.translation.x = plume.0 - length / 2.;
        t.scale.y = length;
    }
    let bases: Vec<Entity> = w
        .query_filtered::<Entity, With<RocketBase>>()
        .iter(w)
        .collect();
    for base in bases {
        crate::systems::block_rocket::pose(w, base, turn * Quat::from_rotation_z(NOSE_UP), now);
    }
    for (head, mut v) in w
        .query::<(&crate::systems::block_rocket::RocketHead, &mut Visibility)>()
        .iter_mut(w)
    {
        if head.0 {
            *v = if boarded {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
    let a = w.resource::<Art>().clone();
    {
        let mut fields = w.resource_mut::<Assets<PrisonField>>();
        if let Some(mut f) = fields.get_mut(&a.plume) {
            f.params = Vec4::new(now, flame.clamp(0., 1.), 3., 0.);
        }
        if let Some(mut f) = fields.get_mut(&a.shock) {
            f.params = Vec4::new(now, heat.clamp(0., 1.), 4., 0.);
        }
    }
    // Smoke: puffs left behind the engines, drifting and growing.
    if flame > 0.55 {
        let nozzle = at - up * 200. - Vec3::Z * 40.;
        let k = (now * 1000.) as u32;
        let side = Vec3::new(-up.y, up.x, 0.) * jitter(k, 0) * 20.;
        mesh(
            w,
            None,
            &a.sphere,
            &a.puff,
            Transform::from_translation(nozzle + side).with_scale(Vec3::splat(12.)),
            Puff {
                v: -up * 180. + side * 2.,
                age: 0.,
            },
        );
    }
    let mut done = Vec::new();
    for (e, mut puff, mut t) in w.query::<(Entity, &mut Puff, &mut Transform)>().iter_mut(w) {
        puff.age += dt;
        t.translation += puff.v * dt;
        let grow = 12. + puff.age * 45.;
        let fade = (1. - puff.age / 1.1).max(0.);
        t.scale = Vec3::splat(grow * fade.sqrt());
        if puff.age > 1.1 {
            done.push(e);
        }
    }
    for e in done {
        w.despawn(e);
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
    // A magnet pulls when it is the chosen one, or all of them in an overload.
    let overload = cell.overload > 0.;
    let active = |m: usize| cell.magnet == Some(m) || overload;
    if cell.magnet.is_some() && cell.magnet != magnet_was {
        play(w, a.zap.clone(), 0.35);
    }
    for (pole, mut m) in w
        .query::<(&MagnetPole, &mut MeshMaterial3d<StandardMaterial>)>()
        .iter_mut(w)
    {
        let mat = if active(pole.0) { &a.pole_hot } else { &a.pole };
        if m.0 != *mat {
            m.0 = mat.clone();
        }
    }
    // Coils glow hot and the lamp turns red on the pulling magnet.
    for (coil, mut m) in w
        .query::<(&CoilWinding, &mut MeshMaterial3d<StandardMaterial>)>()
        .iter_mut(w)
    {
        let mat = if active(coil.0) && coil.1 {
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
        let mat = if active(lamp.0) {
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
                let on = if active(i) && !gone { 1. } else { 0. };
                f.params = Vec4::new(now, on, 0., 0.);
            }
        }
        if let Some(mut f) = fields.get_mut(&a.atmosphere) {
            f.params = Vec4::new(now, 1., 1., 0.);
        }
        if let Some(mut f) = fields.get_mut(&a.containment) {
            let strength = if gone {
                0.
            } else if cell.magnet.is_some() || overload {
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
        if !active(m) || gone {
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
    // Alarm: red lights pulse harder with every level.
    for (light, mut l) in w.query::<(&AlarmLight, &mut PointLight)>().iter_mut(w) {
        let pulse = 0.5 + 0.5 * (cell.time * 7. + light.0).sin();
        l.intensity = cell.alarm as f32 * 120_000_000. * pulse;
    }
    // Power-ups: one visual per item, spinning and bobbing where it floats.
    let shown: Vec<(Entity, u32)> = w
        .query::<(Entity, &PowerUpVisual)>()
        .iter(w)
        .map(|(e, p)| (e, p.0))
        .collect();
    for &(e, id) in &shown {
        if !cell.powerups.iter().any(|p| p.id == id) {
            w.despawn(e);
        }
    }
    for pu in &cell.powerups {
        if shown.iter().any(|(_, id)| *id == pu.id) {
            continue;
        }
        let item = w
            .spawn((
                PowerUpVisual(pu.id),
                Transform::from_translation(pu.p.extend(40.)),
                Visibility::Inherited,
                ChildOf(root),
            ))
            .id();
        // A pixel-art sprite in front of a soft aura in its colour, with two
        // sparkles that twinkle around it.
        let (sprite, aura) = match pu.kind {
            sim::PowerKind::Fuel => (&a.item_fuel, &a.aura_fuel),
            sim::PowerKind::Shield => (&a.item_shield, &a.aura_shield),
            sim::PowerKind::Key => (&a.item_key, &a.aura_key),
        };
        mesh(
            w,
            Some(item),
            &a.quad,
            aura,
            Transform::from_xyz(0., 0., -4.).with_scale(Vec3::splat(110.)),
            ItemAura,
        );
        mesh(
            w,
            Some(item),
            &a.quad,
            sprite,
            Transform::from_scale(Vec3::splat(52.)),
            (),
        );
        for (i, at) in [Vec3::new(24., 22., 4.), Vec3::new(-22., -18., 4.)]
            .into_iter()
            .enumerate()
        {
            mesh(
                w,
                Some(item),
                &a.quad,
                &a.sparkle,
                Transform::from_translation(at).with_scale(Vec3::splat(14.)),
                Sparkle(i as f32 * 1.7 + pu.id as f32),
            );
        }
    }
    for (pv, mut t) in w.query::<(&PowerUpVisual, &mut Transform)>().iter_mut(w) {
        if let Some(pu) = cell.powerups.iter().find(|p| p.id == pv.0) {
            let phase = cell.time * 3. + pv.0 as f32;
            t.translation = (pu.p + Vec2::Y * phase.sin() * 7.).extend(40.);
            // Upright on screen, swaying a little (never edge-on), pulsing.
            t.rotation = turn.inverse() * Quat::from_rotation_y(phase.cos() * 0.45);
            t.scale = Vec3::splat(1. + phase.sin() * 0.06);
        }
    }
    for (aura, mut t) in w.query::<(&ItemAura, &mut Transform)>().iter_mut(w) {
        let _ = aura;
        t.scale = Vec3::splat(110. + (cell.time * 4.).sin() * 10.);
    }
    for (spark, mut t) in w.query::<(&Sparkle, &mut Transform)>().iter_mut(w) {
        let twinkle = (cell.time * 5. + spark.0).sin().max(0.);
        t.scale = Vec3::splat(4. + twinkle * 14.);
        t.rotation = Quat::from_rotation_z(cell.time * 2. + spark.0);
    }
    // Shield bubble and stun stars around the hero.
    let hero = cell.bodies[0];
    for (mut t, mut v) in w
        .query_filtered::<(&mut Transform, &mut Visibility), With<ShieldBubble>>()
        .iter_mut(w)
    {
        t.translation = hero.p.extend(34.);
        *v = if cell.shield > 0. && !hero.escaped {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (star, mut t, mut v) in w
        .query::<(&StunStar, &mut Transform, &mut Visibility)>()
        .iter_mut(w)
    {
        let angle = cell.time * 6. + star.0 as f32 * std::f32::consts::TAU / 3.;
        let around = sim::rotate(
            Vec2::new(angle.cos() * 34., 40. + angle.sin() * 8.),
            -cell.angle,
        );
        t.translation = (hero.p + around).extend(60.);
        *v = if hero.stun > 0. && !hero.escaped {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
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
                let seed = (frame % 97) as f32;
                let mat = w.resource_mut::<Assets<PrisonField>>().add(PrisonField {
                    params: Vec4::new(0., 0., 5., seed),
                });
                let light = w
                    .spawn((
                        Scene,
                        PointLight {
                            color: Color::srgb(1., 0.6, 0.25),
                            intensity: 400_000_000.,
                            range: 1600.,
                            radius: 60.,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Transform::from_translation(world + Vec3::Z * 140.),
                        RenderLayers::layer(1),
                    ))
                    .id();
                mesh(
                    w,
                    None,
                    &a.quad,
                    &mat,
                    Transform::from_translation(world + Vec3::Z * 10.)
                        .with_scale(Vec3::splat(460.)),
                    Fireball {
                        age: 0.,
                        mat: mat.clone(),
                        light,
                    },
                );
                // Hot sparks, faster than the chips.
                for k in 0..10 {
                    let angle = k as f32 / 10. * std::f32::consts::TAU + jitter(k, frame) * 0.5;
                    let speed = 900. + (jitter(frame, k + 40) + 1.) * 300.;
                    mesh(
                        w,
                        None,
                        &a.cube,
                        &a.spark,
                        Transform::from_translation(world).with_scale(Vec3::new(14., 4., 4.)),
                        Debris {
                            v: Vec3::new(angle.cos(), angle.sin(), 0.) * speed,
                            age: 0.25,
                        },
                    );
                }
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
            Event::CaptainIn(_) => play(w, a.bell.clone(), 0.9),
            Event::Stunned => play(w, a.hurt.clone(), 0.9),
            Event::ShieldBlock => play(w, a.zap.clone(), 0.6),
            Event::Alarm(_) => play(w, a.alarm.clone(), 0.8),
            Event::Overload => play(w, a.zap.clone(), 1.),
            Event::OverloadRelease => {
                let i = (cell.time * 5.) as usize % 4;
                play(w, a.boom[i].clone(), 0.7);
            }
            Event::PowerUpTaken(_) => play(w, a.pickup.clone(), 0.7),
            Event::KeyDrop => play(w, a.click.clone(), 0.9),
        }
    }
    let dt = w.resource::<Time<Real>>().delta_secs();
    let mut done = Vec::new();
    let now = w.resource::<Time<Real>>().elapsed_secs();
    let mut burnt = Vec::new();
    for (e, mut f) in w.query::<(Entity, &mut Fireball)>().iter_mut(w) {
        f.age += dt;
        burnt.push((e, f.age / FIREBALL, f.mat.clone(), f.light));
    }
    for (e, progress, mat, light) in burnt {
        if let Some(mut m) = w.resource_mut::<Assets<PrisonField>>().get_mut(&mat) {
            m.params.x = now;
            m.params.y = progress;
        }
        if let Some(mut l) = w.get_mut::<PointLight>(light) {
            l.intensity = 400_000_000. * (1. - progress).max(0.).powi(2);
        }
        if progress >= 1. {
            done.push(e);
            done.push(light);
            w.resource_mut::<Assets<PrisonField>>().remove(&mat);
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
    // The clock counts up, or down when a time limit is set.
    let shown = cell.limit.map_or(cell.time, |l| (l - cell.time).max(0.));
    let clock = format!(
        "{}{}:{:02}",
        if cell.limit.is_some() { "left " } else { "" },
        shown as u32 / 60,
        shown as u32 % 60
    );
    let doors_open = cell.doors.iter().filter(|d| d.open > 0.).count();
    for mut text in w.query_filtered::<&mut Text, With<Hud>>().iter_mut(w) {
        **text = if escaping {
            "ESCAPED! Heading home to Earth...".into()
        } else {
            // What the hero holds: fuel time, shield, key card.
            let mut held = Vec::new();
            if cell.fuel > 0. {
                held.push(format!("FUEL {:.0}s", cell.fuel.ceil()));
            }
            if cell.shield > 0. {
                held.push("SHIELD".to_string());
            }
            if cell.key {
                held.push("KEY CARD".to_string());
            }
            let alarm = if cell.alarm > 0 {
                format!("  |  ALARM {}", cell.alarm)
            } else {
                String::new()
            };
            let hint = if cell.overload > 0. {
                "MAGNET OVERLOAD!"
            } else if cell.bodies[0].stun > 0. {
                "STUNNED!"
            } else if doors_open > 0 {
                "TRAPDOOR OPEN - GO!"
            } else {
                "Red button opens the far trapdoor"
            };
            format!(
                "SPACE PRISON  {clock}{alarm}  |  villagers {}/{villagers}  |  guards {}\n{hint}{}  |  WASD thrust  |  Esc give up",
                cell.saved,
                cell.guards(),
                if held.is_empty() {
                    String::new()
                } else {
                    format!("  |  {}", held.join(" "))
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
