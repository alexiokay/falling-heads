# Minecraft Head Fall - Game Design Document

## Overview
A 2.5D infinite falling game built with Bevy where Minecraft-style heads descend through an endless column of destructible blocks. The game features external service integration through command-based powers and a resource collection system.

## Core Gameplay

### Player Character
- **Hero**: A Minecraft-style head that continuously falls downward
- **Movement**: Can move horizontally to collect resources and avoid obstacles
- **Gravity**: Constant downward velocity with physics-based bouncing

### Block System
- **Infinite Generation**: Blocks spawn infinitely as the player descends
- **Health System**: Each block has 5 hit points
- **Visual Feedback**: Blocks show progressive damage (cracks/visual changes)
- **Destruction**: Blocks break after 5 hits and drop resources
- **Collision**: Hero bounces off blocks, dealing 1 damage per touch

### Block Types & Resources

#### Terrain Blocks
1. **Stone** - Basic terrain (most common)
2. **Dirt** - Common terrain
3. **Grass** - Surface terrain
4. **Sand** - Desert terrain
5. **Sandstone** - Desert underground
6. **Snow** - Mountain peaks

#### Resource/Ore Blocks
1. **Coal** - Common resource
2. **Iron** - Uncommon resource
3. **Gold** - Rare resource
4. **Redstone** - Rare resource
5. **Diamond** - Very rare resource
6. **Emerald** - Very rare resource

#### Decorative/Natural Blocks
1. **Wood** - Found in forests
2. **Leaves** - Found in forests
3. **Pumpkin** - Found in plains
4. **Melon** - Found in plains
5. **Cactus** - Found in deserts
6. **Dead Bush** - Found in deserts
7. **Mushroom** - Found underground
8. **Vines** - Found underground

Ore blocks drop collectible resources that rotate on the X axis when destroyed. Terrain and decorative blocks do not drop resources.

## External Service Commands

These commands are triggered by external services and execute special powers:

### 1. **TNT**
- Spawns a TNT entity near the hero
- Explodes after 3 seconds (classic Minecraft timing)
- Destroys all blocks in a 5-block radius
- Deals area-of-effect damage

### 2. **FAST**
- Increases hero fall speed by 2x
- Duration: 10 seconds
- Visual effect: Speed lines/trail

### 3. **SLOW**
- Decreases hero fall speed by 0.5x
- Duration: 10 seconds
- Visual effect: Slow-motion particles

### 4. **FUSS** (Confusion/Chaos)
- Randomly triggers 3-5 smaller effects
- Could spawn multiple weak entities
- Creates visual chaos on screen

### 5. **BOLT** (Lightning)
- Strikes lightning from top of screen
- Creates a vertical beam that destroys multiple blocks
- Width: 3 blocks
- Destroys blocks instantly in the column
- Chain lightning effect to nearby blocks

### 6. **ARROW**
- Fires arrows in a circular pattern around the hero
- 8 arrows in clockwise rotation
- Each arrow destroys blocks on contact (2 damage per hit)
- Duration: Arrows persist until off-screen

### 7. **LASER**
- Fires a large laser beam downward from the hero
- Width: 5 blocks
- Length: Extends down 20 blocks
- Destroys all blocks in path instantly
- Duration: 2 seconds continuous beam

### 8. **GHAST**
- Spawns a floating Ghast enemy above the hero
- Fires explosive fireballs periodically
- Fireballs destroy blocks in 3-block radius
- Ghast follows hero horizontally
- Duration: 15 seconds or until destroyed

### 9. **BOX**
- Spawns a special loot box/chest
- Falls with gravity
- When collected: Grants random resources (5-15 items)
- Glows with particle effects

### 10. **MAGNET**
- Attracts all nearby resources to the hero
- Radius: 10 blocks
- Duration: 8 seconds
- Visual: Magnetic field effect around hero

## Game Mechanics

### Biome System
The world is divided into different biomes, each with unique characteristics:

1. **Plains** - Grass terrain with pumpkins and melons
2. **Forest** - Dirt terrain with wood and leaf blocks
3. **Desert** - Sand and sandstone with cacti and dead bushes
4. **Mountains** - Stone terrain with snow at peaks, higher ore concentration
5. **Underground** - Deep caves with mushrooms, vines, and rich ore veins

Biomes generate based on horizontal (X) position and depth (Y), creating diverse landscapes.

### World Generation
- **Chunk-Based System**: World generates in 16x16 block chunks
- **Infinite Horizontal**: Chunks spawn infinitely left and right as hero moves
- **Infinite Vertical**: Chunks spawn downward infinitely as hero descends
- **Cave Generation**: Natural caves carved using noise algorithms
- **Smart Despawning**: Distant chunks (>5 chunks away) despawn to save memory
- **Biome Transitions**: Smooth transitions between different biomes

### Resource Collection
- Resources drop from destroyed ore blocks
- **Rotation Effect**: Resources rotate on the X axis for visual appeal
- Float/fall with physics
- Auto-collected on contact with hero
- Display resource counter in UI

### Scoring System
- Points for blocks destroyed
- Bonus points for collected resources
- Multiplier for combo destruction
- Distance traveled (depth)

### Difficulty Scaling
- Block density increases with depth
- Harder block types appear more frequently
- Fall speed gradually increases
- Resource values increase with rarity

### Visual Style (2.5D Isometric)
- **Isometric Camera**: Orthographic projection positioned at an angle for depth perception
- **Quasi-Isometric View**: Camera creates 2.5D effect with proper depth sorting
- Pixel art or voxel-style graphics
- Parallax background layers
- Particle effects for destruction
- Smooth animations and transitions
- Rotating resource pickups for dynamic visuals

## Technical Architecture

### External Service Integration
```
HTTP/WebSocket endpoint listening for commands:
POST /command
Body: { "action": "TNT" | "FAST" | "SLOW" | ... }
Response: { "success": true, "effect_triggered": true }
```

### Core Systems (Bevy ECS)
1. **Physics System**: Gravity, collision, bouncing
2. **World Generation System**: Chunk-based infinite generation with biomes and caves
3. **Chunk Management System**: Dynamic loading/unloading of chunks
4. **Damage System**: Hit detection and block health
5. **Resource System**: Drops, rotation, and collection
6. **Power System**: Command execution and effects
7. **Render System**: 2.5D isometric rendering and animations
8. **UI System**: HUD, resources, score
9. **Audio System**: SFX and background music

### Components
- `Hero` - Player entity
- `Block` - Destructible blocks with health and type (terrain, ore, decorative)
- `BlockType` - Enum defining all block variants across biomes
- `BiomeType` - Enum defining biome characteristics
- `Chunk` - Chunk entity marker with coordinates and biome
- `ChunkMember` - Component linking blocks to their parent chunk
- `Resource` - Collectible resource items
- `RotatingResource` - Component for resources that rotate on X axis
- `PowerEffect` - Active power effects
- `Projectile` - Arrows, fireballs, etc.
- `Explosive` - TNT and explosions
- `Enemy` - Ghast and other entities

### Resources (Bevy)
- `GameState` - Current game status
- `Score` - Points and statistics
- `Inventory` - Collected resources
- `LoadedChunks` - Tracks which chunks are currently loaded
- `WorldConfig` - Configuration for block sizes and world generation
- `ActiveEffects` - Currently running powers
- `ExternalCommandQueue` - Pending commands from services

## UI Elements

### HUD
- Resource counters (coal, iron, gold, etc.)
- Score display
- Depth/distance meter
- Active effects indicators
- FPS counter (debug)

### Effects Indicators
- Icon + timer for temporary effects (FAST, SLOW, MAGNET)
- Power activation notifications
- Combo counter

## Audio

### Sound Effects
- Block hit sounds (different per material)
- Block break sounds
- Resource collection ping
- Power activation sounds
- Explosion effects
- Laser/arrow/bolt firing

### Music
- Dynamic background music
- Intensity increases with depth
- Special themes for active powers

## Implemented Features ✅
- ✅ 2.5D Isometric camera view
- ✅ Biome system (Plains, Forest, Desert, Mountains, Underground)
- ✅ Infinite horizontal and vertical world generation
- ✅ Chunk-based loading/unloading system
- ✅ Cave generation with noise algorithms
- ✅ 23 unique block types (terrain, ores, decorative)
- ✅ Rotating resource drops on X axis
- ✅ Biome-specific decorative blocks

## Future Enhancements
- Multiple hero skins/characters
- Boss blocks with special mechanics
- Power combinations/synergies
- Leaderboards
- Replay system
- Additional biomes (Jungle, Swamp, Ice)
- Weather effects per biome
- Day/night cycle in background
- Improved cave systems (ravines, mineshafts)

## Development Roadmap

### Phase 1: Core Mechanics ✅
- [x] Basic hero movement and physics
- [x] Block generation and collision
- [x] Block damage and destruction system
- [x] Simple resource drops
- [x] Chunk-based infinite world generation
- [x] Biome system implementation
- [x] Cave generation

### Phase 2: Visual & Audio
- [x] 2.5D isometric camera setup
- [x] Block and hero sprites/models
- [x] Rotating resource visuals
- [ ] Particle effects
- [ ] Sound effects and music

### Phase 3: Powers System
- [ ] External command listener
- [ ] Implement all 10 power commands
- [ ] Visual effects for each power
- [ ] Power activation system

### Phase 4: Polish
- [ ] UI/HUD implementation
- [ ] Score and inventory systems
- [ ] Difficulty scaling
- [ ] Performance optimization

### Phase 5: Testing & Balance
- [ ] Gameplay testing
- [ ] Balance adjustments
- [ ] Bug fixes
- [ ] External service integration testing

---

**Engine**: Bevy Game Engine (Rust)
**Target Platform**: PC (Windows/Linux/macOS)
**Art Style**: Minecraft-inspired pixel/voxel art
**Perspective**: 2.5D orthographic
