# Release Notes

## New Features
- **Natural Cave Generation System**: Complete rewrite using advanced procedural generation
  - **3D Perlin/Simplex Noise**: Organic, geological cave structures instead of mathematical patterns
  - **Multi-Layered Approach**: Combines multiple scales for complexity
    - Layer 1: Large caverns (vast underground halls using Perlin noise)
    - Layer 2: Medium tunnels (worm-based carving creates winding passages)
    - Layer 3: Small crevices (high-frequency detail for fractures and cracks)
    - Layer 4: Biome-specific features (vertical lava tubes, ice fractures, tomb chambers)
  - **Variable Cave Sizing**: Dynamic width variation (1-8 blocks wide)
    - Narrow squeezes in tight passages
    - Vast chambers in large caverns
    - Width determined by noise for natural feel
  - **Cave Decorations**: Natural features in empty cave spaces
    - Stalactites hanging from ceilings
    - Stalagmites growing from floors
    - Stone columns where formations meet
    - Rich ore veins exposed in cave walls (rarer ores in caves)
    - Glowing features (glowstone, mushrooms, redstone)
    - Cobwebs in forest/underground caves
    - Biome-specific decorations (icicles, lava drips, end crystals)
  - **Worm-Based Carving**: Simulates natural erosion paths
    - 3 independent "worms" carve through terrain
    - Creates interconnected tunnel networks
    - Variable paths based on Simplex noise
  - **Biome-Specific Cave Systems**:
    - Nether: Vertical lava tubes (3-8 blocks wide)
    - Underground: Classic winding dungeons (2-6 blocks)
    - Mountains: Rocky caverns and mine shafts (2-5 blocks)
    - Forest: Organic root caves (1.5-4 blocks)
    - Iceland: Jagged ice cracks (1-3 blocks)
    - Desert: Rectangular tomb chambers (2-5 blocks)
    - End: Diagonal void corridors (2.5-6 blocks)
    - Plains: Shallow burrows (1.5-3.5 blocks)

## Technical Improvements
- Added `noise` crate (v0.9) for Perlin and Simplex noise generation
- Replaced simple sine/cosine math with proper 3D noise algorithms
- Cave decorations check adjacent blocks to determine ceiling/floor placement
- Fragile decoration blocks (2 health instead of 3)
