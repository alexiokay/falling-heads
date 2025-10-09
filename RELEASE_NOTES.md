# MineHeads - Release Notes

## Version 0.2.10 - Physics & AI Movement Fixes (October 9, 2025)

### ⚙️ Character-to-Character Physics Improvements
- **IMPROVED**: Character collisions now use unconditional velocity swapping
  - Simple elastic collision physics (billiard ball style)
  - Restitution applied to both horizontal and vertical bounces
  - Separation clamped to 100 units to prevent teleportation
  - Removed "approaching check" that caused sticking issues
- **FIXED**: Characters now bounce naturally from all angles
  - Bouncing from heads, sides, all directions feels consistent
  - No more sticking together in clusters
  - Ball-like physics behavior achieved

### 🎮 AI Movement System Overhaul
- **FIXED**: AI no longer interferes with player/physics velocity
  - AI only applies movement when velocity < 50 (nearly stationary)
  - Prevents AI from killing existing momentum from bounces/player input
  - Character preserves velocity from physics interactions
- **NEW**: Turbo-aware AI movement
  - When SpeedModifier (turbo/speed boost) is active, AI continues to help
  - Allows AI to provide extra momentum even when already moving
  - Better feel when using speed power-ups
- **IMPROVED**: Direction changes only when stationary
  - AI direction timer only triggers when velocity < 50
  - Prevents erratic direction flips during bouncing

### 🏐 Horizontal vs Vertical Bounce Parity
- **FIXED**: Horizontal wall bounces now match vertical bounce power
  - Horizontal collisions now apply restitution coefficient
  - Was: `velocity.x = -velocity.x * 1.0` (no energy loss)
  - Now: `velocity.x = -velocity.x * bouncy.restitution` (consistent physics)
  - Characters bounce off walls with same energy as bouncing off floor

### 🛡️ Crash Prevention
- **FIXED**: Integer overflow crash in spatial hash system
  - Used `saturating_add()` instead of regular addition
  - Prevents panic when extreme positions occur from velocity accumulation
  - Game no longer crashes when many villagers cluster together

### 🔧 Technical Details
- **Files Modified**:
  - `src/plugins/physics_plugin.rs` - Character collision physics, AI movement logic
  - `src/resources/spatial_hash.rs` - Overflow protection with saturating math
- **Physics Constants**:
  - MAX_BOUNCE_VELOCITY: 400.0 (horizontal)
  - MAX_BOUNCE_VELOCITY * 2.0: 800.0 (vertical)
  - Separation clamp: 100.0 units
  - AI activation threshold: 50.0 velocity
- **AI Behavior**:
  - Checks for `SpeedModifier` component (turbo power)
  - Applies movement when `velocity.x.abs() < 50.0 || has_turbo`
  - Maintains VILLAGER_MOVE_SPEED and VILLAGER_MAX_SPEED constants

### 📊 Physics Comparison
| Aspect | Before | After |
|--------|--------|-------|
| Horizontal bounce | No restitution (1.0x) | Restitution applied |
| AI interference | Active at velocity < 300 | Only at < 50 or with turbo |
| Character collisions | Complex formulas | Simple velocity swap |
| Spatial hash | Panic on overflow | Saturating math |
| Cluster behavior | Teleporting/sticking | Natural bouncing |

---

## Version 0.2.9 - Dragon Boss System & Dev Panel Hotkey (October 9, 2025)

### ⌨️ Dev Panel Hotkey
- **NEW**: Tab key toggle for dev panel
  - Press `Tab` during gameplay to instantly open/close dev panel
  - No need to navigate through Settings menu
  - Quick access to weather controls, powers, and biome teleport
  - Visual feedback in console when toggled
- **IMPROVED**: Dev panel now has keyboard shortcut accessibility
  - Settings menu toggle still available
  - Both methods work simultaneously

### 🐉 Dragon Boss Enemy (Work in Progress)
- **NEW**: Ender Dragon boss mob spawnable via DRAGON command
  - 3D animated dragon model (minecraft_dragon_-_free_-_black_and_white.glb)
  - **SCALE**: 80x size - massive, impressive boss appearance
  - **HIGH HP**: 20 health points (10x more than regular enemies)
  - **FLYING AI**: Ignores block collisions, flies through the air

- **NEW**: Dragon State Machine AI
  - **Emerging**: Flies onto screen from right to left (400 units)
  - **Flying**: Active flight with dynamic movement (200 units/s)
  - **Hovering**: Floats in place with sine wave bobbing
  - **Landing**: Smart descent to exposed surface blocks only
  - **Landed**: Sits on blocks for 3 seconds before taking off again

- **NEW**: Smart Landing System
  - Dragons only land on exposed blocks (not buried blocks)
  - Clearance checking: 6 blocks height + 3 blocks radius
  - Avoids landing in cramped spaces
  - Finds nearest valid landing spot within 800 unit radius

- **NEW**: Dynamic Flight Behavior
  - Rotation toward flight direction (yaw, pitch, roll)
  - Hover bobbing effect (30 unit amplitude sine wave)
  - Random flight patterns (70% land, 30% keep flying)
  - 40% chance to fly offscreen after landing

- **COMPONENTS**: Dragon-specific systems
  - `Dragon` - State machine component with timer, hover offset, targets
  - `DragonState` - Enum: Emerging, Flying, Hovering, Landing, Landed
  - `Flying` - Marker component to bypass block collision
  - `DragonAnimationClip` - Stores animation handle for future animation

- **NOTE**: Animation system infrastructure added but playback disabled
  - Bevy 0.17 AnimationGraph API requires complex setup
  - Dragon flies and behaves correctly without wing animation
  - Wing flapping animation TODO for future update

### 📦 Dragon Asset System
- **MODELS**: Three dragon GLB models available
  - `realistic_dragon_minecraft.glb` - Original, no animations (unused)
  - `minecraft_ender_hydra_3_headed_ender_dragon.glb` - 3-headed variant (unused)
  - `minecraft_dragon_-_free_-_black_and_white.glb` - **Active model** with built-in "fly" animation

- **ANIMATION LOADING**:
  - Dragon animation clip loaded via `GltfAssetLabel::Animation(0)`
  - Stored in `DragonAnimation` resource
  - Animation system searches GLB hierarchy for AnimationPlayer component

### 🔧 Technical Details
- **Dev Panel Hotkey**:
  - Added `toggle_dev_panel_with_tab()` system to dev_panel_system.rs
  - Listens for `KeyCode::Tab` and toggles `GameSettings.dev_mode`
  - Works alongside existing Settings menu toggle

- **Dragon System Files Created**:
  - `src/systems/dragon_ai_system.rs` - Complete dragon AI and movement
  - `src/systems/dragon_animation_system.rs` - Animation infrastructure (WIP)
  - `inspect_glb.py` - Python GLB model analyzer tool

- **Dragon System Files Modified**:
  - `src/plugins/asset_loader_plugin.rs` - Dragon model and animation loading
  - `src/plugins/external_api_plugin.rs` - spawn_dragon() function, DRAGON command
  - `src/plugins/gameplay_plugin.rs` - Added dragon_ai_system to game loop
  - `src/plugins/physics_plugin.rs` - Exclude Flying entities from collisions
  - `src/components/enemy.rs` - Dragon, DragonState, Flying, DragonAnimationClip
  - `src/components/bundles.rs` - PhysicsBundle::dragon() with minimal gravity
  - `src/resources/asset_handles.rs` - EnemyModels, DragonAnimation resources
  - `src/resources/command_queue.rs` - PowerCommandType::Dragon
  - `src/systems/mod.rs` - Export dragon systems

- **Constants**:
  - `DRAGON_FLIGHT_SPEED`: 200.0 - Fast flight speed
  - `HOVER_AMPLITUDE`: 30.0 - Vertical bobbing range
  - `HOVER_FREQUENCY`: 2.0 - Bobbing speed
  - `FLYING_DURATION`: 4.0s - Time to fly across screen
  - `HOVERING_DURATION`: 2.0s - Time to hover in place
  - `LANDED_DURATION`: 3.0s - Time sitting on blocks

### 🎮 Future Dragon Plans (NOT IMPLEMENTED)
- Kill dragon to collect dragon eggs (drops)
- Dragon egg collection unlocks friendly dragon summon power
- Friendly dragon does flyby fire attack and leaves
- Dragon boss battles and level completion mechanics
- Dragon fireball attacks while flying
- Full wing flapping animation with Bevy 0.17 AnimationGraph
- Multiple dragon variants (3-headed hydra, different colors)

---

## Version 0.2.8 - Death Particle System & Collision Fixes (October 9, 2025)

### 💀 Death Particle System
- **NEW**: Textured death particles when characters die
  - Particles use actual character textures (not solid colors)
  - Characters fragment into 8-12 textured pieces
  - Each particle shows piece of character's skin/material
  - Example: Black skeleton spawns black bone texture particles, white skeleton spawns white bone particles
- **NEW**: Dynamic particle physics with rotation
  - Random velocities: explosive scatter effect
  - Gravity applied: particles fall realistically
  - **Random initial rotation**: Each particle spawns at random angle (0-360° all axes)
  - **Spinning motion**: Particles rotate while falling (-5 to +5 rad/sec per axis)
  - **Size variation**: 6-18 units base with non-uniform scaling (0.8-1.2x per axis)
  - **Lifetime**: 1.5-2.5 seconds (longer for natural tumbling effect)
  - Upward bias: particles fly up before falling (100-400 units/s vertical)

### 🛡️ Dead Entity Collision Fix
- **FIXED**: Dead characters no longer interact with game
  - Dead entities excluded from hero melee damage system
  - Dead entities excluded from enemy collision damage
  - Dead entities excluded from arrow collision
  - Prevents duplicate death particles from hitting corpses
- **IMPROVED**: Death animation isolation
  - Entities with `DeathAnimation` component ignored by collision systems
  - Entities with `MarkedForDeath` component ignored by collision systems
  - Clean death animation without interference

### 🎨 Damage Visual Color Fix
- **FIXED**: Death particles no longer affected by red damage overlay
  - DamageVisual component now stores `original_color` before red tint
  - Death particles use stored original color (not red-tinted material)
  - Particles show true character appearance even when damaged
- **FIXED**: Progressive red tinting bug
  - When entity gets damaged multiple times, system now reuses stored original color
  - Prevents entities from becoming progressively redder with each hit
  - Hero no longer turns permanently red from repeated arrow hits
- **IMPROVED**: Material color restoration
  - Damage visual resets to original color (not just white)
  - Preserves character color tints and variations
  - Better visual consistency

### 🔧 Technical Details
- **Files Modified**:
  - `src/systems/death_particle_system.rs` - Textured particles with rotation and size variation
  - `src/components/death_particle.rs` - Added `rotation_velocity` field
  - `src/components/damage_visual.rs` - Added `original_color` field to DamageVisual
  - `src/systems/damage_visual_system.rs` - Store/restore original color, prevent red accumulation
  - `src/plugins/physics_plugin.rs` - Exclude dead entities from collisions
  - `src/systems/projectile_system.rs` - Arrows ignore dead enemies
  - `src/main.rs` - Added DeathParticlePlugin
- **New Components**:
  - `DeathParticle { velocity: Vec3, rotation_velocity: Vec3, lifetime: Timer }` - Particle physics with rotation
- **Death Particle Pipeline**:
  1. Entity dies → EntityDied event
  2. Get entity's material and DamageVisual
  3. Clone material with original_color (pre-damage)
  4. Spawn 8-12 particles with random rotation and size
  5. Apply physics (velocity + gravity + rotation)
  6. Despawn after lifetime
- **Rotation System**:
  - Initial rotation: Random quaternion on all axes (0-2π)
  - Angular velocity: -5 to +5 rad/sec on each axis
  - Update: `rotation = rotation * delta_rotation` each frame

### 📊 Death Particle Specs
| Property | Value | Notes |
|----------|-------|-------|
| Particle count | 8-12 | Random per death |
| Particle size | 6-18 units base | Random variation |
| Size scaling | 0.8-1.2x per axis | Non-uniform for realism |
| Velocity X | -300 to 300 units/s | Horizontal scatter |
| Velocity Y | 100 to 400 units/s | Upward explosion |
| Velocity Z | -50 to 50 units/s | Depth variation |
| Rotation velocity | -5 to +5 rad/s | Per axis (X, Y, Z) |
| Initial rotation | 0-360° per axis | Random start angle |
| Gravity | -800 units/s² | Realistic fall |
| Lifetime | 1.5-2.5s | Random duration (increased) |
| Material | Clone of character | With original color |
| Texture | Character texture | Actual skin pieces |

---

## Version 0.2.7 - TNT Visual Overhaul & Physics Improvements (October 9, 2025)

### 💥 TNT Visual & Audio Enhancements
- **NEW**: TNT now uses directional 3D cube mesh with proper textures
  - Custom UV-mapped 4x1 atlas: tnt_top | tnt_bottom | tnt_side | tnt_side
  - Removed upside-down texture hack (180° rotation)
  - Fixed backface culling - correct mesh winding order for all faces
  - TNT appears correctly oriented from all angles
- **NEW**: Minecraft-style explosion particle animation system
  - 16-frame explosion animation (assets/textures/effects/explosion/explosion_0-15.png)
  - 12-16 particles per explosion with random positioning
  - Particles spawn across full explosion radius (±273 units, 80% of TNT_EXPLOSION_RADIUS)
  - Staggered delays (0-1200ms) - particles appear gradually over 1.2 seconds
  - Fast animation speed (20ms per frame, 320ms total per particle)
  - Particles hidden until delay finishes (proper visibility control)
  - Particle size variation: 75-125 units (2x smaller than initial)
  - Total explosion effect duration: ~1.7 seconds
- **NEW**: Dynamic explosion sound system
  - 4 random explosion sounds (explode1-4.ogg) play as particles appear
  - Volume controlled via constant: EXPLOSION_VOLUME = 0.3 (30%)
  - Sounds spread over 1.2 seconds matching particle stagger
  - Creates realistic cascading explosion audio

### 🎯 TNT Physics Improvements
- **IMPROVED**: TNT collision with characters now velocity-based
  - Characters pushed based on TNT's velocity (not fixed force)
  - Horizontal: 40% of TNT velocity transferred
  - Vertical: 60% of TNT velocity pushes characters down
  - TNT bounces off characters (40% bounce vertically, -50% horizontally)
- **FIXED**: TNT no longer clips through characters
  - Full separation on collision (no partial overlap)
  - Characters can't "hold" falling TNT on their head
  - TNT bounces dynamically instead of getting stuck
- **FIXED**: TNT no longer gets stuck in mid-air
  - Added ground check system - TNT "wakes up" if block underneath is destroyed
  - Checks for supporting block every frame when settled (gravity = 0)
  - Prevents floating TNT when blocks are destroyed beneath it
- **FIXED**: TNT grounding logic improved
  - TNT touching characters can't be marked as grounded
  - Prevents TNT from freezing in air near characters
  - Gravity re-enabled properly when not truly settled
- **FIXED**: Character jumping on TNT no longer pushes TNT into ground
  - Characters bounce off TNT instead
  - TNT only gets slight downward velocity if not grounded
  - Prevents TNT from clipping into blocks below

### 🔧 Technical Details
- **Files Modified**:
  - `src/resources/cube_mesh.rs` - Fixed mesh winding order for proper backface culling
  - `src/resources/asset_handles.rs` - Added TNT textures and ExplosionTextures resource
  - `src/plugins/asset_loader_plugin.rs` - Load 16 explosion frames
  - `src/plugins/powers_plugin.rs` - TNT material, explosion animation, sound system
  - `src/components/explosive.rs` - Added ExplosionAnimation component
  - `src/plugins/physics_plugin.rs` - Velocity-based TNT collision, ground check, character interaction
  - `src/utils/constants.rs` - Added EXPLOSION_VOLUME constant
- **New Components**:
  - `ExplosionAnimation` - tracks frame timer, delay timer, current frame, visibility state
- **Mesh Optimization**: TNT uses shared mesh from MeshCache (64x64 cube)
- **Audio API**: Uses `PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.3))`

### 📊 TNT Explosion Specs
| Property | Value | Notes |
|----------|-------|-------|
| Particles per explosion | 12-16 | Random count |
| Particle size | 75-125 units | 2x smaller, varied |
| Particle delay | 0-1200ms | Staggered start |
| Frame duration | 20ms | Fast animation |
| Frames per particle | 16 | explosion_0 to explosion_15 |
| Particle lifetime | ~320ms | After delay finishes |
| Total effect duration | ~1.7s | Including delays |
| Explosion radius | 342 units | TNT_EXPLOSION_RADIUS |
| Particle spread | ±273 units | 80% of radius |
| Sound volume | 30% | EXPLOSION_VOLUME constant |
| Sound files | 4 variants | explode1-4.ogg |

---

## Version 0.2.6 - Dev Panel Power Buttons & Menu Navigation Fix (October 8, 2025)

### =' Dev Panel Enhancements
- **NEW**: Power buttons added to dev mode UI
  - 8 instant-trigger power buttons: TNT, FAST, SLOW, ARROW, LASER, MIRROR, VILLAGERS, GHAST
  - Click buttons to activate powers instantly (no API needed)
  - Visual feedback: Green flash on activation, hover effects
  - Powers use same system as API commands (global command queue)
- **Location**: Dev panel appears on right side when dev mode enabled
- **Toggle**: Enable/disable dev mode in Settings menu

### <� Menu Navigation Fix
- **FIXED**: ESC key now works in Settings menu
  - Previously: ESC only worked in Playing � Paused states
  - Now: ESC works in Settings � Paused (go back to pause menu)
  - Navigation flow: Playing � Paused � Settings � Paused � Playing
- **System**: Added `toggle_pause` to Settings state update loop

### = Bug Fixes
- **FIXED**: Power buttons now correctly trigger effects
  - Dev panel now uses global `COMMAND_QUEUE` instead of separate resource
  - Powers execute through same pipeline as API commands
  - Added `get_command_queue()` accessor function

### =' Technical Details
- **Files Modified**:
  - `src/systems/dev_panel_system.rs` - Power button handler uses global queue
  - `src/plugins/external_api_plugin.rs` - Added `get_command_queue()` public function
  - `src/plugins/ui_plugin.rs` - Added `toggle_pause` to Settings state
  - `src/components/dev_panel.rs` - Added `PowerButton` component
  - `src/systems/pause_menu_system.rs` - Added Settings � Paused ESC handling

### =� Dev Panel Controls
| Section | Controls | Description |
|---------|----------|-------------|
| Weather | 6 buttons | Clear, Rain, Snow, Sandstorm, Ash Storm, Fire Rain |
| Powers | 8 buttons | TNT, FAST, SLOW, ARROW, LASER, MIRROR, VILLAGERS, GHAST |
| Info | Live status | Current weather type and intensity |

---

## Version 0.2.5 - Weather Particle Improvements (October 8, 2025)

### <' Weather Particle Visual Enhancements
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

### <( Weather Particle Lifetime System
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

### <� Weather Particle Movement & Distribution
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

### =' Technical Details
- **Files Modified**:
  - `src/systems/weather_system.rs` - Particle size, lifetime, spawning, distribution
  - `src/resources/weather_config.rs` - Updated PARTICLE_LIFETIME constant documentation
- **Query Optimization**: Uses `windows.iter().next()` for proper window dimension access
- **Performance**: Reduced particle counts prevent excessive spawning while maintaining visual quality

### =� Particle Lifetime Comparison
| Weather Type | Old Lifetime | New Lifetime | Reason |
|--------------|--------------|--------------|--------|
| Rain | 5s | 3s | Fast falling, optimized |
| Snow | 5s | **12s** | Slow drift needs more time |
| FireRain | 5s | 2s | Very fast, optimized |
| AuroraStorm | 5s | **15s** | Very slow floating |
| Sandstorm | 5s | 10s | Moderate horizontal |
| AshStorm | 5s | 12s | Erratic movement |
