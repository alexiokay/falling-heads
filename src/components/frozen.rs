use bevy::prelude::*;

/// Frozen debuff - stacks up to 6 times, each snowball hit adds 1 stack
/// When reaching 6 stacks, entity becomes fully frozen
#[derive(Component)]
pub struct FrozenStacks {
    pub stacks: Vec<FrozenStack>,  // Each snowball hit adds one stack
}

impl FrozenStacks {
    pub fn new() -> Self {
        Self {
            stacks: Vec::new(),
        }
    }

    /// Add a snowball hit (adds 12 second stack)
    pub fn add_stack(&mut self) {
        if self.stacks.len() < 6 {
            self.stacks.push(FrozenStack {
                timer: Timer::from_seconds(12.0, TimerMode::Once),
            });
        }
    }

    /// Get current number of stacks
    pub fn stack_count(&self) -> usize {
        self.stacks.len()
    }

    /// Check if fully frozen (6 stacks)
    pub fn is_fully_frozen(&self) -> bool {
        self.stack_count() >= 6
    }

    /// Get freeze intensity (0.0 to 1.0)
    pub fn freeze_intensity(&self) -> f32 {
        (self.stack_count() as f32 / 6.0).min(1.0)
    }

    /// Get movement speed multiplier based on freeze level
    pub fn speed_multiplier(&self) -> f32 {
        if self.is_fully_frozen() {
            0.0  // Completely frozen at 6 stacks, can't move
        } else {
            1.0  // No slowdown until fully frozen
        }
    }

    /// Get overlay opacity for visual feedback (0.0 to 1.0)
    pub fn overlay_opacity(&self) -> f32 {
        self.freeze_intensity() * 0.6  // Max 60% opacity
    }
}

/// Single frozen stack from one snowball hit
pub struct FrozenStack {
    pub timer: Timer,  // 12 seconds per stack
}

/// Visual overlay for frozen effect
#[derive(Component)]
pub struct FreezeOverlay {
    pub parent_entity: Entity,
    pub stage: usize,  // 0-5 (6 stages total)
}
