# Slime rift

The first minigame is a compact 3D auto-bounce course. It uses a separate `GameState::Rift` and a fixed 120 Hz course simulation. Main-world virtual time and Avian physics pause together; the original hero, terrain, fluids and enemies remain in place. A course-only presentation of the hero moves through the arch as the existing camera turns toward the platforms. The scene uses its own render layer and lights. Both raster rendering and the Solari proxy filter must respect that layer.

## Playing

- Press **F8**, choose **SLIME RIFT** in the dev panel, or send `{"action":"RIFT"}` to the existing `POST http://127.0.0.1:3029/command` endpoint.
- Natural arches can appear near clear ground in Forest and Plains. Enter an arch to start. Successful or cancelled encounters have a 60-second cooldown.
- The hero automatically hops along nine floating platforms. **A/D** adjusts travel across the course; **W/S** adjusts depth. Two platforms move sideways. Purple markers distinguish moving pads.
- Eight crystals come from reaching platforms; four bonus crystals sit away from the automatic flight path and reward steering.
- Slime pads hold the hero briefly before releasing a spring jump. A missed landing returns to the most recently reached checkpoint.
- **R** or `{"action":"REWIND"}` rewinds up to five seconds once per encounter. This reverses course movement, moving pads, checkpoint progress and crystal collection. It is not a whole-world rewind.
- **P** pauses/resumes the course. **Esc** leaves without rewards.
- Reaching the exit grants 50 XP plus 15 per collected crystal and restores one heart, capped at maximum health. Rewards are committed once on exit, never while rewinding.

The course takes roughly 17 seconds on its automatic route, plus transitions; steering, missed jumps and rewind extend it. Platforms have swept top-surface landing checks in X/Y/Z. They are one-way platforms, not a general-purpose rigid-body sandbox.

The exit uses a clear, already loaded opening near the entry when available; otherwise it returns to the entry position. It does not destroy a corridor through the main world. Camera layers, visibility and simulation clocks are restored on both success and cancellation. Ordinary API powers wait in the queue until the main world resumes; REWIND is handled inside the rift.

## Plants and slime contacts

Wheat, short grass, ferns, berries and fungi are pass-through scenery, as are legacy mushrooms and dead bushes. They retain destruction/fire/loot identity without stopping heads, arrows or fireballs. Crystals and dripstone retain their separate interaction rules. Floor plants require support below; hanging dripstone requires support above. Losing support uses the normal destruction event path, and an unloaded neighbouring chunk is not treated as destroyed ground.

Slimes and heads that collect a slimeball use a 0.22-second supported adhesion interval followed by a bounded spring release. Slimeball coating lasts eight simulation seconds; it replaces the former generic speed boost. Destroyed support or moving away cancels adhesion. This is a contact response, not a change to ordinary crowd collider shapes. Snow and ice keep their existing response.

## Assets and verification

The slime surface uses the supplied Faithful 64x Release 15 `slime_block.png`, copied without conversion; provenance is in `assets/textures/biome_content/rift-source.json`. Other course textures and the jump sound reuse the existing imported assets. The sound uses the central effects-volume path.

The automated native probe is `tools/Test-FallingHeadsOverhaul.ps1 -SkipBuild -Scene rift`; `-RayTracing` requests the RT path. It checks the course, rewind, pause, frozen main-world body position, queued power isolation, successful return, cancellation and scene cleanup. Unit tests cover the route, high-speed/depth-sensitive landings, replayed state, support loss, and the real Avian slime hold/rebound. See MODLOG for accepted build/test results rather than assuming the presence of this document confirms validation.
