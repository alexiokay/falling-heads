use crate::components::death_particle::DeathParticle;
use crate::events::damage_events::EntityDied;
use bevy::prelude::*;
use rand::RngExt;

pub struct DeathParticlePlugin;

impl Plugin for DeathParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_death_particles, update_death_particles).run_if(in_state(crate::states::InGame)),
        );

        info!("DeathParticlePlugin loaded");
    }
}

/// Spawn death particles when entities die - uses actual character texture pieces
fn spawn_death_particles(
    mut commands: Commands,
    mut death_events: MessageReader<EntityDied>,
    entity_query: Query<(
        &Transform,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&crate::components::damage_visual::DamageVisual>,
        Option<&Mesh3d>,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for event in death_events.read() {
        // Get entity info
        let Ok((transform, material_handle, damage_visual, face_mesh)) =
            entity_query.get(event.entity)
        else {
            continue;
        };

        // Get the actual material to clone its texture
        // Use original color from DamageVisual if available (prevents red-tinted particles)
        let entity_material = if let Some(material_3d) = material_handle {
            if let Some(mut material) = materials.get(&material_3d.0).cloned() {
                // If entity has damage visual, use the stored original color instead of current red-tinted color
                if let Some(dmg_visual) = damage_visual {
                    material.base_color = dmg_visual.original_color;
                }
                Some(material)
            } else {
                None
            }
        } else {
            None
        };

        let face = super::head_geometry::bounds(&meshes, face_mesh);
        let dimensions = super::head_geometry::effect_scale(face, transform);
        let area_scale = dimensions.x.min(dimensions.y).sqrt().clamp(0.25, 2.);
        // Spawn 8-12 particles flying in different directions
        let mut rng = rand::rng();
        let particle_count = rng.random_range(8..=12);

        for _ in 0..particle_count {
            // Random velocity in all directions (explosion effect)
            let velocity = Vec3::new(
                rng.random_range(-300.0..300.0) * area_scale,
                rng.random_range(100.0..400.0) * area_scale, // Bias upward
                rng.random_range(-50.0..50.0),
            );

            // Random rotation velocity for spinning effect (in radians/sec)
            let rotation_velocity = Vec3::new(
                rng.random_range(-5.0..5.0), // X-axis rotation
                rng.random_range(-5.0..5.0), // Y-axis rotation
                rng.random_range(-5.0..5.0), // Z-axis rotation
            );

            // Random particle size with more variation (6-18 units)
            let size_base = rng.random_range(6.0..18.0);
            // Add slight non-uniform scaling for more natural look
            let size_x = size_base * rng.random_range(0.8..1.2) * dimensions.x;
            let size_y = size_base * rng.random_range(0.8..1.2) * dimensions.y;
            let size_z = size_base * rng.random_range(0.8..1.2) * dimensions.x.min(dimensions.y);

            // Create particle mesh (small cube with slight size variation)
            let particle_mesh = meshes.add(Cuboid::new(size_x, size_y, size_z));

            // Use the actual character's texture on the particle
            let particle_material = if let Some(mat) = &entity_material {
                materials.add(StandardMaterial {
                    base_color: mat.base_color,
                    base_color_texture: mat.base_color_texture.clone(), // Use actual texture!
                    unlit: true,                                        // Particles glow
                    alpha_mode: mat.alpha_mode,
                    ..default()
                })
            } else {
                // Fallback to gray if no material
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.5, 0.5, 0.5),
                    unlit: true,
                    ..default()
                })
            };

            // Random initial rotation
            let initial_rotation = Quat::from_euler(
                EulerRot::XYZ,
                rng.random_range(0.0..std::f32::consts::TAU),
                rng.random_range(0.0..std::f32::consts::TAU),
                rng.random_range(0.0..std::f32::consts::TAU),
            );

            // Spawn particle at death location with random rotation
            commands.spawn((
                DeathParticle::new(velocity, rotation_velocity, rng.random_range(1.5..2.5)),
                Mesh3d(particle_mesh),
                MeshMaterial3d(particle_material),
                Transform::from_translation(transform.transform_point(
                    face.center
                        + Vec3::new(
                            rng.random_range(-0.4..0.4) * face.size.x,
                            rng.random_range(-0.4..0.4) * face.size.y,
                            face.size.z * 0.5,
                        ),
                ))
                .with_rotation(initial_rotation),
            ));
        }

        info!(
            "💀 Death particles spawned for entity {:?} with actual texture pieces",
            event.entity
        );
    }
}

/// Update death particle positions and lifetime
fn update_death_particles(
    mut commands: Commands,
    mut particle_query: Query<(Entity, &mut Transform, &mut DeathParticle)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut particle) in particle_query.iter_mut() {
        // Apply velocity
        transform.translation += particle.velocity * time.delta_secs();

        // Apply rotation (spinning)
        let delta_rotation = Quat::from_euler(
            EulerRot::XYZ,
            particle.rotation_velocity.x * time.delta_secs(),
            particle.rotation_velocity.y * time.delta_secs(),
            particle.rotation_velocity.z * time.delta_secs(),
        );
        transform.rotation = transform.rotation * delta_rotation;

        // Apply gravity
        particle.velocity.y -= 800.0 * time.delta_secs();

        // Update lifetime
        particle.lifetime.tick(time.delta());

        // Despawn if expired
        if particle.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
