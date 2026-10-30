use super::*;
#[test]
fn shared_heat_ignites_and_propagation_cannot_cross_four_cells() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default())
        .init_resource::<crate::components::weather::ActiveWeather>()
        .init_resource::<super::super::fluids::FluidWorld>()
        .insert_resource(DragonAssets {
            stage: Entity::PLACEHOLDER,
            cube: default(),
            quad: default(),
            wing: default(),
            skin: default(),
            membrane: default(),
            eyes: default(),
            scales: default(),
            belly: default(),
            bone: default(),
            breath: default(),
        })
        .add_message::<TerrainHeat>()
        .add_message::<crate::events::block_events::BlockDamaged>()
        .add_message::<crate::events::damage_events::EntityDamaged>()
        .add_systems(Update, (apply_heat, ground_fire).chain());
    let line: Vec<_> = (0..10)
        .map(|i| {
            app.world_mut()
                .spawn((
                    Block {
                        health: 100,
                        max_health: 100,
                    },
                    BlockType::Wood,
                    Transform::from_xyz(i as f32 * 64., 0., 0.),
                ))
                .id()
        })
        .collect();
    app.world_mut().write_message(TerrainHeat {
        position: Vec2::ZERO,
        radius: 1.,
    });
    for _ in 0..12 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.01));
        app.update();
        for f in app.world_mut().query::<&GroundFire>().iter(app.world()) {
            assert!(f.hops <= 4);
            assert!(!line[5..].contains(&f.owner));
        }
    }
    assert_eq!(
        app.world_mut()
            .query::<&GroundFire>()
            .iter(app.world())
            .count(),
        0
    );
}
#[test]
fn wet_or_rained_on_material_does_not_ignite_from_heat() {
    let mut app = App::new();
    app.init_resource::<crate::components::weather::ActiveWeather>()
        .init_resource::<super::super::fluids::FluidWorld>()
        .insert_resource(DragonAssets {
            stage: Entity::PLACEHOLDER,
            cube: default(),
            quad: default(),
            wing: default(),
            skin: default(),
            membrane: default(),
            eyes: default(),
            scales: default(),
            belly: default(),
            bone: default(),
            breath: default(),
        })
        .add_message::<TerrainHeat>()
        .add_message::<crate::events::block_events::BlockDamaged>()
        .add_systems(Update, apply_heat);
    app.world_mut().spawn((
        Block {
            health: 10,
            max_health: 10,
        },
        BlockType::Wood,
        Transform::IDENTITY,
    ));
    app.world_mut()
        .resource_mut::<super::super::fluids::FluidWorld>()
        .deposit((0, 0), super::super::fluids::Liquid::Water, 2);
    app.world_mut().write_message(TerrainHeat {
        position: Vec2::ZERO,
        radius: 1.,
    });
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&GroundFire>()
            .iter(app.world())
            .count(),
        0
    );
    {
        let mut w = app
            .world_mut()
            .resource_mut::<crate::components::weather::ActiveWeather>();
        w.weather_type = crate::components::weather::WeatherType::Rain;
        w.intensity = 1.;
        w.duration_remaining = 20.;
    }
    app.world_mut().write_message(TerrainHeat {
        position: Vec2::ZERO,
        radius: 1.,
    });
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&GroundFire>()
            .iter(app.world())
            .count(),
        0
    );
}
#[test]
fn breath_reaches_three_layers_but_surface_exposure_stays_geometric() {
    let mut world = World::new();
    let mut wall = Vec::new();
    for x in 0..3 {
        for y in -1..=1 {
            wall.push((
                world.spawn_empty().id(),
                Vec2::new(x as f32 * 64., y as f32 * 64.),
            ));
        }
    }
    let touched = exposed_breath_blocks(Vec2::new(-220., 32.), Vec2::X, &wall);
    assert_eq!(touched.len(), 6);
    assert!(
        touched
            .iter()
            .any(|e| wall.iter().any(|(id, p)| id == e && p.x == 128.))
    );
    let occupied = wall.iter().map(|(_, p)| p.as_ivec2()).collect();
    assert_eq!(
        surface_exposure(Vec3::ZERO, &occupied),
        Vec4::new(1., 0., 0., 0.)
    );
    assert_eq!(
        surface_exposure(Vec3::new(64., 0., 0.), &occupied),
        Vec4::ZERO
    );
}
#[test]
fn fourth_layer_stops_breath_in_both_directions() {
    let mut world = World::new();
    let blocks: Vec<_> = (0..5)
        .map(|x| (world.spawn_empty().id(), Vec2::new(x as f32 * 64., 0.)))
        .collect();
    assert!((breath_reach(Vec2::new(-50., 0.), Vec2::X, &blocks) - 210.).abs() < 0.01);
    assert!((breath_reach(Vec2::new(306., 0.), Vec2::NEG_X, &blocks) - 210.).abs() < 0.01);
    assert_eq!(
        exposed_breath_blocks(Vec2::new(-50., 0.), Vec2::X, &blocks).len(),
        4
    );
}
#[test]
fn impact_is_visual_only_and_dissolves_without_scaling_or_leaking_materials() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default())
        .init_resource::<BreathSurfaces>()
        .add_message::<TerrainHeat>()
        .init_resource::<Assets<SurfaceFireMaterial>>()
        .insert_resource(DragonAssets {
            stage: Entity::PLACEHOLDER,
            cube: default(),
            quad: default(),
            wing: default(),
            skin: default(),
            membrane: default(),
            eyes: default(),
            scales: default(),
            belly: default(),
            bone: default(),
            breath: default(),
        })
        .add_systems(Update, surface_fire);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(1. / 60.));
    let owner = app
        .world_mut()
        .spawn((
            Block {
                health: 4,
                max_health: 4,
            },
            Transform::IDENTITY,
        ))
        .id();
    for _ in 0..3 {
        app.world_mut().resource_mut::<BreathSurfaces>().0.insert(
            owner,
            SurfaceContact {
                position: Vec3::ZERO,
                direction: Vec2::X,
                strength: 1.,
            },
        );
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(app.world().get::<Block>(owner).unwrap().health, 4);
        assert_eq!(
            app.world_mut()
                .query::<&GroundFire>()
                .iter(app.world())
                .count(),
            0
        );
        let (entity, original, opacity) = app
            .world_mut()
            .query::<(Entity, &Transform, &SurfaceFlame)>()
            .iter(app.world())
            .map(|(e, t, f)| (e, *t, f.opacity))
            .next()
            .unwrap();
        assert!(opacity > 0.9);
        app.world_mut().resource_mut::<BreathSurfaces>().0.clear();
        app.update();
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert_eq!(transform.scale, original.scale);
        assert_eq!(transform.translation, original.translation);
        assert!(app.world().get::<SurfaceFlame>(entity).unwrap().opacity < opacity);
        for _ in 0..30 {
            app.update();
        }
        assert!(app.world().get_entity(entity).is_err());
        assert_eq!(
            app.world().resource::<Assets<SurfaceFireMaterial>>().len(),
            0
        );
    }
    // Destroyed owners also dissolve, even if this frame still carries a contact.
    app.world_mut().resource_mut::<BreathSurfaces>().0.insert(
        owner,
        SurfaceContact {
            position: Vec3::ZERO,
            direction: Vec2::X,
            strength: 1.,
        },
    );
    app.update();
    app.world_mut().despawn(owner);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        app.world_mut()
            .query::<&SurfaceFlame>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(
        app.world().resource::<Assets<SurfaceFireMaterial>>().len(),
        0
    );
}
#[test]
fn ground_flames_damage_destroy_and_spread_only_to_flammable_blocks() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default())
        .init_resource::<crate::components::weather::ActiveWeather>()
        .init_resource::<super::super::fluids::FluidWorld>()
        .insert_resource(DragonAssets {
            stage: Entity::PLACEHOLDER,
            cube: default(),
            quad: default(),
            wing: default(),
            skin: default(),
            membrane: default(),
            eyes: default(),
            scales: default(),
            belly: default(),
            bone: default(),
            breath: default(),
        })
        .add_message::<crate::events::block_events::BlockDamaged>()
        .add_message::<crate::events::block_events::BlockDestroyed>()
        .add_message::<crate::events::damage_events::EntityDamaged>()
        .add_message::<TerrainHeat>()
        .add_systems(
            Update,
            (
                ground_fire,
                crate::plugins::gameplay_plugin::handle_block_damage,
            )
                .chain(),
        );
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(1.1));
    let wood = app
        .world_mut()
        .spawn((
            Block {
                health: 2,
                max_health: 2,
            },
            BlockType::Wood,
            Transform::IDENTITY,
        ))
        .id();
    let leaves = app
        .world_mut()
        .spawn((
            Block {
                health: 3,
                max_health: 3,
            },
            BlockType::Leaves,
            Transform::from_xyz(64., 0., 0.),
        ))
        .id();
    let stone = app
        .world_mut()
        .spawn((
            Block {
                health: 4,
                max_health: 4,
            },
            BlockType::Stone,
            Transform::from_xyz(-64., 0., 0.),
        ))
        .id();
    app.world_mut().spawn((
        GroundFire {
            owner: wood,
            remaining: 5.,
            tick: 0.,
            origin: Vec2::ZERO,
            hops: 0,
        },
        Transform::from_xyz(0., 12., 48.),
    ));
    app.update();
    assert_eq!(app.world().get::<Block>(wood).unwrap().health, 1);
    assert!(
        app.world_mut()
            .query::<&GroundFire>()
            .iter(app.world())
            .any(|fire| fire.owner == leaves)
    );
    assert!(
        !app.world_mut()
            .query::<&GroundFire>()
            .iter(app.world())
            .any(|fire| fire.owner == stone)
    );
    app.update();
    assert!(app.world().get_entity(wood).is_err());
    assert_eq!(app.world().get::<Block>(leaves).unwrap().health, 2);
    assert_eq!(app.world().get::<Block>(stone).unwrap().health, 4);
}
#[test]
fn articulated_wing_hit_test_rejects_empty_rectangle_corners() {
    let part = DragonHitPart {
        owner: Entity::PLACEHOLDER,
        half: Vec2::splat(-1.),
    };
    let transform = GlobalTransform::IDENTITY;
    assert!(
        part.hit(&transform, Vec2::new(-20., 40.), Vec2::new(100., 40.))
            .is_some()
    );
    assert!(
        part.hit(&transform, Vec2::new(100., 125.), Vec2::new(120., 125.))
            .is_none()
    );
    let rotated = GlobalTransform::from(
        Transform::from_xyz(400., -100., 90.)
            .with_rotation(Quat::from_rotation_z(0.4))
            .with_scale(Vec3::new(-1., 1., 1.)),
    );
    let from = rotated.transform_point(Vec3::new(-20., 40., 0.)).xy();
    let to = rotated.transform_point(Vec3::new(100., 40., 0.)).xy();
    assert!(part.hit(&rotated, from, to).is_some());
}
#[test]
fn reversal_has_intermediate_poses_and_same_duration_at_all_frame_rates() {
    for fps in [30, 60, 120, 144] {
        let mut yaw = 0.;
        let mut velocity = 0.;
        let dt = 1. / fps as f32;
        let first = turn_towards(yaw, velocity, std::f32::consts::PI, dt);
        assert!(first.0 < 0.05, "Turn must accelerate gently from rest");
        for _ in 0..fps {
            (yaw, velocity) = turn_towards(yaw, velocity, std::f32::consts::PI, dt);
        }
        let expected = std::f32::consts::PI * (1. - 6. * (-5_f32).exp());
        assert!((yaw - expected).abs() < 0.0001);
        assert!(
            velocity > 0. && velocity < 0.6,
            "Turn must decelerate toward its heading"
        );
        for _ in 0..fps {
            (yaw, velocity) = turn_towards(yaw, velocity, std::f32::consts::PI, dt);
        }
        assert!((yaw - std::f32::consts::PI).abs() < 0.002);
    }
}
#[test]
fn reversing_a_turn_preserves_angular_momentum() {
    let (yaw, velocity) = turn_towards(0., 0., std::f32::consts::PI, 0.2);
    let (next, next_velocity) = turn_towards(yaw, velocity, 0., 0.001);
    assert!(
        next > yaw && next_velocity > 0.,
        "A changed target must not instantly flip angular velocity"
    );
    let (settled, speed) = turn_towards(next, next_velocity, 0., 3.);
    assert!(settled.abs() < 0.001 && speed.abs() < 0.001);
}
#[test]
fn turning_cuboid_keeps_its_visible_side_hittable() {
    let part = DragonHitPart {
        owner: Entity::PLACEHOLDER,
        half: Vec2::splat(0.5),
    };
    let transform = GlobalTransform::from(
        Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::new(100., 40., 46.)),
    );
    assert!(
        part.hit(&transform, Vec2::new(-40., 0.), Vec2::new(40., 0.))
            .is_some()
    );
    assert!(
        part.hit(&transform, Vec2::new(-40., 30.), Vec2::new(40., 30.))
            .is_none()
    );
}
#[test]
fn dragon_depth_layer_keeps_rotating_geometry_in_front_of_blocks() {
    let stage = dragon_stage_transform().to_matrix();
    for n in 0..=32 {
        let rig = Transform::from_xyz(0., 0., 90.)
            .with_rotation(Quat::from_rotation_y(n as f32 * std::f32::consts::PI / 32.))
            .to_matrix();
        for corner in [Vec3::new(-160., -200., -100.), Vec3::new(300., 200., 100.)] {
            let logical = rig.transform_point3(corner);
            let visible = (stage * rig).transform_point3(corner);
            assert!(visible.z > 32.);
            assert!(visible.xy().abs_diff_eq(logical.xy(), 0.001));
        }
    }
}
#[test]
fn perch_requires_space_for_body_wings_and_tail() {
    let candidate = Vec3::new(0., 91., 90.);
    assert!(perch_clear(candidate, &[Vec3::ZERO]));
    assert!(!perch_clear(
        candidate,
        &[Vec3::ZERO, Vec3::new(128., 128., 0.)]
    ));
    assert!(flammable(BlockType::Leaves));
    assert!(!flammable(BlockType::Obsidian));
}
