#!/bin/bash
# Quick script to restore physics with Mass and LinearDamping

cat > temp_character.txt << 'EOF'
    /// Hero/Villager physics (64x64, bouncy) with collision layer
    pub fn character() -> (RigidBody, LinearVelocity, GravityScale, Mass, Collider, Restitution, Friction, LinearDamping, LockedAxes, CollisionLayers, CollisionEventsEnabled) {
        (
            RigidBody::Dynamic,
            LinearVelocity(Vec2::ZERO),
            GravityScale(1.5),
            Mass(3.0),
            Collider::rectangle(HERO_SIZE, HERO_SIZE),
            Restitution::new(HERO_BOUNCE),
            Friction::new(0.5),
            LinearDamping(0.3),
            LockedAxes::ROTATION_LOCKED,
            CollisionLayers::new(GameLayer::Character, GameLayer::Character.filters()),
            CollisionEventsEnabled,
        )
    }
EOF

echo "Created temp file - apply manually to bundles.rs"
