# Falling Heads - Falling Block Breaker

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
- Rust 1.99.1 (the version pinned in `tools/build-environment.json`)
- Bevy 0.20.0 and Avian 0.8.0 (registry dependencies)

See [the upgrade and validation checklist](FALLING_HEADS_UPGRADE_2026-10-23.md) for current fixes, validation limits and gameplay proposals.

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

Settings are saved per user in `%APPDATA%\FallingHeads\` (`game-settings.json`,
`graphics-settings.json`). Test, probe and benchmark launches never overwrite them.

### Running
```bash
cargo run --release
```

## 🎯 External API

The game exposes an HTTP API for triggering powers, normally on `http://127.0.0.1:3029/command`:

```bash
# Example: Trigger TNT explosion
curl -X POST http://localhost:3029/command \
  -H "Content-Type: application/json" \
  -d '{"action": "TNT"}'
```

Windows can reserve port 3029 (Hyper-V, WSL and Docker reserve ranges that may
change at every boot; see `netsh interface ipv4 show excludedportrange protocol=tcp`).
The game then moves on by itself to 7329, 17329, 27329, 37329, 47329 or any free
port. The address in use is shown under **Settings > Game modes** and written to
`%APPDATA%\FallingHeads\api.json` (`{"url": ..., "port": ...}`) for tools such as
VortiDeck. `FALLING_HEADS_API_PORT` forces one port.

### Space prison
`PRISON` (or `JAIL`) locks the hero and one or two villagers in a 10x10 bedrock cell
floating in space above Earth. The cell spins (faster or slower every few seconds,
sometimes reversing), so heads are flung outward and curve as the walls sweep past;
four electromagnets on a space-station truss ring (solar wings, antenna, beacons)
take turns pulling everyone to one side (the active one heats its coil, arcs and
fires a field beam), a containment field shimmers around the cell, and
micro-blasts throw heads around. The station, cell and Earth are lit by the
scene's own lights whatever Graphics > Lighting says. Hitting a red button opens the trapdoor
across the room for 1.8 s; fly out through it (WASD or arrows thrust). Guards come
in through the trapdoors and hit with batons (a hit costs a heart but never kills).
Fly out and an escape rocket picks you up (the same voxel block rocket as the LAUNCHER:
your head in the glass cockpit block, saved villagers in the portholes) and dives to Earth with a shader engine plume, smoke
and a re-entry shock; you land home with every villager who got out: 100 XP, up to
150 more for a quick escape, 40 per villager and a heart. There is no time limit by
default: it ends when you escape, or when you give up with Esc (no reward).
Settings > Game modes can set a limit (60 s to 3 min). Every 30 s the alarm rises
(up to level 4): red lights, a faster spin, more guards and blasts, and at level 2
the guard captain (red cap; his baton stuns, ram him hard to knock his key card
loose). Now and then all four magnets overload, pinning everyone to the walls
before flinging them across the cell. Power-ups float in: thruster fuel (double
thrust 5 s), a shield (blocks one hit) and a key card (your next button holds its
trapdoor open 5 s). Villagers go for the buttons and dash for any open trapdoor. The world is paused and
silent while you are inside. Its cooldown is under Settings > Cooldowns.

### Combo streaks
Break blocks within 1.5 s of each other to build a combo. The counter climbs with the
break notes; milestones at x10 (ON FIRE), x25 (UNSTOPPABLE) and x50 (FRENZY) add a
shake and a glowing trail. XP from blocks is multiplied (x1.5 from 5, x2 from 10, x3 from
25, capped) and long combos drop extra ores. Viewer heads keep their own combos; big
ones are announced. A split twin's breaks count for its Splitter head.

### Power cooldowns
Settings > Cooldowns lists every power. Click one to step its cooldown (off, 5 s, 10 s,
15 s, 30 s, 1 m, 2 m, 5 m, 10 m); every power starts off, so it can be used again right
away. While a power recharges, a repeat shows "<Power> recharging: Ns" and is dropped.
The choice is saved in `power-cooldowns.json` in the settings folder.

### Play styles
Each viewer chooses how their heads play the ball. The choice is remembered,
so heads they spawn later use it too. The streamer cycles the hero's style with **C**, or picks
the hero's face and style under **Settings > Character** (saved for next time).

| Style | Behaviour |
|-------|-----------|
| `bouncy` | Default: full bounces off every surface |
| `heavy` | Falls fast, barely bounces, hard landings do double block damage (digs down) |
| `floaty` | Low gravity, steady soft hops, long hang time |
| `sticky` | Grabs walls, hangs on briefly, then wall-jumps up and away |
| `splitter` | Bounces; a hard landing splits it into two small heads that merge back after 4 s |

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
- `WIND` ✅ - Wind pushes every head sideways for 7-14 seconds (breeze, strong or storm)
- `BLACKHOLE` ✅ - Vortex pulls things in for 10 seconds, then pops them outward
- `ICEAGE` ✅ - Snow and slippery floors for 15 seconds
- `BEES` ✅ - Swarm chases a random head; a big bounce shakes it off
- `PRISON` ✅ - Locks the hero in a spinning zero-gravity space prison (see Space prison)
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
├── assets/
│   ├── packs/free/     # Free theme pack: textures, sounds, models (public)
│   ├── packs/<name>/   # Other theme packs (installed locally, not in git)
│   ├── shaders/        # Shared by every pack
│   └── configs/        # Shared by every pack
├── tests/              # Tests
└── docs/               # Documentation
    ├── GAME_DESIGN.md
    ├── ARCHITECTURE.md
    └── IMPLEMENTATION_PLAN.md
```

## 🎨 Theme packs

Textures, sounds and models come from a **theme pack** in `assets/packs/<name>/`, laid out like
`assets/` itself (`textures/`, `audio/`, `models/`, `icon/`) with a `pack.json`:

```json
{ "name": "Winter", "author": "you", "license": "CC0", "extends": "free", "credits": [] }
```

The game looks for each file in the active pack, then the packs it `extends`, then the free
pack, so a pack only needs the files it changes. Pick the pack under **Settings > Theme**: it switches while you play
(textures, sounds and models reload) and is remembered, or set `FALLING_HEADS_PACK=<name>` for one run.
New pack folders are found when the game starts.

This repository ships only the **free pack**. Most of its art is from Kenney's CC0 packs
(`tools/Apply-Kenney.ps1` with the table `tools/kenney-map.csv`; unzip the packs into
`target/kenney/<pack>/files/` first). The rest are stand-ins from `tools/Make-FreePack.ps1`, which
never overwrites files already in the pack, so replace them with your own or CC0 art one at a time.

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

MIT OR Apache-2.0 for the code. The free pack lists its own credits and licences in
`assets/packs/free/pack.json`. Falling Heads is a fan project and is not affiliated with Mojang or
Microsoft; no Minecraft assets are included.

## 🤝 Contributing

This is a game development project. Contributions welcome!
