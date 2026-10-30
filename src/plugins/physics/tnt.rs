use super::*;

/// TNT vs Blocks collision - simpler version that just stops TNT when it hits blocks
pub(super) fn check_tnt_block_collisions(
    time: Res<Time>,
    spatial_hash: Res<SpatialHash>,
    mut tnt_query: Query<(&mut Transform, &mut LinearVelocity), With<Tnt>>,
    block_query: Query<(&Transform, Option<&SnowLayer>), (With<Block>, Without<Tnt>)>,
) {
    let tnt_half = BLOCK_SIZE / 2.0;
    // Ground friction as a per-second decay (0.9 per frame at 60 FPS)
    let friction = 0.9_f32.powf(time.delta_secs() * 60.0);
    for (mut tnt_transform, mut tnt_velocity) in tnt_query.iter_mut() {
        // Only blocks near this TNT, not every block in the world
        let reach = Vec2::splat(tnt_half + BLOCK_SIZE);
        let center = tnt_transform.translation.xy();
        let mut grounded = false;
        for block_entity in spatial_hash.rect_iter(center - reach, center + reach) {
            let Ok((block_transform, snow)) = block_query.get(block_entity) else {
                continue;
            };
            let (block_pos, block_half_size) =
                block_contact_bounds(block_transform.translation, snow);

            // Use the position after earlier pushes so two neighbouring blocks
            // don't both push the TNT out by the full overlap.
            let tnt_pos = tnt_transform.translation;
            let dx = tnt_pos.x - block_pos.x;
            let dy = tnt_pos.y - block_pos.y;

            let overlap_x = tnt_half + block_half_size.x - dx.abs();
            let overlap_y = tnt_half + block_half_size.y - dy.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                // Collision resolution
                if overlap_x < overlap_y {
                    // Horizontal
                    let separation = overlap_x + 0.5;
                    if dx > 0.0 {
                        tnt_transform.translation.x += separation;
                    } else {
                        tnt_transform.translation.x -= separation;
                    }
                    tnt_velocity.x = 0.0;
                } else {
                    // Vertical
                    let separation = overlap_y + 0.5;
                    if dy > 0.0 {
                        tnt_transform.translation.y += separation;
                        if tnt_velocity.y < 0.0 {
                            tnt_velocity.y = 0.0;
                        }
                        grounded = true;
                    } else {
                        tnt_transform.translation.y -= separation;
                        if tnt_velocity.y > 0.0 {
                            tnt_velocity.y = 0.0;
                        }
                    }
                }
            }
        }
        if grounded {
            tnt_velocity.x *= friction;
        }
    }
}
