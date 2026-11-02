use super::*;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fluid_mesh_layouts_keep_all_vertex_attribute_lengths_equal() {
        let mut world = FluidWorld::default();
        world.deposit((0, 0), Liquid::Water, 8);
        world.supported.insert((0, 0));
        for mesh in [
            empty_mesh(),
            liquid_mesh(&world, Liquid::Water, |_| true),
            steam_mesh(&world),
        ] {
            assert!(mesh.contains_attribute(Mesh::ATTRIBUTE_POSITION));
            let count = mesh.count_vertices();
            for (_, values) in mesh.attributes() {
                assert_eq!(values.len(), count);
            }
        }
    }
    #[test]
    fn spill_tracks_lip_through_multiple_falling_cells() {
        for direction in [-1, 1] {
            let mut w = FluidWorld::default();
            let origin = (0, 3);
            let lip = (direction, 3);
            let blocked = HashSet::from([(0, 2)]);
            w.deposit(origin, Liquid::Water, 8);
            w.supported.insert(origin);
            w.transfer(origin, lip, 2, &blocked, &|_| true);
            w.transfer(lip, (direction, 2), 2, &blocked, &|_| true);
            w.transfer((direction, 2), (direction, 1), 2, &blocked, &|_| true);
            for y in 1..=3 {
                let r = fluid_sections(&w, (direction, y), Liquid::Water);
                let expected = direction as f32 * 38.;
                assert!(
                    r.iter()
                        .any(|r| ((r.0 + r.1) * 0.5 - expected).abs() < 0.01)
                );
            }
        }
    }
    #[test]
    fn full_airborne_parcels_cannot_become_supported_pools() {
        let mut w = FluidWorld::default();
        w.deposit((0, 4), Liquid::Water, 8);
        w.deposit((0, 3), Liquid::Water, 8);
        w.step(&HashSet::new(), |_| true);
        assert!(w.supported.is_empty());
        for &p in w.cells.keys() {
            assert!(
                fluid_sections(&w, p, Liquid::Water)
                    .iter()
                    .all(|r| r.1 - r.0 <= 12.)
            );
        }
    }
    #[test]
    fn water_has_one_connected_surface_at_terrain_depth_behind_heads() {
        for (kind, front) in [(Liquid::Water, 24.), (Liquid::Lava, 24.)] {
            let mut world = FluidWorld::default();
            world.deposit((0, 0), kind, 8);
            world.supported.insert((0, 0));
            let mesh = liquid_mesh(&world, kind, |_| true);
            let bevy::mesh::VertexAttributeValues::Float32x3(p) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!()
            };
            assert!(p[..4].iter().all(|v| v[2] == front));
            assert!(p.iter().all(|v| v[2] <= front && v[2] >= -40.));
            assert_eq!(mesh.count_vertices(), 20); // front and four exposed sides, no detached sheet
        }
    }
    #[test]
    fn joined_fluid_mesh_has_no_interior_transparent_faces() {
        let mut w = FluidWorld::default();
        for p in [(0, 0), (1, 0), (0, 1)] {
            w.deposit(p, Liquid::Water, 8);
            w.supported.insert(p);
        }
        let mesh = liquid_mesh(&w, Liquid::Water, |_| true);
        let bevy::mesh::VertexAttributeValues::Float32x3(pos) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!()
        };
        let bevy::mesh::VertexAttributeValues::Float32x3(norm) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap()
        else {
            panic!()
        };
        for (p, n) in pos.chunks_exact(4).zip(norm.chunks_exact(4)) {
            assert!(
                !(n[0][0] != 0. && p.iter().all(|v| v[0] == 32. && v[1] <= 32.)),
                "internal side face"
            );
            assert!(
                !(n[0][1] != 0. && p.iter().all(|v| v[1] == 32. && v[0] <= 32.)),
                "internal top/bottom face"
            );
        }
    }
    #[test]
    fn airborne_stream_never_ballons_near_ground_or_transport_parcels() {
        for kind in [Liquid::Water, Liquid::Lava] {
            let mut w = FluidWorld::default();
            w.deposit((0, 0), kind, 4);
            w.supported.insert((0, 0));
            let expected = if kind == Liquid::Water { 12. } else { 18. };
            for y in 1..8 {
                w.deposit((0, y), kind, if y % 2 == 0 { 8 } else { 1 });
                w.falling.insert((0, y), (kind, 1));
            }
            for y in 1..8 {
                let sections = fluid_sections(&w, (0, y), kind);
                assert_eq!(sections.len(), 1);
                assert_eq!(
                    sections[0].1 - sections[0].0,
                    expected,
                    "Airborne width changed at row {y}"
                );
                assert_eq!(sections[0].3 - sections[0].2, 64.);
            }
            assert_eq!(
                fluid_sections(&w, (0, 0), kind)[0].1 - fluid_sections(&w, (0, 0), kind)[0].0,
                64.
            );
        }
    }
    #[test]
    fn tiny_cascade_joins_vertical_rows_and_side_pool() {
        let mut w = FluidWorld::default();
        w.deposit((0, 0), Liquid::Water, 1);
        w.deposit((0, 1), Liquid::Water, 1);
        w.deposit((-1, 0), Liquid::Water, 4);
        w.supported.insert((-1, 0));
        let low = fluid_sections(&w, (0, 0), Liquid::Water);
        let high = fluid_sections(&w, (0, 1), Liquid::Water);
        assert!(low.iter().any(|r| r.3 == 32.));
        assert!(high.iter().any(|r| r.2 == 32.));
        assert!(
            low.iter().any(|r| r.0 == -32. && r.2 == -32. && r.3 >= 0.),
            "Pool lip must join the sheet at its shared edge"
        );
    }
    #[test]
    fn side_fed_spill_does_not_rise_above_its_feeding_pool() {
        let mut w = FluidWorld::default();
        w.deposit((-1, 0), Liquid::Water, 2);
        w.supported.insert((-1, 0));
        w.deposit((0, 0), Liquid::Water, 1);
        let sections = fluid_sections(&w, (0, 0), Liquid::Water);
        assert!(sections.iter().all(|r| r.3 <= -16.));
        assert!(sections.iter().any(|r| r.0 == -32. && r.3 == -16.));
        w.falling.insert((-1, 0), (Liquid::Water, 2));
        assert_eq!(
            fluid_sections(&w, (-1, 0), Liquid::Water).len(),
            1,
            "Outgoing flux must not produce an upward column"
        );
    }
    #[test]
    fn transport_trail_survives_a_tick_but_expires_without_more_flow() {
        let mut w = FluidWorld::default();
        w.deposit((0, 2), Liquid::Water, 8);
        w.step(&HashSet::new(), |_| true);
        w.step(&HashSet::new(), |_| true);
        assert!(w.falling.contains_key(&(0, 2)));
        for _ in 0..4 {
            w.step(&HashSet::new(), |_| true);
        }
        assert!(!w.falling.contains_key(&(0, 2)));
        w.step(&HashSet::from([(0, -4)]), |_| true);
        assert!(!w.falling.contains_key(&(0, -4)));
    }
    #[test]
    fn shared_heat_consumes_finite_water_and_creates_steam_without_refilling() {
        let mut w = FluidWorld::default();
        w.deposit((0, 0), Liquid::Water, 3);
        for _ in 0..3 {
            assert!(w.heat_cell((0, 0)));
        }
        assert!(!w.heat_cell((0, 0)));
        assert!(!w.has_liquid_at(0, 0));
        assert!(w.steam.contains_key(&(0, 0)));
        w.deposit((1, 0), Liquid::Lava, 4);
        assert!(!w.heat_cell((1, 0)));
        assert_eq!(w.cells[&(1, 0)].amount, 4);
    }
    #[test]
    fn rain_collects_finite_water_only_on_local_exposed_ledges() {
        use crate::components::weather::WeatherType;
        let mut w = FluidWorld::default();
        w.tick = 240;
        let terrain = HashSet::from([(0, 0), (2, 0), (2, 2), (100, 0)]);
        let mut rain = weather(WeatherType::Rain);
        rain.duration_remaining = 0.;
        w.react_weather(&rain, Vec2::ZERO, &terrain, &HashSet::new());
        assert!(w.cells.is_empty());
        rain.duration_remaining = 10.;
        w.react_weather(&rain, Vec2::ZERO, &terrain, &HashSet::new());
        assert_eq!(w.cells[&(0, 1)].amount, 1);
        assert!(w.cells.values().map(|v| v.amount as u32).sum::<u32>() <= 2);
        assert_eq!(w.cells[&(0, 1)].reserve, 0);
        assert!(!w.cells.contains_key(&(2, 1)));
        assert!(!w.cells.contains_key(&(100, 1)));
    }
    #[test]
    fn steam_fades_to_zero_and_does_not_pop_at_expiry() {
        assert_eq!(steam_opacity(0.), 0.);
        assert_eq!(steam_opacity(1.), 0.);
        assert!(steam_opacity(29. / 30.) < 0.01);
        for n in 12..30 {
            assert!(steam_opacity(n as f32 / 30.) >= steam_opacity((n + 1) as f32 / 30.));
        }
    }
    #[test]
    fn bucket_messages_respect_solids_finite_volume_and_have_no_built_in_cooldown() {
        use crate::{
            events::power_events::PowerActivated,
            resources::command_queue::{PowerCommand, PowerCommandType},
        };
        let mut app = App::new();
        app.init_resource::<FluidOccupancy>();
        let mut loaded = LoadedChunks::default();
        for x in -1..=1 {
            for y in -1..=1 {
                loaded.add(x, y);
            }
        }
        app.insert_resource(loaded)
            .init_resource::<FluidWorld>()
            .init_resource::<crate::systems::world_portals::PowerNotice>()
            .insert_resource(Time::<()>::default())
            .add_message::<PowerActivated>()
            .add_systems(Update, bucket_powers);
        app.world_mut().spawn((
            crate::components::hero::Hero {
                move_direction: 1.,
                direction_change_timer: Timer::default(),
            },
            Transform::IDENTITY,
        ));
        app.world_mut().spawn((
            Block {
                health: 1,
                max_health: 1,
            },
            BlockType::Stone,
            Transform::from_xyz(64., 64., 0.),
        ));
        app.world_mut().write_message(PowerActivated {
            command: PowerCommand::new(PowerCommandType::WaterBucket, "test".into()),
        });
        app.update();
        let w = app.world().resource::<FluidWorld>();
        assert!(!w.cells.contains_key(&(1, 1)));
        assert_eq!(w.cells.values().map(|v| v.amount as u32).sum::<u32>(), 16);
        // A second bucket straight away pours too: waits come from Settings > Cooldowns.
        app.world_mut().write_message(PowerActivated {
            command: PowerCommand::new(PowerCommandType::WaterBucket, "test".into()),
        });
        app.update();
        let w = app.world().resource::<FluidWorld>();
        assert!(w.cells.values().map(|v| v.amount as u32).sum::<u32>() > 16);
        assert!(
            w.cells
                .values()
                .all(|v| v.kind == Liquid::Water && v.reserve == 0)
        );
        assert!(
            app.world()
                .resource::<crate::systems::world_portals::PowerNotice>()
                .text
                .contains("WATER BUCKET")
        );
    }
    fn weather(
        kind: crate::components::weather::WeatherType,
    ) -> crate::components::weather::ActiveWeather {
        crate::components::weather::ActiveWeather {
            weather_type: kind,
            intensity: 1.,
            duration_remaining: 100.,
            ..default()
        }
    }
    #[test]
    fn falling_stream_contact_matches_visible_midsection_and_not_empty_cell_sides() {
        let mut app = App::new();
        app.init_resource::<FluidOccupancy>();
        let mut world = FluidWorld::default();
        world.falling.insert((0, 0), (Liquid::Lava, 4));
        app.insert_resource(world)
            .insert_resource(Time::<()>::default())
            .add_message::<crate::events::damage_events::EntityDamaged>()
            .add_systems(Update, contact);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.1));
        let wet = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0., 10., 0.).with_scale(Vec3::splat(0.2)),
                Health::new(5),
                LinearVelocity(Vec3::X * 100.),
            ))
            .id();
        let dry = app
            .world_mut()
            .spawn((
                Transform::from_xyz(26., 10., 0.).with_scale(Vec3::splat(0.2)),
                Health::new(5),
                LinearVelocity(Vec3::X * 100.),
            ))
            .id();
        app.update();
        assert!(app.world().get::<LinearVelocity>(wet).unwrap().x < 100.);
        assert_eq!(app.world().get::<LinearVelocity>(dry).unwrap().x, 100.);
        let hits: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<crate::events::damage_events::EntityDamaged>>()
            .drain()
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, wet);
    }
    #[test]
    fn weather_reactions_are_local_exposed_and_produce_steam_or_ice() {
        use crate::components::weather::WeatherType;
        let mut w = FluidWorld::default();
        w.tick = 240;
        w.cells.insert(
            (0, 0),
            Volume {
                kind: Liquid::Lava,
                amount: 8,
                reserve: 0,
            },
        );
        w.react_weather(
            &weather(WeatherType::Rain),
            Vec2::ZERO,
            &HashSet::new(),
            &HashSet::new(),
        );
        assert!(w.cooled.contains(&(0, 0)));
        assert!(w.steam.contains_key(&(0, 0)));
        w.tick = 120;
        w.cells.insert(
            (2, 0),
            Volume {
                kind: Liquid::Water,
                amount: 8,
                reserve: 0,
            },
        );
        w.react_weather(
            &weather(WeatherType::FireRain),
            Vec2::ZERO,
            &HashSet::new(),
            &HashSet::new(),
        );
        assert_eq!(w.cells[&(2, 0)].amount, 7);
        w.react_weather(
            &weather(WeatherType::Snow),
            Vec2::ZERO,
            &HashSet::from([(2, 1)]),
            &HashSet::new(),
        );
        assert!(!w.ice.contains_key(&(2, 0)));
        w.cells.get_mut(&(2, 0)).unwrap().amount = 8;
        w.supported.insert((2, 0));
        for _ in 0..120 {
            w.react_weather(
                &weather(WeatherType::Snow),
                Vec2::ZERO,
                &HashSet::new(),
                &HashSet::new(),
            );
        }
        assert!(w.ice.contains_key(&(2, 0)));
        assert!(!w.cells.contains_key(&(2, 0)));
    }
    #[test]
    fn water_persists_without_heat_and_new_solids_displace_it() {
        let mut w = FluidWorld::default();
        w.cells.insert(
            (0, 1),
            Volume {
                kind: Liquid::Water,
                amount: 8,
                reserve: 0,
            },
        );
        let walls = HashSet::from([(-1, 1), (1, 1), (0, 0)]);
        for _ in 0..400 {
            w.step(&walls, |p| p.0.abs() < 2 && p.1 >= 0 && p.1 <= 2);
        }
        assert!(w.has_liquid_at(0, 1));
        for _ in 0..1400 {
            w.step(&walls, |p| p.0.abs() < 2 && p.1 >= 0 && p.1 <= 2);
        }
        assert!(
            w.has_liquid_at(0, 1),
            "Water must not disappear merely because time elapsed"
        );
        w.cells.insert(
            (0, 1),
            Volume {
                kind: Liquid::Water,
                amount: 8,
                reserve: 0,
            },
        );
        let mut filled = walls;
        filled.insert((0, 1));
        w.step(&filled, |_| true);
        assert!(!w.has_liquid_at(0, 1));
    }
    #[test]
    fn tiny_droplets_do_not_create_whole_obsidian_blocks() {
        let mut w = FluidWorld::default();
        w.cells.insert(
            (0, 0),
            Volume {
                kind: Liquid::Lava,
                amount: 1,
                reserve: 0,
            },
        );
        w.cells.insert(
            (1, 0),
            Volume {
                kind: Liquid::Water,
                amount: 1,
                reserve: 0,
            },
        );
        w.step(&HashSet::new(), |_| true);
        assert!(!w.cooled.contains(&(0, 0)));
        assert!(w.steam.contains_key(&(0, 0)));
    }
    #[test]
    fn falling_water_stays_vertical_and_renders_a_connected_stream() {
        let mut w = FluidWorld::default();
        w.cells.insert(
            (0, 3),
            Volume {
                kind: Liquid::Water,
                amount: 8,
                reserve: 40,
            },
        );
        let floor: HashSet<_> = (-4..=4).map(|x| (x, 0)).collect();
        w.step(&floor, |_| true);
        assert!(
            w.cells.keys().all(|p| p.0 == 0),
            "Falling fluid must not spread into mid-air shelves"
        );
        assert!(w.falling.contains_key(&(0, 3)) && w.falling.contains_key(&(0, 2)));
        let mesh = liquid_mesh(&w, Liquid::Water, |_| true);
        let bevy::mesh::VertexAttributeValues::Float32x3(vertices) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!()
        };
        let seam = 160.0; // shared boundary between rows 2 and 3
        let bevy::mesh::VertexAttributeValues::Float32x3(normals) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            vertices
                .iter()
                .zip(normals)
                .filter(|(v, n)| n[2] == 1. && (v[1] - seam).abs() < 0.001)
                .count(),
            4
        );
        for _ in 0..10 {
            w.step(&floor, |p| p.1 >= 0);
        }
        assert!(
            w.cells.keys().any(|p| p.1 == 1 && p.0 != 0),
            "Water spreads once supported"
        );
    }
    #[test]
    fn small_pool_requires_closed_floor_walls_and_loaded_space() {
        let walls: HashSet<_> = (-3..=3)
            .flat_map(|x| {
                (-2..=0).filter_map(move |y| {
                    if y == -2 || x == -3 || x == 3 {
                        Some((x, y))
                    } else {
                        None
                    }
                })
            })
            .collect();
        assert_eq!(basin((0, 0), &walls, |_| true).unwrap().len(), 10);
        let mut leaking = walls.clone();
        leaking.remove(&(0, -2));
        assert!(basin((0, 0), &leaking, |_| true).is_none());
        assert!(basin((0, 0), &walls, |p| p.0 < 1).is_none());
    }
    #[test]
    fn partial_immersion_slows_and_extinguishes_then_lava_burns() {
        let mut app = App::new();
        app.init_resource::<FluidOccupancy>();
        let mut water = FluidWorld::default();
        water.cells.insert(
            (0, 0),
            Volume {
                kind: Liquid::Water,
                amount: 8,
                reserve: 0,
            },
        );
        water.supported.insert((0, 0));
        app.insert_resource(water)
            .insert_resource(Time::<()>::default())
            .add_message::<crate::events::damage_events::EntityDamaged>()
            .add_systems(Update, contact);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.1));
        let body = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0.0, 48.0, 10.0),
                LinearVelocity(Vec3::new(100.0, -100.0, 0.0)),
                Health::new(5),
                Buff::new(BuffType::Burning, 3.0),
            ))
            .id();
        app.update();
        assert!(app.world().get::<Buff>(body).is_none());
        assert!(app.world().get::<LinearVelocity>(body).unwrap().x < 100.0);
        app.world_mut()
            .resource_mut::<FluidWorld>()
            .cells
            .get_mut(&(0, 0))
            .unwrap()
            .kind = Liquid::Lava;
        app.update();
        assert_eq!(
            app.world().get::<Buff>(body).unwrap().buff_type,
            BuffType::Burning
        );
    }
    #[test]
    fn immersion_swims_and_lava_hits_immediately_without_repeating_each_frame() {
        let mut app = App::new();
        app.init_resource::<FluidOccupancy>();
        let mut world = FluidWorld::default();
        world.cells.insert(
            (0, 0),
            Volume {
                kind: Liquid::Water,
                amount: 8,
                reserve: 0,
            },
        );
        world.supported.insert((0, 0));
        app.insert_resource(world)
            .insert_resource(Time::<()>::default())
            .add_message::<crate::events::damage_events::EntityDamaged>()
            .add_systems(Update, contact);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.1));
        let e = app
            .world_mut()
            .spawn((
                Transform::IDENTITY,
                Health::new(5),
                LinearVelocity(Vec3::new(400., -300., 0.)),
            ))
            .id();
        app.update();
        let v = app.world().get::<LinearVelocity>(e).unwrap();
        assert!(
            v.x <= 180. && v.y < 0.,
            "An entering head must be able to dive"
        );
        app.world_mut()
            .resource_mut::<FluidWorld>()
            .cells
            .get_mut(&(0, 0))
            .unwrap()
            .kind = Liquid::Lava;
        app.update();
        assert!(app.world().get::<LinearVelocity>(e).unwrap().x <= 65.);
        let hits: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<crate::events::damage_events::EntityDamaged>>()
            .drain()
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, e);
        assert_eq!(hits[0].damage, 1);
        app.update();
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<crate::events::damage_events::EntityDamaged>>()
                .drain()
                .count(),
            0
        );
    }
    #[test]
    fn finite_liquid_conserves_volume_flows_down_and_stops_at_unloaded_edges() {
        let mut w = FluidWorld::default();
        w.cells.insert(
            (0, 2),
            Volume {
                kind: Liquid::Water,
                amount: 8,
                reserve: 40,
            },
        );
        let wall: HashSet<_> = (-4..=4).map(|x| (x, 0)).collect();
        for _ in 0..40 {
            w.step(&wall, |p| p.0.abs() <= 3 && p.1 >= 0 && p.1 <= 2);
        }
        assert_eq!(
            w.cells
                .values()
                .map(|v| v.amount as u32 + v.reserve as u32)
                .sum::<u32>(),
            48
        );
        assert!(w.cells.keys().any(|p| p.1 == 1 && p.0 != 0));
        assert!(w.cells.keys().all(|p| !wall.contains(p) && p.0.abs() <= 3));
    }
    #[test]
    fn lava_moves_slower_and_water_contact_forms_solid_obsidian() {
        let mut w = FluidWorld::default();
        w.cells.insert(
            (0, 1),
            Volume {
                kind: Liquid::Lava,
                amount: 8,
                reserve: 0,
            },
        );
        w.step(&HashSet::new(), |_| true);
        assert_eq!(w.cells[&(0, 1)].amount, 8);
        w.cells.insert(
            (1, 1),
            Volume {
                kind: Liquid::Water,
                amount: 8,
                reserve: 0,
            },
        );
        w.step(&HashSet::new(), |_| true);
        assert!(w.cooled.contains(&(0, 1)));
        assert!(!w.cells.contains_key(&(0, 1)));
    }
    #[test]
    fn nether_has_deterministic_lava_and_rare_water_sources() {
        let kinds: Vec<_> = (0..2000)
            .filter_map(|x| spring_kind(x, 10, 42, BiomeType::Nether))
            .collect();
        assert!(kinds.contains(&Liquid::Water) && kinds.contains(&Liquid::Lava));
        assert_eq!(
            spring_kind(1, 2, 3, BiomeType::Nether),
            spring_kind(1, 2, 3, BiomeType::Nether)
        );
    }
}

#[cfg(test)]
mod encounter_tests {
    use super::*;
    #[test]
    fn ice_refuses_occupied_or_falling_water_and_preserves_shallow_volume() {
        let mut world = FluidWorld::default();
        for x in 0..3 {
            world.deposit((x, 0), Liquid::Water, 8);
            world.supported.insert((x, 0));
        }
        let mut occupied = occupied_cells(std::iter::once((Vec2::new(32., 0.), 32.)));
        assert!(!world.freeze((0, 0), 120, &occupied));
        assert!(!world.freeze((1, 0), 120, &occupied));
        assert!(world.freeze((2, 0), 120, &occupied));
        assert!(!world.has_liquid_at(2, 0));
        assert_eq!(world.cells[&(0, 0)].amount, 8);
        occupied.clear();
        world.cells.get_mut(&(0, 0)).unwrap().amount = 7;
        assert!(world.freeze((0, 0), 120, &occupied));
        assert!(world.thaw((0, 0)));
        assert_eq!(world.cells[&(0, 0)].amount, 7);
        world.supported.remove(&(1, 0));
        assert!(!world.freeze((1, 0), 120, &occupied));
    }
    #[test]
    fn a_fast_snowball_freezes_shallow_supported_water() {
        let mut app = App::new();
        app.init_resource::<FluidOccupancy>();
        let mut water = FluidWorld::default();
        water.deposit((0, 0), Liquid::Water, 2);
        water.supported.insert((0, 0));
        app.insert_resource(water)
            .insert_resource(Time::<()>::default())
            .add_systems(Update, (update_occupancy, frost_projectiles).chain());
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.1));
        let ball = app
            .world_mut()
            .spawn((
                crate::components::projectile::Snowball::new(),
                Transform::from_xyz(0., -25., 0.),
                LinearVelocity(Vec3::new(0., -800., 0.)),
            ))
            .id();
        app.update();
        assert!(
            app.world()
                .resource::<FluidWorld>()
                .ice
                .contains_key(&(0, 0))
        );
        assert!(app.world().get_entity(ball).is_err());
        let mut water = app.world_mut().resource_mut::<FluidWorld>();
        water.thaw((0, 0));
        assert_eq!(water.cells[&(0, 0)].amount, 2);
    }
    #[test]
    fn snowfall_freezes_six_unit_exposed_pools_without_creating_water() {
        let mut water = FluidWorld::default();
        water.deposit((0, 0), Liquid::Water, 6);
        water.supported.insert((0, 0));
        water.tick = 120;
        let weather = crate::components::weather::ActiveWeather {
            weather_type: crate::components::weather::WeatherType::Snow,
            intensity: 1.,
            duration_remaining: 20.,
            ..default()
        };
        for _ in 0..119 {
            water.react_weather(
                &weather,
                Vec2::ZERO,
                &HashSet::from([(0, -1)]),
                &HashSet::new(),
            );
        }
        assert!(
            !water.ice.contains_key(&(0, 0)),
            "Freezing needs sustained snowfall"
        );
        water.react_weather(
            &weather,
            Vec2::ZERO,
            &HashSet::from([(0, -1)]),
            &HashSet::new(),
        );
        assert!(water.ice.contains_key(&(0, 0)));
        water.thaw((0, 0));
        assert_eq!(water.cells[&(0, 0)].amount, 6);
    }
    #[test]
    fn snowfall_freezes_the_top_pool_layer_and_resets_when_sheltered() {
        let mut water = FluidWorld::default();
        for y in 0..=1 {
            water.deposit((0, y), Liquid::Water, 8);
            water.supported.insert((0, y));
        }
        let snow = crate::components::weather::ActiveWeather {
            weather_type: crate::components::weather::WeatherType::Snow,
            intensity: 1.,
            duration_remaining: 100.,
            ..default()
        };
        for _ in 0..100 {
            water.react_weather(&snow, Vec2::ZERO, &HashSet::new(), &HashSet::new());
        }
        water.react_weather(&snow, Vec2::ZERO, &HashSet::from([(0, 2)]), &HashSet::new());
        assert!(water.frost_exposure.is_empty());
        for _ in 0..119 {
            water.react_weather(&snow, Vec2::ZERO, &HashSet::new(), &HashSet::new());
        }
        assert!(water.ice.is_empty());
        water.react_weather(&snow, Vec2::ZERO, &HashSet::new(), &HashSet::new());
        assert!(water.ice.contains_key(&(0, 1)));
        assert!(!water.ice.contains_key(&(0, 0)));
        for _ in 0..150 {
            water.react_weather(&snow, Vec2::ZERO, &HashSet::new(), &HashSet::new());
        }
        assert!(
            water.has_liquid_at(0, 0),
            "Ice surface shelters deeper water"
        );
    }
    #[test]
    fn manual_dive_and_rise_override_automatic_buoyancy() {
        let mut app = App::new();
        app.init_resource::<FluidOccupancy>();
        let mut fluids = FluidWorld::default();
        fluids.deposit((0, 0), Liquid::Water, 8);
        fluids.supported.insert((0, 0));
        app.insert_resource(fluids)
            .insert_resource(Time::<()>::default())
            .init_resource::<ButtonInput<KeyCode>>()
            .add_message::<crate::events::damage_events::EntityDamaged>()
            .add_systems(Update, contact);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1. / 30.));
        let actor = app
            .world_mut()
            .spawn((
                crate::components::hero::Hero {
                    move_direction: 1.,
                    direction_change_timer: Timer::default(),
                },
                Health::new(5),
                Transform::IDENTITY,
                LinearVelocity::ZERO,
            ))
            .id();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyS);
        for _ in 0..20 {
            app.update();
        }
        assert!(app.world().get::<LinearVelocity>(actor).unwrap().y < -60.);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowUp);
        for _ in 0..20 {
            app.update();
        }
        assert!(app.world().get::<LinearVelocity>(actor).unwrap().y > 180.);
    }
    #[test]
    fn landing_and_respilling_keep_edge_origin() {
        let mut world = FluidWorld::default();
        world.deposit((0, 1), Liquid::Water, 8);
        world.fall_offset.insert((0, 1), -26.);
        world.transfer((0, 1), (0, 0), 8, &HashSet::new(), &|_| true);
        world.supported.insert((0, 0));
        assert_eq!(world.fall_offset[&(0, 0)], -26.);
        world.transfer((0, 0), (0, -1), 8, &HashSet::new(), &|_| true);
        assert_eq!(world.fall_offset[&(0, -1)], -26.);
    }
    #[test]
    fn small_flowing_lava_contacts_water_and_forms_obsidian() {
        let mut world = FluidWorld::default();
        world.deposit((0, 0), Liquid::Lava, 2);
        world.deposit((1, 0), Liquid::Water, 2);
        world.step(&HashSet::from([(0, -1), (1, -1)]), |_| true);
        assert!(world.cooled.contains(&(0, 0)));
    }
}
