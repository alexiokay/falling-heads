# Minecraft Head Fall - Architecture & Repository Structure

## Repository Structure

```
minecraft-head-fall/
├── Cargo.toml                      # Main project manifest
├── Cargo.lock                      # Dependency lock file
├── README.md                       # Project overview
├── GAME_DESIGN.md                  # Game design document
├── ARCHITECTURE.md                 # This file
│
├── assets/                         # Game assets
│   ├── textures/
│   │   ├── blocks/
│   │   │   ├── stone.png
│   │   │   ├── coal.png
│   │   │   ├── iron.png
│   │   │   ├── gold.png
│   │   │   ├── diamond.png
│   │   │   ├── emerald.png
│   │   │   └── redstone.png
│   │   ├── entities/
│   │   │   ├── hero.png
│   │   │   ├── ghast.png
│   │   │   └── tnt.png
│   │   ├── projectiles/
│   │   │   ├── arrow.png
│   │   │   ├── fireball.png
│   │   │   └── laser.png
│   │   ├── effects/
│   │   │   ├── explosion.png
│   │   │   ├── particles.png
│   │   │   └── lightning.png
│   │   └── ui/
│   │       ├── icons.png
│   │       └── font.ttf
│   ├── audio/
│   │   ├── music/
│   │   │   └── background.ogg
│   │   └── sfx/
│   │       ├── block_hit.ogg
│   │       ├── block_break.ogg
│   │       ├── explosion.ogg
│   │       ├── collect.ogg
│   │       └── power_activate.ogg
│   └── shaders/                    # Custom shaders if needed
│       └── laser.wgsl
│
├── src/
│   ├── main.rs                     # Entry point, app setup
│   ├── lib.rs                      # Library root (optional for testing)
│   │
│   ├── states/                     # Game states
│   │   ├── mod.rs
│   │   ├── menu.rs                 # Main menu state
│   │   ├── playing.rs              # Playing state
│   │   ├── paused.rs               # Paused state
│   │   └── game_over.rs            # Game over state
│   │
│   ├── components/                 # ECS Components
│   │   ├── mod.rs
│   │   ├── hero.rs                 # Hero component
│   │   ├── block.rs                # Block components (BlockType, BiomeType)
│   │   ├── resource.rs             # Resource/collectible components
│   │   ├── physics.rs              # Velocity, Gravity, Bouncy
│   │   ├── projectile.rs           # Arrow, Laser, Fireball components
│   │   ├── enemy.rs                # Enemy, EnemyType, Health components
│   │   ├── explosive.rs            # TNT, Explosion components
│   │   ├── power_effect.rs         # PowerEffect, Duration components
│   │   ├── visual.rs               # AnimationState, ParticleEmitter
│   │   ├── villager.rs             # Villager AI component
│   │   ├── chunk.rs                # Chunk and ChunkMember components
│   │   ├── health_ui.rs            # HealthBar component
│   │   └── background.rs           # Background component
│   │
│   ├── resources/                  # ECS Resources (global state)
│   │   ├── mod.rs
│   │   ├── game_state.rs           # GameState resource
│   │   ├── score.rs                # Score and statistics
│   │   ├── inventory.rs            # Player inventory
│   │   ├── command_queue.rs        # External command queue
│   │   ├── world_config.rs         # World generation config
│   │   └── asset_handles.rs        # GameTextures, BlockTextures handles
│   │
│   ├── systems/                    # ECS Systems
│   │   ├── mod.rs
│   │   │
│   │   ├── physics/                # Physics systems
│   │   │   ├── mod.rs
│   │   │   ├── gravity.rs          # Apply gravity
│   │   │   ├── velocity.rs         # Apply velocity
│   │   │   ├── collision.rs        # Collision detection
│   │   │   └── bounce.rs           # Bounce mechanics
│   │   │
│   │   ├── gameplay/               # Core gameplay systems
│   │   │   ├── mod.rs
│   │   │   ├── hero_movement.rs    # Hero input handling
│   │   │   ├── block_damage.rs     # Block health and damage
│   │   │   ├── world_generation.rs # Chunk-based world generation with biomes
│   │   │   ├── resource_drop.rs    # Resource spawning from blocks
│   │   │   ├── resource_collect.rs # Resource collection
│   │   │   └── camera_follow.rs    # Camera tracking hero
│   │   │
│   │   ├── powers/                 # Power effect systems
│   │   │   ├── mod.rs
│   │   │   ├── tnt.rs              # TNT spawning and explosion
│   │   │   ├── speed_modifier.rs   # FAST/SLOW effects
│   │   │   ├── bolt.rs             # Lightning strike
│   │   │   ├── arrow.rs            # Arrow circular attack
│   │   │   ├── laser.rs            # Laser beam
│   │   │   ├── ghast.rs            # Ghast spawning and AI
│   │   │   ├── box_spawn.rs        # Loot box spawning
│   │   │   ├── magnet.rs           # Magnet effect
│   │   │   ├── fuss.rs             # Chaos effect
│   │   │   └── effect_duration.rs  # Timer for temporary effects
│   │   │
│   │   ├── combat/                 # Combat-related systems
│   │   │   ├── mod.rs
│   │   │   ├── projectile_movement.rs
│   │   │   ├── projectile_collision.rs
│   │   │   ├── explosion.rs
│   │   │   └── enemy_ai.rs
│   │   │
│   │   ├── rendering/              # Visual systems
│   │   │   ├── mod.rs
│   │   │   ├── animation.rs        # Sprite animation
│   │   │   ├── particles.rs        # Particle effects
│   │   │   ├── damage_visual.rs    # Block crack visuals
│   │   │   └── screen_shake.rs     # Camera shake effects
│   │   │
│   │   ├── ui/                     # UI systems
│   │   │   ├── mod.rs
│   │   │   ├── hud.rs              # HUD updates
│   │   │   ├── menu.rs             # Menu interactions
│   │   │   └── notifications.rs    # Power notifications
│   │   │
│   │   ├── health_system.rs        # Entity damage and death handling
│   │   ├── health_ui_system.rs     # Health bar/hearts rendering
│   │   └── background_system.rs    # Biome-based background rendering
│   │
│   ├── events/                     # Custom events
│   │   ├── mod.rs
│   │   ├── block_events.rs         # BlockDestroyed, BlockDamaged
│   │   ├── resource_events.rs      # ResourceCollected
│   │   ├── power_events.rs         # PowerActivated
│   │   ├── damage_events.rs        # EntityDamaged, EntityDied
│   │   └── game_events.rs          # GameOver, ScoreChanged
│   │
│   ├── plugins/                    # Bevy plugins (feature modules)
│   │   ├── mod.rs
│   │   ├── physics_plugin.rs       # Physics systems bundle
│   │   ├── gameplay_plugin.rs      # Gameplay systems bundle
│   │   ├── powers_plugin.rs        # Power systems bundle
│   │   ├── rendering_plugin.rs     # Rendering systems bundle
│   │   ├── ui_plugin.rs            # UI systems bundle
│   │   ├── audio_plugin.rs         # Audio systems
│   │   └── external_api_plugin.rs  # HTTP/WebSocket server
│   │
│   ├── api/                        # External service integration
│   │   ├── mod.rs
│   │   ├── server.rs               # HTTP/WebSocket server
│   │   ├── commands.rs             # Command parser and validator
│   │   └── handlers.rs             # Command execution handlers
│   │
│   ├── utils/                      # Utility functions
│   │   ├── mod.rs
│   │   ├── constants.rs            # Game constants
│   │   ├── math.rs                 # Math helpers
│   │   └── spawn_helpers.rs        # Entity spawning helpers
│   │
│   └── config/                     # Configuration
│       ├── mod.rs
│       ├── game_config.rs          # Gameplay configuration
│       └── render_config.rs        # Rendering configuration
│
├── tests/                          # Integration tests
│   ├── gameplay_tests.rs
│   ├── physics_tests.rs
│   └── power_tests.rs
│
└── benches/                        # Performance benchmarks
    └── block_generation.rs
```

---

## Bevy Architecture Patterns

### 1. **Plugin-Based Architecture**

Following Bevy's modular design, organize features as plugins:

```rust
// src/main.rs
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            PhysicsPlugin,
            GameplayPlugin,
            PowersPlugin,
            RenderingPlugin,
            UiPlugin,
            AudioPlugin,
            ExternalApiPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}
```

### 2. **Component Organization**

**Marker Components** (zero-sized types):
```rust
#[derive(Component)]
struct Hero;

#[derive(Component)]
struct Ghast;
```

**Data Components** (hold state):
```rust
#[derive(Component)]
struct Health {
    current: u32,
    max: u32,
}

#[derive(Component)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Component)]
struct BlockType {
    material: Material,
}

#[derive(Component)]
struct PowerEffect {
    kind: PowerKind,
    duration: Timer,
}
```

### 3. **System Organization**

**System Sets for Ordering**:
```rust
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum PhysicsSet {
    Input,
    Movement,
    Collision,
    Response,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum GameplaySet {
    Generation,
    Damage,
    Collection,
    Effects,
}
```

**System Registration**:
```rust
impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                PhysicsSet::Input,
                PhysicsSet::Movement,
                PhysicsSet::Collision,
                PhysicsSet::Response,
            ).chain()
        )
        .add_systems(
            Update,
            (
                apply_gravity.in_set(PhysicsSet::Movement),
                apply_velocity.in_set(PhysicsSet::Movement),
                detect_collisions.in_set(PhysicsSet::Collision),
                handle_bounce.in_set(PhysicsSet::Response),
            )
        );
    }
}
```

### 4. **Event-Driven Communication**

```rust
// src/events/block_events.rs
#[derive(Event)]
struct BlockDestroyed {
    entity: Entity,
    block_type: Material,
    position: Vec3,
}

#[derive(Event)]
struct BlockDamaged {
    entity: Entity,
    damage: u32,
    remaining_health: u32,
}

// In systems:
fn block_damage_system(
    mut events: EventWriter<BlockDamaged>,
    mut destroyed_events: EventWriter<BlockDestroyed>,
    // ...
) {
    // Write events when blocks are damaged/destroyed
}

fn resource_drop_system(
    mut commands: Commands,
    mut events: EventReader<BlockDestroyed>,
) {
    for event in events.read() {
        // Spawn resources based on destroyed blocks
    }
}
```

### 5. **Resource Management**

**Global Game State**:
```rust
#[derive(Resource)]
struct GameState {
    depth: f32,
    blocks_destroyed: u32,
    game_speed: f32,
}

#[derive(Resource)]
struct Inventory {
    coal: u32,
    iron: u32,
    gold: u32,
    diamond: u32,
    emerald: u32,
    redstone: u32,
}

#[derive(Resource)]
struct CommandQueue {
    pending: VecDeque<PowerCommand>,
}
```

### 6. **State Management**

```rust
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Menu,
    Playing,
    Paused,
    GameOver,
}

// State-specific systems
app.add_systems(OnEnter(GameState::Playing), setup_game)
   .add_systems(Update, gameplay_systems.run_if(in_state(GameState::Playing)))
   .add_systems(OnExit(GameState::Playing), cleanup_game);
```

---

## Core System Details

### Physics Plugin
```rust
// src/plugins/physics_plugin.rs
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .configure_sets(Update, (
                PhysicsSet::Input,
                PhysicsSet::Movement,
                PhysicsSet::Collision,
                PhysicsSet::Response,
            ).chain())
            .add_systems(Update, (
                apply_gravity,
                apply_velocity,
                detect_collisions,
                handle_bounce,
            ));
    }
}
```

### Gameplay Plugin
```rust
pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<BlockDestroyed>()
            .add_event::<BlockDamaged>()
            .add_event::<ResourceCollected>()
            .add_systems(Startup, spawn_initial_blocks)
            .add_systems(Update, (
                hero_input,
                block_generation,
                block_damage,
                resource_spawning,
                resource_collection,
                camera_follow,
                update_depth,
            ).chain());
    }
}
```

### Powers Plugin
```rust
pub struct PowersPlugin;

impl Plugin for PowersPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<PowerActivated>()
            .add_systems(Update, (
                process_command_queue,
                // Individual power systems
                tnt_explosion_timer,
                speed_modifier_effect,
                lightning_bolt,
                arrow_rotation,
                laser_beam,
                ghast_ai,
                loot_box_spawn,
                magnet_attraction,
                chaos_effect,
                // Cleanup
                expire_effects,
            ));
    }
}
```

### External API Plugin
```rust
pub struct ExternalApiPlugin;

impl Plugin for ExternalApiPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(CommandQueue::default())
            .add_systems(Startup, start_api_server)
            .add_systems(Update, process_incoming_commands);
    }
}
```

---

## External API Design

### HTTP Server (using axum)

```rust
// src/api/server.rs
use axum::{Router, Json};
use tokio::sync::mpsc;

#[derive(Deserialize)]
struct CommandRequest {
    action: String,
}

async fn command_handler(
    Json(cmd): Json<CommandRequest>,
    tx: State<mpsc::Sender<PowerCommand>>,
) -> impl IntoResponse {
    match cmd.action.as_str() {
        "TNT" => tx.send(PowerCommand::Tnt).await,
        "FAST" => tx.send(PowerCommand::Fast).await,
        "SLOW" => tx.send(PowerCommand::Slow).await,
        // ... other commands
        _ => return Json(json!({"error": "Unknown command"})),
    }

    Json(json!({"success": true}))
}

pub fn start_server(tx: mpsc::Sender<PowerCommand>) {
    tokio::spawn(async move {
        let app = Router::new()
            .route("/command", post(command_handler))
            .with_state(tx);

        axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
}
```

### Command Processing in Bevy

```rust
// src/systems/powers/mod.rs
fn process_command_queue(
    mut commands: Commands,
    mut queue: ResMut<CommandQueue>,
    hero_query: Query<&Transform, With<Hero>>,
) {
    while let Some(command) = queue.pending.pop_front() {
        match command {
            PowerCommand::Tnt => spawn_tnt(&mut commands, hero_query.single()),
            PowerCommand::Fast => apply_speed_buff(&mut commands),
            // ... handle other commands
        }
    }
}
```

---

## Performance Considerations

### 1. **Spatial Partitioning**
- Use spatial hash or quadtree for collision detection
- Only check nearby blocks for collisions

### 2. **Object Pooling**
- Reuse block entities instead of despawning/spawning
- Pool projectiles and particles

### 3. **LOD (Level of Detail)**
- Simplify rendering for distant blocks
- Cull blocks outside camera view

### 4. **Batching**
- Batch similar block types for rendering
- Use sprite sheets for texture atlases

### 5. **Asset Loading**
- Preload common assets at startup
- Use `AssetServer` handles efficiently

---

## Dependencies (Cargo.toml)

```toml
[package]
name = "minecraft-head-fall"
version = "0.1.0"
edition = "2024"
rust-version = "1.88.0"

[dependencies]
bevy = "0.17.1"
bevy_rapier2d = "0.30"  # Optional: Physics engine (compatible with Bevy 0.17)
rand = "0.8"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# External API
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["cors"] }

[dev-dependencies]
criterion = "0.5"

[profile.dev]
opt-level = 1  # Faster debug builds

[profile.dev.package."*"]
opt-level = 3  # Optimize dependencies

[profile.release]
lto = true
codegen-units = 1
```

---

## Development Workflow

### 1. **Phase 1: Core Setup**
- Initialize Bevy app
- Set up basic plugin structure
- Implement state management
- Create basic hero and block entities

### 2. **Phase 2: Physics**
- Implement gravity and velocity
- Add collision detection
- Implement bounce mechanics

### 3. **Phase 3: Gameplay Loop**
- Infinite block generation
- Block damage system
- Resource drops and collection
- Camera following

### 4. **Phase 4: Powers**
- Implement each power command
- Add visual effects
- Add external API server

### 5. **Phase 5: Polish**
- UI/HUD
- Audio integration
- Particle effects
- Performance optimization

---

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_health() {
        // Test block damage logic
    }

    #[test]
    fn test_resource_drops() {
        // Test resource spawning
    }
}
```

### Integration Tests
```rust
// tests/gameplay_tests.rs
#[test]
fn test_hero_bounce() {
    let mut app = App::new();
    app.add_plugins(PhysicsPlugin);
    // Test hero bouncing off blocks
}
```

### Benchmarks
```rust
// benches/block_generation.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_block_generation(c: &mut Criterion) {
    c.bench_function("generate 1000 blocks", |b| {
        b.iter(|| {
            // Benchmark block generation
        });
    });
}
```

---

## Code Style Guidelines

1. **Follow Rust conventions**: Use `snake_case` for functions/variables
2. **Component naming**: Descriptive, single-responsibility
3. **System naming**: Verb-based (e.g., `apply_gravity`, `spawn_resources`)
4. **Document public APIs**: Use doc comments
5. **Error handling**: Use `Result` types where appropriate
6. **Const for magic numbers**: Define constants in `utils/constants.rs`

---

This architecture follows Bevy's ECS paradigm, promotes modularity, and scales well for future features.
