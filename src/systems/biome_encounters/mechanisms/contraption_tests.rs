use super::bubbles::*;
use super::contraptions::*;
use super::music::*;
use super::pads::*;
use super::traps::*;
use super::*;

const BIOMES: [BiomeType; 8] = [
    BiomeType::Plains,
    BiomeType::Forest,
    BiomeType::Desert,
    BiomeType::Mountains,
    BiomeType::Underground,
    BiomeType::Nether,
    BiomeType::End,
    BiomeType::Iceland,
];
fn sites() -> impl Iterator<Item = Site> {
    BIOMES.into_iter().flat_map(|biome| {
        (0..40).map(move |seed| Site {
            origin: IVec2::new(seed * 7 - 90, seed * 3 - 20),
            biome,
            seed: seed as u32 * 31 + 5,
        })
    })
}

#[test]
fn slime_block_launches_landing_and_resting_heads_only_from_the_top() {
    assert_eq!(slime_launch(Some(Side::Top), -400.), Some(SLIME_LAUNCH));
    assert_eq!(slime_launch(Some(Side::Top), 0.), Some(700.));
    // Already flying off: no second kick.
    assert_eq!(slime_launch(Some(Side::Top), SLIME_LAUNCH), None);
    assert_eq!(slime_launch(Some(Side::Left), -400.), None);
    assert_eq!(slime_launch(None, -400.), None);
    // Contact classification: a head resting on a block touches its top.
    let block = (Vec2::ZERO, Vec2::splat(32.));
    let half = Vec2::splat(32.);
    assert_eq!(
        touching(Vec2::new(10., 65.), half, block.0, block.1, 6.),
        Some(Side::Top)
    );
    assert_eq!(
        touching(Vec2::new(-66., 5.), half, block.0, block.1, 6.),
        Some(Side::Left)
    );
    assert_eq!(
        touching(Vec2::new(0., 80.), half, block.0, block.1, 6.),
        None
    );
}

#[test]
fn honey_sides_stick_and_slide_slowly_and_its_top_kills_bounces() {
    let dt = 1. / 64.;
    let mut v = Vec2::new(300., -500.);
    for _ in 0..64 {
        v = honey_velocity(Side::Left, v, dt);
        assert_eq!(v.y, HONEY_SLIDE);
    }
    assert!(
        v.x.abs() < 300. * 0.001,
        "horizontal speed bleeds away: {v}"
    );
    // A head launched upward past the honey is not caught.
    assert_eq!(honey_velocity(Side::Right, Vec2::new(0., 600.), dt).y, 600.);
    let top = honey_velocity(Side::Top, Vec2::new(200., 300.), dt);
    assert!(top.y < 20., "almost no bounce: {top}");
    assert!(top.x < 200.);
    assert_eq!(honey_velocity(Side::Top, Vec2::new(0., -200.), dt).y, -200.);
}

#[test]
fn dispenser_fires_along_its_facing_with_a_cooldown() {
    assert!(shot_velocity(-1., false).x < 0. && shot_velocity(-1., false).y == 0.);
    assert!(shot_velocity(1., true).x > 0. && shot_velocity(1., true).y == 0.);
    // Head standing on the wire trips it; one jumping over it does not.
    let wire = (0., 128., 12.);
    assert!(wire_crossed(Vec2::new(64., 32.), Vec2::splat(32.), wire));
    assert!(!wire_crossed(Vec2::new(64., 100.), Vec2::splat(32.), wire));
    assert!(!wire_crossed(Vec2::new(-40., 32.), Vec2::splat(32.), wire));
    // Continuously tripped: fires at t=0, then not again before 2.5 s.
    let dt = 1. / 64.;
    let mut cooldown = 0.;
    let mut shots = Vec::new();
    for i in 0..400 {
        let (c, fire) = dispenser_step(cooldown, true, dt);
        cooldown = c;
        if fire {
            shots.push(i as f32 * dt);
        }
    }
    assert_eq!(shots.len(), 3, "{shots:?}");
    for pair in shots.windows(2) {
        let gap = pair[1] - pair[0];
        assert!((DISPENSER_COOLDOWN..DISPENSER_COOLDOWN + 0.05).contains(&gap));
    }
    // Untripped dispensers never fire.
    assert!(!dispenser_step(0., false, dt).1);
}

#[test]
fn bubble_columns_push_up_over_soul_sand_and_pull_down_over_magma() {
    let dt = 1. / 64.;
    let (mut up, mut down) = (-400., 400.);
    for _ in 0..64 {
        up = column_velocity(up, true, dt);
        down = column_velocity(down, false, dt);
    }
    assert!((up - COLUMN_UP).abs() < 1., "{up}");
    assert!((down - COLUMN_DOWN).abs() < 1., "{down}");
    // Inside the shaft only: column x=0, water from y=0 to y=256.
    let half = Vec2::splat(32.);
    assert!(in_column(Vec2::new(10., 100.), half, 0., 0., 256.));
    assert!(!in_column(Vec2::new(40., 100.), half, 0., 0., 256.));
    assert!(!in_column(Vec2::new(0., 300.), half, 0., 0., 256.));
}

#[test]
fn note_blocks_climb_in_semitones_and_pick_instruments_from_below() {
    assert_eq!(note_speed(12), 1.);
    assert!((note_speed(0) - 0.5).abs() < 1e-6);
    assert!((note_speed(24) - 2.).abs() < 1e-6);
    let ratio = note_speed(13) / note_speed(12);
    assert!((ratio - 2f32.powf(1. / 12.)).abs() < 1e-6);
    let mut n = 0;
    let mut seen = Vec::new();
    for _ in 0..25 {
        seen.push(n);
        n = next_note(n);
    }
    assert_eq!(n, 0, "cycles back after 24");
    assert_eq!(seen, (0..=24).collect::<Vec<u8>>());
    assert_eq!(instrument(BlockType::Stone), Instrument::BassDrum);
    assert_eq!(instrument(BlockType::Obsidian), Instrument::BassDrum);
    assert_eq!(instrument(BlockType::Sand), Instrument::Snare);
    assert_eq!(instrument(BlockType::Wood), Instrument::Bass);
    assert_eq!(instrument(BlockType::Ice), Instrument::Chime);
    assert_eq!(instrument(BlockType::Dirt), Instrument::Harp);
    assert_eq!(instrument(BlockType::Grass), Instrument::Harp);
}

#[test]
fn bells_stun_enemies_within_six_blocks_for_two_seconds() {
    let bell = Vec2::new(100., 50.);
    assert!(bell_stuns(bell, bell + Vec2::new(383., 0.)));
    assert!(bell_stuns(bell, bell + Vec2::new(0., -200.)));
    assert!(!bell_stuns(bell, bell + Vec2::new(300., 300.)));
    assert!(!bell_stuns(bell, bell + Vec2::new(-390., 0.)));
    // The stun pins velocity and wears off after STUN_SECONDS.
    let mut app = App::new();
    app.init_resource::<Time<Fixed>>()
        .add_systems(Update, hold_stunned);
    let foe = app
        .world_mut()
        .spawn((
            Stunned {
                remaining: STUN_SECONDS,
            },
            LinearVelocity(Vec3::new(250., -10., 0.)),
        ))
        .id();
    let step = |app: &mut App| {
        app.world_mut()
            .resource_mut::<Time<Fixed>>()
            .advance_by(std::time::Duration::from_millis(100));
        app.world_mut().run_schedule(Update);
    };
    for _ in 0..19 {
        step(&mut app);
        assert!(app.world().get::<Stunned>(foe).is_some());
        assert_eq!(app.world().get::<LinearVelocity>(foe).unwrap().x, 0.);
        app.world_mut().get_mut::<LinearVelocity>(foe).unwrap().x = 250.;
    }
    step(&mut app);
    step(&mut app);
    assert!(app.world().get::<Stunned>(foe).is_none());
    assert_eq!(stun_left(0.5, 0.25), Some(0.25));
    assert_eq!(stun_left(0.5, 0.5), None);
}

#[test]
fn tripwire_lights_the_tnt_and_it_explodes_after_its_fuse() {
    let dt = 1. / 64.;
    assert_eq!(tnt_step(None, false, dt), (None, false));
    let (mut fuse, explode) = tnt_step(None, true, dt);
    assert_eq!(fuse, Some(0.));
    assert!(!explode);
    let mut burned = 0.;
    loop {
        // Leaving the wire does not put the fuse out.
        let (f, boom) = tnt_step(fuse, false, dt);
        fuse = f;
        burned += dt;
        if boom {
            break;
        }
        assert!(burned < TNT_TRAP_FUSE + dt);
    }
    assert!((TNT_TRAP_FUSE..TNT_TRAP_FUSE + dt * 1.5).contains(&burned));
}

#[test]
fn contraptions_fit_landmarks_without_overlap_and_every_kind_is_placeable() {
    let mut kinds = std::collections::HashMap::<Gadget, usize>::new();
    let mut landmarks = 0;
    let mut planned = 0;
    for site in sites() {
        assert_eq!(plan(site), plan(site), "deterministic");
        if !plan(site).is_empty() {
            planned += 1;
        }
        landmarks += 1;
        let cells: std::collections::HashMap<_, _> = layout(site).into_iter().collect();
        let (gate, piston) = positions(site);
        let mut taken = std::collections::HashSet::new();
        if let Some(g) = gate {
            taken.extend([g + IVec2::Y, g + IVec2::Y * 2]);
        }
        if let Some(p) = piston {
            taken.extend((1..=3).map(|dy| p + IVec2::Y * dy));
        }
        // Every kind on its own must fit (feasibility), and the full plan
        // must not overlap the landmark, its gate/piston or other gadgets.
        for p in plan_kinds(site, 7, 7) {
            *kinds.entry(p.gadget).or_default() += 1;
            for c in footprint(&p) {
                assert!(!cells.contains_key(&c), "{p:?} overlaps {c}");
                assert!(taken.insert(c), "{p:?} overlaps another machine at {c}");
                assert!(c.x.abs() <= site.width() && (-5..=7).contains(&c.y));
            }
            let support = if p.gadget == Gadget::Bell {
                p.cell + IVec2::Y
            } else {
                p.cell - IVec2::Y
            };
            assert!(cells.contains_key(&support), "{p:?} unsupported");
            match p.gadget {
                Gadget::Bubbles => assert!((3..=5).contains(&p.len)),
                Gadget::Notes => assert!((3..=6).contains(&p.len)),
                Gadget::Tripwire => {
                    assert!(cells.contains_key(&(p.cell - IVec2::X)));
                    assert!(cells.contains_key(&(p.cell + IVec2::X * p.len)));
                }
                _ => {}
            }
        }
    }
    for kind in GADGETS {
        assert!(
            kinds.get(&kind).copied().unwrap_or(0) > 0,
            "{kind:?} never fits"
        );
    }
    assert!(planned * 3 > landmarks, "{planned}/{landmarks} landmarks");
}
