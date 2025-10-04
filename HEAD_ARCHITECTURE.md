# Head Architecture Analysis & Improvement

## Current State: ❌ CODE DUPLICATION

### Problem: Each head entity duplicates the same components

**Hero Spawn (78-95 lines):**
```rust
commands.spawn((
    Hero,
    Health::new(3),
    Buffable,
    Velocity { x: 0.0, y: 0.0 },
    Gravity { force: -500.0 },
    Bouncy { restitution: 0.85 },
    Mesh3d(meshes.add(Rectangle::new(64.0, 64.0))),
    MeshMaterial3d(materials.add(StandardMaterial { ... })),
    Transform::from_xyz(0.0, -25000.0, 10.0),
));
```

**Villager Spawn (122-140 lines):**
```rust
commands.spawn((
    Villager { ai_timer: Timer::from_seconds(1.0, TimerMode::Repeating) },
    Health::new(6),
    Buffable,
    Velocity { x: 0.0, y: 0.0 },
    Gravity { force: -500.0 },
    Bouncy { restitution: 0.8 },
    Mesh3d(meshes.add(Rectangle::new(56.0, 56.0))),
    MeshMaterial3d(materials.add(StandardMaterial { ... })),
    Transform::from_xyz(offset_x, hero_y + offset_y, 10.0),
));
```

**Enemy Spawn (378-397 lines):**
```rust
commands.spawn((
    Enemy::new(enemy_type),
    Health::new(max_health),
    ChunkMember { chunk_x, chunk_y },
    Velocity { x: 0.0, y: 0.0 },
    Gravity { force: -500.0 },
    Bouncy { restitution: 0.6 },
    Mesh3d(mesh_cache.enemy_mesh.clone()),
    MeshMaterial3d(materials.add(StandardMaterial { ... })),
    Transform::from_xyz(world_x, world_y, 50.0),
));
```

### Duplicated Components (EVERY head has these):
- ❌ `Health`
- ❌ `Buffable`
- ❌ `Velocity { x: 0.0, y: 0.0 }`
- ❌ `Gravity { force: -500.0 }`
- ❌ `Bouncy { restitution: ... }`
- ❌ `Mesh3d`
- ❌ `MeshMaterial3d`
- ❌ `Transform`

## ✅ NEW SOLUTION: HeadBundle

### Created: `src/components/head.rs`

```rust
#[derive(Bundle)]
pub struct HeadBundle {
    pub health: Health,
    pub buffable: Buffable,
    pub velocity: Velocity,
    pub gravity: Gravity,
    pub bouncy: Bouncy,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

impl HeadBundle {
    pub fn new(
        health: u32,
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        position: Vec3,
        restitution: f32,
    ) -> Self {
        // All common defaults in ONE place!
    }
}
```

### Benefits:

1. **✅ No Code Duplication** - Common components defined once
2. **✅ Consistent Defaults** - All heads have same gravity, initial velocity
3. **✅ Easy to Modify** - Change all heads by editing HeadBundle
4. **✅ Type Safety** - Bundle ensures all heads have required components
5. **✅ Cleaner Spawning** - Shorter, more readable spawn code

### Example: Spawning with HeadBundle

**Before (9 lines):**
```rust
commands.spawn((
    Hero,
    Health::new(3),
    Buffable,
    Velocity { x: 0.0, y: 0.0 },
    Gravity { force: -500.0 },
    Bouncy { restitution: 0.85 },
    Mesh3d(...),
    MeshMaterial3d(...),
    Transform::from_xyz(0.0, -25000.0, 10.0),
));
```

**After (4 lines):**
```rust
commands.spawn((
    Hero,
    HeadBundle::new(3, mesh, material, Vec3::new(0.0, -25000.0, 10.0), 0.85),
));
```

## Shared Traits Across All Heads

### Physics Components (via HeadBundle):
- `Velocity` - Movement in X/Y
- `Gravity { force: -500.0 }` - Falling physics
- `Bouncy { restitution }` - Bounce behavior (varies by entity)

### Gameplay Components (via HeadBundle):
- `Health` - Current/max HP (varies by entity)
- `Buffable` - Can receive status effects (burning, freezing, etc.)

### Rendering Components (via HeadBundle):
- `Mesh3d` - 3D mesh (Rectangle for sprites)
- `MeshMaterial3d` - Texture and material
- `Transform` - Position, rotation, scale

### Entity-Specific Components (NOT in bundle):
- `Hero` - Player marker
- `Villager { ai_timer }` - Friendly NPC with AI
- `Enemy { enemy_type, shoot_timer }` - Hostile entity
- `ChunkMember` - World chunk tracking (for enemies only)

## Migration Status

### ✅ Completed:
- [x] Created `HeadBundle` struct
- [x] Implemented `HeadBundle::new()` helper
- [x] Added to `src/components/mod.rs`
- [x] Compiles successfully

### 🔄 In Progress:
- [ ] Update `spawn_hero()` to use HeadBundle
- [ ] Update `spawn_villagers()` to use HeadBundle
- [ ] Update enemy spawning to use HeadBundle

### 📋 Future Considerations:
- Create `Head` trait for shared behavior methods
- Add `HeadConfig` for entity-specific stats
- Consider builder pattern for complex heads

## Architecture Decision

**Current:** Each head is a separate Component + common bundles ✅

This is the **correct Bevy pattern** because:
1. **ECS Composition** - Entities composed of components
2. **Query Flexibility** - Can query `With<Hero>`, `Or<(Hero, Villager)>`, etc.
3. **Bundle Reuse** - HeadBundle provides shared components
4. **Marker Components** - Hero, Villager, Enemy are lightweight markers

**Alternative Rejected:** Inheritance/trait objects ❌
- Not idiomatic in Bevy ECS
- Less performant (virtual dispatch)
- Harder to query specific entity types
