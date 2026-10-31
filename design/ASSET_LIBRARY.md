# Local Minecraft asset library

## Confirmed sources

The supplied `C:/Users/alexispace/Downloads/Faithful 64x - Release 15` contains 3,903 PNG textures: 1,295 block, 833 item, 739 entity, 297 particle, 542 GUI, and smaller environment/font/map/effect/painting/trim groups. “64×” describes the pack's block resolution; entity atlases and animation strips have other dimensions. 2,226 files are exactly 64×64.

The earlier bundled set matches Faithful 64x Release 14 pixel-for-pixel in 71 cases. Release 15 is now the source for the 12 tree/grass textures and four structure textures that were previously vanilla additions. Source paths, SHA-256 hashes and the original license are retained with those assets. Original replacements are backed up under `target/pre-faithful15/`. The rest of the game's existing artwork is not silently replaced.

Full searchable inventory: `faithful64-release15-assets.csv`. Summary and copied-file hashes: `faithful64-release15-summary.json`.

The extracted Minecraft audio was found at `F:/Users/alexispace/Downloads/mc_sounds/minecraft/sounds`: 4,013 audio files. It includes ambient, blocks, entities/mobs, fire, fireworks, liquid, minecart, portal, damage, UI and music. Full paths and sizes are in `minecraft-audio-inventory.json`. This inventory does not add all those sounds to the game or establish a Minecraft version; the extraction metadata was not found here.

## Useful next additions from actual available artwork

| Area | Available assets | Gameplay/design use |
|---|---|---|
| Desert aqueduct | chiseled sandstone, cut sandstone, sandstone, decorated pots | Distinguish piers, arch crowns, sanctuary walls and loot alcoves instead of repeating one block face |
| Underground | rails, cracked deepslate, amethyst blocks/clusters, lantern | Branching mine galleries, abandoned crossings, occasional crystal chambers; different landmarks along routes |
| Forest | hanging roots, mossy masonry, bee nests, beehives, mushrooms | Roots around clearings, overgrown ruins, small bee groves and recognisable foraging sites |
| Plains | crop growth stages, planks, bookshelves, stone bricks | Farms and inhabited ruins with paths and yards that join natural ground |
| Nether | cracked nether bricks, polished blackstone variants, lanterns | Forges and broken bastions with a different silhouette and material language |
| Detail/rewards | copper grates, item sprites, particle sheets | Grated vaults, readable loot icons and richer small effects without generating replacement artwork |

These are proposals, not implemented mechanics. New functional blocks still need a material family, health/collision/fire/fluid rules, loot, AI interactions and placement constraints. Item textures alone do not implement crafting or equipment.

The world workbench searches all 3,903 textures. Pin a terrain tile, then select a pack asset to audition that texture across the preview. This does not write game files. Once a design is chosen, its source texture can be copied into the game with provenance and a corresponding material assignment.

## Biome expansion implementation — 27 October 2026

The expansion imports 62 source textures from the supplied Release 15 pack, including ten enemy faces. `tools/Import-BiomeAssets.py` reproduces UV crops, the slime's separate eyes/mouth, tipped-arrow composites, magma frames, and 4×1 directional block atlases. Block faces are 64×64; directional atlases are 256×64. Face crops retain the pack's original texel density (most heads are 32×32; the silverfish and shulker crops are smaller). Nearest-neighbor sampling preserves their pixels. All converted sprites keep alpha; flat terrain plants render 24 units in front of their owner plane to avoid the background at Z=-10.

Texture originals and hashes: `assets/textures/biome_content/source.json`. Audio originals and hashes: `assets/audio/biome_content/source.json`. The existing effects mixer controls the imported blaze, shulker, creeper, slime and magma-cube sounds.

| Biome | Implemented additions |
|---|---|
| Plains | Grounded wheat/hay farms, tilled soil, wild grass, mossy masonry; slimes and occasional creepers |
| Forest | Moss/rooted dirt patches, ferns, berries, flowering azalea, mossy ruins; bogged in damp patches |
| Desert | Red sand, terracotta strata and fossil ribs; husks, including guards in aqueduct chambers |
| Mountains | Tuff and calcite formations; breezes with wind charges |
| Underground | Deepslate, dripstone, infested cracked stone, hollow calcite/amethyst geodes and crystal clusters; silverfish emerge when infested blocks break |
| Nether | Basalt, blackstone, animated magma, crimson/warped soil and fungi; blazes and magma cubes |
| Iceland | Packed/blue ice and sparse powder snow; strays with slowing arrows |
| End | End-stone brick platforms and purpur pillars in ruins; attached shulkers and turning bolts |

Blazes use three fireballs per burst and the existing terrain heat reactions. Creepers telegraph their fuse, then damage nearby heads/blocks through the normal events. Slimes/magma cubes split once into two small heads. Shulker bolts turn at a capped angular rate and apply brief lift. Wind charges push heads and nearby moving projectiles. New projectiles use swept collision and reflect from mirror barriers. All new enemies use normal health, damage, death effects and XP.

Berries heal on contact or drop when broken. Blaze rods and amethyst pickups automatically release a friendly projectile fan toward an enemy; slime pickups give a short speed boost; shulker shells heal. This is automatic pickup behavior, not an equipment/crafting inventory.

The exact generator export and the local preview workbench include these material variants. `tools/Export-BiomeContentPreview.py` exports eight textured, windowless previews to `design/biome-expansion`. Those previews deliberately omit lighting, dynamic fluids and actors; native screenshots are captured separately by `Test-FallingHeadsOverhaul.ps1 -Scene content`.
