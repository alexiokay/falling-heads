# Unified Head Physics System

## Problem
We had 3 separate collision functions with duplicate code:
- `check_collisions()` - for Hero
- `check_villager_collisions()` - for Villagers
- `check_enemy_collisions()` - for Enemies

All had the SAME physics logic repeated 3 times!

## Solution
Created a unified `Head` marker component that ALL heads share:

```rust
#[derive(Component)]
pub struct Head {
    pub size: f32,
    pub max_bounce_velocity: f32,
}
```

## Implementation

### 1. Added Head to all entities
- **Hero**: `Head { size: HERO_SIZE, max_bounce_velocity: 300.0 }`
- **Villagers**: `Head { size: VILLAGER_SIZE, max_bounce_velocity: 280.0 }`
- **Enemies**: `Head { size: 64.0, max_bounce_velocity: 280.0 }`

### 2. Created ONE unified collision function
Replace the 3 separate functions with:
```rust
fn check_head_block_collisions(
    spatial_hash: Res<SpatialHash>,
    mut head_query: Query<(&Head, &mut Transform, &mut Velocity, &Bouncy), Without<Block>>,
    block_query: Query<(Entity, &Transform), (With<Block>, Without<Head>)>,
    mut damage_events: MessageWriter<BlockDamaged>,
) {
    // Single implementation works for ALL heads!
}
```

### 3. Shared Physics Behavior
All heads now have the SAME default behaviors:
- ✅ Stronger horizontal bounce (0.8 vs 0.5)
- ✅ Random horizontal kicks on vertical bounce (-150..150)
- ✅ Lower max vertical bounce velocity
- ✅ Same collision detection logic
- ✅ Entity-to-entity bouncing

## Benefits
1. **DRY Principle** - No code duplication
2. **Consistency** - All heads behave the same by default
3. **Maintainability** - Change physics in ONE place
4. **Extensibility** - Easy to add new head types
