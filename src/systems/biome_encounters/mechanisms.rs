//! Sparse landmark machines. Moving parts use kinematic colliders, not terrain
//! tiles: animation must not rebuild the entire terrain spatial hash each tick.
use super::*;
use crate::components::collision_layers::GameLayer;
mod bubbles;
mod contraption_probe;
#[cfg(test)]
mod contraption_tests;
mod contraptions;
mod music;
mod pads;
pub(super) mod probe;
mod traps;
pub(crate) use music::Stunned;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Gate,
    Piston,
    Slime,
}
#[derive(Component)]
struct Machine {
    kind: Kind,
    home: Vec3,
    amount: f32,
    hold: f32,
    cycle: f32,
    fired: bool,
    support: Option<Entity>,
    floor: Vec2,
    unbound_age: f32,
}
#[derive(Component)]
struct Housing(Entity);
/// Pivot of a gate's door panel (its left edge).
#[derive(Component)]
struct GateHinge;
#[derive(Resource, Clone)]
pub(super) struct MachineArt {
    cube: Handle<Mesh>,
    face: Handle<Mesh>,
    side: Handle<StandardMaterial>,
    regular: Handle<StandardMaterial>,
    sticky: Handle<StandardMaterial>,
    door: [Handle<StandardMaterial>; 2],
    metal: Handle<StandardMaterial>,
    gadgets: contraptions::GadgetArt,
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(
            FixedUpdate,
            (
                drive,
                pads::drive_pads,
                traps::drive_traps,
                bubbles::push_columns,
                music::drive_music,
                music::hold_stunned,
            )
                .run_if(in_state(crate::states::InGame)),
        )
        .add_systems(
            Update,
            (
                maintain_support,
                animate_gates,
                contraptions::maintain_anchors,
                bubbles::animate_bubbles,
                music::animate_music,
                traps::animate_tnt,
            )
                .run_if(in_state(crate::states::InGame)),
        );
}
/// How far an open door swings toward the camera: about 66 degrees, not 90,
/// so its face stays visible (narrowed) like an open door in a 2D game. A full
/// 90 degree swing is seen edge-on from the side and looked shut.
const DOOR_OPEN_ANGLE: f32 = 1.15;
/// Swings each gate door open on its hinge, toward the camera, and lets heads
/// through while it is open; it blocks again once fully shut.
fn animate_gates(
    mut commands: Commands,
    machines: Query<(Entity, &Machine, Has<ColliderDisabled>)>,
    mut hinges: Query<(&ChildOf, &mut Transform), With<GateHinge>>,
) {
    for (hinge, mut t) in &mut hinges {
        if let Ok((_, m, _)) = machines.get(hinge.parent()) {
            let swing = Transform::from_xyz(-30., 0., 0.)
                .with_rotation(Quat::from_rotation_y(-m.amount * DOOR_OPEN_ANGLE));
            if *t != swing {
                *t = swing;
            }
        }
    }
    for (e, m, disabled) in &machines {
        if m.kind != Kind::Gate {
            continue;
        }
        let open = m.amount > 0.02;
        if open && !disabled {
            commands.entity(e).try_insert(ColliderDisabled);
        } else if !open && disabled {
            commands.entity(e).try_remove::<ColliderDisabled>();
        }
    }
}
fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let mut texture = |name: &str| {
        mats.add(StandardMaterial {
            base_color_texture: Some(
                server
                    .load_builder()
                    .with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
                        s.sampler = bevy::image::ImageSampler::nearest()
                    })
                    .load(format!("textures/mechanisms/{name}.png")),
            ),
            perceptual_roughness: 0.85,
            ..default()
        })
    };
    let side = texture("piston_side");
    let regular = texture("piston_top");
    let sticky = texture("piston_top_sticky");
    let door = [texture("iron_door_bottom"), texture("iron_door_top")];
    let metal = mats.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.38, 0.39),
        perceptual_roughness: 0.7,
        ..default()
    });
    let gadgets = contraptions::load_art(&server, &mut meshes, &mut mats);
    commands.insert_resource(MachineArt {
        gadgets,
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        face: meshes.add(Rectangle::new(1., 1.)),
        side,
        regular,
        sticky,
        door,
        metal,
    });
}
fn solid_cells(site: Site) -> std::collections::HashSet<IVec2> {
    layout(site)
        .into_iter()
        .filter(|(_, k)| !crate::systems::plant_rules::soft(*k, None))
        .map(|(p, _)| p)
        .collect()
}
fn positions(site: Site) -> (Option<IVec2>, Option<IVec2>) {
    let cells = solid_cells(site);
    let empty = |p| !cells.contains(&p);
    let mut gate = None;
    let mut piston = None;
    for y in -5..=2 {
        for x in -site.width() + 1..site.width() {
            let p = IVec2::new(x, y);
            if !cells.contains(&p) || !empty(p + IVec2::Y) || !empty(p + IVec2::Y * 2) {
                continue;
            }
            if gate.is_none()
                && cells.contains(&(p + IVec2::Y * 3))
                && [-1, 1]
                    .into_iter()
                    .all(|dx| (1..=2).all(|dy| empty(p + IVec2::new(dx, dy))))
            {
                gate = Some(p);
            }
        }
    }
    for y in -5..=1 {
        for x in -site.width() + 2..site.width() - 1 {
            let p = IVec2::new(x, y);
            if gate.is_some_and(|g| (g - p).abs().max_element() < 3) {
                continue;
            }
            // Open on both sides at head height above the piston body, so a head
            // that lands on it can walk off instead of being caught in a shaft.
            let walk_off = [-1, 1]
                .into_iter()
                .all(|dx| (2..=3).all(|dy| empty(p + IVec2::new(dx, dy))));
            if cells.contains(&p) && (1..=4).all(|dy| empty(p + IVec2::Y * dy)) && walk_off {
                piston = Some(p);
                break;
            }
        }
        if piston.is_some() {
            break;
        }
    }
    (gate, piston)
}
fn part(
    commands: &mut Commands,
    parent: Entity,
    mesh: &Handle<Mesh>,
    mat: &Handle<StandardMaterial>,
    pos: Vec3,
    size: Vec3,
) {
    commands.spawn((
        ChildOf(parent),
        Mesh3d(mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(pos).with_scale(size),
    ));
}
pub(super) fn spawn(commands: &mut Commands, site: Site, art: &MachineArt) {
    contraptions::spawn_site(commands, site, &art.gadgets);
    let h = hash(
        site.seed
            ^ (site.origin.x as u32).wrapping_mul(19349663)
            ^ (site.origin.y as u32).rotate_left(9),
    );
    if h % 100 >= 40 {
        return;
    }
    spawn_selected(commands, site, art, h.is_multiple_of(2));
}
fn spawn_selected(commands: &mut Commands, site: Site, art: &MachineArt, slime: bool) {
    let (gate, piston) = positions(site);
    for (floor, kind) in [
        (gate, Kind::Gate),
        (piston, if slime { Kind::Slime } else { Kind::Piston }),
    ] {
        let Some(floor) = floor else {
            continue;
        };
        let floor = ((site.origin + floor).as_vec2() * 64.).extend(-10.);
        // Gate: door centred on the two cells above the floor. Piston: the
        // head plate rests on top of a full piston block standing on the floor.
        let home = floor + Vec3::Y * if kind == Kind::Gate { 96. } else { 88. };
        let chunk = site.origin.div_euclid(IVec2::splat(16));
        let root = commands
            .spawn((
                Machine {
                    kind,
                    home,
                    amount: 0.,
                    hold: 0.,
                    cycle: 0.,
                    fired: false,
                    support: None,
                    floor: floor.xy(),
                    unbound_age: 0.,
                },
                ChunkMember {
                    chunk_x: chunk.x,
                    chunk_y: chunk.y,
                },
                Transform::from_translation(home),
                Visibility::Inherited,
                RigidBody::Kinematic,
                Position(home),
                LinearVelocity::ZERO,
                Collider::cuboid(60., if kind == Kind::Gate { 124. } else { 16. }, 60.),
                CollisionLayers::new(GameLayer::Block, GameLayer::Block.filters()),
                Friction::new(0.1),
                Restitution::new(0.3),
            ))
            .id();
        if kind == Kind::Gate {
            // The door turns on a hinge at its left edge (Minecraft-style);
            // `animate_gates` sets the hinge angle from `amount`.
            let hinge = commands
                .spawn((
                    ChildOf(root),
                    GateHinge,
                    Transform::from_xyz(-30., 0., 0.),
                    Visibility::Inherited,
                ))
                .id();
            part(
                commands,
                hinge,
                &art.cube,
                &art.metal,
                Vec3::new(30., 0., 0.),
                Vec3::new(60., 124., 16.),
            );
            for (i, mat) in art.door.iter().enumerate() {
                part(
                    commands,
                    hinge,
                    &art.face,
                    mat,
                    Vec3::new(30., i as f32 * 62. - 31., 9.),
                    Vec3::new(60., 62., 1.),
                );
            }
        } else {
            // Moving head: a full-width plate with the piston face on top.
            part(
                commands,
                root,
                &art.cube,
                &art.side,
                Vec3::ZERO,
                Vec3::new(62., 16., 62.),
            );
            // Arm: hidden inside the block when retracted, visible when extended.
            part(
                commands,
                root,
                &art.cube,
                &art.metal,
                Vec3::new(0., -40., 0.),
                Vec3::new(14., 64., 14.),
            );
            let cap = if kind == Kind::Slime {
                &art.sticky
            } else {
                &art.regular
            };
            commands.spawn((
                ChildOf(root),
                Mesh3d(art.face.clone()),
                MeshMaterial3d(cap.clone()),
                Transform::from_xyz(0., 8.2, 0.)
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                    .with_scale(Vec3::new(62., 62., 1.)),
            ));
            // The piston body: a full, solid block standing on the floor. It stays
            // put while the head extends one block up out of it.
            commands.spawn((
                Housing(root),
                ChunkMember {
                    chunk_x: chunk.x,
                    chunk_y: chunk.y,
                },
                Mesh3d(art.cube.clone()),
                MeshMaterial3d(art.side.clone()),
                Transform::from_translation(floor + Vec3::Y * 64.).with_scale(Vec3::splat(64.)),
                RigidBody::Static,
                // Unit collider: the scale of 64 makes it exactly one block.
                Collider::cuboid(1., 1., 1.),
                CollisionLayers::new(GameLayer::Block, GameLayer::Block.filters()),
                Friction::new(0.1),
                Restitution::new(0.3),
            ));
        }
    }
}
fn maintain_support(
    mut commands: Commands,
    time: Res<Time>,
    spatial: Res<crate::resources::spatial_hash::SpatialHash>,
    blocks: Query<&Transform, With<Block>>,
    mut machines: Query<(Entity, &mut Machine)>,
    housing: Query<(Entity, &Housing)>,
) {
    for (e, mut m) in &mut machines {
        if let Some(support) = m.support {
            if !blocks.contains(support) {
                commands.entity(e).despawn();
            }
        } else {
            m.support = spatial
                .rect_iter(m.floor - Vec2::splat(2.), m.floor + Vec2::splat(2.))
                .find(|e| {
                    blocks
                        .get(*e)
                        .is_ok_and(|t| t.translation.xy().distance_squared(m.floor) < 4.)
                });
            m.unbound_age += time.delta_secs();
            // Permit the chunk's deferred terrain commands to reach the index first.
            if m.support.is_none() && m.unbound_age > 1. {
                commands.entity(e).despawn();
            }
        }
    }
    for (e, h) in &housing {
        if !machines.contains(h.0) {
            commands.entity(e).despawn();
        }
    }
}
fn gate_obstructed(home: Vec3, p: Vec3, half: Vec2) -> bool {
    (p.x - home.x).abs() < 32. + half.x + 12.
        && p.y + half.y > home.y - 64. - 12.
        && p.y - half.y < home.y + 64. + 144. + 12.
}
fn piston_extension(age: f32) -> f32 {
    let t = if age < 0.22 {
        age / 0.22
    } else if age < 0.52 {
        1.
    } else {
        1. - (age - 0.52) / 0.38
    };
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}
type MachineHeads = (
    With<Health>,
    Without<Machine>,
    Without<crate::components::enemy::Flying>,
    Without<crate::systems::launcher::LauncherPassenger>,
);
fn drive(
    time: Res<Time<Fixed>>,
    mut machines: Query<(&mut Machine, &Position, &mut LinearVelocity)>,
    mut heads: Query<(&Transform, &Position, &mut LinearVelocity), MachineHeads>,
) {
    let dt = time.delta_secs();
    if dt <= 0. {
        return;
    }
    for (mut m, p, mut velocity) in &mut machines {
        let target = if m.kind == Kind::Gate {
            let near = heads.iter().any(|(t, p, _)| {
                ((p.0.x - m.home.x).abs() < 280. && (p.0.y - m.home.y).abs() < 190.)
                    || gate_obstructed(m.home, p.0, t.scale.xy().abs() * 32.)
            });
            if near {
                m.hold = 2.5;
            } else {
                m.hold = (m.hold - dt).max(0.);
            }
            let goal = if m.hold > 0. { 1. } else { 0. };
            m.amount = (m.amount + (goal - m.amount).clamp(-dt * 1.5, dt * 3.5)).clamp(0., 1.);
            // The door swings on its hinge (see `animate_gates`); the body stays put.
            m.home
        } else {
            m.hold = (m.hold - dt).max(0.);
            if m.cycle == 0.
                && m.hold == 0.
                && heads.iter().any(|(t, p, _)| {
                    (p.0.x - m.home.x).abs() < 30. + 32. * t.scale.x.abs()
                        && p.0.y > m.home.y
                        && p.0.y < m.home.y + 110.
                })
            {
                m.cycle = dt;
                m.fired = false;
                m.hold = 2.4;
            }
            if m.cycle > 0. {
                m.cycle += dt;
                let release = if m.kind == Kind::Slime { 0.38 } else { 0.16 };
                if !m.fired && m.cycle >= release {
                    for (t, p, mut v) in &mut heads {
                        if (p.0.x - m.home.x).abs() < 30. + 32. * t.scale.x.abs()
                            && p.0.y > m.home.y
                            && p.0.y < m.home.y + 145.
                        {
                            v.y = v.y.max(if m.kind == Kind::Slime { 620. } else { 420. });
                            // A sideways nudge, so the head lands beside the piston
                            // rather than back on it for another launch.
                            if v.x.abs() < 150. {
                                let side = if p.0.x >= m.home.x { 1. } else { -1. };
                                v.x = side * 180.;
                            }
                        }
                    }
                    m.fired = true;
                }
                m.amount = piston_extension(m.cycle);
                if m.cycle >= 0.9 {
                    m.cycle = 0.;
                    m.amount = 0.;
                }
            }
            // The head extends one full block out of the piston body.
            m.home + Vec3::Y * (m.amount * 64.)
        };
        velocity.0 = (target - p.0) / dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controller_opens_occupied_gates_and_slime_launches_harder_than_pistons() {
        for kind in [Kind::Gate, Kind::Piston, Kind::Slime] {
            let mut app = App::new();
            app.init_resource::<Time<Fixed>>()
                .add_systems(Update, drive);
            let machine = app
                .world_mut()
                .spawn((
                    Machine {
                        kind,
                        home: Vec3::ZERO,
                        amount: 0.,
                        hold: 0.,
                        cycle: 0.,
                        fired: false,
                        support: None,
                        floor: Vec2::ZERO,
                        unbound_age: 0.,
                    },
                    Position::default(),
                    LinearVelocity::ZERO,
                ))
                .id();
            let head = app
                .world_mut()
                .spawn((
                    Health::new(3),
                    Transform::from_xyz(0., 65., 0.),
                    Position(Vec3::Y * 65.),
                    LinearVelocity::ZERO,
                ))
                .id();
            for _ in 0..100 {
                app.world_mut()
                    .resource_mut::<Time<Fixed>>()
                    .advance_by(std::time::Duration::from_millis(10));
                app.world_mut().run_schedule(Update);
            }
            if kind == Kind::Gate {
                assert_eq!(app.world().get::<Machine>(machine).unwrap().amount, 1.);
            } else {
                assert_eq!(
                    app.world().get::<LinearVelocity>(head).unwrap().y,
                    if kind == Kind::Slime { 620. } else { 420. }
                );
            }
        }
    }
    #[test]
    fn gate_never_closes_through_a_large_or_ordinary_head() {
        let home = Vec3::ZERO;
        assert!(gate_obstructed(
            home,
            Vec3::new(50., 0., 0.),
            Vec2::splat(32.)
        ));
        assert!(gate_obstructed(
            home,
            Vec3::new(110., 140., 0.),
            Vec2::splat(96.)
        ));
        assert!(!gate_obstructed(
            home,
            Vec3::new(200., 0., 0.),
            Vec2::splat(32.)
        ));
    }
    #[test]
    fn piston_has_a_hold_and_returns_to_its_housing() {
        assert_eq!(piston_extension(0.), 0.);
        assert_eq!(piston_extension(0.3), 1.);
        assert_eq!(piston_extension(1.), 0.);
        for i in 0..100 {
            assert!((0. ..=1.).contains(&piston_extension(i as f32 / 100.)));
        }
    }
    #[test]
    fn mechanisms_only_use_supported_open_space() {
        let mut gates = 0;
        let mut pistons = 0;
        for biome in [
            BiomeType::Plains,
            BiomeType::Forest,
            BiomeType::Desert,
            BiomeType::Mountains,
            BiomeType::Underground,
            BiomeType::Nether,
            BiomeType::End,
            BiomeType::Iceland,
        ] {
            for seed in 0..20 {
                let site = Site {
                    origin: IVec2::new(seed * 3, seed),
                    biome,
                    seed: seed as u32,
                };
                let cells = solid_cells(site);
                let (g, p) = positions(site);
                if let Some(g) = g {
                    gates += 1;
                    assert!(cells.contains(&g));
                    assert!(!cells.contains(&(g + IVec2::Y)));
                    assert!(!cells.contains(&(g + IVec2::Y * 2)));
                }
                if let Some(p) = p {
                    pistons += 1;
                    assert!(cells.contains(&p));
                    assert!((1..=4).all(|y| !cells.contains(&(p + IVec2::Y * y))));
                    for dx in [-1, 1] {
                        assert!((2..=3).all(|y| !cells.contains(&(p + IVec2::new(dx, y)))));
                    }
                }
            }
        }
        assert!(gates > 0 && pistons > 0);
    }
}
