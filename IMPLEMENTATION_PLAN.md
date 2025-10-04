# Minecraft Head Fall - Implementation Plan

## Overview
This document outlines the step-by-step implementation plan for building the Minecraft Head Fall game using Bevy 0.17.1.

---

## Phase 1: Project Setup & Foundation (Day 1)

### 1.1 Initialize Project Structure
- [x] ~~Create new Cargo project~~ *(We have Bevy 0.17.1 source available)*
- [ ] **Create separate game project directory**
- [ ] Set up folder structure (src/, assets/, tests/, benches/)
- [ ] Configure Cargo.toml with Bevy 0.17.1 as dependency
- [ ] Create placeholder asset folders
- [ ] Set up .gitignore

### 1.2 Basic Bevy Setup
- [x] ~~Bevy 0.17.1 available~~ *(Using local Bevy source)*
- [ ] Create main.rs with basic Bevy app
- [ ] Implement plugin architecture skeleton
- [ ] Set up game states (Menu, Playing, Paused, GameOver)
- [ ] Configure window settings (title, resolution, vsync)
- [ ] Test that app runs successfully

### 1.3 Core Module Structure
- [ ] Create component modules with placeholder structs
- [ ] Create resource modules
- [ ] Create event modules
- [ ] Create plugin modules
- [ ] Set up proper module exports in mod.rs files

### 1.4 Documentation Setup
- [x] ~~GAME_DESIGN.md created~~
- [x] ~~ARCHITECTURE.md created~~
- [x] ~~IMPLEMENTATION_PLAN.md created~~

**Deliverable**: Running Bevy app with empty window and proper architecture

**STATUS**: ~30% complete (docs ready, need to create actual game project)

---

## Phase 2: Core Components & Resources (Day 1-2)

### 2.1 Define Components
```rust
// src/components/
- [ ] Hero component (marker)
- [ ] Block component (health, type, position)
- [ ] Velocity component
- [ ] Gravity component
- [ ] Bouncy component
- [ ] BlockType enum (Stone, Coal, Iron, Gold, Diamond, Emerald, Redstone)
- [ ] Health component
```

### 2.2 Define Resources
```rust
// src/resources/
- [ ] GameState resource (depth, score, speed multiplier)
- [ ] Inventory resource (resource counts)
- [ ] CommandQueue resource (power commands)
- [ ] WorldConfig resource (generation settings)
- [ ] AssetHandles resource (texture handles)
```

### 2.3 Define Events
```rust
// src/events/
- [ ] BlockDestroyed event
- [ ] BlockDamaged event
- [ ] ResourceCollected event
- [ ] PowerActivated event
```

**Deliverable**: All core data structures defined and compiled

---

## Phase 3: Basic Rendering & Camera (Day 2)

### 3.1 Camera Setup
- [ ] Spawn 2.5D orthographic camera
- [ ] Set up camera bounds
- [ ] Implement basic camera follow system
- [ ] Test camera movement

### 3.2 Placeholder Graphics
- [ ] Create simple colored squares for blocks (in code)
- [ ] Create colored circle for hero (in code)
- [ ] Spawn hero entity at startup
- [ ] Spawn initial grid of blocks
- [ ] Verify rendering works

### 3.3 Background & Visual Setup
- [ ] Add background color/gradient
- [ ] Set up sprite rendering layers
- [ ] Configure 2D rendering settings

**Deliverable**: Visible hero and blocks on screen with working camera

---

## Phase 4: Physics System (Day 2-3)

### 4.1 Gravity System
- [ ] Implement apply_gravity system
- [ ] Add gravity constant to config
- [ ] Apply gravity to hero
- [ ] Test hero falling

### 4.2 Velocity System
- [ ] Implement apply_velocity system
- [ ] Update entity positions based on velocity
- [ ] Add velocity damping/friction

### 4.3 Collision Detection
- [ ] Implement AABB collision detection
- [ ] Detect hero-block collisions
- [ ] Emit collision events
- [ ] Add debug visualization for collisions

### 4.4 Bounce Mechanics
- [ ] Implement bounce response system
- [ ] Calculate bounce velocity
- [ ] Apply bounce to hero
- [ ] Fine-tune bounce physics (elasticity, damping)
- [ ] Test bouncing behavior

**Deliverable**: Hero falls, collides, and bounces realistically off blocks

---

## Phase 5: Block Damage System (Day 3)

### 5.1 Damage on Collision
- [ ] Deal 1 damage to block on hero collision
- [ ] Update block health component
- [ ] Emit BlockDamaged event
- [ ] Prevent multiple hits per collision

### 5.2 Block Destruction
- [ ] Check for block health <= 0
- [ ] Emit BlockDestroyed event
- [ ] Despawn block entity
- [ ] Add simple destruction effect (flash/fade)

### 5.3 Visual Damage Feedback
- [ ] Create damage states (crack textures/colors)
- [ ] Update block appearance based on health
- [ ] Add damage number display (optional)

**Deliverable**: Blocks show damage and break after 5 hits

---

## Phase 6: Infinite World Generation (Day 3-4)

### 6.1 Block Generation System
- [ ] Track camera/hero position
- [ ] Generate new blocks below visible area
- [ ] Use random block types with weighted distribution
- [ ] Maintain proper grid layout

### 6.2 Block Despawning
- [ ] Detect blocks above visible area
- [ ] Despawn off-screen blocks
- [ ] Prevent memory leaks

### 6.3 Difficulty Scaling
- [ ] Increase block density with depth
- [ ] Adjust rare block spawn rates
- [ ] Track depth in GameState resource

**Deliverable**: Infinite scrolling world with varying block types

---

## Phase 7: Resource System (Day 4)

### 7.1 Resource Drops
- [ ] Spawn resource entity on block destruction
- [ ] Match resource type to block type
- [ ] Apply physics to dropped resources (fall, bounce)
- [ ] Add visual representation (colored gems)

### 7.2 Resource Collection
- [ ] Detect hero-resource collisions
- [ ] Emit ResourceCollected event
- [ ] Update Inventory resource
- [ ] Despawn collected resource
- [ ] Add collection particle effect

### 7.3 Resource Display
- [ ] Create basic HUD
- [ ] Display resource counts per type
- [ ] Update UI in real-time

**Deliverable**: Resources drop, can be collected, and are tracked

---

## Phase 8: Hero Input & Movement (Day 4)

### 8.1 Horizontal Movement
- [ ] Read keyboard input (A/D or Arrow keys)
- [ ] Apply horizontal velocity to hero
- [ ] Add movement acceleration/deceleration
- [ ] Limit max horizontal speed

### 8.2 Movement Bounds
- [ ] Prevent hero from moving off-screen
- [ ] Add screen boundary collision
- [ ] Test edge cases

**Deliverable**: Hero can be controlled horizontally while falling

---

## Phase 9: External API Server (Day 5)

### 9.1 HTTP Server Setup
- [ ] Create axum server in separate tokio runtime
- [ ] Define PowerCommand enum
- [ ] Create /command POST endpoint
- [ ] Parse incoming JSON commands
- [ ] Add CORS middleware

### 9.2 Command Queue Integration
- [ ] Create mpsc channel for commands
- [ ] Send commands from HTTP handler to channel
- [ ] Create Bevy system to poll channel
- [ ] Add commands to CommandQueue resource

### 9.3 Command Processing
- [ ] Implement process_command_queue system
- [ ] Match on PowerCommand enum
- [ ] Emit PowerActivated events
- [ ] Add command validation

### 9.4 Testing
- [ ] Test API with curl/Postman
- [ ] Verify commands reach Bevy
- [ ] Add error handling

**Deliverable**: Working HTTP API that queues commands to game

---

## Phase 10: Power Implementation - Part 1 (Day 5-6)

### 10.1 TNT Power
- [ ] Create TNT component
- [ ] Spawn TNT entity near hero
- [ ] Add countdown timer (3 seconds)
- [ ] Implement explosion system
- [ ] Detect blocks in radius
- [ ] Destroy blocks in explosion
- [ ] Add explosion visual effect

### 10.2 FAST/SLOW Powers
- [ ] Create SpeedModifier component
- [ ] Apply speed multiplier to gravity
- [ ] Add duration timer
- [ ] Visual effect (speed lines/slow-mo)
- [ ] Remove effect after duration

### 10.3 BOLT Power (Lightning)
- [ ] Create Lightning component
- [ ] Spawn lightning beam from top
- [ ] Define column width (3 blocks)
- [ ] Detect blocks in column
- [ ] Instantly destroy blocks
- [ ] Add lightning visual effect
- [ ] Add chain lightning to nearby blocks

**Deliverable**: TNT, FAST, SLOW, and BOLT powers working

---

## Phase 11: Power Implementation - Part 2 (Day 6-7)

### 11.1 ARROW Power
- [ ] Create Arrow component (Projectile)
- [ ] Spawn 8 arrows around hero
- [ ] Calculate circular positions
- [ ] Apply rotation velocities (clockwise)
- [ ] Implement arrow-block collision
- [ ] Deal damage (2 per hit)
- [ ] Despawn arrows off-screen

### 11.2 LASER Power
- [ ] Create Laser component
- [ ] Spawn laser beam downward from hero
- [ ] Define beam dimensions (5 width, 20 length)
- [ ] Detect all blocks in beam area
- [ ] Instantly destroy blocks
- [ ] Add laser beam visual (glowing rectangle)
- [ ] Add duration timer (2 seconds)

### 11.3 GHAST Power
- [ ] Create Ghast component
- [ ] Spawn Ghast above hero
- [ ] Implement simple AI (follow hero horizontally)
- [ ] Add fireball shooting timer
- [ ] Spawn Fireball projectiles
- [ ] Implement fireball-block collision
- [ ] Create explosion on fireball hit
- [ ] Despawn Ghast after 15 seconds

**Deliverable**: ARROW, LASER, and GHAST powers working

---

## Phase 12: Power Implementation - Part 3 (Day 7)

### 12.1 BOX Power
- [ ] Create LootBox component
- [ ] Spawn box entity above hero
- [ ] Apply physics (falls with gravity)
- [ ] Detect hero-box collision
- [ ] Spawn random resources (5-15 items)
- [ ] Add glowing particle effect
- [ ] Despawn box on collection

### 12.2 MAGNET Power
- [ ] Create Magnet component
- [ ] Apply to hero for duration
- [ ] Detect all resources in radius (10 blocks)
- [ ] Apply attraction force toward hero
- [ ] Update resource velocities
- [ ] Add magnetic field visual effect
- [ ] Remove effect after 8 seconds

### 12.3 FUSS Power (Chaos)
- [ ] Implement random effect selection
- [ ] Trigger 3-5 mini-effects
- [ ] Spawn random entities (mini TNT, arrows, etc.)
- [ ] Add screen shake
- [ ] Add chaotic particle effects

**Deliverable**: BOX, MAGNET, and FUSS powers working

---

## Phase 13: Visual Effects & Polish (Day 8)

### 13.1 Particle Systems
- [ ] Create particle emitter component
- [ ] Implement particle spawning system
- [ ] Add destruction particles
- [ ] Add collection particles
- [ ] Add power activation particles
- [ ] Add trail effects

### 13.2 Animations
- [ ] Implement sprite animation system
- [ ] Animate hero (idle, falling)
- [ ] Animate blocks (crack progression)
- [ ] Animate explosions
- [ ] Animate Ghast

### 13.3 Screen Effects
- [ ] Implement screen shake on explosions
- [ ] Add flash effects
- [ ] Add vignette for intensity
- [ ] Add power activation flashes

**Deliverable**: Polished visual effects for all game elements

---

## Phase 14: Audio System (Day 8-9)

### 14.1 Sound Effects
- [ ] Set up audio plugin
- [ ] Add block hit sounds (per material type)
- [ ] Add block break sounds
- [ ] Add resource collection sound
- [ ] Add explosion sound
- [ ] Add power activation sounds
- [ ] Add laser/arrow/lightning sounds

### 14.2 Background Music
- [ ] Add looping background track
- [ ] Implement volume controls
- [ ] Add intensity scaling with depth

### 14.3 Audio Polish
- [ ] Add spatial audio (optional)
- [ ] Balance audio levels
- [ ] Add audio settings

**Deliverable**: Full audio implementation

---

## Phase 15: UI & HUD (Day 9)

### 15.1 Game HUD
- [ ] Create HUD layout
- [ ] Display resource counts with icons
- [ ] Display score
- [ ] Display depth meter
- [ ] Show active effects (icons + timers)
- [ ] Style UI elements

### 15.2 Menu System
- [ ] Create main menu UI
- [ ] Add start button
- [ ] Add settings menu (audio, controls)
- [ ] Create pause menu
- [ ] Create game over screen with stats

### 15.3 Notifications
- [ ] Create notification system
- [ ] Show power activation messages
- [ ] Show milestone notifications
- [ ] Add notification animations

**Deliverable**: Complete UI/UX

---

## Phase 16: Game Balance & Testing (Day 10)

### 16.1 Balance Tweaking
- [ ] Adjust block health values
- [ ] Balance power effectiveness
- [ ] Tune fall speed and physics
- [ ] Adjust resource spawn rates
- [ ] Balance difficulty scaling

### 16.2 Bug Fixes
- [ ] Test all powers thoroughly
- [ ] Fix collision edge cases
- [ ] Fix memory leaks
- [ ] Fix performance issues

### 16.3 Performance Optimization
- [ ] Profile frame times
- [ ] Optimize collision detection
- [ ] Optimize rendering (batching)
- [ ] Reduce entity count where possible
- [ ] Test on different hardware

**Deliverable**: Balanced, bug-free game

---

## Phase 17: Assets & Art (Day 11-12)

### 17.1 Block Textures
- [ ] Create/find textures for all block types
- [ ] Create damage state textures
- [ ] Set up texture atlas
- [ ] Implement in-game

### 17.2 Character & Entity Art
- [ ] Create hero sprite/model
- [ ] Create Ghast sprite
- [ ] Create TNT sprite
- [ ] Create projectile sprites
- [ ] Create resource gem sprites

### 17.3 Effects & UI Art
- [ ] Create particle textures
- [ ] Create explosion sprite sheets
- [ ] Design UI elements
- [ ] Create icons for resources/powers
- [ ] Create background layers

**Deliverable**: Complete art assets

---

## Phase 18: Documentation & Deployment (Day 13)

### 18.1 Code Documentation
- [ ] Add doc comments to public APIs
- [ ] Document power system usage
- [ ] Document API endpoints
- [ ] Create architecture diagrams

### 18.2 User Documentation
- [ ] Write README with game instructions
- [ ] Create API usage guide
- [ ] Document build process
- [ ] Create quick-start guide

### 18.3 Build & Package
- [ ] Test release builds
- [ ] Create platform-specific builds (Windows, Linux, macOS)
- [ ] Package with assets
- [ ] Create installer (optional)

### 18.4 Deployment
- [ ] Set up CI/CD (optional)
- [ ] Deploy to itch.io or similar (optional)
- [ ] Create demo video

**Deliverable**: Fully documented and deployable game

---

## Testing Checklist

### Functional Tests
- [ ] Hero movement in all directions
- [ ] Block collision and bouncing
- [ ] Block damage and destruction
- [ ] Resource drops and collection
- [ ] All 10 powers work correctly
- [ ] API commands execute properly
- [ ] Game over conditions
- [ ] UI updates correctly

### Performance Tests
- [ ] Stable 60 FPS with 1000+ blocks
- [ ] No memory leaks after 30 min play
- [ ] Collision detection scales well
- [ ] API has low latency (<50ms)

### Edge Cases
- [ ] Multiple power activations simultaneously
- [ ] Rapid command spamming
- [ ] Hero at screen boundaries
- [ ] Destroying all visible blocks
- [ ] Zero resources collected

---

## Risk Mitigation

### High-Risk Areas
1. **Collision Performance**: Use spatial partitioning early
2. **API Integration**: Test thoroughly with concurrent requests
3. **Memory Management**: Profile regularly, use object pooling
4. **Physics Stability**: Tune carefully, add debug visualization

### Contingency Plans
- If custom physics is too complex, use bevy_rapier2d
- If API performance is poor, consider WebSocket instead of HTTP polling
- If rendering is slow, reduce entity count or use instancing

---

## Success Criteria

### Minimum Viable Product (MVP)
- ✓ Hero falls and bounces
- ✓ Blocks take damage and break
- ✓ Infinite world generation
- ✓ Resources drop and collect
- ✓ All 10 powers implemented
- ✓ External API working
- ✓ Basic UI/HUD

### Polish Goals
- ✓ Smooth animations
- ✓ Particle effects
- ✓ Sound effects
- ✓ Balanced gameplay
- ✓ Good performance (60 FPS)

### Stretch Goals
- Multiple hero skins
- Boss encounters
- Leaderboards
- Replay system
- Mobile builds

---

## Timeline Summary

| Phase | Duration | Focus |
|-------|----------|-------|
| 1-2 | Days 1-2 | Setup & Foundation |
| 3-5 | Days 2-3 | Physics & Blocks |
| 6-8 | Days 3-4 | World & Resources |
| 9-12 | Days 5-7 | API & Powers |
| 13-15 | Days 8-9 | Polish & UI |
| 16-18 | Days 10-13 | Testing & Deployment |

**Total Estimated Time**: 13 days (2-3 weeks)

---

## Development Tips

1. **Start Simple**: Get core loop working before adding complexity
2. **Test Often**: Test each feature immediately after implementation
3. **Use Debug Tools**: Bevy inspector for entity debugging
4. **Profile Early**: Don't wait for performance problems
5. **Commit Regularly**: Commit after each working feature
6. **Modular Development**: Each plugin should work independently
7. **Placeholder Assets**: Use colored shapes until art is ready
8. **API First**: Test API separately before integrating

---

## Next Steps

1. Create new project directory: `minecraft-head-fall/`
2. Initialize with `cargo init`
3. Copy architecture and design docs
4. Start Phase 1: Project Setup
5. Follow plan sequentially, checking off items as completed

Ready to implement! 🚀
