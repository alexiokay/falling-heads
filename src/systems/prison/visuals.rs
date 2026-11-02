//! The prison scene: a bedrock cell with trapdoors and buttons, magnets on the
//! corners, heads, guards with batons, blast flashes, a starfield and the HUD.
//! Everything is unlit and on render layer 1, which the camera alone shows.
use super::sim::{self, Cell, Event, Kind};
use super::*;

#[derive(Resource)]
pub(super) struct Art {
    cube: Handle<Mesh>,
    quad: Handle<Mesh>,
    bedrock: Handle<StandardMaterial>,
    backwall: Handle<StandardMaterial>,
    trapdoor: Handle<StandardMaterial>,
    button: Handle<StandardMaterial>,
    button_pressed: Handle<StandardMaterial>,
    guard: Handle<StandardMaterial>,
    villager: Handle<StandardMaterial>,
    stars: Handle<StandardMaterial>,
    flash: Handle<StandardMaterial>,
    magnet: Handle<StandardMaterial>,
    coil: Handle<StandardMaterial>,
    baton: Handle<StandardMaterial>,
    body: Handle<StandardMaterial>,
    boom: [Handle<AudioSource>; 4],
    click: Handle<AudioSource>,
    bell: Handle<AudioSource>,
    hurt: Handle<AudioSource>,
}

/// The cell, turned with the simulation's angle.
#[derive(Component)]
struct CellRoot;
#[derive(Component)]
struct DoorTile(usize);
#[derive(Component)]
struct ButtonTile(usize);
#[derive(Component)]
struct Magnet;
#[derive(Component)]
struct HeadVisual(usize);
#[derive(Component)]
struct Baton;
#[derive(Component)]
struct Flash(f32);
#[derive(Component)]
struct Hud;
#[derive(Component)]
struct Veil;

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
    let magnet = flat(None, Color::srgb(0.85, 0.12, 0.1));
    let coil = flat(None, Color::srgb(0.75, 0.55, 0.25));
    let baton = flat(None, Color::srgb(0.12, 0.1, 0.1));
    let body = flat(None, Color::srgb(0.2, 0.2, 0.24));
    let flash = mats.add(StandardMaterial {
        base_color: Color::srgba(1., 0.75, 0.3, 0.75),
        emissive: LinearRgba::rgb(4., 2.2, 0.6),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.insert_resource(Art {
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        quad: meshes.add(Rectangle::new(1., 1.)),
        bedrock,
        backwall,
        trapdoor,
        button,
        button_pressed,
        guard,
        villager,
        stars,
        flash,
        magnet,
        coil,
        baton,
        body,
        boom: std::array::from_fn(|i| {
            server.load(format!("audio/sfx/explode/explode{}.ogg", i + 1))
        }),
        click: server.load("audio/mechanisms/click.ogg"),
        bell: server.load("audio/mechanisms/bell_use1.ogg"),
        hurt: server.load("audio/sfx/mobs/zombie/hurt1.ogg"),
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

fn art(w: &World) -> Art {
    let a = w.resource::<Art>();
    Art {
        cube: a.cube.clone(),
        quad: a.quad.clone(),
        bedrock: a.bedrock.clone(),
        backwall: a.backwall.clone(),
        trapdoor: a.trapdoor.clone(),
        button: a.button.clone(),
        button_pressed: a.button_pressed.clone(),
        guard: a.guard.clone(),
        villager: a.villager.clone(),
        stars: a.stars.clone(),
        flash: a.flash.clone(),
        magnet: a.magnet.clone(),
        coil: a.coil.clone(),
        baton: a.baton.clone(),
        body: a.body.clone(),
        boom: a.boom.clone(),
        click: a.click.clone(),
        bell: a.bell.clone(),
        hurt: a.hurt.clone(),
    }
}

pub(super) fn scene(w: &mut World, origin: Vec3, hero: Entity) {
    let a = art(w);
    let root = w
        .spawn((
            Scene,
            CellRoot,
            Transform::from_translation(origin),
            Visibility::Inherited,
        ))
        .id();
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
    // Buttons on the walls' inner faces.
    let buttons = sim::Cell::new(1, 0).buttons;
    for (i, b) in buttons.iter().enumerate() {
        // Seen from the side a flat button is a sliver: make it a chunky bump
        // sticking out of the wall, its face toward the camera.
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
    // Electromagnets on the outer corners: red horseshoes with copper coils.
    for corner in [
        Vec2::new(1., 1.),
        Vec2::new(-1., 1.),
        Vec2::new(1., -1.),
        Vec2::new(-1., -1.),
    ] {
        let p = corner * (sim::HALF + step + 40.);
        let m = mesh(
            w,
            Some(root),
            &a.cube,
            &a.magnet,
            Transform::from_translation(p.extend(0.)).with_scale(Vec3::new(70., 70., 50.)),
            Magnet,
        );
        mesh(
            w,
            Some(m),
            &a.cube,
            &a.coil,
            Transform::from_scale(Vec3::new(1.1, 0.35, 1.1)),
            (),
        );
    }
    // Starfield far behind, not turning with the cell.
    for x in -4..4 {
        for y in -4..4 {
            mesh(
                w,
                None,
                &a.quad,
                &a.stars,
                Transform::from_translation(
                    origin + Vec3::new(x as f32 * 1500. + 750., y as f32 * 1500. + 750., -2500.),
                )
                .with_scale(Vec3::splat(1500.)),
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

/// The hero's face material for this visit.
#[derive(Resource)]
struct HeroFace(Handle<StandardMaterial>);

fn spawn_head(w: &mut World, root: Entity, index: usize, kind: Kind) {
    let a = art(w);
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

fn play(w: &mut World, clip: Handle<AudioSource>, volume: f32) {
    w.spawn((
        Scene,
        AudioPlayer::new(clip),
        PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(volume)),
    ));
}

pub(super) fn animate(
    w: &mut World,
    origin: Vec3,
    cell: &Cell,
    events: &[Event],
    villagers: usize,
    veil: f32,
) {
    let a = art(w);
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
    let pulse = 1. + (cell.time * 6.).sin() * 0.06;
    for mut t in w
        .query_filtered::<&mut Transform, With<Magnet>>()
        .iter_mut(w)
    {
        t.scale = Vec3::new(70., 70., 50.) * pulse;
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
    // Blasts: a flash that swells and fades.
    for e in events {
        match *e {
            Event::Blast(at) => {
                let i = (cell.time * 7.) as usize % 4;
                play(w, a.boom[i].clone(), 0.5);
                mesh(
                    w,
                    Some(root),
                    &a.cube,
                    &a.flash,
                    Transform::from_translation(at.extend(40.)).with_scale(Vec3::splat(40.)),
                    Flash(0.),
                );
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
    for e in done {
        w.despawn(e);
    }
    let left = (sim::TIME_LIMIT - cell.time).max(0.);
    let doors_open = cell.doors.iter().filter(|d| d.open > 0.).count();
    for mut text in w.query_filtered::<&mut Text, With<Hud>>().iter_mut(w) {
        **text = format!(
            "SPACE PRISON  {left:.0}s  |  villagers out {}/{villagers}  |  guards {}\n{}\nWASD / arrows: thrust  |  Esc: give up",
            cell.saved,
            cell.guards(),
            if doors_open > 0 {
                "TRAPDOOR OPEN - GO GO GO!"
            } else {
                "Hit a red button: it opens the trapdoor across the room"
            }
        );
    }
    for mut c in w
        .query_filtered::<&mut BackgroundColor, With<Veil>>()
        .iter_mut(w)
    {
        c.0 = Color::srgba(0., 0., 0., veil);
    }
}
