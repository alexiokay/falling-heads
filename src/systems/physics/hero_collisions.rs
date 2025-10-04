use bevy::prelude::*;
use crate::components::physics::{Velocity, Bouncy};
use crate::components::hero::Hero;
use crate::components::block::{Block, BlockType};
use crate::components::chunk::ChunkMember;
use crate::events::block_events::BlockDamaged;

/// Hero vs Block collision detection and resolution
pub fn check_hero_block_collisions(
    mut hero_query: Query<(Entity, &mut Transform, &mut Velocity, &Bouncy), (With<Hero>, Without<Block>)>,
    block_query: Query<(Entity, &Transform), (With<Block>, Without<Hero>)>,
    mut damage_events: MessageWriter<BlockDamaged>,
) {
    let Ok((hero_entity, mut hero_transform, mut hero_velocity, bouncy)) = hero_query.single_mut() else {
        return;
    };

    let hero_pos = hero_transform.translation;
    let hero_size = 64.0;
    let block_size = 64.0;

    for (block_entity, block_transform) in block_query.iter() {
        let block_pos = block_transform.translation;

        let hero_half = hero_size / 2.0;
        let block_half = block_size / 2.0;

        let dx = hero_pos.x - block_pos.x;
        let dy = hero_pos.y - block_pos.y;

        let overlap_x = hero_half + block_half - dx.abs();
        let overlap_y = hero_half + block_half - dy.abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            // Collision detected - damage block
            damage_events.write(BlockDamaged {
                entity: block_entity,
                damage: 1,
                remaining_health: 0,
            });

            // Resolve collision based on smallest overlap
            if overlap_x < overlap_y {
                // Horizontal resolution
                let separation = overlap_x + 1.0;
                if dx > 0.0 {
                    hero_transform.translation.x += separation;
                } else {
                    hero_transform.translation.x -= separation;
                }
                hero_velocity.x = -hero_velocity.x * 0.5;
            } else {
                // Vertical resolution
                let separation = overlap_y + 1.0;
                if dy > 0.0 {
                    hero_transform.translation.y += separation;
                    if hero_velocity.y < 0.0 {
                        let max_bounce_velocity = 400.0;
                        hero_velocity.y = -hero_velocity.y * bouncy.restitution;
                        hero_velocity.y = hero_velocity.y.min(max_bounce_velocity);
                    }
                } else {
                    hero_transform.translation.y -= separation;
                    if hero_velocity.y > 0.0 {
                        hero_velocity.y = -hero_velocity.y * bouncy.restitution * 0.3;
                    }
                }
            }
        }
    }
}
