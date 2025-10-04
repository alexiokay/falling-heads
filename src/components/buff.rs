use bevy::prelude::*;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BuffType {
    Burning,
    Freezing,
    Poisoned,
    Regeneration,
    Speed,
    Slowness,
}

#[derive(Component)]
pub struct Buff {
    pub buff_type: BuffType,
    pub duration: Timer,
    pub tick_timer: Timer,  // For damage/healing over time
}

impl Buff {
    pub fn new(buff_type: BuffType, duration_secs: f32) -> Self {
        Self {
            buff_type,
            duration: Timer::from_seconds(duration_secs, TimerMode::Once),
            tick_timer: Timer::from_seconds(0.5, TimerMode::Repeating),  // Tick every 0.5s
        }
    }
}

impl BuffType {
    /// Returns the color overlay for visual effect
    pub fn overlay_color(&self) -> Color {
        match self {
            BuffType::Burning => Color::srgba(1.0, 0.3, 0.0, 0.4),      // Orange/red overlay
            BuffType::Freezing => Color::srgba(0.3, 0.7, 1.0, 0.4),     // Light blue overlay
            BuffType::Poisoned => Color::srgba(0.3, 0.8, 0.0, 0.4),     // Green overlay
            BuffType::Regeneration => Color::srgba(1.0, 0.4, 0.8, 0.3), // Pink overlay
            BuffType::Speed => Color::srgba(1.0, 1.0, 0.3, 0.3),        // Yellow overlay
            BuffType::Slowness => Color::srgba(0.4, 0.4, 0.4, 0.4),     // Gray overlay
        }
    }

    /// Returns damage dealt per tick (negative = healing)
    pub fn damage_per_tick(&self) -> i32 {
        match self {
            BuffType::Burning => 1,      // 1 damage every 0.5s
            BuffType::Freezing => 0,     // No damage, just slows
            BuffType::Poisoned => 1,     // 1 damage every 0.5s
            BuffType::Regeneration => -1, // Heals 1 every 0.5s
            BuffType::Speed => 0,
            BuffType::Slowness => 0,
        }
    }

    /// Returns velocity multiplier
    pub fn velocity_multiplier(&self) -> f32 {
        match self {
            BuffType::Burning => 1.0,
            BuffType::Freezing => 0.5,    // 50% slower
            BuffType::Poisoned => 1.0,
            BuffType::Regeneration => 1.0,
            BuffType::Speed => 1.5,       // 50% faster
            BuffType::Slowness => 0.6,    // 40% slower
        }
    }

    /// Returns the name for UI display
    pub fn name(&self) -> &str {
        match self {
            BuffType::Burning => "Burning",
            BuffType::Freezing => "Freezing",
            BuffType::Poisoned => "Poisoned",
            BuffType::Regeneration => "Regeneration",
            BuffType::Speed => "Speed",
            BuffType::Slowness => "Slowness",
        }
    }
}

/// Marker component for entities that can have buffs
#[derive(Component)]
pub struct Buffable;
