use super::*;
#[cfg(test)]
mod continuous_contact_tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    fn fixture(fps: u32, speed: f32) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .add_plugins(
                PhysicsPlugins::default()
                    .with_length_unit(64.)
                    .build()
                    .disable::<PhysicsInterpolationPlugin>()
                    .add(PhysicsInterpolationPlugin::interpolate_all()),
            )
            .insert_resource(State::new(crate::states::GameState::Playing))
            .insert_resource(State::new(crate::states::InGame))
            .insert_resource(Gravity(Vec3::ZERO))
            .insert_resource(SubstepCount(1))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                1. / fps as f64,
            )))
            .init_resource::<DamageCooldown>()
            .insert_resource(SpatialHash::new(256.))
            .add_message::<BlockDamaged>()
            .add_message::<crate::events::block_events::HeadHitBlock>()
            .add_message::<EntityDamaged>();
        install_terrain_contacts(&mut app);
        let front = app
            .world_mut()
            .spawn((
                Block {
                    health: 100,
                    max_health: 100,
                },
                BlockType::Stone,
                Transform::IDENTITY,
                ChunkMember {
                    chunk_x: 0,
                    chunk_y: 0,
                },
            ))
            .id();
        app.world_mut().spawn((
            Block {
                health: 100,
                max_health: 100,
            },
            BlockType::Stone,
            Transform::from_xyz(64., 0., 0.),
            ChunkMember {
                chunk_x: 0,
                chunk_y: 0,
            },
        ));
        let hero = app
            .world_mut()
            .spawn((
                Hero {
                    move_direction: 1.,
                    direction_change_timer: Timer::default(),
                },
                RigidBody::Dynamic,
                Collider::cuboid(64., 64., 64.),
                Mass(1.),
                Transform::from_xyz(-200., 0., 30.),
                LinearVelocity(Vec3::X * speed),
                Restitution::new(1.),
            ))
            .id();
        app.finish();
        app.cleanup();
        app.update();
        (app, hero, front)
    }

    #[test]
    fn fixed_contacts_prevent_tunneling_with_interpolation_and_low_render_fps() {
        for fps in [10, 30, 60, 90, 144] {
            for speed in [1200., 30000.] {
                let (mut app, hero, front) = fixture(fps, speed);
                let mut hits = 0;
                for _ in 0..fps {
                    app.update();
                    let physical = app.world().get::<Position>(hero).unwrap().x;
                    let rendered = app.world().get::<Transform>(hero).unwrap().translation.x;
                    assert!(
                        physical <= -64.,
                        "{fps} FPS / {speed}: physics crossed wall: {physical}"
                    );
                    assert!(
                        rendered <= -64.,
                        "{fps} FPS / {speed}: visual crossed wall: {rendered}"
                    );
                    for damage in app
                        .world_mut()
                        .resource_mut::<Messages<BlockDamaged>>()
                        .drain()
                    {
                        assert_eq!(
                            damage.entity, front,
                            "Damaged a tile behind the blocking face"
                        );
                        hits += 1;
                    }
                }
                assert_eq!(hits, 1, "{fps} FPS / {speed}: incorrect damage count");
                assert!(app.world().get::<LinearVelocity>(hero).unwrap().x < 0.);
            }
        }
    }

    #[test]
    fn final_hit_option_passes_fragile_tile_but_stops_at_next_solid() {
        for enabled in [false, true] {
            let (mut app, head, front) = fixture(60, 1200.);
            app.insert_resource(ContactTuning {
                break_through: enabled,
            });
            app.world_mut().get_mut::<Block>(front).unwrap().health = 1;
            let mut furthest = -200_f32;
            let mut hit_front = 0;
            let mut other_hits = 0;
            for _ in 0..15 {
                app.update();
                furthest = furthest.max(app.world().get::<Position>(head).unwrap().x);
                for event in app
                    .world_mut()
                    .resource_mut::<Messages<BlockDamaged>>()
                    .drain()
                {
                    if event.entity == front {
                        hit_front += 1;
                    } else {
                        other_hits += 1;
                    }
                }
            }
            assert!(app.world().get::<LinearVelocity>(head).unwrap().x < 0.);
            assert_eq!(hit_front, 1);
            assert_eq!(other_hits, usize::from(enabled));
            if enabled {
                assert!(furthest > -64. && furthest <= -1.);
            } else {
                assert!(furthest <= -64.);
            }
        }
    }

    #[test]
    fn explicit_teleport_and_rocket_flight_do_not_sweep_through_intervening_terrain() {
        let (mut app, hero, _) = fixture(60, 0.);
        app.world_mut()
            .get_mut::<Transform>(hero)
            .unwrap()
            .translation
            .x = 800.;
        for _ in 0..3 {
            app.update();
        }
        assert!((app.world().get::<Position>(hero).unwrap().x - 800.).abs() < 0.01);
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<BlockDamaged>>()
                .drain()
                .count(),
            0
        );
        let (mut app, hero, _) = fixture(60, 1200.);
        app.world_mut()
            .entity_mut(hero)
            .insert(crate::components::enemy::Flying);
        for _ in 0..30 {
            app.update();
        }
        assert!(app.world().get::<Position>(hero).unwrap().x > 64.);
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<BlockDamaged>>()
                .drain()
                .count(),
            0
        );
    }

    /// Direct move that skips the swept step (both components, as gameplay code does).
    fn nudge(app: &mut App, hero: Entity, x: f32) {
        app.world_mut().get_mut::<Position>(hero).unwrap().x = x;
        app.world_mut()
            .get_mut::<Transform>(hero)
            .unwrap()
            .translation
            .x = x;
    }

    #[test]
    fn small_unswept_nudge_into_a_wall_stays_on_the_side_it_came_from() {
        let (mut app, hero, _) = fixture(60, 0.);
        nudge(&mut app, hero, -70.);
        for _ in 0..3 {
            app.update();
        }
        // Past the middle of the front block: nearest-face recovery alone pushes
        // the head out of the bottom of the wall (y = -65) instead of back out.
        nudge(&mut app, hero, 50.);
        for _ in 0..3 {
            app.update();
        }
        let p = app.world().get::<Position>(hero).unwrap().0;
        assert!(
            p.x <= -64. && p.y.abs() < 1.,
            "nudged head left through the far side or bottom: {p}"
        );
    }

    #[test]
    fn heavy_heads_thud_and_hit_harder_sticky_heads_grab_walls() {
        use crate::components::play_style::{ClingSpent, PlayStyle, WallCling};
        // Landing: drop onto the front block from above.
        for (style, max_rebound, damage) in [
            (PlayStyle::Bouncy, f32::INFINITY, 1),
            (PlayStyle::Heavy, 260., 2),
        ] {
            let (mut app, hero, front) = fixture(60, 0.);
            app.world_mut().entity_mut(hero).insert(style);
            nudge_to(&mut app, hero, Vec2::new(0., 200.));
            app.world_mut().get_mut::<LinearVelocity>(hero).unwrap().0 = Vec3::new(0., -800., 0.);
            let mut hits = Vec::new();
            let mut rebound = 0_f32;
            for _ in 0..30 {
                app.update();
                rebound = rebound.max(app.world().get::<LinearVelocity>(hero).unwrap().y);
                hits.extend(
                    app.world_mut()
                        .resource_mut::<Messages<BlockDamaged>>()
                        .drain()
                        .filter(|d| d.entity == front)
                        .map(|d| d.damage),
                );
            }
            assert_eq!(hits, vec![damage], "{style:?} landing damage");
            assert!(
                rebound > 0. && rebound <= max_rebound,
                "{style:?} rebound {rebound}"
            );
        }
        // Wall: a Sticky head stops on the wall instead of bouncing back.
        let (mut app, hero, _) = fixture(60, 1200.);
        app.world_mut().entity_mut(hero).insert(PlayStyle::Sticky);
        for _ in 0..15 {
            app.update();
        }
        let cling = app.world().get::<WallCling>(hero).copied();
        assert_eq!(
            cling.map(|c| c.side),
            Some(-1.),
            "clings to the wall on its right"
        );
        assert!(
            app.world().get::<ClingSpent>(hero).is_some(),
            "a grab uses up the wall grab until the next landing"
        );
        assert!(app.world().get::<LinearVelocity>(hero).unwrap().x.abs() < 1.);
    }

    fn nudge_to(app: &mut App, hero: Entity, p: Vec2) {
        app.world_mut().get_mut::<Position>(hero).unwrap().0 = p.extend(30.);
        app.world_mut()
            .get_mut::<Transform>(hero)
            .unwrap()
            .translation = p.extend(30.);
        for _ in 0..2 {
            app.update();
        }
    }

    #[test]
    fn terrain_added_after_startup_is_indexed_before_the_next_physics_step() {
        let (mut app, hero, _) = fixture(60, 0.);
        let new_block = app
            .world_mut()
            .spawn((
                Block {
                    health: 100,
                    max_health: 100,
                },
                BlockType::Stone,
                Transform::from_xyz(-130., 0., 0.),
                ChunkMember {
                    chunk_x: 0,
                    chunk_y: 0,
                },
            ))
            .id();
        app.world_mut().get_mut::<LinearVelocity>(hero).unwrap().x = 1200.;
        app.update();
        assert!(app.world().get::<Position>(hero).unwrap().x <= -194.);
        let hits: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<BlockDamaged>>()
            .drain()
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, new_block);
    }
}

#[cfg(test)]
mod snow_contact_tests {
    use super::*;
    #[test]
    fn hero_bounces_at_snow_top_before_touching_underlying_block() {
        for kind in [BlockType::Stone, BlockType::Ice] {
            let mut app = App::new();
            app.insert_resource(Time::<()>::default())
                .insert_resource(Time::<Virtual>::default())
                .init_resource::<DamageCooldown>()
                .init_resource::<TerrainStepStarts>()
                .insert_resource(SpatialHash::new(256.))
                .add_message::<BlockDamaged>()
                .add_message::<crate::events::block_events::HeadHitBlock>()
                .add_message::<EntityDamaged>()
                .add_systems(Update, check_all_entity_block_collisions);
            let block = app
                .world_mut()
                .spawn((
                    Block {
                        health: 3,
                        max_health: 3,
                    },
                    kind,
                    Transform::IDENTITY,
                    ChunkMember {
                        chunk_x: 0,
                        chunk_y: 0,
                    },
                    SnowLayer { stage: 4 },
                ))
                .id();
            app.world_mut()
                .resource_mut::<SpatialHash>()
                .insert(block, 0., 0.);
            let hero = app
                .world_mut()
                .spawn((
                    Hero {
                        move_direction: 1.,
                        direction_change_timer: Timer::default(),
                    },
                    Transform::from_xyz(0., 65., 10.),
                    LinearVelocity(Vec3::new(150., -100., 0.)),
                    Restitution::new(1.5),
                ))
                .id();
            app.update();
            assert_eq!(
                app.world().get::<Transform>(hero).unwrap().translation.y,
                73.
            );
            assert_eq!(app.world().get::<LinearVelocity>(hero).unwrap().y, 0.);
            assert!(app.world().get::<SnowSlide>(hero).unwrap().rebound >= MIN_BOUNCE_VELOCITY);
            assert_eq!(
                app.world()
                    .get::<SurfaceGrip>(hero)
                    .unwrap()
                    .counter_steering,
                0.55
            );
            assert!((app.world().get::<LinearVelocity>(hero).unwrap().x - 177.).abs() < 0.01);
        }
    }
    #[test]
    fn slow_landings_share_snow_carry_on_blocks_and_accumulated_layers() {
        for (kind, layer) in [
            (BlockType::Snow, false),
            (BlockType::Stone, true),
            (BlockType::Ice, true),
        ] {
            let mut app = App::new();
            app.insert_resource(Time::<()>::default())
                .insert_resource(Time::<Virtual>::default())
                .init_resource::<DamageCooldown>()
                .init_resource::<TerrainStepStarts>()
                .insert_resource(SpatialHash::new(256.))
                .add_message::<BlockDamaged>()
                .add_message::<crate::events::block_events::HeadHitBlock>()
                .add_message::<EntityDamaged>()
                .add_systems(Update, check_all_entity_block_collisions);
            let block = app
                .world_mut()
                .spawn((
                    Block {
                        health: 3,
                        max_health: 3,
                    },
                    kind,
                    Transform::IDENTITY,
                    ChunkMember {
                        chunk_x: 0,
                        chunk_y: 0,
                    },
                ))
                .id();
            if layer {
                app.world_mut()
                    .entity_mut(block)
                    .insert(SnowLayer { stage: 4 });
            }
            app.world_mut()
                .resource_mut::<SpatialHash>()
                .insert(block, 0., 0.);
            let hero = app
                .world_mut()
                .spawn((
                    Hero {
                        move_direction: 1.,
                        direction_change_timer: Timer::default(),
                    },
                    Transform::from_xyz(0., 63., 10.),
                    LinearVelocity(Vec3::new(20., -50., 0.)),
                    Restitution::new(1.5),
                ))
                .id();
            app.update();
            let velocity = app.world().get::<LinearVelocity>(hero).unwrap();
            assert!((velocity.x - 64.).abs() < 0.01);
            assert_eq!(velocity.y, 0.);
            assert_eq!(
                app.world().get::<SnowSlide>(hero).unwrap().rebound,
                MIN_BOUNCE_VELOCITY
            );
            assert!(app.world().get::<SurfaceGrip>(hero).is_some());
        }
    }
    #[test]
    fn cactus_contact_is_damage_limited_and_ice_preserves_lateral_glide() {
        for kind in [BlockType::Cactus, BlockType::Ice, BlockType::Snow] {
            let mut app = App::new();
            app.insert_resource(Time::<()>::default())
                .insert_resource(Time::<Virtual>::default())
                .init_resource::<DamageCooldown>()
                .init_resource::<TerrainStepStarts>()
                .insert_resource(SpatialHash::new(256.))
                .add_message::<BlockDamaged>()
                .add_message::<crate::events::block_events::HeadHitBlock>()
                .add_message::<EntityDamaged>()
                .add_systems(Update, check_all_entity_block_collisions);
            let block = app
                .world_mut()
                .spawn((
                    Block {
                        health: 3,
                        max_health: 3,
                    },
                    kind,
                    Transform::IDENTITY,
                    ChunkMember {
                        chunk_x: 0,
                        chunk_y: 0,
                    },
                ))
                .id();
            app.world_mut()
                .resource_mut::<SpatialHash>()
                .insert(block, 0., 0.);
            let actor = app
                .world_mut()
                .spawn((
                    Hero {
                        move_direction: 1.,
                        direction_change_timer: Timer::default(),
                    },
                    Transform::from_xyz(0., 60., 10.),
                    LinearVelocity(Vec3::new(150., -100., 0.)),
                    Restitution::new(1.5),
                ))
                .id();
            app.update();
            let hits = app
                .world_mut()
                .resource_mut::<Messages<EntityDamaged>>()
                .drain()
                .count();
            assert_eq!(hits, if kind == BlockType::Cactus { 1 } else { 0 });
            if kind == BlockType::Ice {
                let v = app.world().get::<LinearVelocity>(actor).unwrap();
                assert_eq!(v.x, 150.);
                assert!(v.y <= 85.);
            }
            if kind == BlockType::Snow {
                let v = app.world().get::<LinearVelocity>(actor).unwrap();
                assert_eq!(v.y, 0.);
                assert!(
                    app.world().get::<SnowSlide>(actor).unwrap().rebound >= MIN_BOUNCE_VELOCITY
                );
                assert!((v.x - 177.).abs() < 0.01);
            }
            app.world_mut()
                .get_mut::<Transform>(actor)
                .unwrap()
                .translation
                .y = 60.;
            app.update();
            assert_eq!(
                app.world_mut()
                    .resource_mut::<Messages<EntityDamaged>>()
                    .drain()
                    .count(),
                0
            );
        }
    }
    #[test]
    fn surface_grip_expires_on_the_gameplay_clock() {
        let mut app = App::new();
        app.insert_resource(Time::<Virtual>::default())
            .add_systems(Update, expire_surface_grip);
        let e = app.world_mut().spawn(SurfaceGrip::snow(0.)).id();
        app.update();
        assert!(app.world().get::<SurfaceGrip>(e).is_some());
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .advance_by(std::time::Duration::from_secs_f32(0.7));
        app.update();
        assert!(app.world().get::<SurfaceGrip>(e).is_none());
    }
    #[test]
    fn snow_surface_matches_visual_top_without_expanding_bottom_or_sides() {
        let origin = Vec3::new(100., -300., 0.);
        for stage in 0..=4 {
            let snow = SnowLayer { stage };
            let (center, half) = block_contact_bounds(origin, Some(&snow));
            assert_eq!(
                (center + half).y,
                origin.y + BLOCK_SIZE / 2. + snow.thickness()
            );
            assert_eq!((center - half).y, origin.y - BLOCK_SIZE / 2.);
            assert_eq!(half.x, BLOCK_SIZE / 2.);
        }
        assert_eq!(
            block_contact_bounds(origin, None),
            (origin, Vec3::splat(BLOCK_SIZE / 2.))
        );
    }
}

#[cfg(test)]
mod grounded_snow_tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;
    fn fixture(fps: u32) -> (App, Entity, Vec<Entity>) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .add_plugins(PhysicsPlugins::default().with_length_unit(64.))
            .insert_resource(State::new(crate::states::GameState::Playing))
            .insert_resource(State::new(crate::states::InGame))
            .insert_resource(Gravity(Vec3::NEG_Y * 750.))
            .insert_resource(SubstepCount(1))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                1. / fps as f64,
            )))
            .init_resource::<DamageCooldown>()
            .insert_resource(SpatialHash::new(256.))
            .add_message::<BlockDamaged>()
            .add_message::<crate::events::block_events::HeadHitBlock>()
            .add_message::<EntityDamaged>();
        install_terrain_contacts(&mut app);
        let blocks = (-4..=4)
            .map(|x| {
                app.world_mut()
                    .spawn((
                        Block {
                            health: 100,
                            max_health: 100,
                        },
                        BlockType::Snow,
                        Transform::from_xyz(x as f32 * 64., 0., -10.),
                        ChunkMember {
                            chunk_x: 0,
                            chunk_y: 0,
                        },
                    ))
                    .id()
            })
            .collect();
        let head = app
            .world_mut()
            .spawn((
                Hero {
                    move_direction: 1.,
                    direction_change_timer: Timer::default(),
                },
                crate::components::bundles::avian_physics::character(),
                Transform::from_xyz(0., 100., 30.),
            ))
            .id();
        app.world_mut()
            .entity_mut(head)
            .insert(LinearVelocity(Vec3::new(100., -100., 0.)));
        app.finish();
        app.cleanup();
        app.update();
        (app, head, blocks)
    }
    #[test]
    fn actual_fixed_physics_slides_then_rebounds_without_repeated_damage() {
        for fps in [30, 60, 144] {
            let (mut app, head, _) = fixture(fps);
            let mut landing = None;
            let mut rebound = false;
            let mut hits = 0;
            for _ in 0..fps * 2 {
                app.update();
                let p = app.world().get::<Position>(head).unwrap().0;
                let v = app.world().get::<LinearVelocity>(head).unwrap().0;
                let now = app.world().resource::<Time<Virtual>>().elapsed_secs();
                hits += app
                    .world_mut()
                    .resource_mut::<Messages<BlockDamaged>>()
                    .drain()
                    .count();
                if app.world().get::<SnowSlide>(head).is_some() {
                    let first = landing.get_or_insert((now, p.x));
                    assert!((p.y - 65.).abs() < 0.01);
                    assert_eq!(v.y, 0.);
                    if now > first.0 + 0.05 {
                        assert!(p.x > first.1 + 3.);
                    }
                } else if let Some((time, x)) = landing {
                    // Render-frame observation may lag either fixed-step edge
                    // by one frame, but the old 140–200 ms hold must not return.
                    assert!(now - time >= 0.015 && now - time <= 0.12);
                    assert!(v.y > 350. && v.y <= 550.);
                    assert!(p.x > x + 2.);
                    rebound = true;
                    break;
                }
            }
            assert!(rebound, "No supported slide/rebound at {fps} FPS");
            assert!(
                hits > 0 && hits <= 2,
                "Sustained contact repeatedly damaged the support: {hits}"
            );
        }
    }
    #[test]
    fn removed_support_and_teleports_never_cause_an_air_bounce() {
        for teleport in [false, true] {
            let (mut app, head, blocks) = fixture(60);
            for _ in 0..60 {
                app.update();
                if app.world().get::<SnowSlide>(head).is_some() {
                    break;
                }
            }
            assert!(app.world().get::<SnowSlide>(head).is_some());
            if teleport {
                app.world_mut().get_mut::<Position>(head).unwrap().x = 1000.;
            } else {
                for block in blocks {
                    app.world_mut().despawn(block);
                }
            }
            for _ in 0..3 {
                app.update();
            }
            assert!(app.world().get::<SnowSlide>(head).is_none());
            assert!(app.world().get::<LinearVelocity>(head).unwrap().y <= 0.);
        }
    }
    #[test]
    fn slime_contact_holds_releases_and_cannot_bounce_after_support_loss() {
        use crate::systems::slime_physics::{SlimeCoat, SlimeContact};
        for remove in [false, true] {
            let (mut app, head, blocks) = fixture(60);
            app.world_mut()
                .entity_mut(head)
                .insert(SlimeCoat { until: 10. });
            for _ in 0..60 {
                app.update();
                if app.world().get::<SlimeContact>(head).is_some() {
                    break;
                }
            }
            assert!(app.world().get::<SlimeContact>(head).is_some());
            assert_eq!(app.world().get::<LinearVelocity>(head).unwrap().y, 0.);
            if remove {
                for block in blocks {
                    app.world_mut().despawn(block);
                }
            }
            for _ in 0..18 {
                app.update();
            }
            assert!(app.world().get::<SlimeContact>(head).is_none());
            let speed = app.world().get::<LinearVelocity>(head).unwrap().y;
            if remove {
                assert!(speed < 0.);
            } else {
                assert!(speed > 300.);
            }
        }
    }
    #[test]
    fn flight_and_death_clear_stored_rebound() {
        let mut app = App::new();
        app.add_systems(Update, cancel_snow_slides);
        let flight = app
            .world_mut()
            .spawn((SnowSlide::new(0., 400.), crate::components::enemy::Flying))
            .id();
        let death = app
            .world_mut()
            .spawn((
                SnowSlide::new(0., 400.),
                crate::components::damage_visual::DeathAnimation::new(),
            ))
            .id();
        app.update();
        assert!(app.world().get::<SnowSlide>(flight).is_none());
        assert!(app.world().get::<SnowSlide>(death).is_none());
    }
}
