# Visual Effects (VFX) Guide for Falling Heads

## 🎨 Industry Standards for 2.5D Platform Games

### Overview
This guide covers best practices for implementing visual effects in 2.5D platform games, based on industry standards from games like Hollow Knight, Minecraft, Terraria, and Dead Cells.

---

## 📊 VFX Approaches Comparison

### 1. **Sprite Sheet Animations** ⭐ RECOMMENDED
**Used by**: Minecraft, Terraria, Stardew Valley, Celeste, Dead Cells

**Pros:**
- ✅ Professional look
- ✅ Efficient GPU batching (one texture load)
- ✅ Easy to preview/edit all frames
- ✅ Standard format (works with all engines)
- ✅ Best performance

**Cons:**
- ❌ Requires asset creation/sourcing
- ❌ Takes up texture memory

**Typical Format:**
```
Frame Size: 64x64 or 128x128 pixels
Frame Count: 8-16 frames (8 = basic, 16 = smooth)
Duration: 0.3-0.5 seconds total
Animation FPS: 20-30 fps
File Format: PNG with transparency
Layout: Grid (e.g., 4x4 = 16 frames)
```

---

### 2. **Procedural Effects** (Flash/Circle)
**Used by**: Prototypes, minimalist indie games

**Pros:**
- ✅ No assets needed
- ✅ Quick to implement
- ✅ Easy to tweak parameters
- ✅ Small file size

**Cons:**
- ❌ Less detailed/professional
- ❌ Limited visual variety
- ❌ Can look generic

**Best for:**
- Prototyping
- Minimalist art styles
- Quick iteration

---

### 3. **Particle Systems**
**Used by**: Hollow Knight, Dead Cells (combined with sprites)

**Pros:**
- ✅ Most dynamic/organic
- ✅ Highly customizable
- ✅ Can simulate physics (smoke, sparks, debris)

**Cons:**
- ❌ Complex to implement
- ❌ Requires particle framework
- ❌ Higher CPU cost

**Best for:**
- High-budget polish
- Complex effects (fire, smoke, magic)
- Ambient effects

---

## 🎮 Hollow Knight Approach (Premium Quality)

### VFX Layering Technique
Hollow Knight achieves its signature "punchy" feel through **layered effects**:

```
Impact Effect Composition:
├─ Layer 1: White flash (additive blend, 2 frames, full screen)
├─ Layer 2: Core explosion sprite (12-24 frames, normal blend)
├─ Layer 3: Glow ring (6-8 frames, additive blend)
├─ Layer 4: Particles (8-20 sprites, procedural movement)
└─ Layer 5: Screen effects (shake, chromatic aberration)
```

### Timing Breakdown
```
Frame 0:   Spawn white flash (full brightness)
Frame 1:   Flash starts fading
Frame 2-3: Hitpause (freeze game for impact)
Frame 4:   Resume game, explosion sprite frame 1
Frame 5:   Particles spawn and spread outward
Frame 6-15: Explosion animates through frames
Frame 16+: Particles linger and fade
```

### Key Principles
1. **Anticipation**: Small windup before explosion
2. **Impact**: Brightest/biggest frame at moment of contact
3. **Follow-through**: Effects linger after main animation
4. **Contrast**: Bright effects against darker backgrounds
5. **Screen response**: Camera shake, flash, freeze frames

---

## 🔥 Implementation Tiers (For Our Game)

### Tier 1: Minimum Viable Effect (Prototype)
**Effort**: 10 minutes | **Quality**: 60%

```rust
Components:
- Procedural expanding circle (orange/yellow)
- Scale animation: 0 → explosion_radius
- Alpha fade: 1.0 → 0.0
- Duration: 0.3 seconds
- Despawn on completion

Code:
struct ExplosionFlash {
    timer: Timer,
    max_radius: f32,
}
```

**When to use**: Early development, testing gameplay

---

### Tier 2: Standard Quality (Production Ready)
**Effort**: 1-2 hours | **Quality**: 85%

```rust
Components:
- 8-12 frame explosion sprite sheet (64x64 or 128x128)
- White flash overlay (1 frame, additive blend)
- Screen shake (0.1-0.2 seconds)
- Sound effect with bass

Assets needed:
- explosion_spritesheet.png (512x512, 4x4 grid)
- flash_overlay.png (64x64, white circle)
```

**When to use**: Final game, public releases

---

### Tier 3: Premium Polish (Hollow Knight Level)
**Effort**: 4-8 hours | **Quality**: 100%

```rust
Components:
- 16-24 frame hand-drawn explosion sprite
- Additive glow layer (8 frames)
- 10-20 debris particles with physics
- Screen shake + chromatic aberration
- Hitpause (2-3 frame freeze)
- Layered sound (impact + explosion + rumble)

Assets needed:
- explosion_main.png (2048x2048, high detail)
- explosion_glow.png (1024x1024, soft edges)
- particle_debris.png (atlas of 5-10 variations)
- particle_smoke.png (atlas of 3-5 variations)
```

**When to use**: High-budget games, showcase trailers

---

## 📦 Asset Resources

### Free Sprite Sheet Sources
1. **OpenGameArt.org**
   - Search: "explosion sprite sheet"
   - License: CC0, CC-BY
   - Quality: Varies (check previews)

2. **Kenney.nl**
   - Path: Asset Packs → Particle Effects
   - License: CC0 (public domain)
   - Quality: Professional, consistent style

3. **itch.io**
   - Search: "explosion sprite pixel"
   - License: Varies (check each)
   - Quality: Indie/hobbyist

4. **Craftpix.net**
   - Section: Free Game Assets → Effects
   - License: Free with attribution
   - Quality: Professional

### Recommended Sizes for Block-Style Games
- **Small explosions**: 32x32 per frame (TNT, grenades)
- **Medium explosions**: 64x64 per frame (recommended for TNT)
- **Large explosions**: 128x128 per frame (boss attacks, mega bombs)

---

## 🔧 Bevy Implementation Pattern

### Sprite Sheet Animation System

```rust
// 1. Component Definition
#[derive(Component)]
struct ExplosionAnimation {
    timer: Timer,
    frame_count: usize,
}

// 2. Constants
pub const EXPLOSION_FRAME_SIZE: f32 = 64.0;
pub const EXPLOSION_GRID_SIZE: usize = 4; // 4x4 = 16 frames
pub const EXPLOSION_DURATION: f32 = 0.4; // seconds
pub const EXPLOSION_FPS: f32 = 40.0; // frames per second

// 3. Spawn Function
pub fn spawn_explosion(
    commands: &mut Commands,
    position: Vec3,
    textures: &GameTextures,
    atlas_layouts: &mut Assets<TextureAtlasLayout>,
) {
    let layout = TextureAtlasLayout::from_grid(
        UVec2::splat(64), // Frame size
        4,                // Columns
        4,                // Rows
        None,             // No padding
        None,             // No offset
    );
    let atlas_layout = atlas_layouts.add(layout);

    commands.spawn((
        Sprite::from_atlas_image(
            textures.explosion.clone(),
            TextureAtlas {
                layout: atlas_layout,
                index: 0,
            },
        ),
        Transform::from_translation(position),
        ExplosionAnimation {
            timer: Timer::from_seconds(EXPLOSION_DURATION, TimerMode::Once),
            frame_count: 16,
        },
    ));
}

// 4. Update System
fn animate_explosions(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut ExplosionAnimation, &mut Sprite)>,
) {
    for (entity, mut anim, mut sprite) in query.iter_mut() {
        anim.timer.tick(time.delta());

        // Calculate current frame based on time
        let progress = anim.timer.fraction();
        let frame_index = (progress * anim.frame_count as f32) as usize;

        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = frame_index.min(anim.frame_count - 1);
        }

        // Despawn when animation completes
        if anim.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
```

---

## 🎯 Recommendations for Falling Heads

### Current Game Style Analysis
- **Art style**: Blocky/pixel art (Minecraft-inspired)
- **Target feel**: Satisfying, punchy impacts
- **Performance needs**: Fast (lots of blocks/entities)
- **Development stage**: Production (ready for polish)

### Recommended Approach: **Tier 2** (Standard Quality)

#### Phase 1: Quick Win (Now)
```
✓ Implement procedural flash effect (10 minutes)
✓ Add screen shake on explosion
✓ Test feel/timing
```

#### Phase 2: Asset Integration (Next)
```
□ Source/create 8-frame explosion sprite (64x64)
□ Implement sprite sheet animation system
□ Add white flash overlay
□ Fine-tune timing
```

#### Phase 3: Polish (Later)
```
□ Add 5-10 block debris particles
□ Add dust cloud sprite
□ Add glow layer (optional)
□ Add sound effects
```

### Expected Results
- **Phase 1**: 70% visual quality, immediate feedback
- **Phase 2**: 90% visual quality, professional look
- **Phase 3**: 100% visual quality, AAA polish

---

## 💡 Additional VFX to Consider

### Other Effects Needing Polish
1. **Arrow Hit Impact**
   - Small spark sprite (3-5 frames)
   - Dust puff on block hit
   - Blood splash on entity hit (optional)

2. **Block Break Effect**
   - Block fragments (4-8 particles)
   - Dust cloud (sprite or procedural)
   - Flash on destruction

3. **Damage Flash** (Already Implemented ✓)
   - Red overlay
   - Red border
   - Death freeze

4. **Laser Beam**
   - Animated texture scrolling
   - Edge glow particles
   - Impact sparks at bottom

5. **Power-Up Collection**
   - Sparkle particles
   - Glow fade-in
   - UI pop animation

---

## 📐 Technical Specifications

### Performance Guidelines
```
Target FPS: 60 fps
Max simultaneous effects: 20-30
Texture memory budget: 2-4 MB for all VFX
Particle count per effect: 5-20
Animation frame budget: 50 ms per update
```

### Bevy-Specific Considerations
```rust
// Use systems with run conditions to optimize
.add_systems(Update, (
    animate_explosions,
    update_particles,
    screen_shake,
).run_if(in_state(GameState::Playing)));

// Batch sprite rendering automatically
// TextureAtlas handles this in Bevy 0.17+

// Despawn promptly to avoid entity bloat
if animation.finished() {
    commands.entity(entity).despawn();
}
```

---

## 🎨 Color Theory for Explosions

### TNT/Fire Explosions (Recommended)
```
Core: Bright yellow-white (#FFFACD)
Mid: Orange-red (#FF6B35)
Outer: Dark red-brown (#8B0000)
Glow: Additive orange (#FFA500)
```

### Magic/Energy Explosions
```
Core: Bright cyan (#00FFFF)
Mid: Electric blue (#1E90FF)
Outer: Deep purple (#4B0082)
Glow: Additive blue (#0080FF)
```

### Impact/Physical
```
Core: White flash (#FFFFFF)
Mid: Light gray (#CCCCCC)
Outer: Dark gray dust (#666666)
Debris: Brown/stone colors
```

---

## 📚 Further Reading

### Recommended Resources
1. **"The Art of Screenshake"** by Jan Willem Nijman (Vlambeer)
   - Essential for understanding game feel
   - Video available on YouTube

2. **"Juice It or Lose It"** - GDC Talk
   - How small effects create big impact
   - Before/after demonstrations

3. **Hollow Knight Developer Commentary**
   - Team Cherry's approach to VFX
   - Found in behind-the-scenes content

4. **Celeste Postmortem**
   - Maddy Thorson on effect implementation
   - Pixel art animation techniques

---

## ✅ Quick Reference Checklist

When implementing any VFX:
- [ ] Does it have **anticipation** (windup)?
- [ ] Does it have strong **impact** (brightest frame)?
- [ ] Does it have **follow-through** (lingering effects)?
- [ ] Does it provide **contrast** against background?
- [ ] Does it run at consistent **framerate**?
- [ ] Does it **despawn** properly (no memory leaks)?
- [ ] Does it **feel good** when triggered repeatedly?
- [ ] Is it **visible** during gameplay chaos?

---

**Last Updated**: 2025-10-05
**Game Version**: Post-Physics-Refactor
**Status**: Reference Guide for VFX Implementation
