# Release Notes - Version 0.3 (FPS Limiting & Unfocused Rendering)

## ⚙️ Settings & Performance

### FPS Limiting System
- **Added configurable FPS limiter** with precise frame pacing using `bevy_framepace`
  - Default: 60 FPS limit (configurable in settings)
  - Cycle through presets: 30 → 60 → 120 → 144 → 240 → Unlimited
  - Works independently of window focus (no more FPS drops when unfocused)
  - Perfect for streaming/recording - consistent FPS whether game is focused or not

### VSync Integration
- **Smart VSync/FPS limiter coordination**
  - When VSync is ON: FPS limiter automatically disables (VSync controls frame rate)
  - When VSync is OFF: FPS limiter takes over with precise frame timing
  - Default changed to VSync OFF + 60 FPS limit (less input lag than VSync)
  - FPS Limit button visually grays out when VSync is enabled

### Settings Menu Updates
- **New "FPS Limit" button** in settings menu
  - Green button when active, grayed out when VSync enabled
  - Click to cycle through FPS presets
  - Shows current limit or "Unlimited"
  - Disabled interaction when VSync is on (visual + functional)

### Files Modified
1. **Cargo.toml**
   - Added `bevy_framepace = "0.19"` dependency
2. **src/resources/settings.rs**
   - Added `fps_limit: u32` field to `GameSettings`
   - Changed default: `vsync_enabled: false`, `fps_limit: 60`
3. **src/main.rs**
   - Added `WinitSettings` with continuous update mode (focused + unfocused)
   - Integrated `FramepacePlugin` with conditional limiter setup
   - Added `apply_fps_limit()` system to dynamically adjust FPS when settings change
4. **src/components/settings_menu.rs**
   - Added `FpsLimitButton` component
5. **src/systems/settings_menu_system.rs**
   - Added FPS Limit button to UI (grayed out when VSync enabled)
   - Added cycling logic for FPS presets
   - Added dynamic color/text updates when VSync toggles
   - Hover effects disabled on FPS button when VSync is on


# Release Notes - Version 0.3 (Fireball Collision & Latest Fixes)

## 🔥 Ghast Fireball Fixes

### Fireball Collision System
- **Fixed fireballs not exploding on character contact** - Fireballs now properly explode when touching heroes, villagers, and enemies
  - Added missing `CollisionLayers` to fireball spawn (using Snowball layer)
  - Increased fireball spawn offset from 60.0 to 100.0 units to prevent immediate collision with shooting ghast
  - Root cause: Fireballs had Sensor + CollisionEventsEnabled but NO CollisionLayers, so Avian didn't know what to collide with

### Files Modified
1. **src/systems/enemy_ai_system.rs** (lines 267-299)
   - Added `CollisionLayers::new(GameLayer::Snowball, ...)` to fireball spawn
   - Increased spawn offset to 100.0 (outside ghast's 80-radius collider)


# Release Notes - Version 0.3 (Cave Generation Overhaul)

## 🕳️ Cave System Complete Redesign

### Corridor-Based Cave Generation
- **Completely redesigned cave system** with corridor-based structures instead of large organic caves
- Caves are now **much smaller** (2-4 blocks wide typically) and **more structured**
- Each biome has **unique corridor patterns and shapes**

### Biome-Specific Cave Structures

#### 🔥 Nether
- **Vertical wavy corridors** (lava tubes)
- 2-4 blocks wide with meandering patterns
- Small pocket rooms scattered throughout

#### 🌌 End
- **Diagonal crystalline corridors** (geometric patterns)
- Straight diagonal tunnels with sparse distribution
- Unique angular geometry matching End theme

#### ⛏️ Underground
- **Classic dungeon layout** with horizontal and vertical winding corridors
- Multiple intersecting tunnels (2-3 blocks wide/tall)
- Larger rooms at corridor intersections

#### 🏔️ Mountains
- **Horizontal mining shaft corridors** at different heights
- Multiple parallel shafts running horizontally
- Small caverns scattered throughout

#### ❄️ Iceland
- **Thin jagged ice cracks** (1-2 blocks wide)
- Irregular crack patterns
- Very small ice pockets

#### 🏜️ Desert
- **Horizontal tomb corridors** (sparse distribution)
- Ancient tomb-like passages
- Small tomb rooms

#### 🌳 Forest
- **Twisting organic root corridors**
- Both horizontal and vertical twisting paths
- Root cavities (small natural chambers)

#### 🌾 Plains
- **Rare shallow horizontal burrows** (very sparse)
- Minimal cave presence

### Key Features
- ✅ **Biome-specific generation** - Each biome has unique cave patterns
- ✅ **Small and structured** - No more massive open caves
- ✅ **Corridor-focused** - Explorable tunnel systems
- ✅ **Generative variety** - Each cave has natural variation
- ✅ **Better enemy spawning** - Enemies spawn in cave systems

### Technical Implementation
- Replaced large noise-based caves with corridor generation algorithms
- Each biome uses custom pattern matching for corridor shapes
- Vertical, horizontal, and diagonal corridor support
- Dynamic corridor width variation based on biome

### Files Modified
1. **src/systems/world_generation.rs** (lines 226-358)
   - Complete rewrite of `should_generate_cave()` function
   - Implemented 8 unique biome-specific cave patterns
   - Changed from organic noise caves to structured corridors

---

# Release Notes - Version 0.3 (Physics & Audio Fixes)

## =' Critical Bug Fixes

### Ghast Collision System
- **Fixed visual overlap with Ghasts** - Players can no longer partially phase through Ghast entities
  - Changed Ghast AI from direct velocity assignment to acceleration-based movement
  - Added `LinearDamping(2.0)` to allow Avian's collision response to work properly
  - Adjusted collider size to 80.0 (160x160) to match pattern used by other entities
  - Root cause: AI was overwriting velocity every frame, preventing physics collision separation

### Audio System Memory Leak
- **Fixed permanent audio distortion** - Sounds no longer become corrupted after many entities take damage
  - Added `PlaybackSettings::DESPAWN` to ALL audio spawns (damage, death, bow, blocks)
  - Root cause: AudioPlayer entities were never cleaned up, accumulating infinitely and clogging the audio mixer
  - Affected sounds now auto-despawn when finished playing

### Extreme Velocity Bug (Teleportation Fix)
- **Fixed players being launched to extreme coordinates** - No more teleporting millions of units away
  - Added `clamp_entity_velocities()` system that runs after physics updates
  - Limits horizontal velocity to �1200 units/second (`MAX_ENTITY_HORIZONTAL_SPEED`)
  - Limits vertical velocity to �1500 units/second (`MAX_ENTITY_VERTICAL_SPEED`)
  - Root cause: When many entities overlap, Avian applies cumulative push forces causing exponential velocity buildup

### Dead Entities Buff System
- **Fixed dead entities continuing to take buff damage** - Dead characters no longer burn or generate buff sounds
  - Added `Without<MarkedForDeath>` filter to buff effect systems
  - Added automatic buff removal on death in `handle_entity_death()`
  - Root cause: Buff systems didn't check if entities were dead before applying effects

## <� Code Quality Improvements

### Constants Centralization
- Moved velocity limit constants to `constants.rs`:
  - `MAX_ENTITY_HORIZONTAL_SPEED: f32 = 1200.0`
  - `MAX_ENTITY_VERTICAL_SPEED: f32 = 1500.0`
- Ensures consistent velocity limits across the codebase

### Physics System
- **Ghast AI Movement System** (`ghast_floating_movement`):
  - Now uses gentle acceleration instead of direct velocity control
  - Integrates properly with Avian's collision response
  - Applies damping for smooth, natural floating movement

## =� Technical Details

### Files Modified
1. **src/systems/enemy_ai_system.rs** (lines 363-376)
   - Rewrote ghast movement to use acceleration-based forces
   - Changed from `velocity.x = ...` to `velocity.x += acceleration * delta_time`

2. **src/plugins/external_api_plugin.rs** (lines 294-297)
   - Added `LinearDamping(2.0)` to ghast physics
   - Updated collider size to 80.0 (half-extents) for 160x160 total collision box

3. **src/systems/audio/mob_sounds.rs** (lines 49-175)
   - Added `PlaybackSettings::DESPAWN` to 7 different sound spawn locations
   - Prevents AudioPlayer entity memory leak

4. **src/plugins/physics_plugin.rs** (lines 1070-1088)
   - Added new `clamp_entity_velocities()` system
   - Runs in `PhysicsSet::Collision` after spatial hash rebuild

5. **src/systems/buffs/buff_core.rs** (lines 41, 82)
   - Added `Without<MarkedForDeath>` to buff effect queries
   - Prevents buff effects on dead entities

6. **src/systems/health_system.rs** (line 73)
   - Added `.remove::<Buff>()` to death handler
   - Cleans up all buffs when entity dies

7. **src/utils/constants.rs** (lines 29-30)
   - Added velocity limit constants

## = Known Issues Addressed

### Issue #1: Ghast Overlap
- **Symptom**: Players could stand inside Ghast sprite without bouncing
- **Cause**: Ghast AI set velocity directly, overwriting Avian's collision forces
- **Fix**: Acceleration-based movement + LinearDamping + proper collider size
- **Status**:  RESOLVED

### Issue #2: Audio Distortion
- **Symptom**: After spawning many villagers and hitting them, all sounds become permanently distorted
- **Cause**: Thousands of AudioPlayer entities never despawned, overwhelming the audio mixer
- **Fix**: PlaybackSettings::DESPAWN on all sound spawns
- **Status**:  RESOLVED

### Issue #3: Teleportation Bug
- **Symptom**: Spawning many villagers causes player to be launched millions of units away, crashing with AABB assertion error
- **Cause**: Cumulative collision forces from multiple overlapping entities with no velocity cap
- **Fix**: Velocity clamping system limits maximum speeds
- **Status**:  RESOLVED

### Issue #4: Ghost Buffs on Dead Entities
- **Symptom**: Dead characters continue burning and making damage sounds
- **Cause**: Buff systems didn't filter out dead entities
- **Fix**: Filter dead entities + remove buffs on death
- **Status**:  RESOLVED

## = Comparison to Previous Commit (765c0d8)

**Previous commit message**: "switched physics system to avian, some world generation (caves) fixed bugs with meshed blocks like pumpkins not getting visual damage overaly and changes ome movement and stuff"

**This update focuses on**:
- Critical physics stability (velocity clamping)
- Audio system reliability (memory leak fix)
- Ghast collision accuracy (overlap prevention)
- Buff system correctness (dead entity handling)

All fixes address core gameplay bugs that significantly impact player experience.

---

**Build Status**:  Compiles successfully with 86 warnings (all pre-existing, no new warnings introduced)

**Testing Recommendations**:
1. Spawn multiple ghasts and verify no visual overlap when colliding
2. Spawn many villagers, hit them repeatedly, verify audio stays clean
3. Spawn 20+ villagers around player, verify no extreme velocity/teleportation
4. Kill burning entities, verify they stop taking damage and making sounds

---

## UI/UX Improvements

### Resource Display Overhaul
- **Redesigned resource UI** with vertical icon-based layout in top-left corner
- Resources now display with **icons (32px) instead of text names**
- Added visual icons for: Coal, Iron, Gold, Diamond, Emerald, Redstone, Netherite
- Font size increased to **28pt** for better readability
- All resource UI elements now **scale with window size** using viewport-relative units (vw/vh)

### Experience System
- **Added XP bar** at bottom center of screen with level progression
- XP gained from:
  - **Mob kills**: Skeleton (15 XP), Zombie (10 XP), Yeti (20 XP), Enderman (30 XP), Dragon (100 XP)
  - **Resource collection**: Coal (2), Iron (3), Gold (5), Diamond (10), Emerald (15), Netherite (25)
  - **Block breaking**: 1 XP per block
- XP bar properly scales with window (40vw width, 3vh height)
- Fixed texture clipping for smooth segment display using clip-based rendering

### Game Information Display
- **Added Y coordinate display** showing player's vertical position
- **Added difficulty level indicator** (orange text, 24pt)
- **Added Day/Night cycle indicator** (yellow for day, dark blue for night, 24pt)
- All info displays positioned in top-left below resources with proper spacing

### Heart/Health UI Improvements
- **Hearts now removed when fully lost** instead of showing empty containers
- **Remaining hearts automatically center** above entities after damage
- Fixed centering to work **immediately with any damage type** (melee, TNT, projectiles)
- System now counts actual visible hearts instead of calculating mathematically
- Prevented integer overflow crashes with `saturating_sub()`

### Day/Night Cycle Implementation
- **5-minute day/night cycle** (2.5 minutes day, 2.5 minutes night)
- Created `DayNightCycle` resource to track game time
- Visual indicator updates in real-time with color changes

### Visual Effects Fixes
- **Fixed buff/debuff visual layering** - effects now render **IN FRONT** of characters
- Changed Z-position from -2.0 (behind) to +2.0 (in front)
- Burning debuff fire overlay now properly visible on top of entities

## Technical Improvements

### System Architecture
- Added `WorldConfig` resource initialization to main app
- Fixed all XP system events to use custom `Message` trait instead of Bevy's `Event`
- Improved heart UI system with **two-pass rendering** for accurate centering
- All UI elements converted to viewport-relative units (vw/vh) for proper window scaling

### Bug Fixes
- Fixed resource UI not displaying on first load
- Fixed difficulty display not updating
- Fixed XP bar texture stretching with clip-based rendering
- Fixed heart centering calculation with proper visible heart counting
- Fixed query conflicts in heart UI system (Transform mutable borrow)
- Prevented crashes from integer underflow in heart positioning

### Files Modified (UI Update)
1. **src/plugins/ui_plugin.rs**
   - Redesigned resource display with icons and vertical layout
   - Added Y coordinate, difficulty, and day/night displays
   - Converted all UI to viewport-relative sizing

2. **src/systems/ui/xp_bar.rs**
   - Implemented XP bar with proper scaling
   - Fixed texture clipping with container-based rendering

3. **src/systems/ui/heart_ui.rs**
   - Implemented heart removal and centering logic
   - Two-pass system for accurate positioning

4. **src/systems/progression/experience.rs**
   - Implemented XP gain from mobs, resources, and blocks
   - Fixed event system to use Message trait

5. **src/resources/day_night_cycle.rs** (NEW)
   - Created day/night cycle resource

6. **src/systems/buffs/burning_visual.rs**
   - Fixed Z-axis positioning for fire overlay

7. **src/main.rs**
   - Added WorldConfig resource initialization
