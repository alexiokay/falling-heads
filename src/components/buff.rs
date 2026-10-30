use bevy::prelude::*;

/// Trait for buff/debuff behavior - defines how status effects work
pub trait BuffBehavior {
    /// Get the display name
    fn name(&self) -> &str;

    /// Get the visual overlay color
    fn overlay_color(&self) -> Color;

    /// Get damage dealt per tick (negative = healing)
    fn damage_per_tick(&self) -> i32;

    /// Get movement speed multiplier
    fn velocity_multiplier(&self) -> f32;

    /// Get scale multiplier per stack (1.0 = no change, 1.2 = 20% bigger)
    fn scale_multiplier(&self) -> f32 {
        1.0 // Default: no scale change
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BuffType {
    Burning,
    Freezing,
    Poisoned,
    Speed,
    Slowness,
    Grow, // Increases hero size by 20% per stack (max 2 stacks = 40% bigger)
}

#[derive(Component)]
pub struct Buff {
    pub buff_type: BuffType,
    pub duration: Timer,
    pub tick_timer: Timer, // For damage/healing over time
    pub stacks: u8,        // Number of stacks (1-3)
}

impl Buff {
    pub fn new(buff_type: BuffType, duration_secs: f32) -> Self {
        Self {
            buff_type,
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
            tick_timer: Timer::from_seconds(0.5, TimerMode::Repeating), // Tick every 0.5s
            stacks: 1,
        }
    }

    /// Maximum stacks allowed per buff type
    pub const MAX_STACKS: u8 = 3;

    /// Add a stack (up to MAX_STACKS or buff-specific max) and refresh duration
    pub fn add_stack(&mut self, duration_secs: f32) {
        let max_stacks = match self.buff_type {
            BuffType::Grow => 2, // Grow can only stack 2 times (40% size increase max)
            _ => Self::MAX_STACKS,
        };

        if self.stacks < max_stacks {
            self.stacks += 1;
        }
        // Refresh duration
        self.duration = Timer::from_seconds(duration_secs, TimerMode::Once);
    }

    /// Get damage multiplier based on stacks
    pub fn get_damage_multiplier(&self) -> f32 {
        self.stacks as f32
    }
}

impl BuffBehavior for BuffType {
    fn name(&self) -> &str {
        match self {
            BuffType::Burning => "Burning",
            BuffType::Freezing => "Freezing",
            BuffType::Poisoned => "Poisoned",
            BuffType::Speed => "Speed",
            BuffType::Slowness => "Slowness",
            BuffType::Grow => "Grow",
        }
    }

    fn overlay_color(&self) -> Color {
        match self {
            BuffType::Burning => Color::srgba(1.0, 0.3, 0.0, 0.4),
            BuffType::Freezing => Color::srgba(0.3, 0.7, 1.0, 0.4),
            BuffType::Poisoned => Color::srgba(0.3, 0.8, 0.0, 0.4),
            BuffType::Speed => Color::srgba(1.0, 1.0, 0.3, 0.3),
            BuffType::Slowness => Color::srgba(0.4, 0.4, 0.4, 0.4),
            BuffType::Grow => Color::srgba(0.3, 1.0, 0.3, 0.3), // Green glow
        }
    }

    fn damage_per_tick(&self) -> i32 {
        match self {
            BuffType::Burning => 1,
            BuffType::Freezing => 0,
            BuffType::Poisoned => 1,
            BuffType::Speed => 0,
            BuffType::Slowness => 0,
            BuffType::Grow => 0,
        }
    }

    fn velocity_multiplier(&self) -> f32 {
        match self {
            BuffType::Burning => 1.0,
            BuffType::Freezing => 0.5,
            BuffType::Poisoned => 1.0,
            BuffType::Speed => 1.5,
            BuffType::Slowness => 0.6,
            BuffType::Grow => 1.3, // 30% speed boost - giant strides!
        }
    }

    fn scale_multiplier(&self) -> f32 {
        match self {
            BuffType::Grow => 1.2, // 20% bigger per stack
            _ => 1.0,
        }
    }
}

/// Marker component for entities that can have buffs
#[derive(Component)]
pub struct Buffable;

/// A commanded power has its own lifetime so fire/poison cannot replace it.
#[derive(Component)]
pub struct GrowthPower {
    pub duration: Timer,
    pub stacks: u8,
}
impl Default for GrowthPower {
    fn default() -> Self {
        Self {
            duration: Timer::from_seconds(30., TimerMode::Once),
            stacks: 1,
        }
    }
}
