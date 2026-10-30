use super::*;
#[test]
fn attacks_require_line_of_sight_and_miner_has_recovery_weakness() {
    let block = vec![(Vec2::new(64., 0.), Entity::PLACEHOLDER)];
    assert!(!visible(Vec3::ZERO, Vec3::new(128., 0., 0.), &block));
    assert!(visible(Vec3::ZERO, Vec3::new(0., 128., 0.), &block));
    let mut miner = EncounterFoe {
        role: Role::Miner,
        biome: BiomeType::Underground,
        phase: 0.,
        cooldown: 1.,
        target: Vec3::ZERO,
    };
    assert_eq!(miner.received_damage(3), 2);
    assert_eq!(miner.received_damage(1), 1);
    miner.cooldown = 3.8;
    assert_eq!(miner.received_damage(1), 2);
    miner.role = Role::Witch;
    assert_eq!(miner.received_damage(1), 1);
}
#[test]
fn potion_hits_first_block_or_head_instead_of_its_timed_destination() {
    for hit_head in [false, true] {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<CloudMaterial>>()
            .insert_resource(crate::resources::spatial_hash::SpatialHash::new(256.))
            .add_message::<EntityDamaged>()
            .add_systems(Startup, setup)
            .add_systems(Update, projectiles);
        if hit_head {
            app.world_mut().spawn((
                Villager {
                    ai_timer: Timer::default(),
                    move_direction: 0.,
                    direction_change_timer: Timer::default(),
                },
                Transform::from_xyz(0., 100., 30.),
                LinearVelocity::ZERO,
            ));
        } else {
            let block = app
                .world_mut()
                .spawn((
                    Block {
                        health: 3,
                        max_health: 3,
                    },
                    BlockType::Stone,
                    Transform::from_xyz(0., 100., -10.),
                ))
                .id();
            app.world_mut()
                .resource_mut::<crate::resources::spatial_hash::SpatialHash>()
                .insert(block, 0., 100.);
        }
        let bottle = app
            .world_mut()
            .spawn((
                Potion {
                    from: Vec3::new(-200., 0., 0.),
                    to: Vec3::new(200., 0., 0.),
                    age: 0.,
                    biome: BiomeType::Forest,
                    arc: 100.,
                },
                Transform::IDENTITY,
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.85));
        app.update();
        assert!(app.world().get_entity(bottle).is_err());
        let t = app
            .world_mut()
            .query_filtered::<&Transform, With<Hazard>>()
            .single(app.world())
            .unwrap();
        assert!(
            (t.translation.x + 38.).abs() < 0.01,
            "Impact must be on the entry surface, not the end of the frame"
        );
    }
}
#[test]
fn a_missed_bottle_keeps_falling_and_cloud_colour_and_fade_follow_the_substance() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<CloudMaterial>>()
        .insert_resource(crate::resources::spatial_hash::SpatialHash::new(256.))
        .add_message::<EntityDamaged>()
        .add_systems(Startup, setup)
        .add_systems(Update, projectiles);
    let bottle = app
        .world_mut()
        .spawn((
            Potion {
                from: Vec3::ZERO,
                to: Vec3::X * 200.,
                age: 0.,
                biome: BiomeType::Iceland,
                arc: 100.,
            },
            Transform::IDENTITY,
        ))
        .id();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(1.));
    app.update();
    assert!(app.world().get_entity(bottle).is_ok());
    assert_eq!(
        app.world_mut().query::<&Hazard>().iter(app.world()).count(),
        0
    );
    assert!(app.world().get::<Transform>(bottle).unwrap().translation.y < 0.);
    assert_eq!(cloud_opacity(4.), 1.);
    assert!(cloud_opacity(7.) > 0.);
    assert_eq!(cloud_opacity(8.), 0.);
    let art = app.world().resource::<Art>();
    let mats = app.world().resource::<Assets<StandardMaterial>>();
    for b in [
        BiomeType::Forest,
        BiomeType::Iceland,
        BiomeType::Nether,
        BiomeType::End,
    ] {
        let substance = Substance::for_biome(b);
        let tint = substance.tint();
        assert_eq!(
            mats.get(&art.liquids[substance.index()])
                .unwrap()
                .base_color,
            Color::srgb(tint.x, tint.y, tint.z)
        );
    }
}
#[test]
fn structure_foundations_only_have_grass_on_exposed_soil() {
    for biome in [BiomeType::Plains, BiomeType::Forest] {
        let site = Site {
            origin: IVec2::new(32, 32),
            biome,
            seed: 921,
        };
        let cells = layout(site);
        let occupied = cells
            .iter()
            .filter(|(_, k)| {
                !matches!(
                    k,
                    BlockType::Mushroom
                        | BlockType::DeadBush
                        | BlockType::Vines
                        | BlockType::Cobweb
                )
            })
            .map(|(p, _)| *p)
            .collect();
        let mut surfaces = 0;
        let mut buried = 0;
        for (p, k) in cells {
            if soil_surface(site, p, k, &occupied) {
                surfaces += 1;
                assert!(!occupied.contains(&(p + IVec2::Y)));
            }
            if matches!(k, BlockType::Dirt | BlockType::Grass) && occupied.contains(&(p + IVec2::Y))
            {
                buried += 1;
                assert!(!soil_surface(site, p, k, &occupied));
            }
        }
        assert!(surfaces > 0 && buried > 0);
    }
}
#[test]
fn potion_hazard_damages_once_per_tick_then_expires() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<CloudMaterial>>()
        .insert_resource(crate::resources::spatial_hash::SpatialHash::new(256.))
        .add_message::<EntityDamaged>()
        .add_message::<crate::events::damage_events::EntityDied>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                projectiles,
                crate::systems::health_system::handle_entity_damage,
            )
                .chain(),
        );
    let target = app
        .world_mut()
        .spawn((
            Villager {
                ai_timer: Timer::default(),
                move_direction: 0.,
                direction_change_timer: Timer::default(),
            },
            Transform::default(),
            LinearVelocity::ZERO,
            Health::new(6),
        ))
        .id();
    let hazard = app
        .world_mut()
        .spawn((
            Hazard {
                age: 0.,
                tick: 0.,
                biome: BiomeType::Forest,
            },
            Transform::default(),
        ))
        .id();
    app.update();
    assert_eq!(app.world().get::<Health>(target).unwrap().current, 5);
    assert_eq!(
        app.world().get::<Buff>(target).unwrap().buff_type,
        BuffType::Slowness
    );
    app.update();
    assert_eq!(app.world().get::<Health>(target).unwrap().current, 5);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(0.81));
    app.update();
    assert_eq!(app.world().get::<Health>(target).unwrap().current, 4);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(CLOUD_LIFETIME));
    app.update();
    assert!(app.world().get_entity(hazard).is_err());
}
#[test]
fn landmarks_have_clear_exits_and_stay_in_chunk() {
    for b in [
        BiomeType::Plains,
        BiomeType::Forest,
        BiomeType::Desert,
        BiomeType::Mountains,
        BiomeType::Underground,
        BiomeType::Nether,
        BiomeType::End,
        BiomeType::Iceland,
    ] {
        let s = Site {
            origin: IVec2::new(-8, 7),
            biome: b,
            seed: 921,
        };
        for (p, _) in layout(s) {
            assert!(s.reserved(s.origin + p));
            assert!(!(p.x == 0 && p.y == 0));
        }
        let cells: std::collections::HashSet<_> = layout(s).iter().map(|(p, _)| *p).collect();
        assert_eq!(cells.len(), layout(s).len(), "No stacked duplicate terrain");
        assert!(
            !cells.iter().any(|p| p.y == 0
                && p.x >= 0
                && !(b == BiomeType::Desert && matches!(p.x, 6 | 8..=11 | 14))),
            "Walkable central exit"
        );
        assert!(!s.reserved(s.origin + IVec2::new(s.width() + 1, 0)));
    }
}
#[test]
fn grounded_sites_are_deterministic_partitionable_and_supported() {
    let mut found = 0;
    for cx in -20..20 {
        for cy in -12..12 {
            if let Some(s) = site(cx, cy, BiomeType::Plains, 921) {
                found += 1;
                assert_eq!(
                    site(cx, cy, BiomeType::Plains, 921).unwrap().origin,
                    s.origin
                );
                let cells = layout(s);
                let mut partitions = std::collections::HashSet::new();
                for (p, _) in cells {
                    let p = s.origin + p;
                    assert!(s.reserved(p));
                    assert!(partitions.insert(p));
                }
                let supported = (-s.width()..=s.width())
                    .filter(|dx| {
                        (0..=5).any(|d| {
                            !super::super::world_shapes::is_open(
                                s.origin.x + dx,
                                s.origin.y - 6 - d,
                                s.biome,
                                s.seed,
                            )
                        })
                    })
                    .count();
                assert!(supported as i32 >= (s.width() * 2 + 1) * 9 / 10);
            }
        }
    }
    assert!(found > 0, "No grounded sites generated");
}
#[test]
fn aqueduct_upper_arches_and_steps_connect_to_the_footing() {
    let site = Site {
        origin: IVec2::new(80, 80),
        biome: BiomeType::Desert,
        seed: 921,
    };
    let cells: std::collections::HashSet<_> = layout(site).into_iter().map(|(p, _)| p).collect();
    let mut seen: std::collections::HashSet<_> =
        cells.iter().filter(|p| p.y == -5).copied().collect();
    loop {
        let count = seen.len();
        for p in &cells {
            if [IVec2::X, -IVec2::X, IVec2::Y, -IVec2::Y]
                .iter()
                .any(|d| seen.contains(&(*p + *d)))
            {
                seen.insert(*p);
            }
        }
        if count == seen.len() {
            break;
        }
    }
    assert!(
        cells.iter().filter(|p| p.y >= 0).all(|p| seen.contains(p)),
        "Unsupported arch or staircase fragment"
    );
}
