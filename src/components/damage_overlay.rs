use bevy::prelude::*;

/// Marks the child entity that displays the damage crack overlay on a block
/// The parent entity is the block itself (tracked by Bevy's Parent component)
#[derive(Component)]
pub struct BlockDamageOverlay;

/// Calculate which damage stage texture to use based on health
pub fn get_damage_stage(current_health: u32, max_health: u32) -> usize {
    if current_health >= max_health {
        return 0; // No damage - we'll hide the overlay
    }

    if current_health == 0 {
        return 8; // Maximum damage (about to break)
    }

    // Map health percentage to damage stage (0-8)
    let health_percent = current_health as f32 / max_health as f32;
    let stage = ((1.0 - health_percent) * 9.0) as usize;
    stage.min(8) // Cap at stage 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_stages() {
        // Block with 3 health
        assert_eq!(get_damage_stage(3, 3), 0); // 100% = no cracks
        assert_eq!(get_damage_stage(2, 3), 3); // 67% = medium cracks
        assert_eq!(get_damage_stage(1, 3), 6); // 33% = heavy cracks
        assert_eq!(get_damage_stage(0, 3), 8); // 0% = max cracks

        // Block with 5 health
        assert_eq!(get_damage_stage(5, 5), 0); // 100%
        assert_eq!(get_damage_stage(4, 5), 1); // 80%
        assert_eq!(get_damage_stage(3, 5), 3); // 60%
        assert_eq!(get_damage_stage(2, 5), 5); // 40%
        assert_eq!(get_damage_stage(1, 5), 7); // 20%

        // Block with 10 health
        assert_eq!(get_damage_stage(10, 10), 0); // 100%
        assert_eq!(get_damage_stage(5, 10), 4);  // 50%
        assert_eq!(get_damage_stage(1, 10), 8);  // 10%
    }
}
