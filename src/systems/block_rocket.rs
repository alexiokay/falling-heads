//! The voxel block rocket shared by the LAUNCHER power and the space prison
//! escape, built from blocks like the world it flies through: a riveted iron
//! hull with red painted bands, a stepped red nose, a glass cockpit block with
//! the hero's head inside, porthole blocks with passengers' heads, stepped red
//! fins, an antenna with a blinking lamp and three nozzles that spit flickering
//! pixel-flame blocks.
//!
//! Built in local space with the nose toward +X (about 360 long, in 32-unit
//! blocks); callers scale, turn and pose it, keep `RocketHead`s upright, and
//! `pose` blinks the lamps and flickers the flames.
use bevy::{camera::visibility::RenderLayers, prelude::*};

/// The size of one block of the rocket.
const BLOCK: f32 = 32.;

/// Meshes and materials for one user of the rocket (the prison keeps its own
/// copy lit whatever Graphics > Lighting says; the LAUNCHER one follows it).
#[derive(Clone)]
pub(crate) struct RocketArt {
    cube: Handle<Mesh>,
    quad: Handle<Mesh>,
    hull: Handle<StandardMaterial>,
    paint: Handle<StandardMaterial>,
    glass: Handle<StandardMaterial>,
    window: Handle<StandardMaterial>,
    nozzle: Handle<StandardMaterial>,
    metal: Handle<StandardMaterial>,
    cabin: Handle<StandardMaterial>,
    head: Handle<StandardMaterial>,
    flame: Handle<StandardMaterial>,
    lamp_red: Handle<StandardMaterial>,
    lamp_green: Handle<StandardMaterial>,
    lamp_white: Handle<StandardMaterial>,
}

/// A head in the rocket (`true`: the hero); callers keep it upright.
#[derive(Component)]
pub(crate) struct RocketHead(pub bool);

/// A blinking lamp: phase and period in seconds, and its size when lit.
#[derive(Component)]
pub(crate) struct RocketLight {
    phase: f32,
    period: f32,
    size: f32,
}

/// A pixel-flame block behind a nozzle: where it rests and how big it is.
#[derive(Component)]
pub(crate) struct RocketFlame {
    at: Vec3,
    size: f32,
    seed: f32,
}

/// Where the nozzles end and the nose is, in the rocket's local space.
pub(crate) struct Built {
    pub engines: Vec<Vec3>,
    pub nose: Vec3,
}

impl RocketArt {
    /// New meshes and materials; every surface material id goes in `lit_ids`
    /// (the prison uses that to keep its copy lit).
    pub(crate) fn new(
        meshes: &mut Assets<Mesh>,
        mats: &mut Assets<StandardMaterial>,
        server: &AssetServer,
        lit_ids: &mut Vec<AssetId<StandardMaterial>>,
    ) -> Self {
        let pixel = |name: &str| -> Handle<Image> {
            server
                .load_builder()
                .with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
                    s.sampler = bevy::image::ImageSampler::nearest();
                })
                .load(format!("textures/powers/{name}.png"))
        };
        let mut block =
            |texture: Option<Handle<Image>>, color: Color, metal: f32, see_through: bool| {
                let h = mats.add(StandardMaterial {
                    base_color: color,
                    base_color_texture: texture,
                    metallic: metal,
                    perceptual_roughness: if see_through { 0.1 } else { 0.55 },
                    reflectance: if see_through { 0.8 } else { 0.4 },
                    alpha_mode: if see_through {
                        AlphaMode::Blend
                    } else {
                        AlphaMode::Opaque
                    },
                    ..default()
                });
                lit_ids.push(h.id());
                h
            };
        let hull = block(Some(pixel("rocket_hull")), Color::WHITE, 0.3, false);
        let paint = block(Some(pixel("rocket_paint")), Color::WHITE, 0.2, false);
        let glass = block(Some(pixel("rocket_glass")), Color::WHITE, 0., true);
        let window = block(Some(pixel("rocket_window")), Color::WHITE, 0.3, true);
        let nozzle = block(Some(pixel("rocket_nozzle")), Color::WHITE, 0.6, false);
        let metal = block(None, Color::srgb(0.55, 0.57, 0.62), 0.8, false);
        let cabin = block(None, Color::srgb(0.1, 0.11, 0.14), 0., false);
        let head = block(None, Color::srgb(0.2, 0.2, 0.24), 0., false);
        let mut glow = |texture: Option<Handle<Image>>, color: Color, emissive: LinearRgba| {
            mats.add(StandardMaterial {
                base_color: color,
                base_color_texture: texture.clone(),
                emissive,
                emissive_texture: texture,
                unlit: true,
                ..default()
            })
        };
        Self {
            cube: meshes.add(Cuboid::new(1., 1., 1.)),
            quad: meshes.add(Rectangle::new(1., 1.)),
            hull,
            paint,
            glass,
            window,
            nozzle,
            metal,
            cabin,
            head,
            flame: glow(
                Some(pixel("rocket_flame")),
                Color::WHITE,
                LinearRgba::rgb(3., 1.6, 0.5),
            ),
            lamp_red: glow(
                None,
                Color::srgb(1., 0.2, 0.15),
                LinearRgba::rgb(6., 0.5, 0.4),
            ),
            lamp_green: glow(
                None,
                Color::srgb(0.3, 1., 0.4),
                LinearRgba::rgb(0.5, 6., 0.8),
            ),
            lamp_white: glow(None, Color::WHITE, LinearRgba::rgb(8., 8., 8.)),
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
        size: Vec3,
    ) -> Entity {
        let mut e = self.world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(at).with_scale(size),
            ChildOf(self.parent),
        ));
        if let Some(layer) = &self.layer {
            e.insert(layer.clone());
        }
        e.id()
    }

    /// A full block (or a column of them through the rocket's depth).
    fn block(&mut self, art: &RocketArt, mat: &Handle<StandardMaterial>, column: i32, row: i32) {
        for z in [-0.5, 0.5] {
            self.part(
                &art.cube,
                mat,
                Vec3::new(column as f32, row as f32, z) * BLOCK,
                Vec3::splat(BLOCK),
            );
        }
    }

    /// A head looking out of a glass block: a dark block with the face on its
    /// front, in front of a dark cabin wall through the rocket's middle.
    fn head(
        &mut self,
        art: &RocketArt,
        at: Vec3,
        size: f32,
        face: Handle<StandardMaterial>,
        hero: bool,
    ) {
        self.part(
            &art.cube,
            &art.cabin,
            at,
            Vec3::new(size * 1.35, size * 1.35, 2.),
        );
        let head = self
            .world
            .spawn((
                Transform::from_translation(at),
                Visibility::Inherited,
                RocketHead(hero),
                ChildOf(self.parent),
            ))
            .id();
        let mut inner = Builder {
            world: &mut *self.world,
            parent: head,
            layer: self.layer.clone(),
        };
        inner.part(&art.cube, &art.head, Vec3::ZERO, Vec3::splat(size * 0.96));
        inner.part(&art.quad, &face, Vec3::Z * size * 0.5, Vec3::splat(size));
    }
}

/// Builds the rocket under `parent` (nose toward +X). `hero_face` goes in the
/// cockpit; up to three `passengers` faces in the portholes.
pub(crate) fn build(
    world: &mut World,
    parent: Entity,
    art: &RocketArt,
    layer: Option<RenderLayers>,
    hero_face: Handle<StandardMaterial>,
    passengers: &[Handle<StandardMaterial>],
) -> Built {
    let mut b = Builder {
        world,
        parent,
        layer,
    };
    // Body: columns -4..=3 along X, rows -1..=1, two blocks deep.
    //   3, 2   glass cockpit over a hull floor
    //   1, -3  red bands
    //   0..-2  portholes between hull rows
    //   -4     hull, with the nozzles behind it
    for column in -4..=3 {
        for row in -1..=1 {
            let mat = match (column, row) {
                (2..=3, 0..=1) => continue,
                (-2..=0, 0) => continue,
                (1 | -3, _) => &art.paint,
                _ => &art.hull,
            };
            b.block(art, mat, column, row);
        }
    }
    // The cockpit: one big glass block (2 x 2, through the depth) round the
    // hero's head.
    b.part(
        &art.cube,
        &art.glass,
        Vec3::new(2.5, 0.5, 0.) * BLOCK,
        Vec3::new(2., 2., 2.) * BLOCK,
    );
    b.head(art, Vec3::new(2.5, 0.5, 0.) * BLOCK, 40., hero_face, true);
    // Portholes with passengers.
    for (i, column) in [0, -1, -2].into_iter().enumerate() {
        let at = Vec3::new(column as f32, 0., 0.) * BLOCK;
        b.part(&art.cube, &art.window, at, Vec3::new(1., 1., 2.) * BLOCK);
        match passengers.get(i) {
            Some(face) => b.head(art, at, 22., face.clone(), false),
            None => {
                b.part(&art.cube, &art.cabin, at, Vec3::new(30., 30., 2.));
            }
        }
    }
    // Stepped red nose and a small metal tip.
    for row in -1..=1 {
        b.block(art, &art.paint, 4, row);
    }
    b.block(art, &art.paint, 5, 0);
    b.part(
        &art.cube,
        &art.metal,
        Vec3::new(5.75, 0., 0.) * BLOCK,
        Vec3::new(0.5, 0.5, 1.) * BLOCK,
    );
    // Stepped red fins above and below the tail, and one to each side.
    for (column, row) in [(-4, 2), (-3, 2), (-4, 3), (-4, -2), (-3, -2), (-4, -3)] {
        b.part(
            &art.cube,
            &art.paint,
            Vec3::new(column as f32, row as f32, 0.) * BLOCK,
            Vec3::new(1., 1., 0.5) * BLOCK,
        );
    }
    for side in [-1., 1.] {
        b.part(
            &art.cube,
            &art.paint,
            Vec3::new(-4., 0., 1.5 * side) * BLOCK,
            Vec3::new(1., 1., 1.) * BLOCK,
        );
    }
    // Antenna with a blinking lamp, and lamps on the fin tips.
    b.part(
        &art.cube,
        &art.metal,
        Vec3::new(0., 1.5 * BLOCK + 14., 0.),
        Vec3::new(4., 28., 4.),
    );
    for (at, mat, phase, period, size) in [
        (
            Vec3::new(0., 1.5 * BLOCK + 32., 0.),
            &art.lamp_red,
            0.,
            1.,
            10.,
        ),
        (
            Vec3::new(-4., 3.5, 0.) * BLOCK,
            &art.lamp_green,
            0.5,
            1.,
            9.,
        ),
        (
            Vec3::new(-4., -3.5, 0.) * BLOCK,
            &art.lamp_white,
            0.2,
            1.4,
            9.,
        ),
    ] {
        let lamp = b.part(&art.cube, mat, at, Vec3::splat(size));
        b.world.entity_mut(lamp).insert(RocketLight {
            phase,
            period,
            size,
        });
    }
    // Three nozzles behind the tail, each spitting two flame blocks.
    let mut engines = Vec::new();
    for (i, y) in [-1., 0., 1.].into_iter().enumerate() {
        let y = y * BLOCK;
        b.part(
            &art.cube,
            &art.nozzle,
            Vec3::new(-4.5 * BLOCK - 12., y, 0.),
            Vec3::new(24., 26., 26.),
        );
        let end = Vec3::new(-4.5 * BLOCK - 24., y, 0.);
        engines.push(end);
        for (k, (back, size)) in [(14., 22.), (36., 14.)].into_iter().enumerate() {
            let at = end - Vec3::X * back;
            let flame = b.part(&art.cube, &art.flame, at, Vec3::splat(size));
            b.world.entity_mut(flame).insert(RocketFlame {
                at,
                size,
                seed: i as f32 * 2.1 + k as f32 * 0.7,
            });
        }
    }
    Built {
        engines,
        nose: Vec3::new(6. * BLOCK, 0., 0.),
    }
}

/// Keeps the heads upright and facing the camera (`turn` is the rocket's world
/// rotation), blinks its lamps and flickers its flames (`now` in seconds).
pub(crate) fn pose(world: &mut World, rocket: Entity, turn: Quat, now: f32) {
    let children: Vec<Entity> = world
        .get::<Children>(rocket)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    for child in children {
        let Ok(mut e) = world.get_entity_mut(child) else {
            continue;
        };
        let head = e.contains::<RocketHead>();
        let light = e
            .get::<RocketLight>()
            .map(|l| ((now + l.phase) % l.period < 0.25, l.size));
        let flame = e.get::<RocketFlame>().map(|f| (f.at, f.size, f.seed));
        let Some(mut t) = e.get_mut::<Transform>() else {
            continue;
        };
        if head {
            t.rotation = turn.inverse();
        }
        if let Some((on, size)) = light {
            t.scale = Vec3::splat(if on { size } else { 0. });
        }
        if let Some((at, size, seed)) = flame {
            // Blocky flicker: the flame jumps between a few sizes and offsets.
            let step = (now * 14. + seed * 5.).floor();
            let pick = (step * 12.9898 + seed * 78.233).sin().abs();
            let grow = 0.7 + pick * 0.6;
            t.scale = Vec3::splat(size * grow);
            t.translation = at - Vec3::X * size * (grow - 1.) * 0.8
                + Vec3::Y * ((pick * 7.).floor() - 3.) * 1.5;
        }
    }
}
