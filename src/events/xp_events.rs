use bevy::prelude::*;

/// Event fired when XP is gained
#[derive(Message, Debug)]
pub struct XpGained {
    pub amount: u32,
    pub source: XpSource,
}

/// Source of XP gain
#[derive(Debug, Clone, Copy)]
pub enum XpSource {
    MobKill,
    ResourceCollected,
    BlockBroken,
    /// Extra XP from a combo streak's multiplier.
    Combo,
}

/// Event fired when player levels up
#[derive(Message, Debug)]
pub struct LevelUp {
    pub new_level: u32,
}
