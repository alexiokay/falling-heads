use bevy::prelude::*;
use crate::components::{
    block::Block,
    explosive::{Tnt, Explosion},
    physics::Velocity,
    projectile::{Laser, Arrow},
};
use crate::resources::asset_handles::GameTextures;
use crate::states::GameState;

pub struct PowersPlugin;

impl Plugin for PowersPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, tnt_countdown.run_if(in_state(GameState::Playing)))
            .add_systems(Update, tnt_explosion.run_if(in_state(GameState::Playing)))
            .add_systems(Update, laser_update.run_if(in_state(GameState::Playing)))
            .add_systems(Update, arrow_update.run_if(in_state(GameState::Playing)));

        info!("PowersPlugin loaded");
    }
}

// TNT spawning will be triggered by API
pub fn spawn_tnt(
    commands: &mut Commands,
    position: Vec3,
    textures: &GameTextures,
) {
    use crate::components::physics::{Gravity, Bouncy};

    commands.spawn((
        Tnt {
            timer: Timer::from_seconds(3.0, TimerMode::Once),
        },
        Sprite {
            image: textures.tnt.clone(),
            custom_size: Some(Vec2::new(32.0, 32.0)),  // 2D scale
            ..default()
        },
        Transform::from_translation(position),
        Velocity { x: 0.0, y: 0.0 },
        Gravity { force: -500.0 },  // 2D scale
        Bouncy { restitution: 0.5 },
    ));

    info!("TNT spawned at {:?}", position);
}

fn tnt_countdown(
    time: Res<Time>,
    mut commands: Commands,
    mut tnt_query: Query<(Entity, &mut Tnt, &Transform)>,
) {
    for (entity, mut tnt, transform) in tnt_query.iter_mut() {
        tnt.timer.tick(time.delta());

        if tnt.timer.is_finished() {
            // Explode!
            commands.spawn((
                Explosion { radius: 150.0 },  // 2D scale
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

        info!("Explosion destroyed blocks in radius {}", explosion.radius);

        // Clean up explosion entity immediately
        commands.entity(explosion_entity).despawn();
    }
}

// LASER spawning - fires downward from hero
pub fn spawn_laser(commands: &mut Commands, position: Vec3) {
    let width = 5.0; // 3D scale: 5 blocks wide
    let height = 30.0; // 3D scale: tall laser beam

    commands.spawn((
        Laser {
            timer: Timer::from_seconds(0.5, TimerMode::Once),
            width,
        },
        Sprite {
            color: Color::srgba(1.0, 0.0, 1.0, 0.7), // Purple semi-transparent
            custom_size: Some(Vec2::new(width, height)),
            ..default()
        },
        Transform::from_translation(position + Vec3::new(0.0, -height / 2.0, 0.0)),
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
                let block_half = 0.5; // 3D scale: Block size / 2

                // Check if block is within laser's horizontal range and below hero
                if block_pos.x > laser_pos.x - laser_half_width - block_half
                    && block_pos.x < laser_pos.x + laser_half_width + block_half
                    && block_pos.y < laser_pos.y + 15.0 // 3D scale: Below the laser origin
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
pub fn spawn_arrows(commands: &mut Commands, position: Vec3) {
    let num_arrows = 12;
    let speed = 5.0;  // 3D scale

    for i in 0..num_arrows {
        let angle = (i as f32 / num_arrows as f32) * std::f32::consts::TAU;

        commands.spawn((
            Arrow { angle, speed },
            Sprite {
                color: Color::srgb(0.6, 0.4, 0.2),
                custom_size: Some(Vec2::new(0.2, 0.5)),  // 3D scale
                ..default()
            },
            Transform::from_translation(position),
            Velocity {
                x: angle.cos() * speed,
                y: angle.sin() * speed,
            },
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

            if distance < 0.7 {  // 3D scale
                commands.entity(block_entity).despawn();
                commands.entity(arrow_entity).despawn();
                break;
            }
        }

        // Despawn arrows that go too far (3D scale)
        if arrow_pos.y < -30.0 || arrow_pos.y > 30.0
            || arrow_pos.x < -30.0 || arrow_pos.x > 30.0 {
            commands.entity(arrow_entity).despawn();
        }
    }
}
