# Avian Physics Migration Plan

## Component Mapping

### Custom → Avian Component Replacements

| Custom Component | Avian Component | Notes |
|------------------|-----------------|-------|
| `Velocity { x, y }` | `LinearVelocity(Vec2)` | Direct 1:1 mapping |
| `Gravity { force }` | `GravityScale(f32)` | Multiply by global gravity |
| `Bouncy { restitution }` | `Restitution::new(f32)` | Collider property |

### Entity Type Mappings

#### Dynamic Entities (Affected by physics)
- **Hero, Villager, Enemy, TNT, Arrows, Resources**
  - `RigidBody::Dynamic`
  - `LinearVelocity(Vec2::ZERO)`
  - `GravityScale(1.0)` or `GravityScale(0.0)` for flying entities
  - `Collider::circle(radius)` or `Collider::rectangle(width, height)`
  - `Restitution::new(1.0)` for bouncy, `0.0` for non-bouncy
  - `Friction::new(0.5)` for ground friction
  - `LockedAxes::ROTATION_LOCKED` to prevent rotation in 2D

#### Static Entities (Don't move)
- **Blocks**
  - `RigidBody::Static`
  - `Collider::rectangle(64.0, 64.0)` (BLOCK_SIZE)
  - No velocity, gravity, or restitution needed

### Physics Constants Conversion

```rust
// Old constants (utils/constants.rs)
STANDARD_GRAVITY: -500.0
ARROW_GRAVITY: -600.0
HERO_BOUNCE: 1.0
VILLAGER_BOUNCE: 1.0
ENEMY_BOUNCE: 1.0
NO_BOUNCE: 0.0

// New Avian setup
Gravity(Vec2::NEG_Y * 500.0)  // Global gravity
GravityScale(1.0)             // Standard entities
GravityScale(1.2)             // Arrows (600/500 = 1.2)
GravityScale(0.0)             // Flying enemies
Restitution::new(1.0)         // Bouncy entities
Restitution::new(0.0)         // Non-bouncy (TNT, resources)
```

## Migration Steps

### Phase 1: Setup ✅
1. ✅ Add `avian2d = "0.3"` to Cargo.toml
2. ✅ Add `PhysicsPlugins` to main.rs
3. ✅ Set global gravity: `Gravity(Vec2::NEG_Y * 500.0)`

### Phase 2: Component Migration
4. Update `components/physics.rs` with Avian re-exports
5. Update all spawn functions to use Avian components:
   - `systems/entity_spawn.rs` (hero, villagers, enemies)
   - `systems/world_generation.rs` (blocks, TNT)
   - `systems/projectile_system.rs` (arrows)

### Phase 3: Physics System Replacement
6. Replace `physics_plugin.rs` collision systems with Avian collision events
7. Handle collision events using `MessageReader<CollisionStarted>`
8. Implement damage/buff logic in collision event handlers

### Phase 4: Cleanup
9. Remove old custom physics systems
10. Remove `SpatialHash` resource (Avian has builtin broadphase)
11. Test all gameplay mechanics

## Collision Event Handling Pattern

```rust
fn handle_entity_block_collisions(
    mut collision_events: MessageReader<CollisionStarted>,
    entity_query: Query<&EntityType>,
    block_query: Query<&BlockType>,
) {
    for CollisionStarted(entity_a, entity_b) in collision_events.read() {
        // Check which is entity, which is block
        // Apply damage, buffs, etc.
    }
}
```

## Benefits
- ✅ Automatic spatial partitioning (no manual SpatialHash)
- ✅ Optimized broadphase/narrowphase (4-6x faster than custom)
- ✅ Handles 1000+ entities efficiently
- ✅ Built-in sleeping for static entities
- ✅ ECS-native (no separate physics world)

## Risks
- Physics feel may change (need tuning)
- Collision callback patterns different from current approach
- TNT locking mechanic needs custom implementation
