# Architecture cleanup — current implementation

Verified installed build: **20261027-032456-b6e29650ed**. The Play shortcut resolves this executable.

Crowded actor contacts use the original rounded cuboids. The sphere experiment has been removed at the user's request: a small whole-frame difference did not justify changed diagonal packing. No collision-shape performance tradeoff is part of this cleanup.

This section supersedes the earlier review below. The runtime remains Bevy 0.20 / Avian 0.8; this is an incremental refactor, not an engine replacement.

## Ownership and schedules

- `plugins/physics_plugin.rs` is the coordinator. `plugins/physics/{terrain,terrain_motion,movement,combat,tnt}.rs` own swept grid contacts, geometric resolution, steering, Avian contact consumers and TNT terrain response. Regression tests are separate. Unregistered legacy collision algorithms and the unused Velocity/Gravity/Bouncy bundle are removed.
- Update steering (`PhysicsSet::AI`) precedes crowd stall assistance and the Update collision consumers (`PhysicsSet::Collision`). The unused global GameplaySystemSet declaration is removed. This does **not** move all gameplay into one serial chain or silently change it to fixed-rate AI.
- `systems/target_index.rs` rebuilds an exact balanced k-d target index once per Update. Core enemy steering, skeleton/yeti/ghast attacks and ghast floating explicitly run after `TargetsReady`. It preserves 3D distance/range tests; equal distances resolve deterministically by entity ID. Build O(H log H), typical query O(log H), worst case O(H).
- Avian still owns authoritative actor Position. Terrain captures step starts after physics preparation and resolves at PhysicsSchedule Last, before interpolation/writeback. Existing swept square collision, snow/ice rules and teleport/rocket exclusions remain tested.
- `systems/actor_depth.rs` owns the bounded depth policy, separately from damage visuals. It reuses its ordering buffer and never feeds interpolated XY back into physics. Root collider Z still follows the existing 30–38 / 55–63 depth bands. A complete physics-root/render-child migration would require migrating effects, projectile hits, shadows and spawning together; this pass explicitly preserves that contract instead of pretending the coupling is gone.
- `systems/fluids/mod.rs` owns finite-volume state and rules; runtime.rs owns gameplay integration and fresh FluidOccupancy input; render.rs owns mesh/material generation and the explicit FluidRenderRevision. Revision compares only rendered state (volume/kind, falling offsets, support, steam, ice/cooled cells). Clock/reserve/occupancy bookkeeping does not rebuild geometry. Loaded-chunk changes remain a render input.
- Launcher and dragon visual construction/animation and native probes are separate modules. Biome landmark layouts are separated from encounter AI. Existing module APIs and tests retain their semantics.
- Health displays use a shared mesh row per healthy actor, plus a temporary half-heart child only for odd health above one. They preserve the same heart count, scale and screen-facing layout. Shared geometry/materials and health-state changes are reused; owners' removal recursively cleans row children.

## Scaling changes

Terrain candidates, expanded bounds, contacts, damaged-block sets and chunk-biome lookup storage are retained across actors/steps. Chunk metadata is indexed once per terrain pass. The sweep remains exact; overlap recovery can still be quadratic in the local candidate count. Dense actors can still create many physical contacts; this is not an O(n) claim for the whole game.

370 healthy villagers now require 370 health-row entities rather than 1,110 heart entities. The same visual quads are rendered. This is an entity/update-cost reduction, not a promise of a particular FPS gain. See PERFORMANCE_NOTES.md for measured native results.

## Verification and maintenance

`tools/Check-FallingHeadsArchitecture.ps1` checks the recorded compiler version, formats the explicitly cleaned modules, runs Clippy against a diagnostic baseline, and runs tests. `-Build` also builds; `-Native` adds water, snow/ghast and 370 named-villager checks. Updating the warning baseline is explicit (`-UpdateBaseline`), never automatic during a normal check. Legacy compiler/style warnings remain visible; no blanket lint/unsafe allowance was introduced.

A manually dispatched GitHub workflow targets a prepared Windows runner with the native SDK, patched crates and offline dependency cache. It has not been executed on GitHub. `tools/build-environment.json` records the tested toolchain; Cargo.lock and the vendored patches pin Rust dependencies. External SDK paths in .cargo/config.toml still require provisioning on another machine.

`native/SAFETY.md` records the FFI ownership and schedule assumptions. Matching Rust/C++ ABI assertions now guard structure size and field offsets. Full driver/device-loss correctness is not claimed. The launcher stress run still emits Avian mass/inertia warnings also present in the 24 October launcher log; their origin requires separate investigation. They were not hidden, and this pass does not claim every runtime warning is resolved.

The release profile now retains crash symbols and uses thin LTO. `tools/Build-FallingHeadsOptimized.ps1` builds a full-optimization application candidate with the existing optimized engine dependencies, assertions and PDBs for a practical before-distribution benchmark. Distribution must name the actual tested binary; do not infer that a release profile or an FPS improvement has been validated merely because configuration exists.

---

# Earlier audit (historical; statuses below predate the cleanup)

# Architecture review — 27 October 2026

This review covers the active Bevy/Avian scheduling, head spawning and visuals, terrain collision, fluid rendering, and the 370-villager report. It is not a complete correctness audit of the native Vulkan/DLSS bridge. The older review below predates the Avian migration and must not be used as a description of the current engine.

## Assessment

The project has a reusable ECS foundation, but it is no longer uniformly clean. Plugin boundaries, typed damage/power messages, shared mesh/material handles, cached terrain queries, geometry-aware effects, reproducible native probes and crash logs are useful foundations. Several integration decisions and legacy paths need consolidation; rewriting the entire game is not warranted.

## Findings from active code

1. **Fluid render invalidation mixed with simulation inputs (fixed in this pass).** `simulate` refreshed the occupied-head-cell set each rendered frame through `ResMut<FluidWorld>`. `render` interpreted this as changed geometry and regenerated/uploaded every fluid mesh, even between 10 Hz fluid ticks. Occupancy remains current for freeze safety but no longer marks rendered fluid state dirty. Actual deposits, transport, freezing, steam and loaded-chunk changes still invalidate rendering. A dedicated visual revision/resource is the longer-term boundary.
2. **Detached water overlay (fixed in this pass).** A body front at Z=-2 and a duplicate transparent sheet at Z=76 had different projected silhouettes. The foreground sheet could paint blue over foreground terrain. Water now has one connected volume at Z=24, immediately in front of terrain (Z=22), behind normal heads (Z=30..38). Rich 72% water opacity is retained. There is no world-wide second tint sheet. A future submerged-face tint should be clipped to actual face geometry/alpha, not duplicate the pool at another depth.
3. **Death collision semantics (fixed in this pass).** Removing `CollisionLayers` restores default collision filtering; making a rigid body static does not make it non-colliding. Dying heads now explicitly receive `CollisionLayers::NONE`. A regression test exercises the actual death system.
4. **Crowd contact cost (measured; targeted improvement).** `crowd_slide` already uses nearby-cell bins, not a global all-pairs scan. It changes nearby heads to rounded cuboids, which take a general convex contact path. A simpler sphere is used for head-to-head contact only; terrain retains swept square bounds. This changes diagonal head contact shape, not the face sprite or terrain footprint. See PERFORMANCE_NOTES.md for measured results and limitations.
5. **Scheduling documentation is misleading (pending).** `GameplaySystemSet` is chained globally but no production system joins those sets. Real dependencies live in per-plugin sets/chains and the fixed Avian schedule. Remove the unused scheme or migrate a tested subsystem at a time; do not blindly place every system into one serial chain. AI/forces still run in Update while authoritative terrain resolution runs in PhysicsSchedule.
6. **Visual depth and physics authority are coupled (pending).** `actor_depth`, hosted in damage visuals, sorts actors and writes their authoritative Z. This prevents transform interpolation feedback, but visual rank changes also affect physical collider depth. Depth policy belongs in a dedicated presentation module with named layers; a future child-render/root-physics split needs migration of effects, shadows, projectiles and contacts together. Simply moving all heads to block Z=0 would regress occlusion and collision behavior.
7. **Legacy physics remains in the active source tree (pending).** Unregistered `check_entity_collisions` contains a five-pass all-pairs algorithm, and other deprecated Velocity/Bouncy routines remain. They are not evidence that current terrain collision is O(H^2), but they obscure which code is authoritative and contribute to warnings. Delete them after confirming no registrations/callers; retain regression coverage of the Avian/custom-terrain integration.
8. **Remaining scaling work (pending).** Terrain contact allocates candidate/bounds vectors per head and scans chunk metadata on impacts. Enemy targeting scans targets per enemy. Hearts share assets but 370 healthy villagers still create 1,110 heart entities. Profile before choosing scratch-buffer reuse, actor-target indexing, batched heart geometry or UI density rules; do not remove collisions or silently skip simulation to increase FPS.
9. **Module size and warnings (pending).** Fluids, dragon, physics, encounters and launcher combine substantial simulation/presentation/test logic. Split by ownership and invariant, not arbitrary line counts. The build currently reports about 130 warnings, largely unused legacy code/variables. Establish a warning baseline and remove old paths before enabling stricter CI gates. Keep unsafe native rendering isolated and explicitly review resource lifetime/synchronization contracts.

10. **Distribution profile (pending).** The current publication workflow uses `target/debug` with application opt-level=1 and optimized dependencies. This is useful for iteration and crash symbols, but a production/release profile should be benchmarked before distribution; neither a Rust rewrite nor DLSS fixes CPU simulation cost.

## Practical next refactoring order

- Consolidate depth constants and active schedule ownership; remove dead scheduling declarations and old physics functions with targeted regression tests.
- Separate fluid simulation input, topology revision and render asset update responsibilities.
- Extract terrain contacts, AI movement, and combat event consumers from the physics plugin without changing update order.
- Extend reproducible crowd workloads (separated, packed, mass spawn/death, named villagers) and track frame-time percentiles, contact time and asset counts. Keep raster and RT runs separate.
- Add CI formatting/lint/test/build checks using the pinned Rust/toolchain and vendored engine patches. Avoid blanket unsafe allowances or large reformat-only changes mixed with gameplay fixes.

## Primary reference used

[Bevy Query documentation](https://docs.rs/bevy/latest/bevy/ecs/system/struct.Query.html) explains query iteration costs, mutable-access scheduling constraints and why performance changes should be benchmarked. [Bevy change detection example](https://github.com/bevyengine/bevy/blob/main/examples/ecs/change_detection.rs) documents mutation-based invalidation. Exact engine behavior was checked against this project's local Bevy 0.20 / Avian 0.8 sources because online latest documentation may describe another release.

---

## Historical review (superseded where it conflicts with the current review)

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


## Final verification for the current installed build

175 tests pass, formatting and the 270-diagnostic Clippy baseline pass without new diagnostics. The final binary passed named 370- and 2,000-head native workloads (all actor centers in the viewport), five water captures, 27 snow/ghast captures, 15 launcher captures and the hidden graphics pause/resize/quality/FG-toggle test, each with successful shutdown. All 24 sampled ghast mouth-transition faces retained their light texture. Representative screenshots were inspected. The hidden graphics test verifies state transitions and shutdown, not the visual quality of interpolated display frames.

The 2,000-head workload measured 8.671 / 8.849 ms median and 10.365 / 10.432 ms p95 on this machine with raster, RT/DLSS/FG off. It uses a larger arena and wider test camera; see PERFORMANCE_NOTES.md for density, hardware and sampling limits. No universal frame-rate improvement is claimed.
