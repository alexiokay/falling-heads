use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(Resource, Default)]
pub struct CommandQueue {
    pub pending: VecDeque<PowerCommand>,
}

/// Trait for power command behavior - defines how each power executes
pub trait PowerBehavior {
    /// Get the display name of this power
    fn name(&self) -> &'static str;

    /// Get the description of what this power does
    fn description(&self) -> &'static str {
        "No description available"
    }

    /// Get the cooldown duration in seconds (0 = no cooldown)
    fn cooldown(&self) -> f32 {
        0.0
    }

    /// Check if this power requires a hero to be present
    fn requires_hero(&self) -> bool {
        true  // Most powers target the hero
    }

    /// Get the category of this power for UI grouping
    fn category(&self) -> PowerCategory {
        PowerCategory::Offensive
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PowerCategory {
    Offensive,   // TNT, Laser, Arrow
    Movement,    // Fast, Slow
    Defensive,   // Mirror, Box
    Summoning,   // Villagers, Ghast
    Utility,     // Magnet, Fuss, Bolt
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PowerCommand {
    pub command_type: PowerCommandType,
    pub username: String,  // Twitch/YouTube viewer username
}

impl PowerCommand {
    pub fn new(command_type: PowerCommandType, username: String) -> Self {
        Self { command_type, username }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PowerCommandType {
    Tnt,
    Fast,
    Slow,
    Fuss,
    Bolt,
    Arrow,
    Laser,
    Ghast,
    Box,
    Magnet,
    Villagers, // Spawn 5 villagers around hero
    Mirror,    // Magic barrier that reflects projectiles
    Dragon,    // Spawn a dragon boss enemy
}

impl PowerBehavior for PowerCommandType {
    fn name(&self) -> &'static str {
        match self {
            PowerCommandType::Tnt => "TNT",
            PowerCommandType::Fast => "FAST",
            PowerCommandType::Slow => "SLOW",
            PowerCommandType::Fuss => "FUSS",
            PowerCommandType::Bolt => "BOLT",
            PowerCommandType::Arrow => "ARROW",
            PowerCommandType::Laser => "LASER",
            PowerCommandType::Ghast => "GHAST",
            PowerCommandType::Box => "BOX",
            PowerCommandType::Magnet => "MAGNET",
            PowerCommandType::Villagers => "VILLAGERS",
            PowerCommandType::Mirror => "MIRROR",
            PowerCommandType::Dragon => "DRAGON",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            PowerCommandType::Tnt => "Spawn explosive TNT above the hero",
            PowerCommandType::Fast => "Double movement speed for 5 seconds",
            PowerCommandType::Slow => "Half movement speed for 5 seconds",
            PowerCommandType::Fuss => "Utility power (not yet implemented)",
            PowerCommandType::Bolt => "Lightning bolt attack (not yet implemented)",
            PowerCommandType::Arrow => "Fire 48 arrows in a circular barrage",
            PowerCommandType::Laser => "Create a powerful laser beam",
            PowerCommandType::Ghast => "Spawn a ghast enemy (not yet implemented)",
            PowerCommandType::Box => "Create a protective box (not yet implemented)",
            PowerCommandType::Magnet => "Attract nearby items (not yet implemented)",
            PowerCommandType::Villagers => "Spawn 5 villagers around the hero",
            PowerCommandType::Mirror => "Create a magic barrier that reflects projectiles",
            PowerCommandType::Dragon => "Spawn a powerful dragon boss enemy",
        }
    }

    fn cooldown(&self) -> f32 {
        match self {
            PowerCommandType::Tnt => 3.0,
            PowerCommandType::Fast => 10.0,
            PowerCommandType::Slow => 10.0,
            PowerCommandType::Arrow => 15.0,
            PowerCommandType::Laser => 8.0,
            PowerCommandType::Villagers => 20.0,
            PowerCommandType::Mirror => 15.0,
            PowerCommandType::Dragon => 30.0,  // Long cooldown for boss spawn
            _ => 0.0,
        }
    }

    fn category(&self) -> PowerCategory {
        match self {
            PowerCommandType::Tnt | PowerCommandType::Arrow | PowerCommandType::Laser | PowerCommandType::Bolt => {
                PowerCategory::Offensive
            }
            PowerCommandType::Fast | PowerCommandType::Slow => PowerCategory::Movement,
            PowerCommandType::Mirror | PowerCommandType::Box => PowerCategory::Defensive,
            PowerCommandType::Villagers | PowerCommandType::Ghast | PowerCommandType::Dragon => PowerCategory::Summoning,
            PowerCommandType::Magnet | PowerCommandType::Fuss => PowerCategory::Utility,
        }
    }
}
