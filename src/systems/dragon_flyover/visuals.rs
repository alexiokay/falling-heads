use super::*;

pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut breath: ResMut<Assets<BreathMaterial>>,
) {
    commands.spawn((
        BossLabel,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(17.),
            ..default()
        },
        TextColor(Color::srgb(0.94, 0.64, 1.)),
        GlobalZIndex(80),
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.),
            left: Val::Px(110.),
            width: Val::Percent(60.),
            ..default()
        },
    ));
    let stage = commands
        .spawn((dragon_stage_transform(), Visibility::Inherited))
        .id();
    commands.insert_resource(DragonAssets {
        stage,
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        quad: meshes.add(Rectangle::new(1., 1.)),
        wing: meshes.add(wing_mesh()),
        skin: mats.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.13, 0.20),
            perceptual_roughness: 0.72,
            ..default()
        }),
        membrane: mats.add(StandardMaterial {
            base_color: Color::srgb(0.42, 0.09, 0.21),
            perceptual_roughness: 0.85,
            double_sided: true,
            cull_mode: None,
            ..default()
        }),
        eyes: mats.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.3, 1.),
            emissive: LinearRgba::rgb(4., 0.4, 7.),
            ..default()
        }),
        scales: mats.add(StandardMaterial {
            base_color: Color::srgb(0.27, 0.36, 0.48),
            perceptual_roughness: 0.8,
            ..default()
        }),
        belly: mats.add(StandardMaterial {
            base_color: Color::srgb(0.48, 0.38, 0.29),
            perceptual_roughness: 0.9,
            ..default()
        }),
        bone: mats.add(StandardMaterial {
            base_color: Color::srgb(0.84, 0.76, 0.58),
            perceptual_roughness: 0.72,
            ..default()
        }),
        breath: breath.add(BreathMaterial { effect: Vec4::ZERO }),
    });
}

pub(super) fn spawn(commands: &mut Commands, p: Vec3, a: &DragonAssets) {
    let root = commands
        .spawn((
            ChildOf(a.stage),
            EndDragon {
                age: 0.,
                origin: p + Vec3::new(650., 210., 70.),
                phase: 0.,
                last_health: 40,
                perch: None,
                attack_tick: 0.,
                facing_right: false,
                yaw: 0.,
                yaw_velocity: 0.,
                velocity: Vec2::ZERO,
                bank: 0.,
                roll: 0.,
            },
            Enemy::new(EnemyType::Dragon),
            Health::new(40),
            avian3d::prelude::LinearVelocity::ZERO,
            Flying,
            Transform::from_translation(p + Vec3::new(650., 210., 70.)),
            Visibility::Inherited,
        ))
        .id();
    let neck_position = Vec3::new(-48., 5., 0.);
    let neck = commands
        .spawn((
            ChildOf(root),
            Joint {
                owner: root,
                phase: 0.,
                amplitude: 0.,
                axis: Vec3::Y,
                base: Quat::IDENTITY,
                motion: JointMotion::Neck,
            },
            Transform::from_translation(neck_position),
            Visibility::Inherited,
        ))
        .id();
    let mut part = |parent: Entity, pos: Vec3, size: Vec3, mat: Handle<StandardMaterial>| {
        let (parent, pos) = if parent == root && pos.x < -52. {
            (neck, pos - neck_position)
        } else {
            (parent, pos)
        };
        commands
            .spawn((
                ChildOf(parent),
                DragonHitPart {
                    owner: root,
                    half: Vec2::splat(0.5),
                },
                Mesh3d(a.cube.clone()),
                MeshMaterial3d(mat),
                Transform::from_translation(pos).with_scale(size),
            ))
            .id()
    };
    let chest = part(root, Vec3::ZERO, Vec3::new(105., 46., 46.), a.skin.clone());
    part(
        root,
        Vec3::new(-83., 10., 0.),
        Vec3::new(62., 39., 40.),
        a.skin.clone(),
    );
    for y in [-1., 1.] {
        part(
            root,
            Vec3::new(-99., 16., y * 21.),
            Vec3::new(15., 8., 3.),
            a.eyes.clone(),
        );
        part(
            root,
            Vec3::new(-65., 36., y * 16.),
            Vec3::new(8., 28., 7.),
            a.skin.clone(),
        );
    }
    for x in [-42., -21., 0., 21., 42.] {
        for side in [-1., 1.] {
            part(
                root,
                Vec3::new(x, -20., side * 24.),
                Vec3::new(17., 9., 5.),
                a.belly.clone(),
            );
            for y in [-6., 10.] {
                part(
                    root,
                    Vec3::new(x + if y > 0. { 6. } else { 0. }, y, side * 24.),
                    Vec3::new(14., 9., 4.),
                    a.scales.clone(),
                );
            }
        }
        part(
            root,
            Vec3::new(x, 30., 0.),
            Vec3::new(9., 19., 8.),
            a.bone.clone(),
        );
    }
    // Brow ridges, nostrils and a tapered snout keep the face readable at game size.
    part(
        root,
        Vec3::new(-116., 3., 0.),
        Vec3::new(28., 22., 31.),
        a.skin.clone(),
    );
    for side in [-1., 1.] {
        part(
            root,
            Vec3::new(-98., 23., side * 22.),
            Vec3::new(24., 6., 5.),
            a.scales.clone(),
        );
        part(
            root,
            Vec3::new(-126., 8., side * 17.),
            Vec3::new(5., 4., 3.),
            a.eyes.clone(),
        );
        for x in [-112., -97., -82.] {
            part(
                root,
                Vec3::new(x, -10., side * 18.),
                Vec3::new(4., 7., 4.),
                a.bone.clone(),
            );
        }
    }
    for side in [-1., 1.] {
        part(
            root,
            Vec3::new(-65., 50., side * 16.),
            Vec3::new(5., 12., 5.),
            a.bone.clone(),
        );
    }
    drop(part);
    commands.entity(chest).insert(Chest);
    let jaw = commands
        .spawn((
            ChildOf(neck),
            Joint {
                owner: root,
                phase: 0.,
                amplitude: 0.18,
                axis: Vec3::Z,
                base: Quat::from_rotation_z(0.),
                motion: JointMotion::Jaw,
            },
            Transform::from_translation(Vec3::new(-57., -8., 0.) - neck_position),
            Visibility::Inherited,
        ))
        .id();
    commands.spawn((
        ChildOf(jaw),
        Mesh3d(a.cube.clone()),
        MeshMaterial3d(a.skin.clone()),
        Transform::from_xyz(-29., -5., 0.).with_scale(Vec3::new(64., 11., 34.)),
    ));
    for side in [-1., 1.] {
        let wing = commands
            .spawn((
                ChildOf(root),
                Joint {
                    owner: root,
                    phase: 0.,
                    amplitude: 0.55 * side,
                    axis: Vec3::X,
                    base: Quat::from_rotation_z(0.),
                    motion: JointMotion::Wing,
                },
                Transform::from_xyz(4., 9., side * 18.),
                Visibility::Inherited,
            ))
            .id();
        commands.spawn((
            ChildOf(wing),
            Mesh3d(a.cube.clone()),
            MeshMaterial3d(a.skin.clone()),
            Transform::from_xyz(31., side * 65., 0.)
                .with_rotation(Quat::from_rotation_z(-side * 0.35))
                .with_scale(Vec3::new(9., 140., 8.)),
        ));
        commands.spawn((
            ChildOf(wing),
            DragonHitPart {
                owner: root,
                half: Vec2::new(-1., -1.),
            },
            Mesh3d(a.wing.clone()),
            MeshMaterial3d(a.membrane.clone()),
            Transform::from_xyz(10., 0., 0.).with_scale(Vec3::new(1., side, 1.)),
        ));
        for tip in [
            Vec3::new(116., 107. * side, 1.),
            Vec3::new(104., 48. * side, 1.),
            Vec3::new(82., 10. * side, 1.),
        ] {
            let from = Vec3::new(55., 130. * side, 1.);
            let delta = tip - from;
            commands.spawn((
                ChildOf(wing),
                Mesh3d(a.cube.clone()),
                MeshMaterial3d(a.scales.clone()),
                Transform::from_translation((from + tip) * 0.5)
                    .with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x)))
                    .with_scale(Vec3::new(delta.length(), 4., 4.)),
            ));
        }
        // Fingered outer wing and independently delayed tip create a folding silhouette.
        let tip = commands
            .spawn((
                ChildOf(wing),
                Joint {
                    owner: root,
                    phase: 0.65,
                    amplitude: 0.25 * side,
                    axis: Vec3::Z,
                    base: Quat::from_rotation_z(0.),
                    motion: JointMotion::Tip,
                },
                Transform::from_xyz(55., side * 130., 0.),
                Visibility::Inherited,
            ))
            .id();
        commands.spawn((
            ChildOf(tip),
            Mesh3d(a.wing.clone()),
            MeshMaterial3d(a.membrane.clone()),
            Transform::from_xyz(0., 0., 0.).with_scale(Vec3::new(0.6, side * 0.35, 1.)),
        ));
        for x in [-25., 30.] {
            let leg = commands
                .spawn((
                    ChildOf(root),
                    Joint {
                        owner: root,
                        phase: x * 0.02,
                        amplitude: 0.14,
                        axis: Vec3::Z,
                        base: Quat::from_rotation_z(0.),
                        motion: JointMotion::Leg,
                    },
                    Transform::from_xyz(x, -29., side * 17.),
                    Visibility::Inherited,
                ))
                .id();
            commands.spawn((
                ChildOf(leg),
                Mesh3d(a.cube.clone()),
                MeshMaterial3d(a.skin.clone()),
                Transform::from_xyz(5., -9., 0.).with_scale(Vec3::new(14., 34., 12.)),
            ));
            for toe in [-1., 0., 1.] {
                commands.spawn((
                    ChildOf(leg),
                    Mesh3d(a.cube.clone()),
                    MeshMaterial3d(a.bone.clone()),
                    Transform::from_xyz(-5., -26., toe * 5.).with_scale(Vec3::new(17., 5., 4.)),
                ));
            }
        }
    }
    let mut tail = root;
    for n in 0..6 {
        tail = commands
            .spawn((
                ChildOf(tail),
                Joint {
                    owner: root,
                    phase: n as f32 * 0.55,
                    amplitude: 0.15,
                    axis: Vec3::Z,
                    base: Quat::from_rotation_z(0.),
                    motion: JointMotion::Tail,
                },
                Transform::from_xyz(if n == 0 { 57. } else { 27. }, 0., 0.),
                Visibility::Inherited,
            ))
            .id();
        commands.spawn((
            ChildOf(tail),
            Mesh3d(a.cube.clone()),
            MeshMaterial3d(a.bone.clone()),
            Transform::from_xyz(13., 19. - n as f32 * 1.8, 0.)
                .with_rotation(Quat::from_rotation_z(-0.35))
                .with_scale(Vec3::new(6., 15. - n as f32, 5.)),
        ));
        commands.spawn((
            ChildOf(tail),
            DragonHitPart {
                owner: root,
                half: Vec2::splat(0.5),
            },
            Mesh3d(a.cube.clone()),
            MeshMaterial3d(a.skin.clone()),
            Transform::from_xyz(13., 0., 0.).with_scale(Vec3::new(
                30.,
                26. - n as f32 * 3.,
                26. - n as f32 * 3.,
            )),
        ));
    }
}

pub(super) fn animate(
    time: Res<Time>,
    mut joints: Query<(&Joint, &mut Transform), Without<Chest>>,
    dragons: Query<&EndDragon>,
    mut chests: Query<&mut Transform, With<Chest>>,
) {
    for (j, mut t) in &mut joints {
        let Ok(d) = dragons.get(j.owner) else {
            continue;
        };
        if matches!(j.motion, JointMotion::Neck) {
            t.rotation = neck_rotation(d);
            continue;
        }
        let resting = (11. ..15.).contains(&d.phase) && d.perch.is_some();
        let breathing = (8. ..11.).contains(&d.phase);
        let effort = (d.yaw_velocity / 3.).clamp(-1., 1.);
        let side = j.amplitude.signum();
        let (frequency, amount, offset) = match j.motion {
            JointMotion::Neck => unreachable!(),
            JointMotion::Jaw => (
                2.1,
                if breathing { 0.12 } else { 0.025 },
                if breathing { 0.28 } else { 0. },
            ),
            JointMotion::Wing => (
                4.5,
                if resting {
                    j.amplitude * 0.05
                } else {
                    j.amplitude * (1. - 0.4 * (effort * side).max(0.))
                },
                if resting { side * 0.70 } else { effort * 0.35 },
            ),
            JointMotion::Tip => (
                4.5,
                if resting { 0.015 } else { j.amplitude },
                if resting {
                    side * 0.32
                } else {
                    side * effort.abs() * 0.3
                },
            ),
            JointMotion::Leg => (
                2.3,
                if resting { 0.025 } else { j.amplitude },
                if resting {
                    -0.10
                } else {
                    0.35 + 0.18 * effort.abs()
                },
            ),
            JointMotion::Tail => (
                2.6,
                if resting {
                    j.amplitude * 0.3
                } else {
                    j.amplitude
                },
                0.,
            ),
        };
        let mut pose = j.base
            * Quat::from_axis_angle(
                j.axis,
                offset + (time.elapsed_secs() * frequency + j.phase).sin() * amount,
            );
        if matches!(j.motion, JointMotion::Tail) {
            pose = Quat::from_rotation_y(-effort * 0.18) * pose;
            // Each successive vertebra responds more slowly, leaving a trailing
            // curve during reversal and a gradual settling motion afterwards.
            t.rotation = t
                .rotation
                .slerp(pose, 1. - (-8. * time.delta_secs() / (1. + j.phase)).exp());
        } else {
            t.rotation = t
                .rotation
                .slerp(pose, 1. - (-16. * time.delta_secs()).exp());
        }
    }
    for mut t in &mut chests {
        t.scale.y = 46. * (1. + 0.025 * (time.elapsed_secs() * 2.2).sin());
    }
}

pub(super) fn wing_mesh() -> Mesh {
    // A swept leading edge and scalloped trailing edge keep the silhouette readable.
    let outline: Vec<[f32; 3]> = vec![
        [0., 0., 0.],
        [55., 130., 0.],
        [116., 107., 0.],
        [86., 83., 0.],
        [95., 66., 0.],
        [65., 47., 0.],
        [69., 28., 0.],
        [40., 19., 0.],
        [48., 0., 0.],
    ];
    let mut indices = Vec::new();
    for n in 1..outline.len() - 1 {
        indices.extend([0, n as u32, (n + 1) as u32]);
    }
    let uv: Vec<[f32; 2]> = outline.iter().map(|p| [p[0] / 116., p[1] / 130.]).collect();
    let count = outline.len();
    Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, outline)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 0., 1.]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}

pub(super) fn boss_label(
    dragons: Query<(&EndDragon, &Health)>,
    mut labels: Query<&mut Text, With<BossLabel>>,
) {
    for mut label in &mut labels {
        **label = if let Some((dragon, health)) = dragons.iter().next() {
            let action = if health.current == 0 {
                "DEFEATED"
            } else if (6. ..8.).contains(&dragon.phase) {
                "INHALING"
            } else if (8. ..11.).contains(&dragon.phase) {
                "FIRE BREATH"
            } else if (11. ..15.).contains(&dragon.phase) && dragon.perch.is_some() {
                "LANDING"
            } else {
                "CIRCLING"
            };
            format!(
                "ENDER DRAGON  {}/{}  {}",
                health.current, health.max, action
            )
        } else {
            String::new()
        };
    }
}
