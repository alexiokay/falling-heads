# Release Notes - Physics & Combat Overhaul

## 🎮 Major Features

### 🎯 Visual Damage Feedback System
- **Red damage overlay**: Characters flash red when taking damage with smooth 1-second fade-out animation
- **Red border effect**: Damaged entities display a red outline that fades out simultaneously
- **Death animation**: Dead entities freeze in place for 1 second with damage visual before despawning
- **Universal application**: Works on all entities (Hero, Villagers, Enemies)
- **New damage visual component system** (`DamageVisual`) with automatic cleanup

### ⚔️ Combat System Improvements

#### Skeleton AI Overhaul
- **Ranged-only combat**: Skeletons now maintain distance and never engage in melee
- **Smart positioning**:
  - Ideal range: 300-600 units (stays and shoots)
  - Too close (<300): Rapidly backs away
  - Too far (>600): Slowly approaches
- **No melee damage**: Skeletons only damage via arrows, not physical contact
- **Tactical behavior**: Creates challenging ranged encounters

#### Arrow & Projectile System
- **Hero now takes arrow damage**: Removed hero invincibility for projectiles
- **Stuck arrow visuals**: Arrows stick to targets for 2 seconds showing impact
- **Smooth arrow rotation**: Arrows rotate to point in direction of travel
- **Proper collision detection**: Uses 2D distance calculations (ignoring Z-axis)
- **Block collision**: Arrows properly stop at walls using spatial hash

### 🏀 Physics Engine Enhancements

#### Enhanced Collision Response
- **Multi-pass resolution**: 5 collision resolution passes (up from 1) to handle complex entity clusters
- **Aggressive separation**:
  - Separation force: `overlap * 0.8 + 5.0` (massively increased)
  - Horizontal push: 500 units/sec when overlapping
  - Vertical explosion: 300 units/sec minimum upward bounce
- **Horizontal spreading**: Vertically stacked entities get pushed sideways to break up clusters
- **No more stuck entities**: Characters forcefully separate when sandwiched between multiple enemies

#### Bouncing Improvements
- **Minimum bounce velocity**: 150 units/sec (characters always bounce satisfyingly)
- **Maximum bounce velocity**: 300 units/sec (prevents unrealistic super-bounces)
- **Configurable bounce**: All bounce values now in `constants.rs`
- **Consistent physics**: All entities (Hero, Villagers, Enemies) use same Bouncy component

#### Entity-Specific Physics
- **Unified entity physics**: Single system handles Hero + Villagers + Enemies
- **Size-aware collisions**: Proper collision detection using actual entity sizes
- **Preserved momentum**: Bouncing now preserves horizontal velocity better

## 🔧 Technical Improvements

### Code Architecture
- **Spatial Hash optimization**: Fast collision queries for blocks (256-unit chunks)
- **Component bundles**: `PhysicsBundle` and `SpriteBundle3d` for cleaner entity spawning
- **Modular constants**: All game constants centralized in `constants.rs`
- **System organization**: Physics systems grouped into logical SystemSets (Forces → AI → Movement → Collision)

### New Components & Systems
- `DamageVisual` - Visual damage effect component with timer
- `DeathAnimation` - Freezes dead entities for 1 second before despawn
- `DamageVisualPlugin` - Handles damage flash overlays and borders
- `SpatialHash` - Fast spatial partitioning for collision queries
- `Bundles` module - Reusable entity component bundles

### Constants Expansion
Added 40+ new configurable constants including:
- Entity sizes (Hero, Villager, Enemy, Arrow, Resource)
- Bounce velocities (min/max)
- Skeleton AI ranges (ideal, min, max)
- Spawn rates per biome
- Buff application chances
- TNT explosion parameters (radius, damage, fuse time, blink speed)
- Laser dimensions
- Resource physics

### Cleanup & Refactoring
- **Deleted old physics systems**: Removed 6 legacy physics files from `systems/physics/`
- **Deleted duplicate code**: Removed `physics_plugin_new.rs`
- **Unified physics**: Merged Hero, Villager, and Enemy collision into single system
- **Code comments**: Added detailed explanations for complex physics calculations

## 🐛 Bug Fixes
- ✅ Fixed entities getting stuck inside each other
- ✅ Fixed entities being unable to escape when sandwiched
- ✅ Fixed hero not taking arrow damage
- ✅ Fixed weak bounces from blocks
- ✅ Fixed skeletons doing melee damage (should be ranged-only)
- ✅ Fixed skeleton AI always chasing (should keep distance)

## 📊 Performance
- **Spatial hash**: O(1) block collision queries instead of O(n)
- **Multi-pass collision**: Better separation with only 5 iterations (still fast)
- **Optimized queries**: Proper query filters reduce unnecessary iterations

## 🎨 Visual Improvements
- Red damage flash effect with smooth fade-out
- Red border outline for damaged entities
- Death freeze animation - entities stay visible for 1 second when killed
- Stuck arrow visuals showing impact points
- TNT blinking effect - blinks faster as explosion approaches (Minecraft-style)
- More dynamic bouncing behavior

## 📝 Configuration
All new constants are easily tunable in `src/utils/constants.rs`:

```rust
// Bounce physics
pub const MIN_BOUNCE_VELOCITY: f32 = 150.0;
pub const MAX_BOUNCE_VELOCITY: f32 = 300.0;

// Skeleton behavior
pub const SKELETON_IDEAL_RANGE: f32 = 400.0;
pub const SKELETON_MIN_RANGE: f32 = 300.0;
pub const SKELETON_MAX_RANGE: f32 = 600.0;

// And 40+ more...
```

## 🎯 Gameplay Impact
- **More challenging combat**: Skeletons now kite the player effectively
- **Better game feel**: Satisfying bounces and visual damage feedback
- **TNT now damages entities**: Explosions deal damage and push entities away
- **Fairer difficulty**: Hero can now be damaged by arrows like other entities
- **Smoother physics**: Entities no longer get stuck in clusters
- **Clearer feedback**: Red flash clearly indicates when taking damage
- **Death clarity**: 1-second freeze shows exactly where and how entities died

## 📁 Files Changed
- **29 files modified**, 791 insertions, 729 deletions
- **4 new files**: `damage_visual.rs`, `damage_visual_system.rs`, `bundles.rs`, `spatial_hash.rs`
- **6 files deleted**: Old physics system modules
- **Key updates**: `physics_plugin.rs` (569 lines), `constants.rs` (74 lines), `projectile_system.rs` (132 lines)

---

**Version**: Post-Physics-Refactor
**Date**: 2025-10-05
**Status**: Ready for Testing
