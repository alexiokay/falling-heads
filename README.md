# MineHeads - Falling Block Breaker

A 3D isometric infinite falling game built with Bevy 0.20.0 where Minecraft-style heads descend through an endless procedurally generated world of destructible blocks.

## 🎮 Game Overview

Control a falling hero as it bounces through an infinite chunk-based world with multiple biomes. Break blocks by colliding with them, collect resources, and use powerful abilities triggered by external HTTP API!

### ✨ Features
- **🌍 Infinite procedural world** with chunk-based generation (16x16 blocks)
- **🏔️ 7 Biomes** - Plains, Forest, Desert, Mountains, Underground, Nether, End
- **🕳️ Biome-specific cave systems** with unique generation patterns
- **📦 35+ Block types** - Terrain, Ores, Decorative blocks with multi-texture support
- **👾 Enemy system** - 4 enemy types (Zombies, Skeletons, Pigmen, Endermen) with AI
- **❤️ Health & Combat** - 3 hearts max, visual health bars, damage cooldown
- **🎨 Dynamic backgrounds** - Biome-based backgrounds that change with depth
- **⚡ 10 Power commands** via HTTP API (TNT, FAST, SLOW, ARROW, LASER, etc.)
- **💎 Resource collection** (Coal, Iron, Gold, Diamond, Emerald, Redstone)
- **🎯 Physics-based** AABB collision with bouncing
- **🧑‍🤝‍🧑 Villager NPCs** with AI and collision
- **🎨 3D isometric rendering** with unified coordinate space
- **📊 Block damage system** - 5 hits to break, visual feedback

## 🚀 Getting Started

### Prerequisites
- Rust 1.97.1 or higher
- Bevy 0.20.0 and Avian 0.8.0 (registry dependencies)

See [the upgrade and validation checklist](MINEHEADS_UPGRADE_2026-10-23.md) for current fixes, validation limits and gameplay proposals.

### Building
```bash
cargo build --release
```

The Windows build compiles NVIDIA Streamline/DLSS glue and needs three SDK paths.
`.cargo/config.toml` points them at `../../south-balcony-twin/vendor/` (this
machine's layout). Cargo only uses those defaults when the variables are not
already set, so on another machine set them yourself before building:

| Variable | Must contain |
|----------|--------------|
| `VULKAN_SDK` | `include/vulkan/vulkan.h` (LunarG Vulkan SDK or a Vulkan-Headers checkout) |
| `DLSS_SDK` | the NVIDIA DLSS SDK (used by `vendor/dlss_wgpu`) |
| `LIBCLANG_PATH` | the folder holding `libclang.dll` (for bindgen) |

Five crates are patched from `vendor/` (`bevy_pbr`, `bevy_solari`,
`bevy_anti_alias`, `wgpu-hal`, `dlss_wgpu`; see `[patch.crates-io]` in
`Cargo.toml`). Every Bevy or wgpu upgrade needs those patches ported again.

Settings are saved per user in `%APPDATA%\MineHeads\` (`game-settings.json`,
`graphics-settings.json`). Test, probe and benchmark launches never overwrite them.

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

If Windows reports the port as forbidden, it is inside a reserved range
(`netsh interface ipv4 show excludedportrange protocol=tcp`; Hyper-V, WSL and
Docker reserve these). Start the game with `MINEHEADS_API_PORT` set to a free
port, or reserve 3029 for MineHeads as admin.

### Play styles
Each viewer chooses how their heads play the ball. The choice is remembered,
so heads they spawn later use it too. The streamer cycles the hero's style with **C**.

| Style | Behaviour |
|-------|-----------|
| `bouncy` | Default: full bounces off every surface |
| `heavy` | Falls fast, barely bounces, hard landings do double block damage (digs down) |
| `floaty` | Low gravity, steady soft hops, long hang time |
| `sticky` | Grabs walls, hangs on briefly, then wall-jumps up and away |

```bash
curl -X POST http://localhost:3029/command -H "Content-Type: application/json" \
  -d '{"action":"STYLE","username":"viewer123","style":"heavy"}'
# or spawn heads with a style straight away
curl -X POST http://localhost:3029/command -H "Content-Type: application/json" \
  -d '{"action":"VILLAGERS","username":"viewer123","style":"sticky"}'
```

### Available Commands
- `TNT` ✅ - Spawn explosive TNT above hero
- `FAST` ✅ - Speed boost (2x for 5 seconds)
- `SLOW` ✅ - Slow motion (0.5x for 5 seconds)
- `LASER` ✅ - Powerful vertical laser beam
- `ARROW` ✅ - Circular arrow barrage (8 arrows)
- `FUSS` ✅ - Mirrored controls for 5 seconds; AI heads turn around
- `BOLT` ✅ - One lightning strike (4 damage) on the toughest enemy nearby
- `FEATHER` ✅ - Low gravity for everyone for 8 seconds
- `ANVIL` ✅ - Three anvils smash down through up to 6 blocks each
- `GOLEM` ✅ - Friendly iron golem fights enemies for 30 seconds
- `PEARL` ✅ - Ender pearl: hero jumps to a free spot up to 10 blocks away
- `QUAKE` ✅ - Ores nearby pop out as drops, blocks crack, heads jolt
- `SHRINK` ✅ - Friendly heads half size for 10 seconds (no block damage)
- `WIND` ✅ - Gust pushes every head sideways for 3 seconds
- `BLACKHOLE` ✅ - Vortex pulls things in for 4 seconds, then pops them outward
- `ICEAGE` ✅ - Snow and slippery floors for 15 seconds
- `BEES` ✅ - Swarm chases a random head; a big bounce shakes it off
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
