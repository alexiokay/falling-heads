use super::*;

pub(crate) fn verify_structure_depth(world: &mut World, ray_tracing: bool) {
    use crate::systems::actor_depth::{
        BACKGROUND, STRUCTURE_BACKING, STRUCTURE_THICKNESS, TERRAIN_CENTER,
    };
    let walls: Vec<_> = world
        .query_filtered::<(
            Entity,
            &Transform,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
        ), With<StructureBackdrop>>()
        .iter(world)
        .map(|(e, t, m, mat)| (e, *t, m.0.clone(), mat.0.clone()))
        .collect();
    assert!(!walls.is_empty(), "Cottage must include recessed walls");
    for (entity, t, mesh, material) in &walls {
        assert_eq!(t.translation.z, STRUCTURE_BACKING);
        assert_eq!(t.scale, Vec3::ONE);
        assert!(t.translation.z + STRUCTURE_THICKNESS / 2. < TERRAIN_CENTER - 32.);
        assert!(t.translation.z - STRUCTURE_THICKNESS / 2. > BACKGROUND);
        let mesh = world.resource::<Assets<Mesh>>().get(mesh).unwrap();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(vertices)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("Backing needs solid mesh positions")
        };
        assert!(vertices.iter().any(|v| v[2] == STRUCTURE_THICKNESS / 2.));
        assert!(vertices.iter().any(|v| v[2] == -STRUCTURE_THICKNESS / 2.));
        assert!(
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some()
                && mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some()
        );
        let material = world
            .resource::<Assets<StandardMaterial>>()
            .get(material)
            .unwrap();
        assert!(!material.unlit && material.base_color_texture.is_some());
        assert!(
            world.get::<Collider>(*entity).is_none() && world.get::<Block>(*entity).is_none(),
            "Recessed walls must not enter gameplay collision"
        );
        assert!(world.get::<bevy::light::NotShadowCaster>(*entity).is_none());
    }
    if ray_tracing {
        assert_eq!(
            world
                .resource::<crate::systems::native_gpu::NativeStatus>()
                .mode,
            2
        );
        let traced = world
            .query_filtered::<&ChildOf, With<bevy::solari::prelude::RaytracingMesh3d>>()
            .iter(world)
            .filter(|parent| world.get::<StructureBackdrop>(parent.parent()).is_some())
            .count();
        assert!(
            traced > 0,
            "Recessed walls must participate in real RT lighting"
        );
    }
    eprintln!(
        "[STRUCTURE DEPTH] verified {} textured solid walls, rt={ray_tracing}",
        walls.len()
    );
}

pub(super) fn style_capture(world: &mut World, name: &str) {
    world
        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(format!(
            "target/visual-style-{name}.png"
        )));
}

pub fn style_probe(world: &mut World) {
    if !world.contains_resource::<StyleProbe>() {
        world.init_resource::<StyleProbe>();
    }
    let now = world.resource::<Time<Real>>().elapsed_secs();
    let step = world.resource::<StyleProbe>().step;
    let since = world.resource::<StyleProbe>().since;
    assert!(now < 100., "Style probe timeout stage {step}");
    let anchor = world.resource::<StyleProbe>().anchor;
    if step > 0 {
        for (mut p, mut v) in world
            .query_filtered::<(&mut Position, &mut LinearVelocity), With<Hero>>()
            .iter_mut(world)
        {
            p.0 = anchor + Vec3::new(-180., 0., 30.);
            v.0 = Vec3::ZERO;
        }
        let mut w = world.resource_mut::<crate::components::weather::ActiveWeather>();
        w.weather_type = crate::components::weather::WeatherType::Clear;
        w.intensity = 0.;
        w.duration_remaining = 999.;
        let others: Vec<_> = world
            .query_filtered::<Entity, With<Villager>>()
            .iter(world)
            .collect();
        for e in others {
            world.despawn(e);
        }
        for mut hp in world
            .query_filtered::<&mut Health, With<Enemy>>()
            .iter_mut(world)
        {
            hp.current = hp.max;
        }
        if let Some(witch) = world.resource::<StyleProbe>().witch {
            if let Some(mut hp) = world.get_mut::<Health>(witch) {
                hp.current = hp.max;
            }
        }
    }
    match step {
        0 if now > 8. => {
            let p = world
                .query_filtered::<&Transform, With<Hero>>()
                .single(world)
                .unwrap()
                .translation;
            let p = (p.xy() / 64.).round().extend(0.) * 64.;
            world.resource_mut::<StyleProbe>().anchor = p;
            world
                .query::<&mut Window>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(1360., 900.);
            let mut g =
                world.resource_mut::<crate::resources::graphics_settings::GraphicsSettings>();
            g.ray_tracing = false;
            g.dlss = false;
            g.frame_generation = false;
            let old: Vec<_> = world
                .query::<(Entity, &Transform, Option<&Block>, Option<&Enemy>)>()
                .iter(world)
                .filter(|(_, t, b, e)| {
                    (b.is_some() || e.is_some()) && t.translation.xy().distance(p.xy()) < 900.
                })
                .map(|(e, _, _, _)| e)
                .collect();
            for e in old {
                world.despawn(e);
            }
            let mesh = world.resource::<MeshCache>().block_mesh.clone();
            let mat = world.resource::<MaterialCache>().materials[&BlockType::Stone].clone();
            let chunk = (p.xy() / 1024.).floor().as_ivec2();
            for x in -10..=10 {
                world.spawn((
                    Block {
                        health: 100,
                        max_health: 100,
                    },
                    BlockType::Stone,
                    ChunkMember {
                        chunk_x: chunk.x,
                        chunk_y: chunk.y,
                    },
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_translation(p + Vec3::new(x as f32 * 64., -64., -10.))
                        .with_scale(Vec3::splat(64.)),
                ));
            }
            let art = world.resource::<Art>().clone();
            let witch = world.resource_scope(|w, muts: Mut<Assets<StandardMaterial>>| {
                let mut mats = muts;
                spawn_foe(
                    &mut w.commands(),
                    p + Vec3::new(140., 0., 30.),
                    BiomeType::Forest,
                    Role::Witch,
                    chunk,
                    &art,
                    &mut mats,
                )
            });
            world.flush();
            world.entity_mut(witch).insert(RigidBody::Static);
            world.resource_mut::<StyleProbe>().witch = Some(witch);
            for (kind, offset) in [
                (EnemyType::Bat, Vec3::new(-90., 160., 30.)),
                (EnemyType::Spider, Vec3::new(340., 0., 30.)),
            ] {
                world.spawn((
                    Enemy::new(kind),
                    Health::new(if kind == EnemyType::Bat { 2 } else { 4 }),
                    Buffable,
                    avian_physics::enemy_with_velocity(Vec3::ZERO),
                    Transform::from_translation(p + offset),
                    Visibility::Inherited,
                ));
            }
            world.resource_scope(|w, m: Mut<MeshCache>| {
                w.resource_scope(|w, a: Mut<MaterialCache>| {
                    crate::systems::world_portals::spawn_ruin(
                        &mut w.commands(),
                        (p.xy() / 64.).as_ivec2() + IVec2::new(7, 2),
                        BiomeType::End,
                        &m,
                        &a,
                    )
                })
            });
        }
        1 if now > since + 2. => {
            assert!(
                world
                    .query::<&crate::systems::bats::Bat>()
                    .iter(world)
                    .count()
                    > 0
            );
            style_capture(world, "heads-bats-portal");
        }
        2 if now > since + 1. => {
            let e = world.resource::<StyleProbe>().witch.unwrap();
            world.write_message(EntityDamaged {
                entity: e,
                damage: 1,
            });
        }
        3 if now > since + 0.12 => {
            let e = world.resource::<StyleProbe>().witch.unwrap();
            assert!(
                world
                    .get::<crate::components::damage_visual::DamageVisual>(e)
                    .is_some()
            );
            style_capture(world, "witch-hit");
        }
        4 if now > since + 1. => {
            let art = world.resource::<Art>().clone();
            let p = anchor + Vec3::new(15., 60., 50.);
            let bottle = world
                .spawn((
                    Potion {
                        from: p,
                        to: p + Vec3::X * 180.,
                        age: 0.,
                        biome: BiomeType::Forest,
                        arc: 100.,
                    },
                    Transform::from_translation(p).with_scale(Vec3::new(32., 32., 1.)),
                    Mesh3d(art.quad.clone()),
                    MeshMaterial3d(art.potion),
                ))
                .id();
            world.spawn((
                ChildOf(bottle),
                Mesh3d(art.quad),
                MeshMaterial3d(art.liquids[0].clone()),
                Transform::from_xyz(0., 0., 0.1),
            ));
        }
        5 if now > since + 0.4 => {
            assert!(world.query::<&Potion>().iter(world).count() > 0);
            style_capture(world, "potion-flight");
        }
        6 if now > since + 0.85 => {
            assert!(world.query::<&Hazard>().iter(world).count() > 0);
            style_capture(world, "cloud-young");
        }
        7 if now > since + 6. => {
            style_capture(world, "cloud-fading");
        }
        8 if now > since + 1. => {
            let owner = world
                .query_filtered::<Entity, With<Hero>>()
                .single(world)
                .unwrap();
            let root = world
                .spawn((
                    crate::components::projectile::MagicBarrier {
                        owner_entity: owner,
                        radius: 150.,
                        lifetime: Timer::from_seconds(60., TimerMode::Once),
                    },
                    Transform::IDENTITY,
                    Visibility::Inherited,
                ))
                .id();
            world.spawn((
                ChildOf(root),
                crate::systems::mirror_vfx::MirrorVisual,
                Transform::from_scale(Vec3::splat(150.)),
            ));
            world.resource_mut::<StyleProbe>().mirror = Some(root);
        }
        9 if now > since + 1. => {
            assert!(
                world
                    .query_filtered::<Entity, (
                        With<crate::systems::mirror_vfx::MirrorVisual>,
                        With<bevy::light::NotShadowCaster>
                    )>()
                    .iter(world)
                    .count()
                    > 0
            );
            style_capture(world, "mirror-raster");
        }
        10 if now > since + 1. => {
            let mut g =
                world.resource_mut::<crate::resources::graphics_settings::GraphicsSettings>();
            g.ray_tracing = true;
            g.dlss = true;
        }
        11 if now > since + 6. => {
            style_capture(world, "mirror-rt");
        }
        12 if now > since + 1. => {
            if let Some(e) = world.resource_mut::<StyleProbe>().mirror.take() {
                world.despawn(e);
            }
        }
        13 if now > since + 2. => {
            style_capture(world, "no-mirror-rt");
        }
        14 if now > since + 1. => {
            eprintln!(
                "[STYLE TEST] completed: sprite damage, bat flight, cloud dissolve, portal, mirror raster/RT"
            );
            world.write_message(AppExit::Success);
        }
        _ => return,
    }
    let mut p = world.resource_mut::<StyleProbe>();
    p.step += 1;
    p.since = now;
}
