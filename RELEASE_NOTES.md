# MineHeads - Release Notes

## Version 0.2.5 - Weather Particle Improvements (October 8, 2025)

### 🌧️ Weather Particle Visual Enhancements
- **IMPROVED**: Weather particles are now 1.5x bigger (192x768 from 128x512)
  - Size variation: 0.9x to 1.3x for natural variety
  - Much more visible, especially snow particles
- **IMPROVED**: All weather particles now use full opacity (alpha = 1.0)
  - Removed transparency reduction that made particles hard to see
  - Rain, snow, and all weather effects now clearly visible
- **IMPROVED**: Weather-specific particle spawn rates to prevent accumulation
  - Snow: 1.5 particles/0.1s (reduced from 3.0 to prevent screen clutter)
  - Other weather: 3.0 particles/0.1s
  - Prevents slow-moving particles from accumulating on screen

### 🌨️ Weather Particle Lifetime System
- **NEW**: Weather-type-specific particle lifetimes
  - Rain: 3s (fast falling, reaches bottom quickly)
  - Snow: 12s (slow drift, needs time to cross screen)
  - FireRain: 2s (very fast)
  - AuroraStorm: 15s (very slow, floating effect)
  - Sandstorm: 10s (moderate horizontal movement)
  - AshStorm/BloodMoon: 12s (slow/erratic movement)
  - VoidStorm: 8s (moderate speed)
- **FIXED**: Snow particles no longer disappear in middle of screen
  - Previous 5s lifetime was too short for slow-falling snow
  - Now properly crosses entire screen before despawning

### 🎨 Weather Particle Movement & Distribution
- **IMPROVED**: Snow movement tuned for realistic drift
  - Horizontal drift: -60 to 60 units
  - Fall speed: -120 to -180 units (slower, more gentle)
  - Creates natural floating snowflake effect
- **IMPROVED**: Better particle distribution across screen
  - Uses actual window dimensions (no hardcoded 1920x1080)
  - Spawns across full screen width with margins
  - Varied spawn heights (100-500 units above screen)
  - Depth variation (Z: 80-120) for layered effect
- **IMPROVED**: Screen-relative particle spawning
  - Particles spawn above current camera position
  - Follow player movement smoothly
  - Despawn when far below camera (500 units)
  - Creates seamless overlay effect

### 🔧 Technical Details
- **Files Modified**:
  - `src/systems/weather_system.rs` - Particle size, lifetime, spawning, distribution
  - `src/resources/weather_config.rs` - Updated PARTICLE_LIFETIME constant documentation
- **Query Optimization**: Uses `windows.iter().next()` for proper window dimension access
- **Performance**: Reduced particle counts prevent excessive spawning while maintaining visual quality

### 📊 Particle Lifetime Comparison
| Weather Type | Old Lifetime | New Lifetime | Reason |
|--------------|--------------|--------------|--------|
| Rain | 5s | 3s | Fast falling, optimized |
| Snow | 5s | **12s** | Slow drift needs more time |
| FireRain | 5s | 2s | Very fast, optimized |
| AuroraStorm | 5s | **15s** | Very slow floating |
| Sandstorm | 5s | 10s | Moderate horizontal |
| AshStorm | 5s | 12s | Erratic movement |

---

## Version 0.2.5 - 3D Arrow Model & Improved Collision (October 8, 2025)

### 🏹 3D Arrow Model System
- **NEW**: Arrows now use proper 3D GLB models instead of 2D textures
  - Model file: `assets/models/projectiles/arrow2.glb`
  - Scale: 15x for proper visibility
  - Automatic rotation based on velocity direction
  - Loaded via `GltfAssetLabel::Scene(0).from_asset()` (Bevy 0.17)
- **Visual Upgrade**: Arrows look like actual 3D objects with depth and proper perspective
- **Model Source**: Exported from Blender with standard PBR materials (not SpecularGlossiness)

### 🎯 Tip-Based Collision Detection
- **IMPROVED**: Arrows now use tip-based collision instead of circular hitbox
  - Arrow tip calculated 40 units ahead along velocity direction
  - Collision radius reduced from 30.0 → 10.0 (thin point)
  - Arrows only hit when **tip** penetrates target, not when body passes nearby
- **Bug Fix**: Arrows no longer hit when flying sideways past characters
- **More Accurate**: Collision detection matches visual arrow trajectory
- **Physics**: Proper line-based collision for long thin projectiles

### 🔗 Parent-Child Transform Hierarchy
- **IMPROVED**: Stuck arrows now use Bevy's parent-child entity system
  - Arrows become **child entities** of hit targets using `set_parent_in_place()`
  - Transform updates handled automatically by Bevy's `TransformPropagateSystem`
  - **Zero lag**: Arrows perfectly follow entities when jumping/moving
  - **Performance boost**: No manual position queries every frame
- **Transform Preservation**: Rotation and scale preserved when arrow sticks
  - Rotation frozen at impact angle
  - Scale maintained at 15x
  - Local coordinates used for offset positioning

### 🐛 Bug Fixes
- **FIXED**: Arrows moving away from characters during movement
  - Old system: Manual position update every frame (laggy)
  - New system: Parent-child hierarchy (automatic, zero lag)
- **FIXED**: Stuck arrows appearing at wrong angles
  - Now preserves rotation from impact moment
- **FIXED**: Arrow hitbox too wide causing false hits
  - Changed from circular (30.0 radius) to tip-based (10.0 radius)
- **FIXED**: GLB model loading with unsupported extensions
  - User re-exported model from Blender with standard PBR materials
  - Changed from `arrow.glb` to `arrow2.glb`

### ⚡ Performance Improvements
- **Parent-child hierarchy** is more performant than manual updates:
  - **Before**: Query parent transform + write arrow transform for every stuck arrow (per frame)
  - **After**: Zero cost - Bevy's transform system handles it automatically
  - Removed entire `entity_transforms` query from running every frame
  - SIMD-optimized and cache-friendly transform propagation
- **One-time setup cost** at impact, but better runtime performance

### 🔧 Technical Details
- **Files Modified**:
  - `src/systems/projectile_system.rs` - Tip-based collision, parent-child hierarchy
  - `src/systems/enemy_ai_system.rs` - 3D arrow spawning with SceneRoot
  - `src/plugins/powers_plugin.rs` - Hero arrow barrage with 3D model
  - `src/plugins/asset_loader_plugin.rs` - GLB model loading
  - `src/resources/asset_handles.rs` - ProjectileModels resource
- **Arrow Offset Adjustment**: 20-unit forward adjustment along velocity for GLB model origin
- **Query Conflict Fix**: Added `Without<Block>` to arrow query to prevent Bevy ECS conflicts

### 📊 Collision Detection Comparison
| Method | Accuracy | Performance | False Positives |
|--------|----------|-------------|-----------------|
| Old: Circular (30.0 radius) | Low | Fast | High (hits when passing sideways) |
| **New: Tip-based (10.0 radius)** | **High** | **Fast** | **Low (only tip hits)** |

---

## Version 0.2.5 - Username Labels for Twitch Integration (October 8, 2025)

### 👥 Username Label System
- **NEW**: Spawned villagers now display the Twitch viewer's username above their heads
- **Rendering**: Uses Text2d with dual-camera setup (Camera3d + Camera2d overlay)
- **Positioning**: Labels float above villagers, below hearts (same as heart system)
- **Responsive Scaling**: Font size adapts to window resolution
  - Formula: `(window_height / 720.0) * 16.0`
  - At 720p: 16px font
  - At 1440p: 32px font
  - At 360p: 8px font
- **Unique Colors**: Each username gets a vibrant color based on hash (HSL with 0.9 saturation)
- **Stable Movement**: Sub-pixel rounding prevents jitter
- **Performance**: Same as hearts (~0.01ms per label) - uses 3D→2D projection
- **API Integration**: Works with VILLAGERS command
  ```bash
  curl -X POST http://localhost:3029/command -H "Content-Type: application/json" \
    -d '{"action":"VILLAGERS","username":"CoolViewer123"}'
  ```

### 🎥 Dual-Camera Architecture
- **Camera3d**: Renders 3D game world (blocks, entities, hearts)
  - Order: 0 (renders first)
  - Clear color: Dark blue background
- **Camera2d**: Renders 2D text overlays (username labels)
  - Order: 1 (renders on top)
  - Clear color: None (transparent, doesn't clear 3D scene)
- **Benefits**:
  - Text rendered in screen space (always crisp)
  - No perspective distortion
  - Proper layering (text on top of 3D)

### 🎨 Visual Features
- **World-to-Screen Projection**: Converts 3D villager positions to 2D screen coordinates
- **Auto-hide**: Labels disappear when villagers go off-screen
- **Color Variety**: 360 unique hues generated from username hash
- **Centering**: Accounts for window dimensions dynamically

### 🔧 Technical Implementation
- **Files Added**:
  - Updated `src/systems/username_label_system.rs` - Dual-camera projection system
  - Updated `src/systems/entity_spawn.rs` - Username parameter in spawn functions
  - Updated `src/main.rs` - Camera2d setup
  - Updated `src/plugins/external_api_plugin.rs` - Username forwarding
  - Updated `src/systems/background_system.rs` - Camera3d explicit query

### 📊 Performance Comparison
| Approach | Cost/Frame | Notes |
|----------|------------|-------|
| UI Overlay (initial) | ~0.1ms | World→Screen + UI layout |
| 3D Rectangles | ~0.05ms | No actual text |
| **Current: Text2d** | **~0.01ms** | Same as hearts, actual text |

### 🎮 Gameplay Impact
- ✅ Viewers can see their username above "their" villagers
- ✅ Easy to identify which villagers belong to which viewer
- ✅ Labels resize properly when window is resized
- ✅ Smooth following, no jitter or lag
- ✅ Professional MMO-style nameplate system

### 🐛 Bug Fixes
- **FIXED**: Camera2d not interfering with 3D background rendering
- **FIXED**: Username labels positioned correctly in 2D screen space
- **FIXED**: Text size scaling with window resolution
- **FIXED**: Sub-pixel jitter by rounding screen coordinates

---

## Version 0.2.5 - Mirror Power & Projectile Fixes (October 7, 2025)

### 🪞 New Power: Magic Barrier (MIRROR)
- **NEW**: Mirror power creates a protective shield that reflects projectiles
  - Duration: 8 seconds
  - Radius: 150 units
  - Visual: Cyan semi-transparent circular barrier
  - Follows hero as they move
  - Reflects both arrows and snowballs back at enemies
  - **Team Switching**: Reflected projectiles change teams (enemy → friendly, friendly → enemy)
  - Amplifies reflected velocity by 1.2x for dramatic effect
- **Component**: `MagicBarrier` ([src/components/projectile.rs:82-87](src/components/projectile.rs:82-87))
- **Systems**:
  - `spawn_magic_barrier()` - Creates barrier around hero
  - `barrier_update()` - Follows hero and manages lifetime
  - `barrier_reflect_projectiles()` - Reflects and switches team affiliation
- **API**: Available via `curl -X POST http://localhost:3029/command -d '{"action":"MIRROR"}'`

### 🔧 Bug Fixes
- **FIXED**: Snowballs no longer go through walls
  - Added block collision detection using spatial hash (same as arrows)
  - Snowballs now despawn on wall impact ([src/systems/frozen_system.rs:42-61](src/systems/frozen_system.rs:42-61))
  - Uses proper 2D distance calculation (ignores Z axis)
  - Collision radius: snowball (12.0) + block (32.0)

### 🎮 Gameplay Improvements
- Reflected projectiles now properly damage their new team
  - Enemy arrows reflected by barrier will hit enemies
  - Enemy snowballs reflected by barrier will freeze enemies
  - Creates strategic defensive gameplay

## Version 0.2.5 - Architecture Refactoring (October 6, 2025)

### 🏗️ Architecture Improvements

#### AssetLoaderPlugin
- **NEW**: Centralized asset loading plugin (`src/plugins/asset_loader_plugin.rs`)
- **Clean main.rs**: Removed 150+ lines of asset loading code
- **Organization**: All texture loading in one place
- **Startup**: Loads all assets (textures, blocks, enemies, projectiles) during startup
- **Logging**: Clear initialization messages for debugging

#### SystemSets
- **NEW**: Explicit system execution ordering (`src/utils/system_sets.rs`)
- **Predictable**: Systems run in defined order: Input → AI → Physics → Combat → Effects → World → Visual → UI
- **No Race Conditions**: `.chain()` ensures exact execution sequence
- **Flexibility**: Easy to add new systems to specific phases
- **Best Practice**: Follows Bevy 0.17 system ordering patterns

#### Config Files (RON)
- **NEW**: Data-driven configuration files in `assets/configs/`
- **enemies.ron**: Enemy stats (health, speed, damage, shoot intervals)
- **biomes.ron**: Biome configuration (blocks, ores, enemies, spawn rates)
- **projectiles.ron**: Projectile properties (speed, damage, lifetime, size)
- **Future**: Systems can load from configs instead of hardcoded values

#### Trait Abstractions (Priority 2) - Complete Polymorphic Architecture
- **NEW**: **ProjectileBehavior** trait ([src/components/projectile.rs:12](src/components/projectile.rs:12))
  - Methods: `team()`, `damage()`, `should_despawn()`, `update()`, `push_force()`, `on_hit()`
  - Implemented for `Arrow` (150 push, 1 dmg) and `Snowball` (50 push, 0 dmg, freeze effect)
  - Extensible: Add fireballs, ice shards, magic missiles with consistent interface

- **NEW**: **EnemyBehavior** trait ([src/components/enemy.rs:16](src/components/enemy.rs:16))
  - Methods: `max_health()`, `movement_speed()`, `shoot_interval()`, `color()`, `attack_damage()`, `can_shoot()`
  - Implemented for all enemies: Zombie (3HP, 0.6 speed), Skeleton (2HP, 0.8 speed, shoots every 3s), Yeti (5HP, 1.0 speed, snowballs every 2s)
  - Centralized stats: No more scattered hardcoded values

- **NEW**: **PowerBehavior** trait ([src/resources/command_queue.rs:10](src/resources/command_queue.rs:10))
  - Methods: `name()`, `description()`, `cooldown()`, `category()`, `requires_hero()`
  - Implemented for 12 powers: TNT, FAST, SLOW, ARROW, LASER, VILLAGERS, MIRROR, etc.
  - Categorized: Offensive, Movement, Defensive, Summoning, Utility
  - Cooldowns ready for UI: TNT (3s), ARROW (15s), VILLAGERS (20s), FAST/SLOW (10s)

- **NEW**: **BuffBehavior** trait ([src/components/buff.rs:4](src/components/buff.rs:4))
  - Methods: `name()`, `overlay_color()`, `damage_per_tick()`, `velocity_multiplier()`, `is_beneficial()`
  - Implemented for 6 status effects: Burning (1 dmg/tick), Freezing (0.5x speed), Poisoned (1 dmg/tick), Regeneration (-1 dmg/tick = heal), Speed (1.5x), Slowness (0.6x)
  - Visual feedback via color overlays + automatic beneficial/harmful detection

#### Code Quality & Best Practices
- **Cleaner main.rs**: Reduced from ~220 lines to ~120 lines (46% reduction)
- **Separation of Concerns**: Assets, systems, and configuration properly separated
- **Trait-Based Architecture**: Projectiles, Enemies, Powers, and Buffs all use polymorphic traits
- **Maintainability**: Game balance changes via trait methods, not scattered constants
- **Scalability**: Adding new content requires only implementing trait + enum variant
- **Extensibility**: Consistent interfaces across all game systems
- **Best Practices**: Follows Rust/Bevy ECS patterns with trait composition
