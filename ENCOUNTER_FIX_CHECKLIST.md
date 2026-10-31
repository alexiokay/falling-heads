## Architecture cleanup — 27 October 2026

Current installed build: **20261027-032456-b6e29650ed** (Falling Heads – Play).

- [x] Remove the sphere-contact experiment and restore original rounded-box crowd contacts; preserve appearance over a marginal frame-time difference.

- [x] Split active physics into terrain, geometric resolution, steering, combat and TNT modules; remove unused legacy physics and unused global schedule declarations.
- [x] Move actor depth into a dedicated policy module with named layers and retained sort storage.
- [x] Separate fluid occupancy, simulation and rendering; explicit visual revision prevents bookkeeping-only mesh rebuilds.
- [x] Reuse terrain scratch buffers and chunk-biome lookup storage; preserve swept terrain collision.
- [x] Share an exact target index across core enemy steering and ranged attacks.
- [x] Batch hearts into shared mesh rows; verify damage/healing and mass owner cleanup.
- [x] Separate launcher/dragon visuals and probes and biome structure layouts from runtime gameplay.
- [x] Remove unused command queue and command-behavior models; retain the real API/dev-panel queue.
- [x] Add reproducible local formatting/lint/test checks and a manually dispatched prepared-runner workflow; no claim that remote CI has run.
- [x] Add Rust/C++ ABI layout assertions and document native ownership/synchronization contracts.
- [x] 175 tests, formatting/lint gate, named 370- and 2,000-head native tests, water/snow/ghast/rocket and graphics-transition checks pass; verified publication.
- [ ] Pre-existing Avian mass/inertia warnings in rocket stress scenes remain to be diagnosed.
- [ ] Full physics-root/render-child depth migration remains a separate coordinated change; current depth/collision behavior is intentionally preserved.
- [ ] Existing compiler/Clippy warnings remain recorded in the baseline; further cleanup must distinguish live features from obsolete code.
- [ ] Long-session/full-world lag and native driver/device-loss diagnostics remain outside what this controlled benchmark proves.

Historical releases and request statuses follow below. See ARCHITECTURE_IMPROVEMENTS.md for current implementation details.

## Latest snow/ghast correction — 27 October 2026

Current build: **20261027-022021-28f7a87f92** (Falling Heads – Play).

- [x] Short supported snow slide before the normal rebound; shared by snow blocks and settled snow layers.
- [x] Cancel on lost support/teleports; avoid repeated support damage or timer refresh.
- [x] Retain both ghast face textures and change material only at animation transitions.
- [x] 170 tests; 27 native captures, including 24 sampled ghast transition frames with raster/RT; clean shutdown and verified publication.
- [ ] User play-feel feedback on the 140–200 ms snow slide and longer-session confirmation of the intermittent ghast flicker.

# Request audit — 27 October 2026

## Current water geometry / crowd / architecture review

Installed build: **20261027-014643-5a4309077a**. Available on the next Falling Heads - Play launch.

- [x] Remove separated world-wide water overlay; retain dense water in a single connected volume.
- [x] Prevent head-occupancy-only changes from rebuilding liquid meshes every rendered frame.
- [x] Fix dying-head collision filtering; use measured cheaper packed-head contacts without skipping terrain sweeps.
- [x] Explain actual depth: block center -10 / front 22; ordinary heads 30..38; water front 24.
- [x] Review architecture and record current findings, fixed items and prioritized refactors in ARCHITECTURE_IMPROVEMENTS.md.
- [x] 165 tests; five water captures and two dense 370-villager comparison runs; clean shutdowns and verified publication.
- [ ] Reproduce the user's severe 370-villager lag with their full world activity; the controlled raster workload ran about 180–200 FPS. Contact costs improved, average FPS did not consistently improve.
- [ ] Optional future submerged-face tint clipped to face geometry; the detached pool-sized sheet is removed.

## Previous water density and snow handling

- [x] Follow-up: stronger low-speed snow carry, faded out by 100 px/s; stationary heads and normal bounce preserved. 164 tests passed and build published on 27 October.

Previous build: **20261027-011947-4b7198a720**.

- [x] Restore dense blue water: 69% body behind faces plus a separate 10% foreground tint (about 72% combined over the background).
- [x] Same noticeable, capped lateral carry and normal bounce for solid snow blocks and accumulated snow layers.
- [x] Brief weaker counter-steering after snow contact (0.65 seconds); stronger ice glide (one second) with low bounce.
- [x] Expiry on the gameplay clock and immediate reset on normal ground; no new buff-slot conflicts.
- [x] 162 tests; five native water captures across raster/DLSS/RT; eight snowfall/freezing captures; verified desktop publication.

## Previous water visibility and crash diagnostics

Diagnostics release: **20261026-221101-49a38527d7**.

- [x] Light, consistent water tint over heads; water opacity reduced from 72% to 24%; bounded actor depth offsets.
- [x] Per-session local runtime/native-output logs, panic backtraces, recent state and launcher exit reports.
- [x] Periodic launcher memory samples; matching debug symbols retained beside published executables.
- [x] 159 unit tests, five native water captures across raster/DLSS/RT, three head-effect captures, clean/panic logger tests and actual-game launcher monitoring.
- [ ] Determine the cause of the user's intermittent crashes from a failing session report. Historical event code alone is inconclusive; machine memory pressure was observed.

## Previous surface and power corrections

Surface/power release: **20261026-204301-3fc013fae7**.

- [x] Potions splash at the first block/head contact, including fast travel; misses continue falling instead of timed airbursts.
- [x] Substance-specific bottle/cloud colours; eight-second clouds with a two-second fade.
- [x] Thirty-second Grow power with independent status storage, two-stack refresh and authored-scale restoration.
- [x] Exposed-top grass only in structure foundations; matching headless world preview.
- [x] No underwater snow caps; delayed freezing of exposed settled water; deeper water remains sheltered.
- [x] Normal bounce/slight glide on snow; low bounce/stronger glide on bare ice.
- [x] 156 unit tests, 30 fresh native captures, 24 generated regions/eight repeats, verified publication.

## Previous collision correction

Collision release: **20261026-201704-43740424af**.

- [x] Sweep fast head movement against terrain on every physics tick, using authoritative positions.
- [x] Resolve adjacent/corner contacts together; prevent wrong-side separation and damage through blocking walls.
- [x] Preserve teleport/rocket exclusions and snow/ice contact behavior.
- [x] 150 passing unit checks; native 100-villager and rocket checks; versioned publication and launcher verification.

## Previous world-generation overhaul

World-generation release: **20261024-135904-f10ddcaa1d**; 343 assets and nine runtime DLLs were checked during publication. Existing game sessions were not restarted.

- [x] Fast 2D preview using the production generator, without a graphics window or audio initialization.
- [x] Eight-biome comparison, seed/region controls, tile inspection, texture auditions, PNG exports and 24 saved seeded layouts.
- [x] Irregular Plains terrain with exposed grass and occasional oaks; branching Underground chambers and tunnels instead of repeated horizontal bands.
- [x] Varied Forest clearings and ground levels; more valid tree locations across chunk boundaries, without duplicate blocks or overlapping crowns.
- [x] Ground-fit structure placement, connected footings, reduced landmark frequency and proper multi-chunk partitioning.
- [x] First playable adaptation of the chosen desert aqueduct/sanctuary concept, with supported arches, buried rooms and sandstone detail.
- [x] Reduced rocket launch swirl, gentler travel weaving and bounded, smoothed heading changes.
- [x] Witch gas uses the forest mist's soft, irregular density field.
- [x] Head-sized damage/fire/freeze/death effects; authored scales preserved; hearts aligned in screen space.
- [x] Identified the supplied Faithful Release 15 assets and the extracted Minecraft audio on F:. Full inventories saved; 18 selected pack textures installed with provenance.
- [x] 143 unit checks; native rocket, cloud, natural terrain, head-size and structure/RT checks. No claim that every historical issue was retested.
- [x] Final aqueduct connectivity render check and versioned desktop publication: 14 fresh structure/RT captures, clean shutdown and launcher integrity verification.

Details: `WORLDGEN_OVERHAUL_2026-10-24.md`, `tools/world-preview/README.md`, `design/ASSET_LIBRARY.md`.

Previous installed build: **20261024-035006-4bb3b7c7b7**. Its 136 unit tests and structure/RT/settings gallery passed, alongside enemy, fog/audio/snow/rocket, natural terrain and 100-visible-villager checks. Older completed items are supported by earlier MODLOG entries.

## Earlier requested fixes

- [x] Torches require a rigid, full block face; mushrooms, bushes, vines, leaves, cacti, webs and chorus plants cannot support them. Existing torches are removed when their support becomes invalid.
- [x] Spider uses the existing CaveSpiderFace texture with three horizontally extending legs on each side.
- [x] Spider silk and terrain webs share slowing behaviour; rendering depth and an existing poison/fire buff cannot disable it. Silk expires and spiders are immune to their own web slow.
- [x] Bat/spider spawn paths have correct textures from creation; teleport snapshots occur after their visuals are initialized.
- [x] Sentinel/Enderman uses the existing purple-eyed EndermanFace asset, copied losslessly from WebP to PNG for the enabled image loader.
- [x] Plains terrain works throughout the biome, including far from the origin; the former shelf layout is superseded by the overhaul above.
- [x] Underground's former layered chambers are superseded by branching routes and offset caverns.
- [x] Larger biome-specific structures: timber cottage/well, hollow grove, stepped temple, watchtower, mine galleries, forge, floating observatory and ice shelter. Seeded variants, clear exits and spaced candidate selection; hollow roofs, plank/masonry finishes and recessed walls/windows.
- [x] Forest mist extends below ledges with a broad irregular fade; foreground terrain naturally hides the lower tail.
- [x] Crowd optimization: shared villager and heart assets, private damage materials only while hit, linear heart updates, nearby terrain lookups, and fewer bat line-of-sight checks.
- [x] Research engine practices: GPU instance batching and spatial/visibility/simulation relevance; references and measurements in PERFORMANCE_NOTES.md.
- [x] Final rendered acceptance and versioned desktop publication: 20261024-035006-4bb3b7c7b7; 338 assets and nine runtime DLLs verified. Restart Falling Heads - Play to use it.

## Earlier implemented requests

- [x] Dragon turning/circling animation and continuous breath.
- [x] Dragon breath penetrates three block layers; direct fire wash is separate from lasting block ignition, with dissolving rather than shrinking impact visuals.
- [x] Shared heat reactions: water/steam, snow and ice melting, combustible wood/leaves, rain extinguishing and bounded fire spread.
- [x] Distinct biome lighting/atmosphere, restrained Nether daylight, local forest mist and restored balanced directional light.
- [x] Longer forest bands; oak, birch and spruce with matching wood/leaves; supported plant placement with clearance.
- [x] Rain/snow lighting adjustments, reduced rain water creation and working Fire Rain developer control.
- [x] Launcher command/API; faster start, camera-facing tilt, travel swirl, increased horizontal range, one default villager, optional passenger count, flight above terrain, impact-only crater and detailed breakup/explosion/audio.
- [x] Effects volume covers all effects and existing loops, including while Settings is open.
- [x] Mushroom snow follows the cap rather than floating across an entire tile.
- [x] Witch, Miner, Sentinel, Spider and Bat enemies; common damage response, textured potions, cloud effects and removal of unwanted target indicators.
- [x] Mirror is excluded from opaque shadow casting; Purpur uses its texture.
- [x] Quality/Balanced/Performance slider and DLSS 4.5 M selection.
- [x] Torch light selection follows the camera viewport with a generous margin.
- [x] RT damage/crack visibility fixes and temporal/shadow adjustments; this does not establish that all intermittent artifacts are gone.

## Pending or design-only requests

- [ ] Dragon background breach, flying debris and entrance animation.
- [ ] Special dragon egg/drop, persistent unlock and hatchling/progression use.
- [ ] Leaf apple drops and rare golden apples.
- [ ] Crafting/automatic crafting and equipment/inventory progression.
- [ ] DLSS 5 integration and 4.5/5 switching; no DLSS 5 files or capability are included.
- [ ] Reproduce and eliminate remaining intermittent RT black spots in a long representative run.
- [ ] Extended mixed crowd/combat/weather stress test; the completed 2560×1440, 100-visible-villager benchmark is bounded and cannot guarantee zero lag.
- [ ] Persistent structure changes across unload/reload. Multi-chunk generation is implemented; permanent edits are not.
- [ ] Full aqueduct puzzle/reward design; forest apothecary and underground liftworks concept implementation.

The accidentally posted first-screen background request was withdrawn and is excluded from this audit.

---

# Earlier pass — 23 October 2026

Published as **20261023-162722-4d9d6d7865**. All 75 unit tests and the final hidden native gameplay/renderer checks passed. The desktop **Falling Heads - Play** shortcut resolves this immutable build.

- [x] Portal: reachable interaction and clear prompt; no frame/effect shadows.
- [x] Bloom: remove scene-wide haze, retain restrained bright-source glow.
- [x] Native lighting: torch/RR black spots, clipped light, night rendering.
- [x] Torch: smaller attached animated flame and correct light origin.
- [x] Remove generated redstone rooms, wiring, plates and traps.
- [x] Swimming: dive/rise controls; autonomous dives and escape strokes.
- [x] Snowfall and frost shots create temporary breakable ice without trapping heads.
- [x] Ice/snow sliding.
- [x] Stable overlapping head/death rendering; foreground ghasts.
- [x] Dragon: reactive circling, valid perches, accurate hit regions, health, continuous breath and terrain ignition.
- [x] Last-ledge waterfall origin and water/lava obsidian regression.
- [x] Cactus height variation (0-3), contact damage, verify two-hit sand.
- [x] Occasional biome mist shader, controlled by haze setting.
- [x] Unit/native render/gameplay validation and versioned desktop publication.


## Controls and behaviour

- Portal: touch the opening, or press E within roughly four blocks horizontally / six blocks vertically. Cooldown remains 20 seconds. Decorative frames are non-solid.
- Water: S/Down dives; W/Up/Space rises. Without input, heads alternate dives and stronger escape strokes, with different timing per head. Dry movement is unchanged.
- Ice: active snowfall freezes exposed supported pools of at least six volume units; winter projectiles can freeze shallow supported water on contact. Occupied cells (including an 8-unit clearance) stay liquid. Snow-created ice thaws after about 18 seconds, projectile ice after 12 seconds; fire rain accelerates melting. Ice is breakable. Ice occupies one cell, but thaw restores the exact original water amount; occupied-cell checks apply to the entire new solid.
- Dragon: persistent circling encounter, attack warning, continuous breath, turns toward the player, seeks a clear perch and reacts to damage. Body/head/tail and animated wing geometry receive friendly arrows. Fire damages terrain, ignites vegetation, spreads to at most 24 blocks, and is extinguished by rain/water. The hero remains invincible under the game's existing rules; villagers/enemies take damage.
- Cactus: deterministic absent / 1 / 2 / 3 segment stalks on Desert cave floors. Contact damage is capped to one hit per 0.8 seconds per actor. Sand has two health.
- Redstone mechanism generation and its circuit modules were removed. Redstone ore/resources remain.
- Atmospheric haze also controls occasional narrow mist bands. This does not apply a permanent full-screen fog overlay.

## Validation evidence

- encounter-tests.log: 75 passed, including dive/rise, freeze occupancy, lava/water reaction, cactus contact, slippery ice, reachable portal interaction, duplicate hit-border prevention, articulated wing intersection and real arrow-to-dragon damage routing.
- encounter-final-native-errors.log: final hidden RT gameplay passed moving labels, portal travel, articulated flight, persistence beyond the previous despawn timer, continuous breath and boss damage.
- encounter-gpu-errors.log: final night fixture passed DLSS SR → Solari/RR → native RR/DLAA → native raster → restored SR. GPU diagnostics confirmed actual Solari and Ray Reconstruction execution.
- Captures inspected for attached small flames, torch illumination, retained night background detail, coherent liquid surfaces, foreground ghasts, dragon geometry and no floating lava-light dots. Earlier low-power emitter and missing environment-fill issues were corrected before publication.
- No shader/runtime errors appeared in the final native logs. Existing 129 compiler warnings remain.
- Publisher verified the executable, both NVIDIA runtime DLLs and all 305 assets. The actual Windows PowerShell desktop launcher passed its executable/DLL integrity checks and resolves the installed build.


Executable SHA256: `4D9D6D78651F69BF90EBF4CDEECC353F034BDD52F55B3008A1B58C22528FB540`

## Remaining scope / limits

- Solari remains experimental. Transparent water is forward rendered, not physically ray traced; masked geometry is excluded from secondary rays.
- Mist uses sparse alpha shader bands, not volumetric fog.
- Fluid simulation remains a finite-volume block grid; this is not a continuous fluid solver.
- Dedicated painted/sculpted dragon art, long-session biome/boss balance and persistent world edits remain future work.
- No user play session was launched or terminated. The new version is selected on the next desktop-shortcut launch.


## High-FPS movement follow-up

- [x] Reproduced and corrected overlap-depth writes feeding interpolated XY back into physics. At 90 FPS the old code moved only 393 of the expected 600 units in two seconds.
- [x] Actual Avian/interpolation regression passes at 30, 60, 90, 120 and 144 FPS; all 76 tests pass.
- [x] Corrected release `20261023-164136-26c1cb2932` published and desktop launcher verified. This supersedes the encounter-pass binary above.
- [x] NVIDIA 2× Frame Generation published and verified with real generated presents. See FG_IMPLEMENTATION.md for current controls, tests and remaining higher-MFG scope.


## Current release

Superseded by `20261023-195826-06c27aa322`: 82 tests passed; old background appearance restored; detailed dragon, shallow-water freeze/thaw, DLSS torch clustering, steam shadow correction, shadow re-enable and material/weather performance fixes included. Burning unchanged. See FG_IMPLEMENTATION.md and PERFORMANCE_NOTES.md for evidence and remaining scope. Falling Heads - Play resolves the current release; existing sessions continue on their original executable.


## Dragon readability and turning follow-up (2026-10-23)

User reported an instant 180-degree facing flip and muddy/blended colors. Source used `scale.x = +/-1` each frame. Replacement uses a bounded 0.8-second Y-axis turn with independent smoothed banking and 45-unit facing hysteresis. Scale stays positive. Side-view breath waits until the mouth faces the play plane, instead of firing a sideways beam while facing the camera. Existing burn damage/spread values are unchanged.

Palette separates charcoal-blue skin, steel-blue raised scales, crimson-purple membranes, warm belly plates and ivory horns/teeth. Face/torso details exist on both sides of the model. Cuboid hit testing now includes projected side faces during a 3D turn, with a bounds rejection before triangle tests; degenerate projected triangles are ignored. Existing wing silhouette testing remains.

Five dragon encounter regressions passed (`target/dragon-turn-tests-out.log`), including 30/60/120/144-FPS turn timing, midpoint side-face hits, empty wing corners and ground fire damage/spread. Rendered quarter/midpoint/facing-right captures are added to the visual probe. Visual validation/publication pending.

The next-content request is recorded as proposed complete biome packs in the latest section of BIOME_DEVELOPMENT_PLAN.md. No new enemy/block/biome is claimed implemented in this correction.


### Dragon turn correction — published

Release `20261023-212645-a04a767617`, executable SHA256 `A04A7676177078A2631E0D33EED4E8F43B822ED749FF7558A556F0599AAEFD84`. Five targeted dragon tests passed. `target/dragon-turn-run-out.log` confirms visual probe exit 0; `target/dragon-turn-visual-err.log` has clean Streamline shutdown and no rendering error. Quarter-turn, head-on midpoint and completed right-facing captures inspected (`target/dragon-turn-visual-dragon-turn-*.png`); the continuous breath remains attached to the mouth. Night lighting still affects the model naturally. New biome/enemy/block packs remain proposals in BIOME_DEVELOPMENT_PLAN.md. Existing game sessions were left untouched.
