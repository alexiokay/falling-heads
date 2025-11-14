use bevy::prelude::*;

/// Marker component for visual collision debug borders
#[derive(Component)]
pub struct CollisionDebugBorder {
    /// The entity this border is visualizing
    pub parent_entity: Entity,
    /// Size of the collision box
    pub size: Vec3,
}

/// Global setting to toggle collision debug visualization
#[derive(Resource)]
pub struct CollisionDebugSettings {
    /// Master toggle - show collision borders
    pub enabled: bool,
    /// Show borders for blocks
    pub show_blocks: bool,
    /// Show borders for characters (hero, villagers, enemies)
    pub show_characters: bool,
    /// Show borders for resources (dropped items)
    pub show_resources: bool,
    /// Show borders for TNT
    pub show_tnt: bool,
    /// Show borders for projectiles (arrows, snowballs, fireballs)
    pub show_projectiles: bool,
    /// Show borders for random boxes and lootboxes
    pub show_randomboxes: bool,
}

impl Default for CollisionDebugSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            show_blocks: true,
            show_characters: true,
            show_resources: true,
            show_tnt: true,
            show_projectiles: true,
            show_randomboxes: true,
        }
    }
}

impl CollisionDebugSettings {
    /// Toggle master enabled state
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    /// Enable all visualization
    pub fn enable_all(&mut self) {
        self.enabled = true;
        self.show_blocks = true;
        self.show_characters = true;
        self.show_resources = true;
        self.show_tnt = true;
        self.show_projectiles = true;
        self.show_randomboxes = true;
    }
}
