//! Landmark contraptions: static Minecraft blocks with small, readable rules
//! (slime and honey blocks, dispenser and TNT tripwires, bubble columns, note
//! blocks and bells). Each is a few avian colliders on the Block layer plus a
//! FixedUpdate rule that edits head velocities; none of them are terrain tiles,
//! so they never touch the terrain spatial hash or the fluid grid.
use super::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Gadget {
    SlimePad,
    Honey,
    Dispenser,
    Bubbles,
    Notes,
    Bell,
    Tripwire,
}
pub(super) const GADGETS: [Gadget; 7] = [
    Gadget::SlimePad,
    Gadget::Honey,
    Gadget::Dispenser,
    Gadget::Bubbles,
    Gadget::Notes,
    Gadget::Bell,
    Gadget::Tripwire,
];
/// Share of landmarks that receive contraptions (independent of gates/pistons).
const GADGET_CHANCE: u32 = 65;
/// One contraption: `cell` is its first block (site-relative while planning,
/// absolute once spawned). `len` is the wire length, water height, note row
/// length or honey height; `dir` the dispenser facing or bubble direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placement {
    pub(crate) gadget: Gadget,
    pub(crate) cell: IVec2,
    pub(crate) len: i32,
    pub(crate) dir: i32,
    /// Terrain under each note block (instrument choice).
    pub(crate) under: [BlockType; 6],
}
impl Placement {
    pub(crate) fn new(gadget: Gadget, cell: IVec2, len: i32, dir: i32) -> Self {
        Self {
            gadget,
            cell,
            len,
            dir,
            under: [BlockType::Dirt; 6],
        }
    }
}
/// The terrain block a contraption stands on (or hangs from). Without it the
/// contraption despawns, like a gate or piston losing its floor.
#[derive(Component)]
pub(crate) struct Anchor {
    pub(crate) gadget: Gadget,
    floor: Vec2,
    support: Option<Entity>,
    age: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side {
    Top,
    Bottom,
    Left,
    Right,
}
/// Which face of a box (`centre`, `half`) a head box is touching, if any.
/// `Right` means the head is on the right of the box.
pub(super) fn touching(
    head: Vec2,
    head_half: Vec2,
    centre: Vec2,
    half: Vec2,
    slop: f32,
) -> Option<Side> {
    let d = head - centre;
    let gap = d.abs() - (head_half + half);
    if gap.x > slop || gap.y > slop {
        return None;
    }
    // The axis with the smaller penetration is the contact normal.
    Some(if gap.y >= gap.x {
        if d.y > 0. { Side::Top } else { Side::Bottom }
    } else if d.x > 0. {
        Side::Right
    } else {
        Side::Left
    })
}
pub(super) fn head_half(t: &Transform) -> Vec2 {
    t.scale.xy().abs() * 32.
}
pub(super) fn cell_centre(cell: IVec2) -> Vec2 {
    cell.as_vec2() * 64.
}

#[derive(Clone)]
pub(crate) struct GadgetArt {
    pub(super) cube: Handle<Mesh>,
    pub(super) face: Handle<Mesh>,
    pub(super) slime: Handle<StandardMaterial>,
    pub(super) honey: Handle<StandardMaterial>,
    pub(super) honey_top: Handle<StandardMaterial>,
    pub(super) dispenser: Handle<StandardMaterial>,
    pub(super) furnace: Handle<StandardMaterial>,
    pub(super) furnace_top: Handle<StandardMaterial>,
    pub(super) note_block: Handle<StandardMaterial>,
    pub(super) wire: Handle<StandardMaterial>,
    pub(super) string: Handle<StandardMaterial>,
    pub(super) hook: Handle<StandardMaterial>,
    pub(super) soul_sand: Handle<StandardMaterial>,
    pub(super) magma: Handle<StandardMaterial>,
    pub(super) water: Handle<StandardMaterial>,
    pub(super) bubble: Handle<StandardMaterial>,
    pub(super) tnt: Handle<StandardMaterial>,
    pub(super) tnt_top: Handle<StandardMaterial>,
    pub(super) tnt_lit: Handle<StandardMaterial>,
    pub(super) bell_body: Handle<StandardMaterial>,
    pub(super) bell_lip: Handle<StandardMaterial>,
    pub(super) iron: Handle<StandardMaterial>,
    pub(super) snowball_mesh: Handle<Mesh>,
    pub(super) snowball: Handle<StandardMaterial>,
    /// One tinted note particle per pitch (0..=24), Minecraft colours.
    pub(super) notes: Vec<Handle<StandardMaterial>>,
    pub(super) sounds: GadgetSounds,
}
#[derive(Clone)]
pub(crate) struct GadgetSounds {
    pub(super) slime: [Handle<AudioSource>; 3],
    pub(super) honey_slide: [Handle<AudioSource>; 2],
    pub(super) honey_step: Handle<AudioSource>,
    pub(super) click: Handle<AudioSource>,
    pub(super) bow: Handle<AudioSource>,
    pub(super) bubble_pop: Handle<AudioSource>,
    /// [up, down] ambient and entering sounds.
    pub(super) column_ambient: [Handle<AudioSource>; 2],
    pub(super) column_inside: [Handle<AudioSource>; 2],
    /// Indexed by `music::Instrument`.
    pub(super) instruments: [Handle<AudioSource>; 5],
    pub(super) bell: [Handle<AudioSource>; 2],
    pub(super) bell_resonate: Handle<AudioSource>,
    pub(super) fuse: Handle<AudioSource>,
}
/// Minecraft's note particle colour for pitch `n` (0..=24).
pub(super) fn note_colour(n: u8) -> Color {
    let t = n as f32 / 24. * std::f32::consts::TAU;
    let c = |phase: f32| ((t + phase).sin() * 0.65 + 0.35).max(0.);
    Color::srgb(
        c(0.),
        c(std::f32::consts::TAU / 3.),
        c(std::f32::consts::TAU * 2. / 3.),
    )
}
pub(super) fn load_art(
    server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
) -> GadgetArt {
    let image = |name: &str| -> Handle<Image> {
        server
            .load_builder()
            .with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
                s.sampler = bevy::image::ImageSampler::nearest()
            })
            .load(format!("textures/mechanisms/{name}.png"))
    };
    // Crop of an atlas or animation strip: (offset, size) in UV units.
    let crop = |offset: Vec2, size: Vec2| {
        bevy::math::Affine2::from_scale_angle_translation(size, 0., offset)
    };
    let mut add = |name: &str, alpha: AlphaMode, uv: bevy::math::Affine2, unlit: bool| {
        mats.add(StandardMaterial {
            base_color_texture: Some(image(name)),
            alpha_mode: alpha,
            uv_transform: uv,
            perceptual_roughness: 0.85,
            unlit,
            cull_mode: if unlit {
                None
            } else {
                Some(bevy::render::render_resource::Face::Back)
            },
            ..default()
        })
    };
    let full = bevy::math::Affine2::IDENTITY;
    let mask = AlphaMode::Mask(0.5);
    let slime = add("slime_block", AlphaMode::Blend, full, false);
    let honey = add("honey_block_side", AlphaMode::Blend, full, false);
    let honey_top = add("honey_block_top", AlphaMode::Blend, full, false);
    let dispenser = add("dispenser_front", AlphaMode::Opaque, full, false);
    let furnace = add("furnace_side", AlphaMode::Opaque, full, false);
    let furnace_top = add("furnace_top", AlphaMode::Opaque, full, false);
    let note_block = add("note_block", AlphaMode::Opaque, full, false);
    // The string occupies the top half of the tripwire texture.
    let wire = add("tripwire", mask, crop(Vec2::ZERO, Vec2::new(1., 0.5)), true);
    let hook = add("tripwire_hook", mask, full, true);
    let soul_sand = add("soul_sand", AlphaMode::Opaque, full, false);
    // Animation strips: show their first frame.
    let magma = add(
        "magma",
        AlphaMode::Opaque,
        crop(Vec2::ZERO, Vec2::new(1., 1. / 3.)),
        false,
    );
    let bubble = add("bubble", AlphaMode::Blend, full, true);
    let tnt = add("tnt_side", AlphaMode::Opaque, full, false);
    let tnt_top = add("tnt_top", AlphaMode::Opaque, full, false);
    // bell_body.png (128px): body sides at y 24..52, lip sides at y 84..92.
    let bell_body = add(
        "bell_body",
        AlphaMode::Opaque,
        crop(Vec2::splat(24. / 128.), Vec2::new(24., 28.) / 128.),
        false,
    );
    let bell_lip = add(
        "bell_body",
        AlphaMode::Opaque,
        crop(Vec2::new(32., 84.) / 128., Vec2::new(32., 8.) / 128.),
        false,
    );
    let water = mats.add(StandardMaterial {
        base_color: Color::srgba(0.32, 0.5, 0.95, 0.55),
        base_color_texture: Some(image("water_still")),
        uv_transform: crop(Vec2::ZERO, Vec2::new(1., 1. / 32.)),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.2,
        cull_mode: None,
        ..default()
    });
    let tnt_lit = mats.add(StandardMaterial {
        base_color_texture: Some(image("tnt_side")),
        emissive: LinearRgba::rgb(4., 4., 4.),
        unlit: true,
        ..default()
    });
    let iron = mats.add(StandardMaterial {
        base_color: Color::srgb(0.27, 0.27, 0.29),
        perceptual_roughness: 0.6,
        metallic: 0.4,
        ..default()
    });
    // Same texture and look as a yeti's snowball.
    let snowball = mats.add(StandardMaterial {
        base_color_texture: Some(server.load("textures/projectiles/snowball.png")),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });
    let string = mats.add(StandardMaterial {
        base_color: Color::srgb(0.78, 0.78, 0.74),
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let note_image = image("note");
    let notes = (0..=24)
        .map(|n| {
            mats.add(StandardMaterial {
                base_color: note_colour(n),
                base_color_texture: Some(note_image.clone()),
                alpha_mode: mask,
                unlit: true,
                cull_mode: None,
                ..default()
            })
        })
        .collect();
    let sound = |name: &str| server.load(format!("audio/mechanisms/{name}.ogg"));
    GadgetArt {
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        face: meshes.add(Rectangle::new(1., 1.)),
        slime,
        honey,
        honey_top,
        dispenser,
        furnace,
        furnace_top,
        note_block,
        wire,
        string,
        hook,
        soul_sand,
        magma,
        water,
        bubble,
        tnt,
        tnt_top,
        tnt_lit,
        bell_body,
        bell_lip,
        iron,
        snowball_mesh: meshes.add(Circle::new(12.)),
        snowball,
        notes,
        sounds: GadgetSounds {
            slime: [
                sound("slime_big1"),
                sound("slime_big2"),
                sound("slime_small1"),
            ],
            honey_slide: [sound("honey_slide1"), sound("honey_slide2")],
            honey_step: sound("honey_step1"),
            click: sound("click"),
            bow: sound("bow"),
            bubble_pop: sound("bubble_pop"),
            column_ambient: [
                sound("bubble_upwards_ambient"),
                sound("bubble_whirlpool_ambient"),
            ],
            column_inside: [
                sound("bubble_upwards_inside"),
                sound("bubble_whirlpool_inside"),
            ],
            instruments: [
                sound("note_harp"),
                sound("note_bd"),
                sound("note_snare"),
                sound("note_bass"),
                sound("note_icechime"),
            ],
            bell: [sound("bell_use1"), sound("bell_use2")],
            bell_resonate: sound("bell_resonate"),
            fuse: sound("fuse"),
        },
    }
}
/// Positional one-shot; `SoundAt` fades and culls far-off contraptions.
pub(super) fn play(
    commands: &mut Commands,
    sound: &Handle<AudioSource>,
    at: Vec2,
    volume: f32,
    speed: f32,
) {
    commands.spawn((
        AudioPlayer::new(sound.clone()),
        PlaybackSettings::DESPAWN
            .with_volume(bevy::audio::Volume::Linear(volume))
            .with_speed(speed),
        crate::systems::audio::effects::SoundAt(at),
    ));
}

// ---------------------------------------------------------------- placement
struct Grid {
    site: Site,
    cells: HashMap<IVec2, BlockType>,
    solid: HashSet<IVec2>,
    used: HashSet<IVec2>,
}
impl Grid {
    fn new(site: Site) -> Self {
        Self {
            site,
            cells: layout(site).into_iter().collect(),
            solid: solid_cells(site),
            used: HashSet::new(),
        }
    }
    /// Reserved landmark air: no layout block, no other machine.
    fn free(&self, p: IVec2) -> bool {
        p.x.abs() <= self.site.width()
            && (-5..=7).contains(&p.y)
            && !self.cells.contains_key(&p)
            && !self.used.contains(&p)
    }
    fn floor(&self, p: IVec2) -> bool {
        self.solid.contains(&p) && self.cells.get(&p).is_some_and(|k| k.supports_fixture())
    }
    fn claim(&mut self, cells: impl IntoIterator<Item = IVec2>) {
        for c in cells {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    self.used.insert(c + IVec2::new(dx, dy));
                }
            }
        }
    }
    fn free_up(&self, p: IVec2, from: i32, to: i32) -> bool {
        (from..=to).all(|dy| self.free(p + IVec2::Y * dy))
    }
}
/// Occupied cells of a site-relative placement (for spacing and tests).
pub(super) fn footprint(p: &Placement) -> Vec<IVec2> {
    let c = p.cell;
    match p.gadget {
        Gadget::SlimePad | Gadget::Bell => vec![c],
        Gadget::Honey => (0..p.len).map(|i| c + IVec2::Y * i).collect(),
        Gadget::Dispenser => (0..=p.len).map(|i| c + IVec2::X * i * p.dir).collect(),
        Gadget::Bubbles => (0..=p.len).map(|i| c + IVec2::Y * i).collect(),
        Gadget::Notes | Gadget::Tripwire => (0..p.len).map(|i| c + IVec2::X * i).collect(),
    }
}
fn find(grid: &Grid, gadget: Gadget, h: u32) -> Option<Placement> {
    let w = grid.site.width();
    let span = 2 * w - 1;
    let shift = (h % span as u32) as i32;
    for y in -6..=5 {
        for i in 0..span {
            let x = -w + 1 + (i + shift) % span;
            let f = IVec2::new(x, y);
            if let Some(p) = fits(grid, gadget, f, h) {
                return Some(p);
            }
        }
    }
    None
}
/// Whether `gadget` fits with its floor (or ceiling, for bells) at `f`.
fn fits(g: &Grid, gadget: Gadget, f: IVec2, h: u32) -> Option<Placement> {
    let up = |dy: i32| f + IVec2::Y * dy;
    let at = |dx: i32, dy: i32| f + IVec2::new(dx, dy);
    match gadget {
        Gadget::SlimePad => {
            (g.floor(f) && g.free_up(f, 1, 4)).then(|| Placement::new(gadget, up(1), 1, 0))
        }
        Gadget::Honey => (g.floor(f)
            && g.free_up(f, 1, 3)
            && [-1, 1]
                .into_iter()
                .any(|dx| g.free(at(dx, 1)) && g.free(at(dx, 2))))
        .then(|| Placement::new(gadget, up(1), 2, 0)),
        Gadget::Dispenser => [1, -1].into_iter().find_map(|dir| {
            let ok = (0..=2).all(|i| g.floor(at(i * dir, 0)) && g.free(at(i * dir, 1)))
                && (1..=2).all(|i| g.free(at(i * dir, 2)));
            ok.then(|| Placement::new(gadget, up(1), 2, dir))
        }),
        Gadget::Bubbles => {
            if !g.floor(f) {
                return None;
            }
            let open = (1..=6).take_while(|dy| g.free(up(*dy))).count() as i32;
            // Base block plus 3..=5 water cells.
            (open >= 4).then(|| {
                Placement::new(
                    gadget,
                    up(1),
                    (open - 1).min(5),
                    if h & 16 == 0 { 1 } else { -1 },
                )
            })
        }
        Gadget::Notes => {
            let run = (0..6)
                .take_while(|i| g.floor(at(*i, 0)) && g.free(at(*i, 1)) && g.free(at(*i, 2)))
                .count() as i32;
            if run < 3 {
                return None;
            }
            let len = run.min(3 + (h >> 5) as i32 % 4);
            let mut p = Placement::new(gadget, up(1), len, 0);
            for i in 0..len {
                p.under[i as usize] = g.cells[&at(i, 0)];
            }
            Some(p)
        }
        Gadget::Bell => {
            // `f` is the ceiling; the bell hangs in the cell below it, low
            // enough for bouncing heads to ring it.
            (g.floor(f)
                && g.free(up(-1))
                && g.free(up(-2))
                && (g.floor(up(-3)) || (g.free(up(-3)) && g.floor(up(-4)))))
            .then(|| Placement::new(gadget, up(-1), 1, 0))
        }
        Gadget::Tripwire => {
            // A corridor: wall, 1..=5 open cells over floor, wall.
            if !g.solid.contains(&at(-1, 1)) || !g.floor(f) || !g.free(at(0, 1)) {
                return None;
            }
            let run = (0..5)
                .take_while(|i| g.floor(at(*i, 0)) && g.free(at(*i, 1)))
                .count() as i32;
            (run >= 1 && g.solid.contains(&at(run, 1)))
                .then(|| Placement::new(gadget, up(1), run, 0))
        }
    }
}
/// Deterministic site-relative contraptions for a landmark: two or three
/// different kinds in ~65% of landmarks, never touching each other or the
/// landmark's gate and piston spots.
pub(super) fn plan(site: Site) -> Vec<Placement> {
    let h = hash(
        site.seed.rotate_left(7)
            ^ (site.origin.x as u32).wrapping_mul(0x9e37_79b1)
            ^ (site.origin.y as u32).wrapping_mul(83_492_791),
    );
    if h % 100 >= GADGET_CHANCE {
        return Vec::new();
    }
    plan_kinds(site, h, 2 + (h >> 8) as usize % 2)
}
pub(super) fn plan_kinds(site: Site, h: u32, want: usize) -> Vec<Placement> {
    let mut grid = Grid::new(site);
    let (gate, piston) = positions(site);
    if let Some(g) = gate {
        grid.claim([g + IVec2::Y, g + IVec2::Y * 2]);
    }
    if let Some(p) = piston {
        grid.claim((1..=3).map(|dy| p + IVec2::Y * dy));
    }
    let start = (h >> 12) as usize % GADGETS.len();
    let mut out = Vec::new();
    for i in 0..GADGETS.len() {
        if out.len() >= want {
            break;
        }
        // Step 3 is coprime with 7: visits every kind once.
        let gadget = GADGETS[(start + i * 3) % GADGETS.len()];
        if let Some(p) = find(&grid, gadget, hash(h ^ i as u32)) {
            grid.claim(footprint(&p));
            out.push(p);
        }
    }
    out
}

// ---------------------------------------------------------------- spawning
pub(super) fn spawn_site(commands: &mut Commands, site: Site, art: &GadgetArt) {
    for p in plan(site) {
        let mut p = p;
        p.cell += site.origin;
        spawn_placement(commands, p, site.biome, site.origin, art);
    }
}
/// Spawns one contraption at absolute cells. `owner` selects the chunk that
/// streams it out (the landmark origin for landmark placements).
pub(crate) fn spawn_placement(
    commands: &mut Commands,
    p: Placement,
    biome: BiomeType,
    owner: IVec2,
    art: &GadgetArt,
) {
    let chunk = owner.div_euclid(IVec2::splat(16));
    let member = ChunkMember {
        chunk_x: chunk.x,
        chunk_y: chunk.y,
    };
    let support = match p.gadget {
        Gadget::Bell => p.cell + IVec2::Y,
        _ => p.cell - IVec2::Y,
    };
    let anchor = Anchor {
        gadget: p.gadget,
        floor: cell_centre(support),
        support: None,
        age: 0.,
    };
    match p.gadget {
        Gadget::SlimePad | Gadget::Honey => super::pads::spawn(commands, p, (anchor, member), art),
        Gadget::Dispenser | Gadget::Tripwire => {
            super::traps::spawn(commands, p, biome, (anchor, member), art)
        }
        Gadget::Bubbles => super::bubbles::spawn(commands, p, (anchor, member), art),
        Gadget::Notes | Gadget::Bell => super::music::spawn(commands, p, (anchor, member), art),
    }
}
/// A full static block collider on the Block layer, like the piston housing.
pub(super) fn solid(size: Vec2, friction: Friction, restitution: Restitution) -> impl Bundle {
    (
        RigidBody::Static,
        Collider::cuboid(size.x, size.y, 64.),
        CollisionLayers::new(GameLayer::Block, GameLayer::Block.filters()),
        friction,
        restitution,
    )
}
pub(super) fn piece(
    commands: &mut Commands,
    parent: Entity,
    mesh: &Handle<Mesh>,
    mat: &Handle<StandardMaterial>,
    transform: Transform,
) -> Entity {
    commands
        .spawn((
            ChildOf(parent),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            transform,
        ))
        .id()
}
/// A textured cube with a separate top face (perspective shows block tops).
pub(super) fn cube_with_top(
    commands: &mut Commands,
    parent: Entity,
    art: &GadgetArt,
    side: &Handle<StandardMaterial>,
    top: &Handle<StandardMaterial>,
    centre: Vec3,
    size: Vec3,
) {
    piece(
        commands,
        parent,
        &art.cube,
        side,
        Transform::from_translation(centre).with_scale(size),
    );
    piece(
        commands,
        parent,
        &art.face,
        top,
        Transform::from_translation(centre + Vec3::Y * (size.y / 2. + 0.2))
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::new(size.x, size.z, 1.)),
    );
}
pub(super) fn maintain_anchors(
    mut commands: Commands,
    time: Res<Time>,
    spatial: Res<crate::resources::spatial_hash::SpatialHash>,
    blocks: Query<&Transform, With<Block>>,
    mut anchors: Query<(Entity, &mut Anchor)>,
) {
    for (e, mut a) in &mut anchors {
        if let Some(support) = a.support {
            if !blocks.contains(support) {
                commands.entity(e).despawn();
            }
            continue;
        }
        let floor = a.floor;
        a.support = spatial
            .rect_iter(floor - Vec2::splat(2.), floor + Vec2::splat(2.))
            .find(|e| {
                blocks
                    .get(*e)
                    .is_ok_and(|t| t.translation.xy().distance_squared(floor) < 4.)
            });
        a.age += time.delta_secs();
        // Permit the chunk's deferred terrain commands to reach the index first.
        if a.support.is_none() && a.age > 1. {
            commands.entity(e).despawn();
        }
    }
}
