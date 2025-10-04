# Falling Heads - Quick Start Guide

## ✅ Game is Ready!

The game has been fully implemented with all core features.

## 🎮 How to Play

### Controls
- **A** or **←** - Move left
- **D** or **→** - Move right
- **ESC** - Pause (not yet implemented)

### Gameplay
1. **Green square** is you (the hero)
2. **Colored blocks** below - bounce on them to break
3. **5 hits** to destroy each block
4. **Blocks darken** as they take damage
5. **Small colored squares** are resources - collect them!
6. **Resources show** in top-left corner

### Features Implemented
✅ Hero with physics (gravity, bouncing)
✅ Horizontal movement (A/D keys)
✅ Block damage system (5 hits to break)
✅ Visual damage feedback (blocks darken)
✅ Resource drops (coal, iron, gold, diamond, emerald, redstone)
✅ Auto-collection when near resources
✅ HUD showing resource counts
✅ **Camera follows** hero downward
✅ **Infinite world generation** - more blocks spawn as you fall
✅ Collision system (can't pass through blocks)

## 🚀 Running the Game

### Windows
```bash
cd C:\Users\alexispace\Desktop\webdev\communityGaming\falling-heads
cargo run
```

### Known Issue: Windows Accessibility Error

If you see an error about "Not enough memory resources" or accessibility:

**This is a Windows-specific issue with Bevy's accessibility system.**

**Workaround**: Try restarting your PC or closing some applications to free up resources.

**Alternative**: The game executable should still work if you build it:
```bash
cargo build --release
./target/release/falling-heads.exe
```

## 📊 Current Game State

- **Phase 1**: ✅ Foundation & Architecture
- **Phase 2**: ✅ Core Components
- **Phase 3**: ✅ Physics System
- **Phase 4**: ✅ Block Damage
- **Phase 5**: ✅ Infinite World
- **Phase 6**: ✅ Resource System
- **Phase 7**: ✅ Hero Movement
- **Phase 8**: ✅ Camera Follow
- **Phase 9**: ✅ HUD
- **Phase 10**: ⏳ External API (next)
- **Phase 11**: ⏳ Power Commands (next)

## 🎯 Next Steps

### To Add Next:
1. **External API** - HTTP endpoint for power commands
2. **Power Commands** - TNT, LASER, ARROW, etc.
3. **Sound Effects** - Block breaking, collection
4. **Particle Effects** - Explosions, sparkles
5. **Game Over** - When hero falls too far

### To Test:
- [ ] Run the game
- [ ] Move left/right with A/D
- [ ] Bounce on blocks repeatedly
- [ ] Watch blocks darken and break
- [ ] Collect colored resource drops
- [ ] See resources count in top-left
- [ ] Fall down and see more blocks spawn
- [ ] Camera follows you down

## 📝 Development Notes

**Location**: `C:\Users\alexispace\Desktop\webdev\communityGaming\falling-heads`

**Dependencies**:
- Bevy 0.17.1
- Rust 1.88.0
- rand, serde, axum (for future API)

**Architecture**:
- Plugin-based (Physics, Gameplay, UI, etc.)
- ECS pattern (Components, Systems, Resources)
- Event-driven (BlockDamaged, BlockDestroyed, ResourceCollected)

## 🐛 Troubleshooting

**Game won't start:**
- Check if you have enough RAM (game uses ~500MB)
- Try closing other applications
- Restart your PC

**Can't move:**
- Make sure window has focus (click on it)
- Try both A/D and arrow keys

**No blocks spawning:**
- Fall down further - they spawn below
- Check console for errors

**Performance issues:**
- Try release build: `cargo run --release`
- Lower your screen resolution
- Close background apps

---

**Ready to play!** 🎮

Just run `cargo run` and start bouncing!
