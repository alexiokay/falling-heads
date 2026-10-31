# Launcher power

`LAUNCHER` (alias `ROCKET`) carries the hero in an original modeled rocket. Its accelerating roll takes 0.75 seconds, followed by a 2.6-second flight in front of the terrain. **Flight does not damage or mine blocks.** Travel is 47–52 blocks horizontally, with the entire arc inside 21 blocks vertically. Startup tilts the nose toward the camera and includes a small lateral movement; flight follows a damped corkscrew around the main route, with launch-dependent phase/turn count, continuous roll and banking that follows the curved trajectory. On impact only, a roughly 5×5 irregular crash pocket has a guaranteed empty 3×3 center. The rocket breaks into individual metal panels and smaller shards, surrounded by staggered TNT blast sprites and their explosion sounds. The hero lands immediately and requested villagers emerge at separate cells, staggered by 0.14 seconds. Terrain destruction produces normal block resource events.

POST to `http://127.0.0.1:3029/command`:

```json
{"action":"LAUNCHER","username":"Alex","direction":"right","villagers":3}
```

`direction`: `auto` (default), `left`, or `right`. `villagers`: integer 0–8, default 1. Invalid values are rejected. The launch cooldown is 25 seconds. A second request during flight/recharge shows a notice rather than stacking launches. A queued API response acknowledges receipt, not completed travel. The destination is streamed before flight; a failed load cancels safely. Teleport interrupts rocket transport and restores the hero's physics.

The ship geometry and colors are original. No Ad Astra art is bundled: its non-code assets have a separate restrictive license.

Original ignition, engine and crash sounds are bundled in `assets/audio/rocket`. They respect Effects Volume. `tools/make-rocket-audio.py` reproduces them through deterministic synthesis and ffmpeg encoding, without external samples. The default is exactly one villager; eight is only an explicitly requested API option used in the stress test.

# DLSS controls

In Settings → Graphics, enable DLSS and use the **Quality — Balanced — Performance slider**. Click or drag, then release to apply; left/right arrows also work while hovering. The three stops are the supported NVIDIA input-resolution modes, not an arbitrary continuous percentage. The same slider controls resolution with ray tracing enabled. Quality is the default. Raster DLSS automatically uses the 4.5 model M; the old separate model/resolution buttons have been removed. Saved resolution preferences are retained.

The renderer sets the official NGX preset hints at feature creation; releasing the slider rebuilds the context once. NVIDIA App driver overrides may override application hints. Ray tracing uses the Ray Reconstruction model, not the SR model selector. DLSS 5 neural rendering is not included. General contains the separate Effects Volume and Music Volume sliders.

# Dragon breath

Each breath cone ray penetrates three block layers and stops at the fourth. Coincident tile-edge hits count as one layer. Contact damage reaches the penetrated blocks; lingering ignition remains a separate effect.

Breath, fireballs and Fire Rain feed the shared terrain heat handler. Block material families decide combustibility or frozen behavior; fluids consume finite water and emit steam. Overlapping heat samples react once per cell per update. Lingering fire lasts five seconds, spreads at most four neighbor steps within four blocks of its original ignition, and has a global cap of 24 active burning blocks. Rain extinguishes it; wet cells cannot ignite in the same heat reaction. A new direct breath can start a separate ignition. Snow surface caps melt into finite runoff; solid frozen terrain takes heat damage.

Rain accumulation is limited to two new one-eighth-cell puddle portions every 24 seconds at full intensity, across the local area, with no reservoir/refilling existing water cells. Existing finite springs and melting snow can still produce water.

# Verification

Run `cargo test --offline --bin falling-heads`, then `tools/Test-FallingHeadsOverhaul.ps1` and `tools/Test-FallingHeadsOverhaul.ps1 -SkipBuild -Scene launcher`. The hidden probes exercise landmarks, RT cracks/destruction, the actual slider handler, both launch directions, default/eight villagers, foreground flight through intact terrain, crater clearance, restored hero physics, and runtime SR/RR switches. Screenshots are written under `target/visual-overhaul-*.png` and `target/visual-launcher-*.png`.

`tools/Test-FallingHeadsOverhaul.ps1 -SkipBuild -Scene refinement` exercises natural forest mist, camera movement, the actual Fire Rain button and the spider model/attack. Launcher captures include three delayed breakup frames per direction. Equipment, fruit and crafting remain proposals in `PROGRESSION_PROPOSAL.md`.
