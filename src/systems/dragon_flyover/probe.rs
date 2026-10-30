use super::*;

pub(crate) fn smoke_spawn(world: &mut World) {
    let p = world
        .query_filtered::<&Transform, With<Hero>>()
        .single(world)
        .unwrap()
        .translation;
    world.resource_scope(|world, a: Mut<DragonAssets>| spawn(&mut world.commands(), p, &a));
}

pub(crate) fn smoke_verify(world: &mut World) {
    let (dragon, transform) = world
        .query::<(&EndDragon, &Transform)>()
        .single(world)
        .expect("Dragon flyover must remain active");
    assert!(dragon.age > 2. && dragon.age < 8.);
    assert!(transform.translation.distance(dragon.origin) > 50.);
    assert!(
        world.query::<&Joint>().iter(world).count() >= 13,
        "Articulated parts missing"
    );
    eprintln!("SMOKE articulated dragon flight and joint hierarchy passed");
}

pub(crate) fn smoke_breath(world: &mut World) {
    let mut dragons = world.query::<&mut EndDragon>();
    let mut dragon = dragons
        .single_mut(world)
        .expect("Boss must persist beyond the old flyby timer");
    assert!(dragon.age > 11.);
    dragon.phase = 8.;
    eprintln!("SMOKE persistent boss passed; exercising continuous fire breath");
}

pub(crate) fn smoke_damage(world: &mut World) {
    let (entity, health) = world
        .query_filtered::<(Entity, &Health), With<EndDragon>>()
        .single(world)
        .unwrap();
    assert!(health.current > 0);
    world.write_message(crate::events::damage_events::EntityDamaged { entity, damage: 3 });
}

pub(crate) fn smoke_damaged(world: &mut World) {
    let (dragon, health) = world
        .query::<(&EndDragon, &Health)>()
        .single(world)
        .unwrap();
    assert!(
        health.current <= 37 && dragon.last_health == health.current,
        "Boss must take damage through shared combat"
    );
    eprintln!("SMOKE reactive boss health and attack system passed");
}

pub(crate) fn smoke_turn(world: &mut World) {
    let hero = world
        .query_filtered::<&Transform, With<Hero>>()
        .single(world)
        .unwrap()
        .translation;
    let (mut dragon, mut transform) = world
        .query::<(&mut EndDragon, &mut Transform)>()
        .single_mut(world)
        .unwrap();
    transform.translation = hero + Vec3::new(-330., 160., 90.);
    dragon.phase = 7.;
    dragon.yaw = 0.;
    dragon.yaw_velocity = 0.;
    dragon.velocity = Vec2::new(-140., 0.);
    dragon.roll = 0.;
    dragon.facing_right = false;
    eprintln!("[DRAGON TURN] exercising full left-to-right transition");
}

pub(crate) fn smoke_verify_turn_depth(world: &mut World) {
    let mut parts = world.query::<(&DragonHitPart, &Mesh3d, &GlobalTransform)>();
    let meshes = world.resource::<Assets<Mesh>>();
    let mut vertices = 0;
    for (_, handle, transform) in parts.iter(world) {
        let Some(mesh) = meshes.get(&handle.0) else {
            continue;
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            continue;
        };
        for position in positions {
            let point = transform.transform_point(Vec3::from_array(*position));
            assert!(
                point.is_finite() && point.z > 32.,
                "Animated dragon geometry clipped terrain: {point:?}"
            );
            vertices += 1;
        }
    }
    assert!(
        vertices > 100,
        "Turn probe must exercise the real articulated meshes"
    );
}

pub(crate) fn smoke_surface_gallery(world: &mut World) {
    let center = world
        .query_filtered::<&Transform, With<Hero>>()
        .single(world)
        .unwrap()
        .translation;
    for (i, kind) in [
        BlockType::Wood,
        BlockType::Leaves,
        BlockType::Stone,
        BlockType::Glowstone,
        BlockType::DeadBush,
    ]
    .into_iter()
    .enumerate()
    {
        let cache = world.resource::<crate::resources::mesh_cache::MeshCache>();
        let (mesh, scale) = match kind {
            BlockType::Wood => (cache.wood_mesh.clone(), Vec3::ONE),
            BlockType::Leaves => (cache.leaves_mesh.clone(), Vec3::ONE),
            BlockType::DeadBush => (cache.dead_bush_mesh.clone(), Vec3::ONE),
            _ => (cache.block_mesh.clone(), Vec3::splat(64.)),
        };
        let mat = world
            .resource::<crate::resources::material_cache::MaterialCache>()
            .get(kind)
            .unwrap();
        let position = center + Vec3::new(-128. + i as f32 * 72., 250., 0.);
        let owner = world
            .spawn((
                Block {
                    health: 12,
                    max_health: 12,
                },
                kind,
                Mesh3d(mesh),
                MeshMaterial3d(mat),
                Transform::from_translation(position).with_scale(scale),
            ))
            .id();
        if flammable(kind) {
            world.resource_scope(|world, a: Mut<DragonAssets>| {
                spawn_ground_fire(&mut world.commands(), owner, position, &a)
            });
        }
    }
    world.flush();
}

pub(crate) fn smoke_impact_gallery(world: &mut World) {
    let center = world
        .query_filtered::<&Transform, With<Hero>>()
        .single(world)
        .unwrap()
        .translation;
    let mut wall = Vec::new();
    for x in 0..3 {
        for y in -1..=1 {
            let mesh = world
                .resource::<crate::resources::mesh_cache::MeshCache>()
                .block_mesh
                .clone();
            let mat = world
                .resource::<crate::resources::material_cache::MaterialCache>()
                .get(BlockType::Stone)
                .unwrap();
            let position = center + Vec3::new(90. + x as f32 * 64., 180. + y as f32 * 64., 0.);
            wall.push(
                world
                    .spawn((
                        Block {
                            health: 20,
                            max_health: 20,
                        },
                        BlockType::Stone,
                        Mesh3d(mesh),
                        MeshMaterial3d(mat),
                        Transform::from_translation(position).with_scale(Vec3::splat(64.)),
                    ))
                    .id(),
            );
        }
    }
    let mesh = world
        .resource::<crate::resources::mesh_cache::MeshCache>()
        .wood_mesh
        .clone();
    let mat = world
        .resource::<crate::resources::material_cache::MaterialCache>()
        .get(BlockType::Wood)
        .unwrap();
    let position = center + Vec3::new(-90., 340., 0.);
    let wood = world
        .spawn((
            Block {
                health: 20,
                max_health: 20,
            },
            BlockType::Wood,
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(position),
        ))
        .id();
    world.resource_scope(|world, a: Mut<DragonAssets>| {
        spawn_ground_fire(&mut world.commands(), wood, position, &a)
    });
    let mouth = center + Vec3::new(-220., 212., 65.);
    let a = world.resource::<DragonAssets>();
    let (mesh, mat) = (a.quad.clone(), a.breath.clone());
    let beam = world
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            bevy::light::NotShadowCaster,
            Transform::from_translation(mouth + Vec3::X * 139.)
                .with_scale(Vec3::new(278., 110., 1.)),
            Visibility::Hidden,
        ))
        .id();
    world.insert_resource(FireGallery { mouth, wall, beam });
    world.flush();
}

pub(crate) fn smoke_verify_impact(world: &mut World) {
    let t = world.resource::<Time<Real>>().elapsed_secs();
    let wall = world.resource::<FireGallery>().wall.clone();
    let mut count = 0;
    for (flame, transform) in world.query::<(&SurfaceFlame, &Transform)>().iter(world) {
        assert_eq!(
            transform.scale,
            Vec3::new(96., 112., 1.),
            "Fire must fade without mesh scaling"
        );
        if wall.contains(&flame.owner) {
            count += 1;
        }
    }
    if (10. ..11.9).contains(&t) {
        assert!(count >= 2, "Breath must wash over multiple exposed faces");
    }
    if t > 12.5 {
        assert_eq!(
            count, 0,
            "Stone must not retain the breath effect once the burst ends"
        );
    }
}
