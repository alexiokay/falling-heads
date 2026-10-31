# World generation and visual consistency — 24 October 2026

Published build: **20261024-135904-f10ddcaa1d**. SHA-256: `F10DDCAA1D42B2D895F0044791B5AD1A928FDC54F2E6E21E999DAF3245EE04F3`. Publisher verified 343 assets and nine runtime DLLs; desktop launcher resolution/integrity check passed. The next Falling Heads - Play launch uses this version.

## World workbench

`tools/Start-WorldPreview.ps1 -Open` starts the local workbench at http://127.0.0.1:48119. It generates layouts with the production Rust code before initializing Bevy, a graphics device or audio. It offers eight-biome comparisons, seeds and coordinates, texture/geometry views, zoom, a grid, landmark outlines, tile inspection and PNG export. The supplied Faithful pack is searchable; texture auditions affect the preview only. Exports are saved in `design/world-previews/`.

The generator was exercised across eight biomes and three seeds. Eight extra repeats verified deterministic static geometry. Results, timings and 24 reusable JSON maps are in `design/world-previews/verification.json`. A 160×112 region takes roughly tens of milliseconds to generate; process startup adds overhead. The tool does not simulate lighting, enemies, dynamic water or VFX. Gateways and procedural plants are schematic front-view representations, not native rendered screenshots.

## Implemented changes

- Plains uses warped, irregular terrain with varied openings, exposed grass and occasional oaks. It no longer uses the repeating horizontal shelf generator.
- Underground uses offset chambers and bent connecting tunnels, including side routes and loops. The fixed repeated layer pattern is removed.
- Forest clearings have rolling floors and staggered connections. Trees can cross an upper chunk boundary. Both chunks compute the same tree and emit only their portion; deterministic conflict resolution prevents overlapping crowns and duplicate blocks.
- Mountain, ice, Nether, End and Desert geometry have distinct noise/fracture parameters. Static block selection is now seeded per cell, so the preview and game agree on ores and terrain materials.
- Landmarks search for natural ground under at least 90% of their footprint. A continuous footing joins their floors to the short foundation fill. Unsupported candidate locations are skipped; regional frequency is reduced. Multi-chunk structures partition their blocks by the actual cell's chunk.
- The selected desert aqueduct/sanctuary is a first 29-tile block adaptation: three supported arches, a crossing, raised access steps, buried rooms, a reward alcove, cut sandstone bands and carved pier details. It is not yet the full illustrated puzzle dungeon.
- Rocket launch roll is reduced to one eighth of a turn. Heading follows a continuous tangent with damping and a maximum turn speed, with smaller, broader travel weaving. Horizontal travel range and passenger rules are retained.
- Witch gas uses the forest mist's integer-hash, layered-noise density field, wider soft falloff and opacity dissolution instead of rigid plane rocking.
- Damage borders, fire, freezing and death debris derive dimensions from the actual head mesh and transform. Grow restores the original authored scale. Hearts use the camera's screen axes near the head face instead of a large world-Z offset.
- Visual fixtures now run after gameplay commands apply, avoiding stale commands targeting actors that a fixture just removed.

## Assets

Faithful 64x Release 15 contains 3,903 PNGs, including 1,295 block textures. Sixteen former vanilla tree/structure textures were replaced with supplied originals; two sandstone details were added. Provenance hashes and the unmodified license are included. Full inventory and possible future uses are in `design/ASSET_LIBRARY.md`.

The extracted sound collection was found at `F:/Users/alexispace/Downloads/mc_sounds/minecraft/sounds`: 4,013 audio files, inventoried in `design/minecraft-audio-inventory.json`. These were located and indexed, not all imported into the game.

The three concept illustrations are retained in `design/structure-concepts/`. The forest apothecary and underground liftworks illustrations remain design proposals.

## Verification

All 143 unit checks passed (`target/world-preview-tests.log`). Native checks passed: style 8 captures, launcher 15, ecology 3, head-size 3 and the final structure/RT gallery 14. They cover witch gas and sprite damage, rocket travel and breakup, the three natural biomes, all eight structure styles, raster/RT crack visibility, graphics controls and differently sized heads. Logs and captures are under `target/{style,launcher,ecology,overhaul,headsize}-run-*` and `target/visual-*`.

The final 24-region export measured median 26.90 ms generation, maximum 132.08 ms, and median 76.72 ms including process startup. Eight direct repeats matched the static data exactly. The browser's texture audition, reset, all-biome comparison and PNG save were exercised; final console check contained no errors. Saved comparison: `design/world-previews/preview-eight-biomes-seed921.png`.

The aqueduct support refinement has a separate connectivity regression and is checked in the final structure gallery. Existing mass/inertia, Streamline hook and deferred-command warnings are not represented as resolved. These bounded checks do not establish that every intermittent RT artifact or long-session gameplay issue is gone.

## Remaining design work

- Full aqueduct water puzzles, dedicated sanctuary rewards and more authored structure families.
- Permanent world edits across unload/reload; multi-chunk generation is implemented, persistence is not.
- Dragon background breach and special egg/progression drop.
- Apple drops, crafting, equipment and inventory progression.
- DLSS 5 integration and further long-session RT/crowd stress testing.

See `ENCOUNTER_FIX_CHECKLIST.md` for the consolidated historical request audit. Desktop publication selects a version for the next launch; existing user play sessions are left running.
