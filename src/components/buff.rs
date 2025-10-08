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

    /// Check if this is a positive buff (true) or debuff (false)
    fn is_beneficial(&self) -> bool {
        self.damage_per_tick() <= 0 && self.velocity_multiplier() >= 1.0
    }
}

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

impl BuffBehavior for BuffType {
    fn name(&self) -> &str {
        match self {
            BuffType::Burning => "Burning",
            BuffType::Freezing => "Freezing",
            BuffType::Poisoned => "Poisoned",
            BuffType::Regeneration => "Regeneration",
            BuffType::Speed => "Speed",
            BuffType::Slowness => "Slowness",
        }
    }

    fn overlay_color(&self) -> Color {
        match self {
            BuffType::Burning => Color::srgba(1.0, 0.3, 0.0, 0.4),
            BuffType::Freezing => Color::srgba(0.3, 0.7, 1.0, 0.4),
            BuffType::Poisoned => Color::srgba(0.3, 0.8, 0.0, 0.4),
            BuffType::Regeneration => Color::srgba(1.0, 0.4, 0.8, 0.3),
            BuffType::Speed => Color::srgba(1.0, 1.0, 0.3, 0.3),
            BuffType::Slowness => Color::srgba(0.4, 0.4, 0.4, 0.4),
        }
    }

    fn damage_per_tick(&self) -> i32 {
        match self {
            BuffType::Burning => 1,
            BuffType::Freezing => 0,
            BuffType::Poisoned => 1,
            BuffType::Regeneration => -1,
            BuffType::Speed => 0,
            BuffType::Slowness => 0,
        }
    }

    fn velocity_multiplier(&self) -> f32 {
        match self {
            BuffType::Burning => 1.0,
            BuffType::Freezing => 0.5,
            BuffType::Poisoned => 1.0,
            BuffType::Regeneration => 1.0,
            BuffType::Speed => 1.5,
            BuffType::Slowness => 0.6,
        }
    }
}

/// Marker component for entities that can have buffs
#[derive(Component)]
pub struct Buffable;
