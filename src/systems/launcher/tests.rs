use super::*;
#[test]
fn launch_preserves_spin_velocity_and_starts_gently() {
    let h = 0.001;
    let before = (roll_angle(WARMUP) - roll_angle(WARMUP - h)) / h;
    let after = (roll_angle(WARMUP + h) - roll_angle(WARMUP)) / h;
    assert!((before - after).abs() < 0.05);
    assert!(roll_angle(h) / h < 0.01);
}
#[test]
fn left_and_right_launches_have_no_angle_wrap_snap_or_reverse_motion() {
    for sign in [-1., 1.] {
        for seed in 0..64 {
            let a = Vec3::new(seed as f32 * 17., -770., 0.);
            let b = destination(
                a,
                if sign < 0. {
                    LaunchDirection::Left
                } else {
                    LaunchDirection::Right
                },
                seed,
            );
            let mut heading = std::f32::consts::FRAC_PI_2;
            let mut previous = path(a, b, 0.);
            for frame in 0..240 {
                let u = frame as f32 / 239.;
                let p = path(a, b, u);
                assert!((p.x - previous.x) * sign >= -0.001);
                previous = p;
                let next = turn_toward(heading, nose_heading(a, b, u), 1. / 60.);
                assert!((next - heading).abs() <= 2.4 / 60. + 0.0001);
                heading = next;
            }
        }
    }
    assert!(roll_angle(WARMUP) < std::f32::consts::FRAC_PI_2);
}
#[test]
fn duplicate_and_cooldown_requests_are_not_queued() {
    let mut app = App::new();
    app.init_resource::<LauncherState>()
        .init_resource::<super::super::world_portals::PowerNotice>()
        .add_message::<PowerActivated>()
        .add_systems(Update, requests);
    for _ in 0..2 {
        app.world_mut().write_message(PowerActivated {
            command: PowerCommand::new(PowerCommandType::Launcher, "test".into()),
        });
    }
    app.update();
    assert!(app.world().resource::<LauncherState>().pending.is_some());
    assert!(
        app.world()
            .resource::<super::super::world_portals::PowerNotice>()
            .remaining
            > 0.
    );
    app.world_mut().resource_mut::<LauncherState>().pending = None;
    app.world_mut().resource_mut::<LauncherState>().cooldown = 2.;
    app.world_mut().write_message(PowerActivated {
        command: PowerCommand::new(PowerCommandType::Launcher, "test".into()),
    });
    app.update();
    assert!(app.world().resource::<LauncherState>().pending.is_none());
}
#[test]
fn cancellation_restores_existing_physics_and_flying_state() {
    let mut world = World::new();
    let hero = world
        .spawn((
            LauncherPassenger,
            Flying,
            RigidBody::Kinematic,
            GravityScale(0.),
            CollisionLayers::NONE,
            Visibility::Hidden,
            LinearVelocity::ZERO,
        ))
        .id();
    let f = Flight {
        hero,
        ship: Entity::PLACEHOLDER,
        exhaust: Entity::PLACEHOLDER,
        start: Vec3::ZERO,
        end: Vec3::X,
        audio: None,
        elapsed: 0.,
        waiting: 0.,
        heading: std::f32::consts::FRAC_PI_2,
        seed: 0,
        count: 1,
        saved: Saved {
            body: RigidBody::Dynamic,
            gravity: GravityScale(0.75),
            layers: CollisionLayers::ALL,
            visibility: Visibility::Inherited,
            flying: false,
        },
    };
    restore(&mut world, &f);
    assert!(world.get::<LauncherPassenger>(hero).is_none());
    assert!(world.get::<Flying>(hero).is_none());
    assert_eq!(*world.get::<RigidBody>(hero).unwrap(), RigidBody::Dynamic);
    assert_eq!(world.get::<GravityScale>(hero).unwrap().0, 0.75);
    assert_eq!(
        *world.get::<CollisionLayers>(hero).unwrap(),
        CollisionLayers::ALL
    );
    assert_eq!(
        *world.get::<Visibility>(hero).unwrap(),
        Visibility::Inherited
    );
}
#[test]
fn routes_stay_horizontal_and_within_vertical_limit() {
    for seed in 0..1000 {
        for direction in [
            LaunchDirection::Left,
            LaunchDirection::Right,
            LaunchDirection::Auto,
        ] {
            let start = Vec3::new(-87., -770., 10.);
            let end = destination(start, direction, seed);
            assert!((end.xy() / TILE - (end.xy() / TILE).round()).length() < 0.001);
            assert!((47. ..=52.).contains(&((end.x - start.x).abs() / TILE)));
            if direction == LaunchDirection::Left {
                assert!(end.x < start.x);
            }
            if direction == LaunchDirection::Right {
                assert!(end.x > start.x);
            }
            for i in 0..101 {
                assert!((path(start, end, i as f32 / 100.).y - start.y).abs() < 21. * TILE);
            }
        }
    }
}
#[test]
fn flight_curves_change_sides_and_finish_at_the_exact_target() {
    let a = Vec3::ZERO;
    let b = Vec3::new(50. * TILE, 0., 0.);
    assert_eq!(path(a, b, 0.), a);
    assert_eq!(path(a, b, 1.), b);
    let mut above = false;
    let mut below = false;
    for i in 1..100 {
        let u = i as f32 / 100.;
        let t = u.powf(1.6);
        let offset = path(a, b, u).y - 5. * TILE * 4. * t * (1. - t);
        above |= offset > 12.;
        below |= offset < -12.;
    }
    assert!(above && below, "Flight must weave around the main arc");
}
#[test]
fn crater_has_room_for_nine_and_irregular_boundary() {
    for seed in 0..100 {
        let mut count = 0;
        for x in -3..=3 {
            for y in -3..=3 {
                let inside = crater_cell(x, y, seed);
                if x.abs() <= 1 && y.abs() <= 1 {
                    assert!(inside);
                }
                if x.abs() > 2 || y.abs() > 2 {
                    assert!(!inside);
                }
                count += inside as usize;
            }
        }
        assert!((9..=21).contains(&count));
    }
}
