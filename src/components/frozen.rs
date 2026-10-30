use bevy::prelude::*;

/// Frozen debuff - stacks up to 6 times, each snowball hit adds 1 stack
/// When reaching 6 stacks, entity becomes fully frozen
#[derive(Component)]
pub struct FrozenStacks {
    pub stacks: Vec<FrozenStack>, // Each snowball hit adds one stack
}

impl FrozenStacks {
    pub fn new() -> Self {
        Self { stacks: Vec::new() }
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
}

/// Single frozen stack from one snowball hit
pub struct FrozenStack {
    pub timer: Timer, // 12 seconds per stack
}
