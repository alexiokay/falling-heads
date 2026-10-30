# MineHeads world workbench

Run `tools/Start-WorldPreview.ps1 -Open`, then open `http://127.0.0.1:48119`.

The workbench uses the local `target/debug/falling-heads.exe` exporter. Build once after changing Rust (`cargo build --offline`); no graphical game, Vulkan renderer or audio device is started for preview generation. The server binds only to localhost and never modifies game assets. Save PNG writes a generated image under `design/world-previews/`. Its PID and logs are under `target/world-preview-server.*`.

Choose a biome and seed, then Generate region. Leave Y blank to select the middle of that biome, or enter an exact block coordinate. X and Y are block coordinates, not 64-unit world coordinates. Compare 8 biomes produces side-by-side maps using the same seed. Click a comparison map to inspect it. Change zoom or toggle textures/grid/landmark bounds. Hover a tile for its coordinates and source texture; click to pin. Filter the local Faithful pack and select a texture to audition it on that block/species in the preview. Reset removes auditions. These substitutions never modify game files.

The terrain, seeded ore, supported decorations, tree plans, structure layout, finishes and placement call shared production Rust functions. Structure walls are shown dimmed. Gateway openings are schematic. Lighting, actor AI, fluid movement and runtime VFX are intentionally outside this layout tool. It is not a replacement for native rendering/gameplay verification.

Direct JSON export:

```powershell
./target/debug/falling-heads.exe --export-world-preview --biome Underground --seed 921 --x 96 --width 160 --height 112 --output target/underground.json
```

Same seed and region produce the same static world. Normal gameplay still chooses a random seed at startup. The preview process starts and exits without creating the Bevy app. Results are cached by executable modification time and generation arguments; rebuilding automatically invalidates the server's geometry cache. Texture changes are visible after a browser reload.

## Design changes

- Plains: warped, irregular land masses and varied cave openings, grass on exposed earth, occasional oak trees.
- Forest: broad clearings with rolling floor heights, staggered connections and three tree species; crowns cross chunk boundaries without clipping.
- Underground: jittered chambers joined by bent horizontal and descending tunnels, loops and side routes. No repeating 16-tile floor bands.
- Desert: broad eroded cavities with occasional aqueduct/sanctuary layouts.
- Mountains/Iceland: different slanted fracture fields.
- Nether: volcanic chambers and thin fissures.
- End: more open void around scattered land masses.
- Landmarks: candidates must have nearby natural ground under at least 90% of their footprint; continuous footings and short fill foundations connect the structure to it. Unsupported candidates are skipped. Each streamed chunk builds only its part, independent of load order.

The aqueduct is a first block-layout adaptation of the selected concept, not a complete puzzle dungeon. Long-session balance, permanent edited-world saves and the other two concept structures remain separate work.
