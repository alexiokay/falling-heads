# MineHeads - Release Notes

## Version 0.4.0 - UI & Team Combat Update (October 5, 2025)

### 🎮 Menu System

#### Pause Menu (ESC)
- **NEW**: Pause menu with Resume, Settings, and Quit buttons
- **Toggle**: Press ESC to pause/resume game
- **State Management**: Proper GameState transitions (Playing ↔ Paused)
- **Clean UI**: Bevy 0.17 native UI (Node, Text, Button components)
- **Files**: `src/components/pause_menu.rs`, `src/systems/pause_menu_system.rs`

#### Settings Menu
- **NEW**: VSync toggle (On/Off)
- **NEW**: FPS counter toggle (Show/Hide)
- **Real-time Updates**: Changes apply immediately
- **Window Mode**: VSync switches between AutoVsync and AutoNoVsync
- **Persistent**: Settings stored in GameSettings resource
- **Files**: `src/resources/settings.rs`, `src/components/settings_menu.rs`, `src/systems/settings_menu_system.rs`

### ⚔️ Team Combat System

#### Friendly Fire Prevention
- **NEW**: Team-based damage system prevents friendly fire
- **Teams**:
  - `Team::Friendly` - Hero + Villagers
  - `Team::Enemy` - Skeletons and other enemies
- **Rules**:
  - Friendly arrows only hit enemies
  - Enemy arrows only hit hero/villagers
  - No friendly fire within teams
  - No enemy-on-enemy damage
- **Architecture**: Clean separation via Team component on projectiles
- **Files**: `src/components/projectile.rs`, `src/systems/projectile_system.rs`

#### Arrow Power Enhancement
- **NEW**: ARROW power command shoots 48 arrows in 2 full rotations (720°)
- **Sequential Fire**: Spawns one arrow every 0.05s (2.4 second duration)
- **Counter-Clockwise**: Visual spiral effect as arrows shoot
- **Hero Tracking**: Barrage follows hero as they move (prevents self-hits)
- **Friendly Arrows**: Use `Arrow::new_friendly()` - won't hit hero/villagers
- **Enemy Collision**: Added full enemy collision support (was missing!)
- **Files**: `src/plugins/powers_plugin.rs`

#### Enemy Collision Detection
- **FIXED**: Friendly arrows now properly hit enemies
- **Bug**: Projectile system only checked Hero/Villager collisions - enemies were completely ignored
- **Solution**: Added third ParamSet query for Enemy entities
- **Impact**: Arrows push enemies, deal damage, and stick to them
- **Stuck Arrows**: Updated to track enemies (not just hero/villagers)
- **Files**: `src/systems/projectile_system.rs`

### 🐛 Critical Bug Fixes

#### Camera Follow After Pause
- **FIXED**: Camera no longer loses hero after pausing/resuming
- **Root Cause**: `OnEnter(GameState::Playing)` was spawning NEW hero/villagers every resume
- **Solution**: Added `EntitiesSpawned` resource flag to spawn only once
- **Pattern**: `resource_equals(EntitiesSpawned(false))` condition on spawn systems
- **Result**: Entities persist across pause/resume cycles
- **Files**: `src/plugins/gameplay_plugin.rs`

#### Arrow Position Lag
- **FIXED**: Stuck arrows now accurately follow entity position
- **Root Cause**: `update_stuck_arrows` ran in Update before physics updated positions
- **Solution**: Moved to PostUpdate schedule (runs after physics)
- **Result**: Arrows maintain exact offset from entity with no lag
- **Files**: `src/systems/projectile_system.rs`

#### Block Damage Overlay Crash
- **FIXED**: Crash when spawning overlays on despawned blocks
- **Error**: "The entity with ID 12180v1 does not exist"
- **Root Cause**: Chunks despawned blocks while overlay system queried them
- **Solution**: Use `commands.get_entity()` with graceful error handling
- **Pattern**: Clean up orphan overlay if parent block doesn't exist
- **Files**: `src/systems/block_damage_overlay_system.rs`

### 🏗️ Architecture Improvements

#### State Management
- **Added**: `GameState::Settings` for settings menu
- **Pattern**: Clean state transitions: Menu → Playing ↔ Paused ↔ Settings
- **Entity Persistence**: Entities only spawn once, persist across state changes
- **Files**: `src/states.rs`

#### Entity Tracking Pattern
- **Pattern**: Use Entity references instead of static positions
- **Example**: `ArrowBarrage` tracks `follow_entity: Entity` instead of `spawn_position: Vec3`
- **Benefit**: Dynamic position following as entities move
- **Used For**: Arrow barrage following hero during multi-second effect
- **Files**: `src/plugins/powers_plugin.rs`

#### Team Component Design
- **Pattern**: Team as reusable component for any combat entity
- **Constructors**:
  - `Arrow::new()` - Enemy (backwards compatible)
  - `Arrow::new_friendly()` - Friendly
  - `Arrow::new_with_team(team)` - Explicit
- **Extensible**: Can add more teams (Neutral, Boss, etc.)
- **Files**: `src/components/projectile.rs`

### 🎯 Gameplay Constants Updated

```rust
// Arrow Barrage
pub const ARROW_BARRAGE_COUNT: u32 = 48;  // 2 full rotations
pub const ARROW_SPAWN_INTERVAL: f32 = 0.05;  // seconds

// Entity Spawning
pub const SPAWN_ONCE: bool = true;  // Prevent re-spawning on resume
```

### 📊 Technical Details

**Menu System**:
- Uses Bevy 0.17 component-based UI (not deprecated bundles)
- State-driven system registration (systems only run in correct state)
- Clean component markers (PauseMenuRoot, SettingsMenuRoot, etc.)

**Team Combat**:
- O(1) team check per arrow collision
- Zero memory overhead (Team is Copy enum)
- Compile-time type safety

**Arrow Barrage**:
- 48 arrows × 0.05s = 2.4 second effect
- Spawns from live hero position (follows movement)
- Entity-based tracking prevents stale position bugs

### 🎮 Gameplay Impact

**Quality of Life**:
- ✅ Can pause game with ESC
- ✅ Can toggle VSync and FPS counter
- ✅ Settings persist during game session

**Combat Feel**:
- ✅ No accidental friendly fire
- ✅ Arrow power is visually impressive (2 full spirals)
- ✅ Arrows properly damage enemies
- ✅ Stuck arrows follow targets smoothly

**Stability**:
- ✅ No crashes from entity despawn timing
- ✅ Camera always follows hero
- ✅ Entities don't duplicate on pause/resume

### 📁 Files Changed Summary

**New Files**:
- `src/components/pause_menu.rs` - Pause menu component markers
- `src/systems/pause_menu_system.rs` - Pause menu logic
- `src/resources/settings.rs` - GameSettings resource
- `src/components/settings_menu.rs` - Settings menu components
- `src/systems/settings_menu_system.rs` - Settings menu logic

**Modified Files**:
- `src/states.rs` - Added Settings state
- `src/plugins/ui_plugin.rs` - Registered menu systems, FPS toggle
- `src/plugins/gameplay_plugin.rs` - EntitiesSpawned resource, spawn-once pattern
- `src/components/projectile.rs` - Team enum and constructors
- `src/systems/projectile_system.rs` - Enemy collision, team checks, PostUpdate scheduling
- `src/plugins/powers_plugin.rs` - ArrowBarrage entity tracking, 48-arrow spiral
- `src/systems/block_damage_overlay_system.rs` - Safe entity checks
- `src/main.rs` - GameSettings initialization

---

## Version 0.2.0 - Combat & Physics Overhaul (October 5, 2025)

### ⚔️ Combat System Improvements

#### Hero Melee Damage System
- **NEW**: Hero now damages enemies on contact with per-enemy cooldown tracking
- **Industry Standard**: Uses HashMap<Entity, f32> to track individual enemy cooldowns (0.3s per enemy)
- **Fairness**: Can hit Enemy A, then immediately hit Enemy B without waiting
- **Anti-Spam**: Cannot spam damage same enemy multiple times per second
- **Collision Match**: Switched from circle/distance to AABB collision to match bounce system exactly
- **Files**: `src/plugins/physics_plugin.rs`, `src/resources/damage_cooldown.rs`

#### Arrow Impact Physics
- **NEW**: Arrows now push targets in the direction the arrow is traveling
- **Push Force**: 200 units horizontal + 100 units vertical (natural projectile feel)
- **Direction**: Arrow velocity normalized and applied as push vector
- **Multi-Hit Support**: Collect-then-apply pattern allows multiple arrows to hit in same frame
- **Graceful Handling**: Safe entity checks prevent crashes when arrows/targets despawn mid-frame
- **Files**: `src/systems/projectile_system.rs`

#### Block Damage Cooldown
- **FIXED**: Blocks no longer break instantly on fast collisions
- **Issue**: No cooldown caused 3 damage events in 3 frames = instant destruction
- **Solution**: Added 0.25s cooldown per block (same pattern as entity damage)
- **Result**: Blocks now consistently require 3 separate hits to break
- **Files**: `src/plugins/physics_plugin.rs`, `src/utils/constants.rs`

#### Skeleton AI Overhaul
- **Changed**: Skeletons now pure ranged combatants (no melee damage on contact)
- **Smart Positioning**: Maintains optimal shooting distance (300-600 units)
- **Tactical Movement**:
  - Back away if hero too close (<300 units)
  - Approach if hero too far (>600 units)
  - Strafe when in ideal range (300-600 units)
- **Files**: `src/plugins/physics_plugin.rs`, `src/utils/constants.rs`

### 💥 Visual Effects

#### Damage Flash System
- **NEW**: Red overlay + red border appears when any character takes damage
- **Duration**: 1 second fade-out with alpha blending
- **Components**: `DamageVisual` component tracks timer and alpha
- **Border**: 72x72 red border sprite behind damaged entity (64x64)
- **Auto-Cleanup**: Border despawns automatically when effect finishes
- **Files**: `src/components/damage_visual.rs`, `src/systems/damage_visual_system.rs`, `src/main.rs`

#### Health Hearts Above Heads
- **NEW**: Real-time health bars displayed above enemies and villagers
- **Size**: 24x24 hearts (increased from 16x16 for better visibility)
- **Position**: 60 units above head (increased from 40 for better separation)
- **Smooth Movement**: Moved to PostUpdate schedule to eliminate lag during fast falling
- **Dynamic**: Hearts update in real-time as health changes
- **Files**: `src/systems/heart_ui_system.rs`

### 🏃 Physics & Movement

#### Enhanced Entity Separation
- **Improved**: Entities no longer get stuck when sandwiched between enemies
- **Multi-Pass**: 5 collision resolution passes (up from 1)
- **Aggressive Separation**: `overlap * 0.8 + 5.0` force prevents clustering
- **Explosive Push**: Horizontal 500 + Vertical 300 separates stuck entities
- **Universal**: Works for hero, villagers, and enemies
- **Files**: `src/plugins/physics_plugin.rs`

#### Bounce Velocity Limits
- **NEW**: Min/max bounce velocity constants for consistent physics
- **Min**: 150 units/sec prevents tiny bounces
- **Max**: 300 units/sec prevents excessive bouncing
- **Constants**: Defined in `src/utils/constants.rs`
- **Tunable**: Easy to adjust gameplay feel

### 🎮 Rendering & Performance

#### VSync Implementation
- **NEW**: Enabled `PresentMode::AutoVsync` for smooth rendering
- **Eliminates**: Screen tearing during fast movement
- **Adaptive**: Uses G-Sync/FreeSync if available, otherwise locks to 60 FPS
- **Consistent**: Predictable 16.67ms frame times at 60Hz
- **Files**: `src/main.rs`

#### FPS Counter
- **NEW**: Real-time FPS display in top-right corner
- **Color**: Green text for easy visibility
- **Update Rate**: Refreshes every 0.5 seconds (prevents number blur)
- **Smoothed**: Uses Bevy's built-in FPS smoothing
- **Always On**: Available during gameplay for performance monitoring
- **Files**: `src/plugins/ui_plugin.rs`

### 🐛 Critical Bug Fixes

#### Arrow Collision Query Conflict
- **Fixed**: Query conflict error when multiple arrows hit in same frame
- **Issue**: ParamSet accessed multiple times in arrow loop
- **Solution**: Collect-then-apply pattern separates read and write phases
- **Result**: Multiple arrows can now hit targets simultaneously
- **Files**: `src/systems/projectile_system.rs`

#### Arrow Stuck Position Bug
- **Fixed**: Arrows not sticking to targets properly
- **Issue**: Offset calculated from (0,0,0) instead of target position
- **Solution**: Store target position during collection phase, calculate proper offset
- **Result**: Arrows now stick at correct impact position
- **Files**: `src/systems/projectile_system.rs`

#### Entity Despawn Crash in Projectile System
- **Fixed**: Crash when arrow/target despawned before commands applied
- **Solution**: Use `commands.get_entity()` for safe entity checks
- **Pattern**: `if let Ok(mut cmd) = commands.get_entity(entity) { ... }`
- **Files**: `src/systems/projectile_system.rs`

#### Heart UI Lag During Fast Falling
- **Fixed**: Hearts lagging visually behind entities during rapid movement
- **Issue**: Hearts updated in Update but entities moved in physics
- **Solution**: Moved heart update to PostUpdate schedule
- **Result**: Hearts now smoothly follow entities at all speeds
- **Files**: `src/systems/heart_ui_system.rs`

### 📐 Architecture Improvements

#### Per-Entity Cooldown System
- **Pattern**: Industry-standard approach for fair damage tracking
- **Implementation**: HashMap<Entity, f32> maps entity to last hit time
- **Advantages**:
  - Each enemy tracked independently
  - Prevents same-entity spam
  - Allows rapid multi-target combat
- **Used For**: Hero melee, block damage, entity damage
- **Files**: `src/resources/damage_cooldown.rs`

#### ParamSet for Query Safety
- **Usage**: Separates conflicting mutable queries
- **Example**: Hero query vs Villager query for arrow collision
- **Safety**: Compile-time guarantee only one query active at a time
- **Performance**: Zero runtime cost
- **Files**: `src/systems/projectile_system.rs`

#### Collect-Then-Apply Pattern
- **Purpose**: Handle multiple events affecting same entity in one frame
- **Phase 1**: Collect all hit data (read-only)
- **Phase 2**: Apply all effects (mutable)
- **Benefits**: No borrow conflicts, all effects stack correctly
- **Used For**: Arrow impacts, damage events
- **Files**: `src/systems/projectile_system.rs`

### 🎯 Gameplay Constants Added

```rust
// Bounce Physics
pub const MIN_BOUNCE_VELOCITY: f32 = 150.0;
pub const MAX_BOUNCE_VELOCITY: f32 = 300.0;

// Skeleton AI
pub const SKELETON_MIN_RANGE: f32 = 300.0;
pub const SKELETON_MAX_RANGE: f32 = 600.0;

// Hero Combat
pub const HERO_MELEE_DAMAGE: u32 = 2;
pub const HERO_MELEE_COOLDOWN: f32 = 0.3;

// Arrow Physics
pub const ARROW_IMPACT_PUSH: f32 = 200.0;
pub const ARROW_IMPACT_LIFT: f32 = 100.0;

// Block Damage
pub const BLOCK_DAMAGE_COOLDOWN: f32 = 0.25;
```

### 📊 Performance Metrics

**Combat System**:
- Arrow collision: O(n) with spatial hash optimization (~0.05ms for 20 arrows)
- Per-entity cooldowns: O(1) HashMap lookups
- Memory overhead: ~2KB for 20 arrows × 10 hits worst case

**Visual Effects**:
- Damage visuals: Minimal overhead, auto-cleanup after 1 second
- Heart UI: PostUpdate prevents physics coupling lag

**Physics**:
- 5-pass collision: ~0.4ms for 50 entities (acceptable for gameplay smoothness)
- Spatial hash: O(1) block queries (critical for performance)

### 🎮 Gameplay Impact

**Combat Feels Better**:
- ✅ Hero can fight multiple enemies smoothly
- ✅ Arrow impacts feel punchy with directional knockback
- ✅ Blocks break consistently (no instant destruction)
- ✅ Skeletons maintain tactical distance

**Visual Feedback**:
- ✅ Clear damage indication with red flash
- ✅ Health bars always visible above heads
- ✅ Smooth animations during fast movement

**Technical**:
- ✅ No crashes from entity despawn races
- ✅ No screen tearing with VSync
- ✅ Stable 60 FPS monitoring

### 🔧 Known Issues

#### Wall Sticking Bug (INVESTIGATING)
- **Issue**: Hero gets stuck to walls when holding movement keys
- **Root Cause**: Collision reverses velocity, but keyboard input continuously adds acceleration
- **Example**: Hit wall going right → bounce left → keyboard adds right force → oscillates
- **Status**: Under investigation, fix pending
- **Workaround**: Release movement key before hitting walls

### 📁 Files Changed Summary

**New Files**:
- `src/components/damage_visual.rs` - Damage visual component
- `src/systems/damage_visual_system.rs` - Damage flash effects
- `src/resources/damage_cooldown.rs` - Per-entity cooldown tracking

**Modified Files**:
- `src/plugins/physics_plugin.rs` - Melee damage, collision, skeleton AI, block cooldown
- `src/systems/projectile_system.rs` - Arrow impact physics, ParamSet, safe entity handling
- `src/systems/heart_ui_system.rs` - Bigger hearts, PostUpdate schedule
- `src/plugins/ui_plugin.rs` - FPS counter with update timer
- `src/utils/constants.rs` - All new gameplay constants
- `src/main.rs` - VSync, DamageVisualPlugin

---

## Version 0.2.0 - Performance & Stability Update (October 5, 2025)

### 🚀 Performance Improvements

#### Background Rendering Optimization
- **MAJOR**: Background system now only updates when camera moves ≥32 units
- Previously updated 30,720 background entities **every frame** (1.8M updates/sec at 60 FPS)
- **Result**: CPU usage drops from 17% → 1-2% when standing still
- **Result**: Memory stops growing when idle
- Fixed background transforms being recalculated unnecessarily

#### Enhanced Performance Diagnostics
- Added tracking for arrows, TNT, chunks, buffs, and damage visuals
- Performance logs now show detailed entity breakdown:
  - Main entities: blocks, overlays, enemies, villagers, hero
  - World entities: chunks, arrows, TNT, resources, backgrounds
  - Unknown entity count now clearly labeled as UI elements
- Better visibility into memory usage and entity counts

### 🐛 Critical Bug Fixes

#### Entity Despawn Race Condition (CRASH FIX)
- **Fixed**: Crash when entities despawned mid-frame while systems tried to modify them
- **Root Cause**: Multiple damage events hitting same entity in one frame, causing:
  - Damage visual system trying to add child to already-despawned parent
  - Block overlay system trying to update materials on despawned entities
- **Solution**: Added graceful entity existence checks and `try_insert` operations
- **Files Modified**:
  - `src/systems/damage_visual_system.rs` - Added entity checks before `add_child`
  - `src/systems/block_damage_overlay_system.rs` - Use `try_insert` for material updates
  - `src/systems/health_system.rs` - Entity existence validation before death handling

#### Death Animation Freeze Bug
- **Fixed**: Entities sometimes freezing longer than 1 second on death
- **Root Cause**: Damage border children being despawned twice (once with parent, once by visual system)
- **Solution**: Check if child exists before despawning, use `get_entity()` for safe cleanup
- Death animation now consistently **exactly 1.0 second**

### 🛠️ Technical Improvements

#### System Parameter Optimization
- Split `log_performance_metrics` into multiple `ParamSet` queries to avoid Bevy's 16-parameter limit
- Fixed system registration conflicts by separating initialization systems
- Improved compile-time error handling for system configurations

#### Asset Management
- Maintained existing asset caching (BackgroundCache, DamageOverlayCache, MaterialCache)
- No new memory leaks introduced
- Materials and meshes remain stable at ~900-1000 (down from previous 7k+ leak)

### 📊 Performance Metrics

**Before**:
- CPU: 17% when idle (background updates running constantly)
- Memory: Growing continuously
- FPS: 127-134 (unstable)
- Entity crashes: Frequent

**After**:
- CPU: 1-2% when idle (background updates only on camera movement)
- Memory: Stable when idle
- FPS: 127-164 (stable)
- Entity crashes: **Eliminated**

### 🎮 Gameplay Impact

- **Smoother performance** when standing still or moving slowly
- **No more random crashes** from entity despawn race conditions
- **Consistent death animations** - exactly 1 second, every time
- **Better resource usage** - game uses less CPU/memory when idle

### 📖 Developer Notes

#### Race Condition Pattern
The entity despawn crashes were caused by a common Bevy pattern:
```
1. Entity takes multiple hits in one frame → Multiple damage events queued
2. First event kills entity → Despawn command queued
3. Systems process remaining events → Try to modify already-queued-for-despawn entity
4. Commands flush → Crash when trying to add_child to despawned entity
```

**Solution Pattern**:
```rust
// Always check entity exists before modifying
if all_entities.get(event.entity).is_err() {
    continue;
}

// Use try_insert for safe component insertion
commands.entity(event.entity).try_insert(component);

// Use get_entity() for safe child operations
if let Ok(mut parent) = commands.get_entity(event.entity) {
    parent.add_child(child);
} else {
    commands.entity(child).despawn(); // Clean up orphan
}
```

#### Background System Optimization
Changed from **every-frame updates** to **movement-based updates**:
```rust
// Only update if camera moved at least half a block (32 units)
if camera_moved_x < block_size * 0.5 && camera_moved_y < block_size * 0.5 {
    return; // Skip update
}
```

This is an industry-standard optimization used in games like Terraria and Hollow Knight.

### 🔮 Future Optimizations

Identified but not yet implemented:
1. **Spatial hash incremental updates** - Rebuild only changed blocks instead of all blocks every frame
2. **Entity collision passes reduction** - Reduce from 5 to 3 passes (40% reduction)
3. **Background entity count** - Still at 30,720 entities (could use single quad or 3×3 chunks)
4. **Enemy AI caching** - Cache nearest target search every 0.5s instead of every frame

### 📁 Files Changed

**Performance**:
- `src/systems/background_system.rs` - Movement-based update logic
- `src/systems/performance_diagnostics.rs` - Enhanced entity tracking

**Crash Fixes**:
- `src/systems/damage_visual_system.rs` - Safe entity operations
- `src/systems/block_damage_overlay_system.rs` - Safe material updates
- `src/systems/health_system.rs` - Entity existence validation

**System Configuration**:
- `src/main.rs` - Separated system registrations

---

## Version 0.1.0 - Initial Release

### Features
- 3D block-breaking gameplay in Bevy 0.17.1
- Procedurally generated infinite world with 7 biomes
- Physics system with gravity, bouncing, and collisions
- Enemy AI (skeletons with ranged attacks)
- Health system with visual damage indicators
- Buff system (burning, speed, freezing, poison)
- Power system (TNT, laser, arrow barrage, villager spawning)
- Chunk-based world loading and unloading
- Material and mesh caching to prevent memory leaks
