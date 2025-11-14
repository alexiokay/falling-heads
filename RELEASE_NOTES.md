# Release Notes

## New Features
- **Added RandomBox Entity**: Bouncing mystery box that triggers random events
  - Spawns as a physical 48x48 bouncing entity (smaller than villagers)
  - Uses texture at `assets/textures/effects/randombox/randombox.png`
  - Bounces from blocks and other entities like the hero
  - When touched by hero, triggers one of 5 random events:
    - Monster Horde (30%) - Spawns 3-5 enemies
    - Treasure Rain (25%) - Drops resources
    - Travelers Arrive (15%) - Spawns villagers
    - **Giant's Blessing (15%)** - Applies Grow buff to hero
    - Elite Enemy (15%) - Spawns powerful enemy
  - Spawnable via RANDOMBOX button in dev panel
  - Spawns in free areas (not inside blocks) 500-700px away from hero
  - Utility category power
  - Integrates with custom spatial hash collision system for block physics

- **Added Grow Buff**: Makes hero giant-sized with enhanced movement
  - Increases hero size by 20% per stack (max 2 stacks = 44% bigger)
  - Visual scale changes from 1.0x → 1.2x → 1.44x
  - Collision box automatically scales with visual size
  - 30% speed boost (1.3x velocity multiplier) for powerful giant strides
  - 10 second duration per application (refreshes on stack)
  - Green visual overlay effect
  - Triggered by RandomBox "Giant's Blessing" event (15% chance)
  - Buff persists even when other buffs are applied
  - Collision detection uses actual Transform scale for accurate physics

- **Added Audio Settings**: Volume controls for sound effects and music
  - Effects Volume slider (0-100%) in settings menu
  - Music Volume slider (0-100%) in settings menu
  - Settings stored in GameSettings resource
  - Interactive sliders with real-time visual feedback and draggable handles
  - Volume changes apply immediately

- **Implemented Biome-Specific Background Music System**:
  - Music automatically changes based on player's current biome
  - Smooth 1-second crossfade transitions between biome tracks
  - Respects music volume setting from settings menu
  - 8 unique music tracks (one for each biome): Plains, Forest, Desert, Mountains, Underground, Nether, End, Iceland
  - Music tracks loop seamlessly while in the same biome
  - System uses BiomeMusicPlayer resource to track current playback state
  - Music files expected at: `assets/audio/music/biomes/*.ogg` (OGG Vorbis format)

- **Added Lootbox System**: Interactive reward system with animated roulette UI
  - Physical lootbox chests spawn in world (3-5 per game) in cave-like locations
  - Opens automatically when player touches it
  - Displays fullscreen roulette UI showing 6 random rewards from 11 possible options
  - Smooth animation: border cycles through items, speeds up then slows down to land on selected reward
  - Rewards include:
    - **Events**: Monster Horde, Travelers Arrive (25%), Elite Enemy, Mega Treasure
    - **Powers**: Magic Barrier, Arrow Barrage, Thunder Strike, TNT Rain, Laser Beam, Magnet
    - **Nothing**: Small chance of getting nothing (11%)
  - Icon-based UI system for each reward type (currently uses placeholder textures)
  - Available via LOOTBOX command in dev panel
  - 5 second cooldown
  - Utility category power

- **Added THUNDER power**: Spawns 5 lightning strikes that damage random nearby blocks
  - Lightning comes from the sky with realistic zigzag pattern (8 segments)
  - Each strike destroys all blocks within 1 block radius (area effect)
  - Bright blue-white glowing lightning effect with thunder sound
  - Available in dev panel and external API
  - 10 second cooldown
  - Offensive category power

## Bug Fixes
- **Fixed mirror power to reflect fireballs**: Mirror barrier now bounces ghast fireballs back at enemies
  - Added fireball reflection with improved detection for fast-moving projectiles
  - Reflected fireballs switch teams (Enemy → Friendly) and only damage enemies
  - Updated fireball collision detection to respect team affiliation (prevents friendly fire)
  - Added stunning animated cyan impact effect using 4-frame barrier animation
  - Impact is 120x80 pixels with 4-frame animation (001.png → 004.png) at 100ms per frame
  - Positioned at exact collision point and oriented tangentially along circular barrier curve
  - Impact is parented to barrier and moves with it (stays on barrier surface as hero moves)
  - Each projectile only reflects once (reflected flag prevents duplicate impacts)
  - Dramatic cyan tint with emissive glow (5.0 intensity) fading with cubic easing over 0.4 seconds
- Fixed snowball physics: Added Mass component to prevent NaN values in Avian physics engine
- Fixed cactus transparency: Changed alpha mode from Opaque to Blend to properly render transparent pixels
- Fixed redstone ore texture: Now uses correct `redstone_ore.png` instead of coal texture
- Fixed Y coordinate display: Now shows block coordinates (Y / 64) instead of raw world units

## Changes
- **Removed Treasure Rain from Lootbox**: Removed the Treasure Rain reward from the lootbox system
  - Redistributed 14% probability to TravelersArrive (now 25% total) and Nothing (now 11% total)
  - Removed from enum, reward handlers, and asset loading
  - Updated lootbox reward list to 11 possible rewards

## Code Improvements
- Restored world generation constants (WORLD_BLOCK_SIZE, BLOCK_SPACING, TOTAL_BLOCK_SIZE) for easier future adjustments to block spacing and world scale
