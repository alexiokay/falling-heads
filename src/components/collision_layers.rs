use avian2d::prelude::*;

/// Collision layers for filtering what collides with what
#[derive(PhysicsLayer, Clone, Copy, Debug, Default)]
pub enum GameLayer {
    // Static environment
    #[default]
    Block,        // Terrain blocks - collide with entities but not each other

    // Dynamic entities
    Character,    // Heroes, villagers - collide with blocks, enemies, arrows
    Enemy,        // Enemies - collide with blocks, characters, arrows

    // Projectiles
    Arrow,        // Arrows - sensor only, collide with everything for detection
    Snowball,     // Snowballs - sensor only

    // Items
    Resource,     // Dropped resources - collide with blocks only
    Tnt,          // TNT blocks - collide with blocks and characters
}

impl GameLayer {
    /// Get collision memberships for this layer
    pub fn memberships(self) -> LayerMask {
        LayerMask::from(self)
    }

    /// Get what this layer should collide with (filters)
    pub fn filters(self) -> LayerMask {
        match self {
            // Blocks collide with all dynamic entities
            GameLayer::Block => LayerMask::from([
                GameLayer::Character,
                GameLayer::Enemy,
                GameLayer::Resource,
                GameLayer::Tnt,
            ]),

            // Characters collide with blocks, enemies, and items
            GameLayer::Character => LayerMask::from([
                GameLayer::Block,
                GameLayer::Enemy,
                GameLayer::Character,
                GameLayer::Tnt,
                GameLayer::Arrow,
                GameLayer::Snowball,
            ]),

            // Enemies collide with blocks, characters, and items
            GameLayer::Enemy => LayerMask::from([
                GameLayer::Block,
                GameLayer::Character,
                GameLayer::Enemy,
                GameLayer::Arrow,
            ]),

            // Arrows detect collisions with everything (sensor)
            GameLayer::Arrow => LayerMask::ALL,

            // Snowballs detect collisions with characters and blocks (sensor)
            GameLayer::Snowball => LayerMask::from([
                GameLayer::Block,
                GameLayer::Character,
                GameLayer::Enemy,
            ]),

            // Resources only collide with blocks (to settle on ground)
            GameLayer::Resource => LayerMask::from([
                GameLayer::Block,
            ]),

            // TNT collides with blocks and characters
            GameLayer::Tnt => LayerMask::from([
                GameLayer::Block,
                GameLayer::Character,
                GameLayer::Enemy,
            ]),
        }
    }
}
