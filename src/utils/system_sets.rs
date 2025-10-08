use bevy::prelude::*;

/// System execution order sets - ensures predictable system ordering
/// This prevents race conditions and ensures systems run in the correct sequence
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameplaySystemSet {
    /// Input handling - process player/AI input
    Input,
    /// AI decision making - enemy behavior, pathfinding
    AI,
    /// Physics simulation - velocity, gravity, collisions
    Physics,
    /// Combat - damage calculation, projectile hits
    Combat,
    /// Effects - buffs, debuffs, status effects
    Effects,
    /// World - chunk generation, block updates
    World,
    /// Visual - damage overlays, particles, animations
    Visual,
    /// UI - health bars, menus, HUD
    UI,
}

/// Configure the system ordering for the game
/// Call this in your main app setup
pub fn configure_system_sets(app: &mut App) {
    app.configure_sets(
        Update,
        (
            GameplaySystemSet::Input,
            GameplaySystemSet::AI,
            GameplaySystemSet::Physics,
            GameplaySystemSet::Combat,
            GameplaySystemSet::Effects,
            GameplaySystemSet::World,
            GameplaySystemSet::Visual,
            GameplaySystemSet::UI,
        )
            .chain(), // Ensures they run in this exact order
    );

    info!("✅ System sets configured with explicit ordering");
}
