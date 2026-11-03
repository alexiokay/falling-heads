//! The sleek sci-fi shuttle shared by the LAUNCHER rocket and the space prison
//! escape: a long white fuselage with a black heat-shield nose and belly and
//! blue and red accent stripes, a tinted cockpit canopy with the hero's head
//! inside, a swept delta wing and tail fin, navigation lights and a tail strobe,
//! a row of side windows with passengers' heads and three glowing engines.
//!
//! Built in local space with the nose toward +X (about 300 long); callers scale,
//! turn and pose it, keep `ShuttleHead`s upright and blink `ShuttleLight`s.
use bevy::{camera::visibility::RenderLayers, prelude::*};

/// Meshes and materials for one user of the shuttle (the prison keeps its own
/// copy lit whatever Graphics > Lighting says; the LAUNCHER one follows it).
#[derive(Clone)]
pub(crate) struct ShuttleArt {
    cube: Handle<Mesh>,
    sphere: Handle<Mesh>,
    cylinder: Handle<Mesh>,
    cone: Handle<Mesh>,
    quad: Handle<Mesh>,
    hull: Handle<StandardMaterial>,
    shield: Handle<StandardMaterial>,
    blue: Handle<StandardMaterial>,
    red: Handle<StandardMaterial>,
    metal: Handle<StandardMaterial>,
    engine: Handle<StandardMaterial>,
    glow: Handle<StandardMaterial>,
    glass: Handle<StandardMaterial>,
    window: Handle<StandardMaterial>,
    head: Handle<StandardMaterial>,
    nav_red: Handle<StandardMaterial>,
    nav_green: Handle<StandardMaterial>,
    strobe: Handle<StandardMaterial>,
}

/// A head in the shuttle (`true`: the hero); callers keep it upright.
#[derive(Component)]
pub(crate) struct ShuttleHead(pub bool);

/// A blinking light: phase and period in seconds.
#[derive(Component)]
pub(crate) struct ShuttleLight {
    pub phase: f32,
    pub period: f32,
}

/// Where the engines' nozzles end and the nose is, in the shuttle's local space.
pub(crate) struct Built {
    pub engines: Vec<Vec3>,
    pub nose: Vec3,
}

impl ShuttleArt {
    /// New meshes and materials; every surface material id goes in `lit_ids`
    /// (the prison uses that to keep its copy lit).
    pub(crate) fn new(
        meshes: &mut Assets<Mesh>,
        mats: &mut Assets<StandardMaterial>,
        lit_ids: &mut Vec<AssetId<StandardMaterial>>,
    ) -> Self {
        let mut lit = |color: Color, metal: f32, rough: f32| {
            let h = mats.add(StandardMaterial {
                base_color: color,
                metallic: metal,
                perceptual_roughness: rough,
                reflectance: 0.5,
                ..default()
            });
            lit_ids.push(h.id());
            h
        };
        let hull = lit(Color::srgb(0.94, 0.95, 0.97), 0.25, 0.3);
        let shield = lit(Color::srgb(0.08, 0.08, 0.1), 0.1, 0.6);
        let blue = lit(Color::srgb(0.12, 0.38, 0.95), 0.4, 0.3);
        let red = lit(Color::srgb(0.9, 0.16, 0.12), 0.3, 0.35);
        let metal = lit(Color::srgb(0.7, 0.72, 0.78), 1., 0.2);
        let engine = lit(Color::srgb(0.22, 0.23, 0.26), 1., 0.3);
        let window = lit(Color::srgb(0.05, 0.07, 0.12), 0.5, 0.2);
        let head = lit(Color::srgb(0.2, 0.2, 0.24), 0., 0.8);
        let glass = {
            let h = mats.add(StandardMaterial {
                base_color: Color::srgba(0.55, 0.8, 1., 0.25),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.04,
                reflectance: 0.95,
                ..default()
            });
            lit_ids.push(h.id());
            h
        };
        let mut light = |color: Color, emissive: LinearRgba| {
            mats.add(StandardMaterial {
                base_color: color,
                emissive,
                unlit: true,
                ..default()
            })
        };
        Self {
            cube: meshes.add(Cuboid::new(1., 1., 1.)),
            sphere: meshes.add(Sphere::new(0.5).mesh().uv(40, 20)),
            cylinder: meshes.add(Cylinder::new(0.5, 1.)),
            cone: meshes.add(Cone {
                radius: 0.5,
                height: 1.,
            }),
            quad: meshes.add(Rectangle::new(1., 1.)),
            hull,
            shield,
            blue,
            red,
            metal,
            engine,
            glow: light(Color::srgb(0.6, 0.85, 1.), LinearRgba::rgb(2., 4., 7.)),
            glass,
            window,
            head,
            nav_red: light(Color::srgb(1., 0.2, 0.15), LinearRgba::rgb(6., 0.5, 0.4)),
            nav_green: light(Color::srgb(0.3, 1., 0.4), LinearRgba::rgb(0.5, 6., 0.8)),
            strobe: light(Color::WHITE, LinearRgba::rgb(8., 8., 8.)),
        }
    }
}

struct Builder<'a> {
    world: &'a mut World,
    parent: Entity,
    layer: Option<RenderLayers>,
}

impl Builder<'_> {
    fn part(
        &mut self,
        mesh: &Handle<Mesh>,
        mat: &Handle<StandardMaterial>,
        at: Vec3,
        rotation: Quat,
        size: Vec3,
    ) -> Entity {
        let mut e = self.world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(at)
                .with_rotation(rotation)
                .with_scale(size),
            ChildOf(self.parent),
        ));
        if let Some(layer) = &self.layer {
            e.insert(layer.clone());
        }
        e.id()
    }

    /// A head looking out: a thin dark block with the face on front.
    fn head(
        &mut self,
        art: &ShuttleArt,
        at: Vec3,
        size: f32,
        face: Handle<StandardMaterial>,
        hero: bool,
    ) {
        let head = self
            .world
            .spawn((
                Transform::from_translation(at),
                Visibility::Inherited,
                ShuttleHead(hero),
                ChildOf(self.parent),
            ))
            .id();
        let mut inner = Builder {
            world: &mut *self.world,
            parent: head,
            layer: self.layer.clone(),
        };
        inner.part(
            &art.cube,
            &art.head,
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::new(size, size, 4.),
        );
        inner.part(
            &art.quad,
            &face,
            Vec3::Z * 2.5,
            Quat::IDENTITY,
            Vec3::splat(size),
        );
    }
}

/// A triangle (a flattened cone) in the side plane whose tip points along
/// `dir`, with its base of `width` centred on `base`.
fn triangle(base: Vec3, dir: Vec2, width: f32, height: f32, thick: f32) -> (Vec3, Quat, Vec3) {
    let dir = dir.normalize();
    let center = base + (dir * height / 2.).extend(0.);
    // A cone's tip is +Y; turn +Y onto `dir`.
    let rotation = Quat::from_rotation_z(-dir.x.atan2(dir.y));
    (center, rotation, Vec3::new(width, height, thick))
}

/// Builds the shuttle under `parent` (nose toward +X). `hero_face` goes in the
/// cockpit; up to three `passengers` faces in the side windows.
pub(crate) fn build(
    world: &mut World,
    parent: Entity,
    art: &ShuttleArt,
    layer: Option<RenderLayers>,
    hero_face: Handle<StandardMaterial>,
    passengers: &[Handle<StandardMaterial>],
) -> Built {
    let mut b = Builder {
        world,
        parent,
        layer,
    };
    let q = Quat::IDENTITY;
    // Fuselage, raised spine, black nose and belly, accent stripes.
    b.part(
        &art.sphere,
        &art.hull,
        Vec3::ZERO,
        q,
        Vec3::new(280., 72., 84.),
    );
    b.part(
        &art.sphere,
        &art.hull,
        Vec3::new(-15., 16., 0.),
        q,
        Vec3::new(190., 50., 66.),
    );
    b.part(
        &art.sphere,
        &art.shield,
        Vec3::new(-8., -22., 0.),
        q,
        Vec3::new(262., 34., 82.),
    );
    b.part(
        &art.sphere,
        &art.shield,
        Vec3::new(122., -4., 0.),
        q,
        Vec3::new(66., 44., 60.),
    );
    b.part(
        &art.sphere,
        &art.blue,
        Vec3::new(-6., 5., 0.),
        q,
        Vec3::new(270., 7., 85.5),
    );
    b.part(
        &art.sphere,
        &art.red,
        Vec3::new(-6., -3., 0.),
        q,
        Vec3::new(262., 3., 85.8),
    );
    // Both sides carry the same heads, windows and wing, so the shuttle can
    // show either side to the camera (it rolls over to fly left).
    let sides = [1., -1.];
    // Cockpit canopy over the hero.
    for side in sides {
        b.head(
            art,
            Vec3::new(78., 24., 36. * side),
            30.,
            hero_face.clone(),
            true,
        );
    }
    b.part(
        &art.sphere,
        &art.glass,
        Vec3::new(78., 24., 0.),
        q,
        Vec3::new(92., 48., 88.),
    );
    b.part(
        &art.sphere,
        &art.metal,
        Vec3::new(78., 2., 0.),
        q,
        Vec3::new(96., 6., 86.),
    );
    // Side windows with passengers.
    for (i, x, side) in [30., 6., -18.]
        .into_iter()
        .enumerate()
        .flat_map(|(i, x)| sides.map(|s| (i, x, s)))
    {
        let facing = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        b.part(
            &art.cylinder,
            &art.metal,
            Vec3::new(x, -2., 40. * side),
            facing,
            Vec3::new(24., 2., 24.),
        );
        b.part(
            &art.cylinder,
            &art.window,
            Vec3::new(x, -2., 41. * side),
            facing,
            Vec3::new(19., 2., 19.),
        );
        if let Some(face) = passengers.get(i) {
            b.head(
                art,
                Vec3::new(x, -2., 42.5 * side),
                15.,
                face.clone(),
                false,
            );
        }
        b.part(
            &art.cylinder,
            &art.glass,
            Vec3::new(x, -2., 46.5 * side),
            facing,
            Vec3::new(19., 1., 19.),
        );
    }
    // Small thruster bumps by the nose.
    for (y, side) in [(12., 1.), (-12., 1.), (12., -1.), (-12., -1.)] {
        b.part(
            &art.cube,
            &art.metal,
            Vec3::new(100., y, 30. * side),
            q,
            Vec3::new(8., 5., 4.),
        );
    }
    // Swept delta wing (seen from the camera side, angled below the body) and
    // its darker leading edge.
    for side in sides {
        let (at, rot, size) = triangle(
            Vec3::new(-15., -26., 36. * side),
            Vec2::new(-0.55, -1.),
            170.,
            78.,
            8.,
        );
        b.part(&art.cone, &art.hull, at, rot, size);
        let (at, rot, size) = triangle(
            Vec3::new(-8., -26., 37. * side),
            Vec2::new(-0.55, -1.),
            120.,
            60.,
            9.,
        );
        b.part(&art.cone, &art.blue, at, rot, size);
    }
    // Tall swept tail fin with a red tip.
    let (at, rot, size) = triangle(Vec3::new(-102., 28., 0.), Vec2::new(-0.6, 1.), 96., 92., 8.);
    b.part(&art.cone, &art.hull, at, rot, size);
    let (at, rot, size) = triangle(Vec3::new(-131., 72., 0.), Vec2::new(-0.6, 1.), 34., 34., 9.);
    b.part(&art.cone, &art.red, at, rot, size);
    // Navigation lights: red on the wing tip, green on the fin, a white strobe.
    let wing_tip = |side: f32| {
        Vec3::new(-15., -26., 36. * side)
            + (Vec2::new(-0.55, -1.).normalize() * 78.).extend(2. * side)
    };
    let fin_tip = Vec3::new(-102., 28., 0.) + (Vec2::new(-0.6, 1.).normalize() * 92.).extend(6.);
    for (at, mat, phase, period) in [
        (wing_tip(1.), &art.nav_red, 0., 1.),
        (wing_tip(-1.), &art.nav_green, 0., 1.),
        (fin_tip, &art.nav_green, 0.5, 1.),
        (Vec3::new(10., 40., 10.), &art.strobe, 0.2, 1.4),
    ] {
        let light = b.part(&art.cube, mat, at, q, Vec3::splat(9.));
        b.world
            .entity_mut(light)
            .insert(ShuttleLight { phase, period });
    }
    // Three engines at the tail with glowing nozzles.
    let mut engines = Vec::new();
    let back = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    let ring = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    for (y, z) in [(10., 14.), (-8., 14.), (2., -16.)] {
        b.part(
            &art.cone,
            &art.engine,
            Vec3::new(-142., y, z),
            back,
            Vec3::new(30., 30., 30.),
        );
        b.part(
            &art.cylinder,
            &art.glow,
            Vec3::new(-157., y, z),
            ring,
            Vec3::new(22., 2., 22.),
        );
        engines.push(Vec3::new(-158., y, z));
    }
    Built {
        engines,
        nose: Vec3::new(156., -4., 0.),
    }
}

/// Keeps the heads upright and facing the camera (`turn` is the shuttle's world
/// rotation) and blinks its lights (`now` in seconds).
pub(crate) fn pose(world: &mut World, shuttle: Entity, turn: Quat, now: f32) {
    let children: Vec<Entity> = world
        .get::<Children>(shuttle)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    for child in children {
        if world.get::<ShuttleHead>(child).is_some()
            && let Some(mut t) = world.get_mut::<Transform>(child)
        {
            t.rotation = turn.inverse();
        }
        if let Some(light) = world.get::<ShuttleLight>(child) {
            let on = (now + light.phase) % light.period < 0.25;
            if let Some(mut t) = world.get_mut::<Transform>(child) {
                t.scale = Vec3::splat(if on { 9. } else { 0. });
            }
        }
    }
}
