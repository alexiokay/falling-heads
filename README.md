# MineHeads - Falling Block Breaker

A 3D isometric infinite falling game built with Bevy 0.17.1 where Minecraft-style heads descend through an endless procedurally generated world of destructible blocks.

## 🎮 Game Overview

Control a falling hero as it bounces through an infinite chunk-based world with multiple biomes. Break blocks by colliding with them, collect resources, and use powerful abilities triggered by external HTTP API!

### ✨ Features
- **🌍 Infinite procedural world** with chunk-based generation (16x16 blocks)
- **🏔️ 5 Biomes** - Plains, Forest, Desert, Mountains, Underground
- **🕳️ Cave systems** with pseudo-noise generation
- **📦 23 Block types** - Terrain, Ores, and Decorative blocks
- **⚡ 10 Power commands** via HTTP API (TNT, FAST, SLOW, ARROW, LASER, etc.)
- **💎 Resource collection** (Coal, Iron, Gold, Diamond, Emerald, Redstone)
- **🎯 Physics-based** AABB collision with bouncing
- **🧑‍🤝‍🧑 Villager NPCs** with AI and collision
- **🎨 3D isometric rendering** with unified coordinate space
- **📊 Block damage system** - 5 hits to break, visual feedback

## 🚀 Getting Started

### Prerequisites
- Rust 1.88.0 or higher
- Bevy 0.17.1 (included via path dependency)

### Building
```bash
cargo build --release
```

### Running
```bash
cargo run --release
```

## 🎯 External API

The game exposes an HTTP API on `http://localhost:3029` for triggering powers:

```bash
# Example: Trigger TNT explosion
curl -X POST http://localhost:3029/command \
  -H "Content-Type: application/json" \
  -d '{"action": "TNT"}'
```

### Available Commands
- `TNT` ✅ - Spawn explosive TNT above hero
- `FAST` ✅ - Speed boost (2x for 5 seconds)
- `SLOW` ✅ - Slow motion (0.5x for 5 seconds)
- `LASER` ✅ - Powerful vertical laser beam
- `ARROW` ✅ - Circular arrow barrage (8 arrows)
- `FUSS` 🚧 - Create chaos effect (coming soon)
- `BOLT` 🚧 - Lightning strike (coming soon)
- `GHAST` 🚧 - Spawn shooting Ghast enemy (coming soon)
- `BOX` 🚧 - Spawn loot box (coming soon)
- `MAGNET` 🚧 - Attract all resources (coming soon)

## 📁 Project Structure

```
falling-heads/
├── src/
│   ├── components/     # ECS components
│   ├── resources/      # Global resources
│   ├── systems/        # Game systems
│   ├── events/         # Custom events
│   ├── plugins/        # Feature plugins
│   ├── api/            # External API
│   ├── utils/          # Utilities
│   ├── config/         # Configuration
│   └── states.rs       # Game states
├── assets/             # Game assets
├── tests/              # Tests
└── docs/               # Documentation
    ├── GAME_DESIGN.md
    ├── ARCHITECTURE.md
    └── IMPLEMENTATION_PLAN.md
```

## 🛠️ Development

See [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for detailed development roadmap.

## 📚 Documentation

- [Game Design Document](GAME_DESIGN.md)
- [Architecture Document](ARCHITECTURE.md)
- [Implementation Plan](IMPLEMENTATION_PLAN.md)

## 🎨 Controls

- **A/D or ←/→** - Move horizontally
- **ESC** - Pause/Menu

## 📝 License

MIT OR Apache-2.0

## 🤝 Contributing

This is a game development project. Contributions welcome!
