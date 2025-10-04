# Architecture Analysis & Improvement Recommendations

## Current State: ✅ **GOOD Foundation, Needs Refinement**

### What's Working Well ✅

1. **Plugin-Based Architecture** ✅
   - Well-organized into logical plugins (Physics, Gameplay, Powers, etc.)
   - Clean separation of concerns
   - Easy to enable/disable features

2. **Component Composition** ✅
   - Using ECS correctly with marker components (Hero, Villager, Enemy)
   - Created HeadBundle for shared components
   - Proper use of Bevy's component system

3. **Event-Driven Communication** ✅
   - BlockDamaged, EntityDamaged events
   - Clean decoupling between systems
   - Good use of Bevy's event system

4. **Resource Management** ✅
   - Material/Mesh caching for performance
   - Asset handles properly managed
   - Good separation of global state

## Issues Found & Recommendations 🔧

### 1. ⚠️ **PhysicsPlugin is TOO BIG** (600+ lines)

**Problem:**
- Single file contains ALL physics AND collision logic
- Mixes different concerns (gravity, collisions, AI, damage)
- Hard to maintain and test

**Solution:**
```
src/plugins/physics_plugin.rs (coordinator only - 50 lines)
src/systems/physics/
  ├── gravity.rs         (apply_gravity)
  ├── velocity.rs        (apply_velocity)
  ├── collisions.rs      (collision detection)
  ├── hero_collisions.rs (hero-specific)
  ├── enemy_collisions.rs (enemy-specific)
  └── mod.rs
```

**Benefits:**
- Each file < 150 lines
- Easier to find specific logic
- Better testability
- Clearer responsibilities

### 2. ⚠️ **Missing System Sets for Execution Order**

**Problem:**
```rust
// Current: No guaranteed order
.add_systems(Update, (
    apply_gravity,
    apply_velocity,
    check_collisions,
    // ...
))
```

**Solution:**
```rust
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum PhysicsSet {
    Input,      // Read input
    Forces,     // Apply gravity, forces
    Movement,   // Update positions
    Collision,  // Detect collisions
    Response,   // Resolve collisions
}

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .configure_sets(Update, (
                PhysicsSet::Input,
                PhysicsSet::Forces,
                PhysicsSet::Movement,
                PhysicsSet::Collision,
                PhysicsSet::Response,
            ).chain())
            .add_systems(Update, (
                apply_gravity.in_set(PhysicsSet::Forces),
                apply_velocity.in_set(PhysicsSet::Movement),
                check_collisions.in_set(PhysicsSet::Collision),
                resolve_collisions.in_set(PhysicsSet::Response),
            ));
    }
}
```

**Benefits:**
- Deterministic execution order
- Prevents race conditions
- Self-documenting system dependencies

### 3. ⚠️ **No Spatial Partitioning for Collisions**

**Problem:**
```rust
// O(n²) collision detection!
for hero in heroes {
    for block in blocks {  // Checks EVERY block
        if collides(hero, block) { }
    }
}
```

**Solution:**
```rust
// Use chunk-based collision detection
#[derive(Resource)]
struct SpatialHash {
    chunks: HashMap<(i32, i32), Vec<Entity>>,
}

fn check_collisions(
    spatial_hash: Res<SpatialHash>,
    hero_query: Query<&Transform, With<Hero>>,
    block_query: Query<&Transform, With<Block>>,
) {
    for hero_transform in hero_query.iter() {
        let chunk_pos = world_to_chunk(hero_transform.translation);

        // Only check blocks in nearby chunks (9 chunks max)
        for nearby_chunk in get_nearby_chunks(chunk_pos) {
            if let Some(entities) = spatial_hash.chunks.get(&nearby_chunk) {
                for entity in entities {
                    // Check collision
                }
            }
        }
    }
}
```

**Benefits:**
- O(n) instead of O(n²)
- Massive performance improvement with many entities
- Scalable to thousands of blocks

### 4. ⚠️ **Missing Component Bundles for Common Patterns**

**Problem:**
```rust
// Repeated everywhere:
commands.spawn((
    Mesh3d(mesh),
    MeshMaterial3d(material),
    Transform::from_xyz(x, y, z),
    Visibility::default(),
    // ...
));
```

**Solution:**
```rust
#[derive(Bundle)]
struct SpriteBundle3d {
    mesh: Mesh3d,
    material: MeshMaterial3d<StandardMaterial>,
    transform: Transform,
    visibility: Visibility,
}

#[derive(Bundle)]
struct PhysicsBundle {
    velocity: Velocity,
    gravity: Gravity,
    bouncy: Bouncy,
}

// Usage:
commands.spawn((
    Hero,
    SpriteBundle3d::new(mesh, material, pos),
    PhysicsBundle::default(),
    Health::new(3),
));
```

### 5. ⚠️ **Buff System Needs Architecture**

**Current:** Buffs in physics_plugin.rs (wrong place!)

**Better:**
```
src/systems/buffs/
  ├── buff_application.rs  (apply buffs on collision/damage)
  ├── buff_effects.rs      (tick damage, modify stats)
  ├── buff_visuals.rs      (color overlays)
  └── mod.rs

src/plugins/buff_plugin.rs
```

### 6. ✅ **Already Good:**

- Component organization
- Event system usage
- Resource caching
- HeadBundle pattern

## Recommended Improvements Priority

### 🔥 **High Priority (Do Now)**

1. **Split PhysicsPlugin** into multiple files
   - Easier to maintain
   - Current blocker for growth

2. **Add System Sets** for execution order
   - Prevents bugs
   - Self-documenting

3. **Move buff logic** to dedicated plugin
   - Wrong location currently
   - Makes physics plugin smaller

### 🟡 **Medium Priority (Do Soon)**

4. **Add Spatial Partitioning**
   - Performance critical as game grows
   - Prepare for more enemies/blocks

5. **Create more Bundles**
   - Reduce boilerplate
   - Consistent entity spawning

### 🟢 **Low Priority (Nice to Have)**

6. **Add unit tests** for systems
7. **Performance benchmarks**
8. **More documentation**

## Refactoring Plan

### Phase 1: Split Physics Plugin (2-3 hours)
```bash
# Create new structure
src/systems/physics/
  ├── mod.rs
  ├── gravity.rs
  ├── velocity.rs
  ├── hero_collisions.rs
  ├── enemy_collisions.rs
  ├── entity_collisions.rs
  └── tnt_collisions.rs

# Move logic from physics_plugin.rs
# Update imports in physics_plugin.rs
# Test everything still works
```

### Phase 2: Add System Sets (1 hour)
```rust
// Define sets
// Update plugin to use sets
// Verify execution order
```

### Phase 3: Move Buff System (30 min)
```rust
// Move apply_block_buff from physics_plugin
// Create BuffPlugin
// Register in main.rs
```

### Phase 4: Spatial Hash (3-4 hours)
```rust
// Implement SpatialHash resource
// Update collision systems
// Benchmark performance
```

## Code Quality Metrics

### Current:
- ❌ PhysicsPlugin: 600+ lines (should be < 100)
- ⚠️ No system ordering (can cause bugs)
- ⚠️ O(n²) collision (performance issue)
- ✅ Good component design
- ✅ Good event usage
- ✅ Good resource management

### Target:
- ✅ All files < 200 lines
- ✅ System sets for all plugins
- ✅ O(n) collision with spatial hash
- ✅ 80%+ code coverage with tests
- ✅ < 16ms frame time with 1000+ entities

## Conclusion

**Overall Assessment: 7/10** 🎯

The architecture is **fundamentally sound** with good plugin organization, ECS usage, and component design. The main issues are:
1. File size (PhysicsPlugin too big)
2. Missing execution order guarantees
3. Performance optimization needed for scaling

These are **normal issues** in early game development and are **easy to fix**. The foundation is solid!

**Recommendation:** Continue development, but **refactor PhysicsPlugin** soon before it gets harder to manage.
