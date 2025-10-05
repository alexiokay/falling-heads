use bevy::prelude::*;
use crate::components::{
    block::Block,
    explosive::{Tnt, Explosion},
    physics::Velocity,
    projectile::{Laser, Arrow},
    hero::Hero,
    villager::Villager,
    enemy::Enemy,
};
use crate::events::damage_events::EntityDamaged;
use crate::resources::asset_handles::GameTextures;
use crate::states::GameState;
use crate::utils::constants::*;

pub struct PowersPlugin;

impl Plugin for PowersPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                tnt_countdown,
                tnt_explosion,
                laser_update,
            ).run_if(in_state(GameState::Playing)));
            // arrow_update REMOVED - handled by projectile_system with spatial hash instead

        info!("PowersPlugin loaded");
    }
}

// TNT spawning will be triggered by API
pub fn spawn_tnt(
    commands: &mut Commands,
    position: Vec3,
    textures: &GameTextures,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    use crate::components::physics::Gravity;

    // Spawn TNT as 3D cube with corrected texture orientation
    commands.spawn((
        Tnt {
            timer: Timer::from_seconds(TNT_FUSE_TIME, TimerMode::Once),
        },
        Mesh3d(meshes.add(Cuboid::new(BLOCK_SIZE, BLOCK_SIZE, BLOCK_SIZE))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(textures.tnt.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()  // Use default cull_mode (back-face culling)
        })),
        Transform::from_translation(position)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::PI)),  // Rotate 180° on X to flip texture upright
        Velocity { x: 0.0, y: 0.0 },
        Gravity { force: -500.0 },
        // NO Bouncy component - TNT doesn't bounce
    ));

    info!("TNT spawned at {:?}", position);
}

fn tnt_countdown(
    time: Res<Time>,
    mut commands: Commands,
    mut tnt_query: Query<(Entity, &mut Tnt, &Transform, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut tnt, transform, material_handle) in tnt_query.iter_mut() {
        tnt.timer.tick(time.delta());

        // Get how much time is left (0.0 to 1.0, where 0.0 = about to explode)
        let time_remaining = tnt.timer.fraction_remaining();

        // Blink faster as it gets closer to exploding
        // At start (time_remaining=1.0): blink every 0.6 seconds
        // Near end (time_remaining=0.0): blink every 0.2 seconds
        let blink_speed = 0.2 + (time_remaining * 0.4); // Range: 0.6 to 0.2
        let blink_phase = (tnt.timer.elapsed_secs() / blink_speed) % 1.0;

        // Flash white when blink_phase > 0.5
        if let Some(material) = materials.get_mut(&material_handle.0) {
            if blink_phase > 0.5 {
                // Flash bright white
                material.base_color = Color::srgb(2.0, 2.0, 2.0); // Brighter than white
            } else {
                // Normal color
                material.base_color = Color::WHITE;
            }
        }

        if tnt.timer.is_finished() {
            // Explode!
            commands.spawn((
                Explosion { radius: TNT_EXPLOSION_RADIUS },
                Transform::from_translation(transform.translation),
            ));

            commands.entity(entity).despawn();
            info!("TNT exploded at {:?}", transform.translation);
        }
    }
}

fn tnt_explosion(
    mut commands: Commands,
    explosion_query: Query<(Entity, &Explosion, &Transform), Added<Explosion>>,
    block_query: Query<(Entity, &Transform), With<Block>>,
    mut entity_query: Query<(Entity, &Transform, &mut Velocity), Or<(With<Hero>, With<Villager>, With<Enemy>)>>,
    mut damage_events: MessageWriter<EntityDamaged>,
) {
    for (explosion_entity, explosion, explosion_transform) in explosion_query.iter() {
        let explosion_pos = explosion_transform.translation;

        // Destroy all blocks in radius
        for (block_entity, block_transform) in block_query.iter() {
            let distance = explosion_pos.distance(block_transform.translation);

            if distance < explosion.radius {
                commands.entity(block_entity).despawn();
            }
        }

        // Push all entities away from explosion and deal damage
        let push_blocks = 3.0; // Push entities 3 blocks away
        for (entity, entity_transform, mut entity_velocity) in entity_query.iter_mut() {
            let entity_pos = entity_transform.translation;
            let distance = explosion_pos.distance(entity_pos);

            if distance < explosion.radius && distance > 0.0 {
                // Calculate push direction (away from explosion)
                let direction = (entity_pos - explosion_pos).normalize();

                // Push strength decreases with distance (inverse square law)
                let push_strength = 800.0 * (1.0 - distance / explosion.radius);

                // Apply push velocity
                entity_velocity.x += direction.x * push_strength;
                entity_velocity.y += direction.y * push_strength;

                // Deal damage
                damage_events.write(EntityDamaged {
                    entity,
                    damage: TNT_DAMAGE,
                });

                info!("Explosion pushed entity at distance {} with strength {} and dealt {} damage", distance, push_strength, TNT_DAMAGE);
            }
        }

        info!("Explosion destroyed blocks in radius {}", explosion.radius);

        // Clean up explosion entity immediately
        commands.entity(explosion_entity).despawn();
    }
}

// LASER spawning - fires downward from hero
pub fn spawn_laser(
    commands: &mut Commands,
    position: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Laser {
            timer: Timer::from_seconds(LASER_DURATION, TimerMode::Once),
            width: LASER_WIDTH,
        },
        Mesh3d(meshes.add(Rectangle::new(LASER_WIDTH, LASER_HEIGHT))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.0, 1.0, 0.7), // Purple semi-transparent
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_translation(position + Vec3::new(0.0, -LASER_HEIGHT / 2.0, 5.0)), // Z=5 for visibility
    ));

    info!("LASER spawned at {:?}", position);
}

fn laser_update(
    time: Res<Time>,
    mut commands: Commands,
    mut laser_query: Query<(Entity, &mut Laser, &Transform)>,
    block_query: Query<(Entity, &Transform), With<Block>>,
) {
    for (laser_entity, mut laser, laser_transform) in laser_query.iter_mut() {
        laser.timer.tick(time.delta());

        // Destroy blocks that overlap with laser on first frame
        if laser.timer.elapsed_secs() < 0.05 {
            let laser_pos = laser_transform.translation;
            let laser_half_width = laser.width / 2.0;

            for (block_entity, block_transform) in block_query.iter() {
                let block_pos = block_transform.translation;
                let block_half = 32.0; // 3D scale: Block size / 2 (64/2)

                // Check if block is within laser's horizontal range and below hero
                if block_pos.x > laser_pos.x - laser_half_width - block_half
                    && block_pos.x < laser_pos.x + laser_half_width + block_half
                    && block_pos.y < laser_pos.y + 1000.0 // 3D scale: Below the laser origin
                {
                    commands.entity(block_entity).despawn();
                }
            }
        }

        if laser.timer.is_finished() {
            commands.entity(laser_entity).despawn();
            info!("LASER expired");
        }
    }
}

// ARROW spawning - circular barrage around hero
pub fn spawn_arrows(
    commands: &mut Commands,
    position: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let num_arrows = 8;
    let speed = 300.0;  // 3D scale

    for i in 0..num_arrows {
        let angle = (i as f32 / num_arrows as f32) * std::f32::consts::TAU;

        commands.spawn((
            Arrow::new(1), // 1 damage per arrow
            Mesh3d(meshes.add(Rectangle::new(16.0, 48.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.4, 0.2),
                unlit: true,
                ..default()
            })),
            Transform::from_translation(position + Vec3::new(0.0, 0.0, 5.0)), // Z=5 for visibility
            Velocity {
                x: angle.cos() * speed,
                y: angle.sin() * speed,
            },
            crate::components::physics::Gravity { force: -300.0 }, // Arrows have gravity
        ));
    }

    info!("ARROW barrage spawned with {} arrows", num_arrows);
}

fn arrow_update(
    mut commands: Commands,
    arrow_query: Query<(Entity, &Transform, &Velocity), With<Arrow>>,
    block_query: Query<(Entity, &Transform), With<Block>>,
) {
    for (arrow_entity, arrow_transform, _velocity) in arrow_query.iter() {
        let arrow_pos = arrow_transform.translation;

        // Check collision with blocks
        for (block_entity, block_transform) in block_query.iter() {
            let distance = arrow_pos.distance(block_transform.translation);

            if distance < 64.0 {  // 3D scale: block size
                commands.entity(block_entity).despawn();
                commands.entity(arrow_entity).despawn();
                break;
            }
        }

        // Despawn arrows that go too far (3D scale)
        if arrow_pos.y < -2000.0 || arrow_pos.y > 2000.0
            || arrow_pos.x < -2000.0 || arrow_pos.x > 2000.0 {
            commands.entity(arrow_entity).despawn();
        }
    }
}
