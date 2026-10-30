# Unified Physics Refactoring - Complete!

## What Was The Problem?
We had **3 separate collision functions** with the **SAME physics code duplicated**:
- `check_collisions()` - for Hero only
- `check_villager_collisions()` - for Villagers only
- `check_enemy_collisions()` - for Enemies only

Every time we fixed physics (like the sideways bouncing), we had to copy-paste the SAME code 3 times!

## The Solution
Created **ONE unified collision function** that works for ALL "heads" (hero, villagers, enemies):

```rust
fn check_head_block_collisions(
    mut head_query: Query<
        (Entity, &mut Transform, &mut Velocity, &Bouncy),
        Or<(With<Hero>, With<Villager>, With<Enemy>)>  // <-- All heads in ONE query!
    >,
    // ... other params
) {
    for (head_entity, mut head_transform, mut head_velocity, bouncy) in head_query.iter_mut() {
        // ONE implementation for ALL heads!
    }
}
```

## What Changed

### Before:
```rust
.add_systems(Update, (
    check_collisions,          // Hero physics (100 lines)
    check_villager_collisions, // Villagers physics (100 lines - DUPLICATE!)
    check_enemy_collisions,    // Enemies physics (100 lines - DUPLICATE!)
    ...
))
```

### After:
```rust
.add_systems(Update, (
    check_head_block_collisions,  // ALL heads physics (100 lines - NO DUPLICATION!)
    ...
))
```

## Benefits

### ✅ DRY Principle
- **No code duplication** - physics logic in ONE place
- Fixed sideways bouncing ONCE, works for all heads

### ✅ Consistency
- Hero, Villagers, and Enemies have the SAME default behaviors:
  - Stronger horizontal bounce (0.8 vs 0.5)
  - Random horizontal kicks (-150..150)
  - Lower max vertical bounce (300)
  - Same collision detection

### ✅ Maintainability
- Want to change how heads bounce? Change ONE function!
- Add new physics behavior? Write it ONCE!

### ✅ Less Code
- Deleted ~200 lines of duplicate code
- Easier to understand and debug

## Hero Movement Fix
Also fixed hero movement system to preserve bouncing momentum:
- Changed from **SETTING** velocity to **ADDING** to velocity
- Removed friction that was killing sideways movement
- Hero now bounces naturally while still being controllable

## Summary
**Before**: 3 functions, 300+ lines, duplicated logic, hard to maintain
**After**: 1 function, 100 lines, unified behavior, easy to maintain

All "heads" now share the SAME physics by default - just like you wanted! 🎉
