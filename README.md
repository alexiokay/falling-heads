# Falling Heads

A 2.5D infinite falling game built with Bevy 0.17.1 where Minecraft-style heads descend through an endless column of destructible blocks.

## 🎮 Game Overview

Control a falling Minecraft head as it bounces through an infinite world of blocks. Break blocks by colliding with them, collect resources, and use powerful abilities triggered by external services!

### Features
- **Infinite falling gameplay** with procedurally generated blocks
- **10 unique powers** triggered via external API (TNT, FAST, SLOW, ARROW, LASER, BOLT, GHAST, BOX, MAGNET, FUSS)
- **Resource collection system** (Coal, Iron, Gold, Diamond, Emerald, Redstone)
- **Physics-based bouncing** and collision
- **Block damage system** - blocks break after 5 hits
- **2.5D graphics** with particle effects

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

The game exposes an HTTP API on `http://localhost:3000` for triggering powers:

```bash
# Example: Trigger TNT explosion
curl -X POST http://localhost:3000/command \
  -H "Content-Type: application/json" \
  -d '{"action": "TNT"}'
```

### Available Commands
- `TNT` - Spawn explosive TNT
- `FAST` - Speed up the hero
- `SLOW` - Slow down the hero
- `FUSS` - Create chaos
- `BOLT` - Lightning strike
- `ARROW` - Circular arrow barrage
- `LASER` - Powerful laser beam
- `GHAST` - Spawn shooting Ghast enemy
- `BOX` - Spawn loot box
- `MAGNET` - Attract all resources

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
