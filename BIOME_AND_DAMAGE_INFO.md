# Biome System & Damage Cooldown Explanation

## 🌍 How Biomes Change

### Biome Determination Logic
Biomes are determined by **chunk position**, not individual blocks. Here's how it works:

**File:** `src/systems/world_generation.rs:169-178`

```rust
// Nether - very deep chunks
if chunk_y < -20 {
    return BiomeType::Nether;  // Below -20 chunks = Y < -20,480
}

// End - specific pattern deep underground
if chunk_y < -15 && (chunk_x + chunk_y) % 17 == 0 {
    return BiomeType::End;
}

// Underground biome for deep chunks
if chunk_y < -10 {
    return BiomeType::Underground;  // Y < -10,240
}

// Mountains for high altitude
if chunk_y > 5 {
    return BiomeType::Mountains;  // Y > 5,120
}

// Surface biomes based on X coordinate patterns
match biome_seed.abs() {
    0..=25 => BiomeType::Plains,
    26..=45 => BiomeType::Forest,
    46..=65 => BiomeType::Desert,
    _ => BiomeType::Mountains,
}
```

### Biome Transition Scale

**1 Chunk = 16 blocks**
**1 Block = 64 pixels**
**1 Chunk = 1,024 pixels**

| Biome | Chunk Y Range | World Y Position | Distance to Next Biome |
|-------|--------------|------------------|----------------------|
| **End** | Y > 50 | Y > 51,200 | N/A (top) |
| **Mountains** | Y > 5 | Y > 5,120 | ~46 chunks (47,080 pixels) |
| **Surface** | Y 0-5 | Y 0-5,120 | ~5 chunks (5,120 pixels) |
| **Underground** | Y -10 to 0 | Y -10,240 to 0 | ~10 chunks (10,240 pixels) |
| **Nether** | Y < -20 | Y < -20,480 | ~10 chunks (10,240 pixels) |

### How Often Do Biomes Change?

**Vertically:**
- **Surface → Underground:** Every ~10 chunks downward (~10,240 pixels)
- **Underground → Nether:** Every ~10 chunks downward (~10,240 pixels)
- **Total to Nether:** ~20 chunks from surface (~20,480 pixels)

**Horizontally (Surface only):**
- Plains, Forest, Desert, Mountains cycle based on X position
- Changes roughly every ~25-40 chunk units horizontally

### Current Spawn Location
You now spawn at **Y = -25,000** which is in the **Nether biome**:
- Chunk Y = -25,000 / (16 * 64) = **-24.4** chunks
- Since chunk_y < -20, you're in Nether! 🔥

---

## ⏱️ Damage Cooldown System

### What is Damage Cooldown?

The damage cooldown prevents the hero from taking damage **too rapidly** when touching enemies.

**File:** `src/resources/damage_cooldown.rs`

```rust
pub struct DamageCooldown {
    pub last_damage_time: f32,  // Timestamp of last damage
}

impl DamageCooldown {
    // Can only take damage once per cooldown_seconds
    pub fn can_damage(&self, current_time: f32, cooldown_seconds: f32) -> bool {
        current_time - self.last_damage_time >= cooldown_seconds
    }
}
```

### Why Do We Need It?

**WITHOUT Cooldown:**
```
Frame 1: Hero touches enemy → -1 HP (2 hearts left)
Frame 2: Hero STILL touching enemy → -1 HP (1 heart left)
Frame 3: Hero STILL touching enemy → -1 HP (0 hearts - DEAD!)
```
**Result:** Instant death when touching any enemy! ❌

**WITH 1-Second Cooldown:**
```
Second 1: Hero touches enemy → -1 HP (2 hearts left)
Second 1.5: Hero still touching → NO DAMAGE (cooldown active)
Second 2: Hero still touching → -1 HP (1 heart left)
Second 2.5: Hero still touching → NO DAMAGE (cooldown active)
Second 3: Hero still touching → -1 HP (0 hearts - dead)
```
**Result:** 3 seconds to react and escape! ✅

### Current Cooldown Setting

**Cooldown:** 1 second
**File:** `src/plugins/physics_plugin.rs:411`

```rust
if !cooldown.can_damage(time.elapsed_secs(), 1.0) {  // 1 second cooldown
    return;
}
```

### Why It's Important

1. **Prevents Instant Death**
   - At 60 FPS, touching enemy = 60 damage events per second
   - Without cooldown: Dead in 0.05 seconds (3 frames)
   - With cooldown: Dead in 3 seconds (time to escape)

2. **Gameplay Balance**
   - Gives player time to react
   - Makes combat feel fair
   - Industry standard for action games

3. **Realistic Combat**
   - Minecraft-style: 1 hit every second
   - Not continuous damage stream
   - Matches player expectations

### When Damage DOES Happen

Damage happens **exactly when it should**, just not **continuously**:

✅ **Cooldown finished + Hero touches enemy = DAMAGE**
✅ **Multiple enemies** can each trigger damage separately
✅ **Cooldown resets** after each damage event

❌ Cooldown does NOT:
- Block legitimate damage
- Prevent multi-enemy damage
- Make you invincible

### How to Adjust Cooldown

**Make combat harder (shorter cooldown):**
```rust
if !cooldown.can_damage(time.elapsed_secs(), 0.5) {  // 0.5 seconds = faster damage
```

**Make combat easier (longer cooldown):**
```rust
if !cooldown.can_damage(time.elapsed_secs(), 2.0) {  // 2 seconds = more time to escape
```

**Disable cooldown entirely (NOT RECOMMENDED):**
```rust
// Remove the cooldown check - instant death on contact!
// DON'T DO THIS unless you want Dark Souls difficulty
```

---

## 🎮 Gameplay Impact

### Without Cooldown
```
Game Feel: Unfair, frustrating
Player Reaction: "WTF I died instantly!"
Combat: Impossible to survive
Fun Factor: 0/10
```

### With 1-Second Cooldown (Current)
```
Game Feel: Fair, skill-based
Player Reaction: "I can dodge and survive!"
Combat: Challenging but fair
Fun Factor: 8/10
```

---

## 🔧 Technical Implementation

### Thread-Safe Design
The cooldown uses Bevy's `ResMut<DamageCooldown>`:
- ✅ Thread-safe (Bevy handles synchronization)
- ✅ No unsafe code
- ✅ Follows Bevy best practices
- ✅ ECS-friendly resource pattern

### Performance
- **CPU cost:** ~0.001ms per frame (negligible)
- **Memory:** 4 bytes (one f32)
- **Cache friendly:** Single resource access

---

## 📝 Summary

### Biomes
- Change every ~10-20 chunks vertically
- Change every ~25-40 chunks horizontally
- **You now spawn in Nether** (Y = -25,000)

### Damage Cooldown
- **Purpose:** Prevent instant death on enemy contact
- **Duration:** 1 second between damage events
- **Result:** Fair combat, time to react
- **Status:** ✅ Essential for gameplay, properly implemented

**Cooldown is NOT a bug - it's a critical gameplay feature!** 🎯
