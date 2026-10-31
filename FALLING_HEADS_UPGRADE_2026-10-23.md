# Falling Heads upgrade and visual fixes — 23 October 2026

This is a delivery checklist, not a claim that the whole game's backlog is finished.

## Implemented

- [x] Migrated to Bevy 0.20.0 and Avian 0.8.0 from the old Bevy/local Avian setup. Updated affected assets, UI, input, ECS, lighting and physics APIs; no compatibility runtime retained.
- [x] Updated direct dependencies: rand 0.10.3, noise 0.9.0, axum 0.8.9, tokio 1.54.1, tower-http 0.7.1, serde 1.0.229, serde_json 1.0.154 and log 0.4.34. Removed unused ron and criterion dependencies.
- [x] Damage cracks now use subdued dark cores with a restrained earthy edge on a shared six-face shell, replacing the overly bright first revision. The first hit always shows damage; repeated events cannot create duplicate overlays.
- [x] Cactus uses alpha masking and back-face culling instead of two-sided transparent blending, addressing side-face sorting flicker.
- [x] Snow accumulation uses separate thin top caps, with four thickness stages, correct 64-unit block spacing, resize and melt removal. Block faces retain their texture. The cap thickness now also raises the block contact surface for characters, TNT and dropped resources.
- [x] Biome selection uses the same biome function as world generation. It preloads destination chunks incrementally, finds a clear landing in the selected biome and synchronizes the camera, velocity and collision index.
- [x] BURST now uses a procedural pixel shader with amber/cream colours, stepped edges and small square sparks. The generated plasma artwork is no longer loaded.
- [x] MIRROR uses a faceted 3D shell with a pixel-grid enchanted-glass shader, stepped highlights and restrained teal/ivory colours. The smooth sci-fi/iridescent shader has been replaced.
- [x] Removed BURST's old white rectangle visual path and MIRROR's old four-frame animation components. Shared meshes replace per-frame BURST mesh allocation.
- [x] Corrected the custom additive shader's premultiplied blending; transparent texture pixels no longer produce solid colour spikes.
- [x] Desktop Falling Heads shortcut continues to point to the rebuilt executable.

## Gameplay boundaries

Bounce vectors, restitution, targeting, burst collision dimensions, barrier reflection rules and lifetime were not deliberately retuned. Engine migration still warrants a normal play session before claiming identical feel. Generated effects are presentation changes.

## Validation

- Seventeen automated tests pass, including validated drop spawns, swept terrain contacts, repeated-frame bounce stability, developer-panel layout restoration and six-face shell coverage: damage stages; duplicate damage events; snow cap scaling; cell coverage; cap resize/melt lifecycle; biome destinations; teleport completion/camera/collision synchronization.
- Hidden native full smoke run passed all eight biome teleport assertions and rendered gameplay, damage, snow, pause/settings and energy material paths without shader errors.
- Separate hidden VFX capture exercises BURST while firing together with MIRROR. It caught and drove the additive blending correction.
- Screenshots: falling-heads-smoke-world.png, falling-heads-smoke-snow.png, falling-heads-smoke-pause.png, falling-heads-smoke-settings.png and falling-heads-smoke-energy.png.
- Visual inspection is not a complete user-input/playthrough test. Cactus flicker across camera motion and snowfall on every block type still need extended gameplay observation.
- The short --vfx-smoke-test run validates energy rendering only; its completion message now says so.
- Existing dead-code warnings and Avian sensor-body mass/inertia warnings remain. Those need a separate physics audit before altering mechanics.
- The native smoke runner bypasses the external HTTP listener; it does not validate normal API port 3029 availability.

## Assets and shader

- assets/textures/effects/energy/burst.png
- assets/textures/effects/energy/mirror.png (retained generated artwork; no longer loaded by the barrier runtime)
- assets/shaders/energy.wesl
- src/systems/energy_vfx.rs
- src/systems/mirror_vfx.rs
- assets/shaders/mirror_shell.wesl
- Exact built-in imagegen prompts: ENERGY_TEXTURE_PROMPTS.md.

## Proposed only — not implemented or enabled

- [ ] Record time spent in the same region, downward progress, repeated wall contacts and power use. Diagnose stalled runs before changing motion.
- [ ] Add an optional assistance director that acts only after sustained low progress, with a cooldown. Prefer opening a nearby route or offering a power to changing bounce physics.
- [ ] Compare fixed-seed recordings and time-to-next-region distributions before accepting any autonomy change.
- [ ] Introduce connectivity checks during world generation so large enclosed pockets are intentional and have exits.
- [ ] Use biome milestones and short objectives to improve perceived progression without globally speeding up movement.
- [x] Move ordinary chunk generation to bounded work budgets. Normal generation now processes at most four chunks per scheduled tick; teleport generation processes three per frame.
- [ ] Audit the boundary between custom block collision and Avian-controlled actors before adding more physics behaviours.
- [ ] Investigate dynamic sensor mass/inertia warnings with targeted regressions rather than tuning all bodies blindly.

## Commonfield pipe work delivered alongside this upgrade

The pipe cursor now intersects the pipe's placement plane instead of the terrain surface. A hovered socket bypasses grid rounding, and routing prefers available compatible sockets over occupied ones. Twenty plumbing workflow tests, the WASM check and native build passed. The native distribution executable was updated. This addresses concrete preview/selection faults; it is not proof that every pipe joint or placement gesture is correct.


## UI and material follow-up

Developer controls restore their enabled state on resume, Tab updates layout in a defined order, and developer/menu panels have explicit drawing priority. Decorative HUD roots do not block pointer picking. Pause/settings use a lighter transparent overlay, and gameplay HUD roots are cleaned before recreation. The new --ui-smoke-test sends real pointer inputs through Bevy picking to check the menu and developer controls.

Dependency audit: cargo update --dry-run reports zero compatible updates. Avian is registry 0.8.0, not the old local checkout. Axum 0.8.9 pins its indirect matchit dependency to exactly 0.8.4 (newer 0.8.6 exists); it is not overridden. The declared Rust minimum is corrected to 1.97.1 to match Bevy 0.20.

### Follow-up verification

At the earlier UI/material checkpoint, eight tests passed. The newer Escape/snow/damage checkpoint below supersedes that count. The native 3D shell rendered without shader errors and was inspected in falling-heads-smoke-energy.png. A hidden pointer-input test passed Escape, Resume, Tab, Rain, Settings, the developer-mode toggle, Back and Escape-to-resume. The final normal executable was rebuilt at target/debug/falling-heads.exe; the desktop shortcut uses that path.

The barrier is a true geometric energy shell with procedural surface waves, a view-dependent Fresnel rim and a specular highlight. It is not a ray-traced reflecting mirror or a scene-refraction simulation.

## Escape, snow and damage follow-up

The grey pause screen was caused by the button hover query matching the full-screen backdrop: it wrote opaque RGB (0.7,0.7,0.7) into the backdrop when the pointer moved into empty menu space. The regression failed with that exact colour before the fix and passes after filtering actual Buttons. The same correction applies to Settings. Native validation now includes hovering empty space, a sustained pause and window resizing, not only clicks directly onto buttons.

Weather now uses the world generator's biome function. Particle emission retains fractional counts so light snow is visible instead of rounding every emission to zero. Accumulation requires active, unexpired snowfall and an exposed column top; covered cave ledges no longer acquire snow merely because the adjacent cell is empty. Existing snow melts gradually after snowfall ends. Snow and block contact use the same thickness; bare-ground bounce coefficients are unchanged.

Dark crack opacity and restrained earthy edge contrast were increased without restoring white outlines. The six-face shell is separated slightly farther from the block surface, and damage overlays run after health updates. A native gallery checks two damage levels on rotated netherrack, stone and sand blocks.

### Final delivery verification

- [x] All 13 automated tests pass, including empty-backdrop hover, active snowfall/exposure, non-solid cobweb exclusion, and actual hero contact with snow.
- [x] Hidden native input test passes empty-background hover, a 15-second pause, window resize, Escape, Resume, Tab, weather selection, Settings, developer toggle, Back and resume. Capture: falling-heads-ui-long-pause.png.
- [x] Final normal executable rebuilt successfully after closure of the previous game. The desktop shortcut targets target/debug/falling-heads.exe. A normal game process started after this rebuild was observed; it was left running.
- [x] Dark cracks inspected at two damage stages on rotated netherrack, stone and sand, including side faces.

Build evidence: regression-final-build.log. Test evidence: regression-tests.log and regression-ui.log. The hidden native test preceded the final cobweb exclusion; that exclusion is covered by the final automated suite.

## Pixel-style effects and drop-contact follow-up

- [x] Replaced sci-fi BURST and MIRROR presentation with discrete pixel patterns and a limited palette. Collision shapes and power timing remain unchanged.
- [x] Added a redstone-coloured LASER shader in the same effect system. Removed the old three-layer laser entities, markers and manual layer cleanup; child lifetime now follows the parent beam.
- [x] Removed unchecked question-box spawn fallbacks and duplicate safe-spawn helpers. All periodic/API boxes and resources use one terrain resolver before rendering.
- [x] The resolver checks full bounds, sweeps fast movement in bounded increments, checks newly streamed terrain, and searches verified nearby loaded space for embedded spawns. If none exists, the drop waits hidden instead of publishing an invalid position.
- [x] Preserved box restitution and bounce limits; repeated interpolated contacts cannot reflect an already departing box again. Resource wall damping and floor friction remain in the new resolver. Hero motion is unchanged.
- [x] Terrain collision indexing refreshes when blocks change instead of waiting ten frames. Drop occupancy is cached while terrain is unchanged.
- [x] Leaf and leaf-textured grass blocks use alpha masking and back-face culling. Cobwebs and resource sprites use cutout alpha rather than blended sorting.
- [x] Snow now grows on local ledges with two clear cells overhead during active snowfall. This intentionally replaces the previous global highest-loaded-block rule, which prevented visible accumulation in this vertically scrolling world. Tight covered gaps remain bare; distant roofs do not suppress every chamber below them.
- [x] Cracks have a pale chipped edge around a dark core on all six faces, increasing contrast on both light and dark textures.

Validation: 17 tests pass (voxel-followup-tests.log). A hidden native run passed all eight biome teleports and produced 337 snow caps from actual weather, without manually inserting layers (voxel-native-errors.log). Native screenshots: falling-heads-smoke-snow.png, falling-heads-smoke-energy.png, falling-heads-smoke-laser.png. The final native menu check passed background hover, long pause, resize, Escape, Resume, Tab, weather selection, Settings, developer toggle and Back (voxel-ui.log). The final VFX run rendered Mirror, Burst and Laser without shader errors (voxel-final-vfx-errors.log). The normal target/debug/falling-heads.exe has been replaced with the verified build; SHA-256 matches the tested executable, and the desktop shortcut still targets this normal path. Extended play can still reveal additional cases; these checks do not prove that every rendering/contact defect is eliminated.

## Current checkpoint: responsive teleport, crowd assistance, liquids and flowing Mirror

This checkpoint supersedes earlier effect colours and test counts above.

- [x] Mirror's rendered position copies the owner's final transform in PostUpdate, after teleport and before transform propagation. It does not interpolate toward an older position. The earlier Update synchronization remains for projectile reflection, while the late synchronization makes the displayed shell match the rendered character.
- [x] Replaced the static glass/pixel pattern with animated counter-moving magical currents, travelling ribbons and a softly varying Fresnel rim. The centre stays mostly clear. The sphere uses 48 by 32 tessellation. This is a geometric procedural effect, not scene refraction.
- [x] Dense crowds use rounded head-to-head collider corners with the same outer dimensions. A small terrain-checked sideways nudge only happens after at least 0.75 seconds of low progress with three or more neighbours, followed by a 1.5-second cooldown. Ordinary terrain bounce coefficients remain unchanged. Extended play is still required to judge the feel.
- [x] Teleport preloads nine nearby chunks, with bounded work every frame. The portal appears during a 0.22-second charge and reveals the destination afterward. Commit moves the hero, camera, action centre and collision index together. Destinations require loaded, clear space in the requested biome and avoid liquids. Failure leaves the hero at the source and shows a message.
- [x] Cactus geometry and cropped UVs now meet at the actual inset body rather than mapping transparent border pixels onto separated faces. Damage shells match that narrower body.
- [x] Laser has a white core and neutral cool edge. Crack highlights are thinner-looking and less bright while the dark core remains.
- [x] Added deterministic finite-volume water/lava simulation on the block grid, independent of its renderer. Water flows faster; lava flows more slowly. Finite sources fill and spread through loaded space. Liquids have animated pixel shaders and actual volume-dependent visible depth.
- [x] Body immersion applies drag and buoyancy. Lava burns; water extinguishes burning. Water/lava contact creates mineable obsidian. Nether generation includes lava and occasional water sources; selected other biomes generate water.
- [x] Existing obsidian asset is assets/textures/blocks/nether/obsidian.png. The searched FallingHeads/communityGaming directories contain loose Minecraft-style textures; no complete resource pack with pack.mcmeta was found there.

Validation: 24 automated tests pass (gameplay-followup-tests.log). The final full native smoke passed all eight biome destination assertions, with request-to-commit times of 0.220–0.240 seconds on this machine, and generated 333 natural snow caps. It exercised pause/settings and rendered the new shaders without errors (gameplay-final-errors.log). Controlled screenshots show joined cactus faces and separate water/lava basins (falling-heads-smoke-liquids.png), and the portal (falling-heads-smoke-teleport.png). Test timings are not a guarantee on every machine or world seed.

Liquid scope: this is a game-style cellular simulation with eight depth levels per block, not a general 3D fluid solver. Unloaded fluid state is retained for the running session; disk persistence and large-world streaming memory budgets remain future work. Crowded-head assistance is targeted and does not guarantee that every generated enclosure has an exit.

Delivery: the final magic-ribbon shader passed the hidden native VFX run (gameplay-magic-errors.log) and was visually inspected. The verified executable was copied to target/debug/falling-heads.exe after confirming the normal game was closed; SHA-256 hashes match. The existing desktop shortcut points to that executable. No visible game was launched.

## Follow-up: barrier depth, arrows, label timing and connected liquids

- [x] Character cutout textures use alpha masking, so they write depth before the transparent Mirror shell. This removes the unstable transparent-object ordering between heads and the barrier.
- [x] Replaced the striped/glass Mirror appearance with a mostly clear interior and an animated turbulent magical rim. The earlier same-frame owner attachment remains.
- [x] Corrected the requested colour target: Burst is pearl-white; Laser returns to its red/orange palette and warm core.
- [x] Arrow collision now sweeps the tip between frames, selects the nearest block/hostile actor, and lodges the arrow rather than deleting it at terrain impact. Embedded terrain arrows remain for eight seconds; actor arrows keep their two-second lifetime. Parent transforms preserve the original arrow size even on terrain meshes scaled by 64. Destroying the parent removes its arrows. Removed the unused older arrow/block-destruction function.
- [x] Username projection reads the final propagated owner and camera transforms in PostUpdate, publishes its render transform immediately, and runs before visibility testing. Removed inaccurate comments claiming that these projected overlay labels were 3D text. This is a same-frame 3D-to-2D overlay, not a delayed follower.
- [x] Falling liquid prioritizes gravity and does not spread sideways until supported. Transport flux draws connected vertical strips. Settled water uses its actual volume-dependent surface height.
- [x] Water and lava each use one mesh with a world-space animated shader. Removed the old per-cell visual entities and repeated per-cell surface-highlight lines.
- [x] Generated springs can fill small, enclosed natural basins. Pool discovery requires a loaded, closed floor/wall boundary, is capped at 64 cells, and falls back to a finite source for open or oversized cavities. It does not carve artificial lakes into every biome.

Regression coverage added: thin-wall/corner sweeps, a fast arrow in the actual collision system, scaled-parent attachment and expiry, vertical falling-flow geometry, and closed/leaking/unloaded pool boundaries. Native checks also compare the rendered username to the current head/camera projection each frame.

Validation and delivery for this follow-up: all 31 tests pass (presentation-flow-tests.log). The full native run passed eight biome teleports, snowfall, pause/settings and same-frame label assertions (presentation-flow-errors.log). A later effects run exposed a queued attachment targeting an entity that could already be gone; arrow attachment now checks both entities atomically when the command applies, with a regression for target/arrow removal. The final native VFX run passed energy, water/lava, embedded-arrow and same-frame label rendering (presentation-final-errors.log), without a shader or command error. Screenshots: falling-heads-smoke-energy.png, falling-heads-smoke-laser.png, falling-heads-smoke-waterfall.png and falling-heads-smoke-liquids.png. The normal desktop-shortcut executable has been replaced after confirming the game was closed; its SHA-256 matches the tested final executable. No visible game was launched.

The barrier's style remains an artistic choice for user feedback. The corrected depth ordering is separate from that appearance choice. Liquid sources are finite and eight-level; these changes do not claim an infinite waterfall or a general-purpose 3D fluid simulation.

## Current checkpoint: teleport atmosphere, fluid interaction and material pacing

This section supersedes the earlier teleport timings, crack layouts and fluid behaviour above.

- [x] Teleport charges for at least 1.10 seconds and reveals over 0.85 seconds. The portal uses analytic shader edges instead of deliberately quantized 64-pixel UVs. Bevy's camera effect stack provides temporary chromatic separation and pulsing lens distortion across the world.
- [x] Head render meshes collapse into a child visual and expand back; the physics owner's scale/collider never changes. Mesh and material are restored together to re-register Bevy's render material instance. Damage borders are hidden during the collapse. The clean Mirror shell remains unchanged.
- [x] Safe landing searches up to 48 cells from the selected point, prefers a clear 3-by-3 area, and accepts a clear single cell if necessary. It still checks loaded terrain, the selected biome and liquids. It allows more destination chunks to arrive before reporting failure. The message is now ASCII: "No safe landing found - stayed here". It does not teleport into solid terrain to hide failures.
- [x] Liquid shaders now animate continuously, with moving water caustics and lava crust/veins/bubbles. Pool meshes include exposed top/side depth faces and omit shared submerged side faces. Short drops broaden into spills; isolated tiny falling volumes render short droplets instead of full-block rods. This remains a finite, eight-level cellular simulation, so grid-shaped surface steps can still occur.
- [x] Liquid drag runs after the movement/collision systems. Living heads swim upward when sufficiently immersed. Water permits faster movement; lava strongly resists it and applies immediate contact damage followed by burning. Water extinguishes burning. Ordinary dry-terrain bounce coefficients are unchanged.
- [x] Water plus enough lava creates mineable obsidian and steam. Tiny contacts evaporate instead of making a full obsidian block from a droplet.
- [x] Local exposed rain cools lava; fire rain removes water with animated steam; sustained snow freezes deep water into solid snow cells. Frozen cells stay frozen during active snow and melt back into water after about 60 seconds of loaded simulation otherwise. Exposure is the game's local two-cell overhead test, not a full sky-occlusion model.
- [x] Finite pools start gradually draining after a 45-second grace period: one water depth unit per roughly 12 seconds, or one lava unit per 18 seconds. Sources stop replenishing during this retirement. Unloaded state stays dormant; it is retained for this session.
- [x] Re-streamed/new solid terrain displaces retained fluid into adjacent free loaded cells, avoiding liquid embedded in regenerated blocks. Any trapped remainder is absorbed by the solid.
- [x] Leaves and other soft foliage now have 1 health; sand/snow/soul sand/netherrack 2; ordinary stone 3; End stone/purpur 4; obsidian/ancient debris 5. This changes biome material pacing while retaining the existing biome terrain, enemy and weather profiles.
- [x] Cracks have three deterministic shader layouts (original, mirrored/warped, rotated/warped) selected per block. These are derived patterns, not three newly generated raster images. Existing dark cores and restrained pale outlines remain.
- [x] Mirror impact speckles were replaced with a short expanding circular ripple. Burst remains pearl-white and Laser remains red/orange. Arrows still lodge in blocks and expire without damaging terrain.
- [ ] A comprehensive distinct ruleset/objective loop for every biome remains future design work; the changes here differentiate materials and environmental hazards.
- [ ] Fluid disk persistence, long-session streaming memory budgets, continuous sub-cell surface reconstruction and a general 3D fluid solver are not implemented.

Validation: 38 automated tests pass (atmosphere-tests.log), including immediate lava damage without per-frame repeats, swimming/drag, weather exposure, fluid ageing/displacement, head mesh restoration and unchanged owner scale. The first full native run passed all eight biome checks, snowfall and menu schedules. Its captures revealed a material restoration issue after teleport; that was corrected before delivery. The final hidden native run passed all eight biome teleports (1.106–1.110 seconds to commit), 288 natural snow caps, menu schedules and same-frame label checks, with no shader/command errors (atmosphere-final-errors.log). Final screenshots were inspected: the hero renders normally after teleport, the clean Mirror and impact ripples render, and the controlled water/lava gallery includes an embedded arrow. The normal desktop-shortcut executable was replaced only after confirming it was closed; its SHA-256 matches the tested executable: 56F63D3743E582880FE655F89854458719065E297DC60854FABB782DF5E94FF0. No visible game was launched. Visual style and extended gameplay balance still need player feedback.

## Current checkpoint: fluid contact, shallow arrows and redstone portal ruins

- [x] Fluid contact and rendering share the same pool/stream/droplet rectangles. Mid-stream contact now slows/burns; empty space beside narrow streams does not. Regression exercises both cases with the actual contact system.
- [x] Arrow collision uses the GLB's scaled tip extent. Block penetration is two world units. Arrows initially overlapping terrain recover the entry face; three scales are tested. Arrows still cause no block damage.
- [x] Added generated obsidian portal ruins with a pressure plate, visible redstone signal propagation, an animated purple opening and an advertised destination. The full reserved footprint stays inside its streamed chunk. Frame destruction cuts power on the next circuit tick.
- [x] Added TELEPORT power to developer controls and HTTP API, plus a 5% lootbox reward. They share the same safe travel request and twenty-second cooldown. Rejected requests show recharge/in-progress feedback.
- [x] Natural portals require 0.35 seconds inside the powered opening. The existing charge/reveal and biome-safe landing coordinator performs the travel. A pressure plate latches power for twelve seconds.
- [x] Verified the available redstone item, redstone ore and obsidian textures are 64x64. No full resource pack or dedicated mechanism texture set was found in the searched game/communityGaming directories.
- [x] Saved BIOME_DEVELOPMENT_PLAN.md with an honest asset/behaviour audit and proposals for all eight existing biomes.
- [ ] Player-built branching redstone circuits, levers, repeaters, pistons, dispensers and pumps remain future work. The shipped mechanism is a fixed plate/wire/portal ruin, not full Minecraft redstone.
- [ ] Full per-biome encounters/enemies and persistent circuit/ruin changes across chunk regeneration remain pending; detailed priorities are in the development plan.

Validation: 44 tests pass (portals-tests.log). The full hidden native run passed all eight biome destinations, snowfall, menu schedules and the pressure-plate portal journey (portals-errors.log). The final effects run passed the updated label/render path, arrows, fluid gallery, pressure plate, portal dwell and safe destination (portals-final-errors.log), without shader or command errors. Portal and liquid screenshots were inspected. After the user closed Falling Heads, target/debug/falling-heads.exe was replaced and its SHA-256 matched the tested portals executable. The desktop shortcut still targets this path. No visible game was launched.

## 2026-10-23 — connected spills, weather powers, biome mechanisms and versioned desktop builds

Completed:
- Water transport traces persist briefly across simulation steps; tiny streams span cell height; ledge spills join adjacent pool boundaries and start at their feeder's actual surface. Rendering and immersion share the exact same section bounds. Frozen/cooled cells cannot retain invisible fluid contact.
- Teleport charge/reveal shortened to 0.55/0.40 seconds; stronger chromatic and lens distortion. Destination readiness still gates travel. World portal dwell shortened to 0.20 seconds.
- Steam has a small bounded radius, gentle rise and smooth fade-in/out envelope; no final full-opacity disappearance.
- Rain produces finite puddles on local exposed ledges. Fire rain melts exposed snow caps into small finite water deposits plus steam, and rapidly thaws frozen fluid cells. It does not convert every natural Snow terrain block.
- WATERBUCKET/LAVABUCKET API commands and dev buttons, finite sixteen-unit pour, shared eight-second cooldown, terrain/type checks and feedback. Command labels fit the panel.
- Desert pressure-plate arrow traps and Underground relay doors generate in reserved, non-overlapping chunk footprints. Doors hold open for actors/items/liquid; projectile art scale cannot enlarge the doorway sensor. Trap arrows remain non-mining and are globally capped at 24.
- Portals and new devices migrated entirely to one synchronous directed circuit graph; the old fixed-chain signal function was removed.
- Desktop shortcut migrated to tools/Launch-FallingHeads.ps1. tools/Publish-FallingHeads.ps1 publishes a separate executable + asset snapshot and switches the active manifest atomically. Current sessions need not close for publication; next launch uses the latest release.
- BIOME_DEVELOPMENT_PLAN.md updated with completed versus pending encounters and a detailed SpongeBob-inspired underwater proposal. No underwater assets/enemies were enabled by this proposal. No original streaming-channel name could be verified in the searched source/project notes.

Validation:
- 55 automated tests passed (weather-final-tests.log).
- Native full smoke run passed all eight biome destinations, snow, menus, VFX, portal and mechanism scenarios (weather-native-errors.log).
- Final VFX regression and installed-release runs passed, including actual relay open/occupied/close behaviour, Desert arrows, stepped liquid rendering and same-frame labels (weather-final-native-errors.log, weather-installed-native-errors.log).
- Inspected waterfall captures; corrected upward spill columns and wrapped bucket labels before final release.
- Publisher verified all 301 copied assets and executable hash. Launcher resolved successfully both in the development shell and actual Windows PowerShell used by the shortcut.

Installed version: 20261023-051037-69c75bcf57
Executable SHA256: 69C75BCF57F706B6B9506C7816DDD377E811A2BA7177D220CC19F2BA238B9B7F

Limitations: existing 121 compiler warnings remain. The fluid model is grid-based, not a continuous fluid solver. Structures still regenerate across chunk unload/reload; persistent circuit/world edits remain future work. Mechanism art uses existing materials and procedural geometry; a dedicated art set and long-run balance testing are pending. Hidden validation windows exited; no visible play session was started.

## 2026-10-23 — rare background portals, climate and graphics pass

This is the current status; earlier dated portal/evaporation/launcher descriptions above describe superseded builds.

Completed:
- Portals now have one owned hierarchy with decorative non-colliding background frames. There is no activation plate, wire latch, destructible frame requirement or separate orphan frame lifetime. Generation opportunity is 1/127 chunks. The opening retains validated safe travel and shared cooldown.
- Portal material uses continuous animated noise and an opaque fill, removing the repeating fleck/grid pattern. Depth is between the backdrop and actor planes.
- Removed the fluid birth/age drain path entirely. Water persists unless displaced, involved in a reaction, or unloaded with its chunk; world persistence remains unfinished. Biome temperature governs natural snow-cap melting; fire rain still supplies heat.
- Arrow embedding varies deterministically with bounded actual tip penetration, for terrain and heads. Terrain arrows remain non-mining.
- Added the shared climate table, original articulated voxel End dragon flyover, sparse wall torches, bounded local lighting, HDR/bloom, SSAO, directional shadows covering this game's world scale, FXAA, weather/moon colours and haze. The old incomplete dragon AI/animation modules and asset resource path were removed, not retained as a fallback.
- Eleven graphics settings persist to graphics-settings.json. Two stacked cameras share HDR/MSAA configuration; a discovered stale text-pixel regression was fixed before installation. Backdrop tiles do not cast thousands of unnecessary shadows.
- GRAPHICS_EFFECTS.md and GRAPHICS_EFFECTS.txt give the current/pending catalogue, settings mapping, climate ideas and verified DLSS/Solari integration requirements. BIOME_DEVELOPMENT_PLAN.md updated.
- Fixed publisher File.Replace backup-path handling for PowerShell 7 while retaining Windows PowerShell compatibility. Publication remains versioned; it does not overwrite an open game executable.

Validation:
- 59 automated tests passed, including persistent water, bounded varied arrow embedding, portal activation with no frame/plate, rare placement, independently serialized graphics settings and actual camera-component toggling (graphics-tests-final.log).
- Final delivery native effects run passed same-frame moving labels, portal destination, relay/Desert traps, articulated dragon flight and joint hierarchy; screenshots checked for portal layering, improved wings and day/night lighting (graphics-delivery-native-errors.log).
- Native UI pointer test passed long pause, resize, Escape/Resume, Tab, weather selection, Settings, dev toggle and Back. Graphics panel screenshot inspected at 720x1180 (graphics-final-ui-errors.log, falling-heads-ui-graphics.png). This UI test preceded only the final portal hierarchy ownership change.
- No shader/runtime errors appeared in those native test logs. Existing 123 compiler warnings remain; the build is not warning-free.
- Publisher verified executable SHA256 and all 302 asset copies. The actual Windows PowerShell launcher resolves the installed build successfully.

Installed version: 20261023-055424-cd0506836a
Executable SHA256: CD0506836A755327FE14BC06F2FF95EF1C6DF70E83899F15E165EADF3C80C479

Still pending: DLSS SDK/toolchain and renderer integration, Solari/custom material migration, richer authored dragon art/boss behaviours, volumetric fog and advanced water optics, a local heat field/humidity/wind simulation, long-session biome balance, and persistent world edits. This pass does not turn every item in the modern graphics catalogue into an implemented feature. Moonlight colour is implemented; a visible moon/sky model is not. No visible game was launched.

## 2026-10-23 — constant waterfall width and native DLSS / Solari

This entry supersedes the previous pending-DLSS/toolchain status. The SDK was already available in Commonfield's vendor directory; the earlier environment-only check was incomplete.

Completed:
- Removed the airborne short-drop widening rule and volume-based ribbon width. Falling water keeps a 12-unit cross-section (lava 18) until it actually joins a supported pool/ledge. Drawing and immersion still use the same bounds. The regression covers alternating parcel volumes and all rows above support.
- Enabled Bevy 0.20 DLSS and Solari using the existing local DLSS/Vulkan/libclang toolchain. Locked prettyplease 0.2.37 to avoid bindgen's incompatible syn 2/3 dependency combination. Commonfield files were only read.
- Added DLSS Quality and Ray tracing settings, actual runtime-support/status reporting, Solari + Ray Reconstruction and native-resolution RR/DLAA mode. DLSS is on by default; ray tracing is opt-in. Existing saved preferences were preserved while adding the two new fields.
- Opaque lit geometry has owned, bounded RT-only instances with validated mesh layout. Source mesh/material changes update the instances. Local torch/lava illumination has emissive RT proxies. Custom transparent/masked effects remain forward rendered.
- Replaced the shared world/UI render target with an independent HDR world image presented behind native-resolution text/UI. Resize, pause and camera-cut history handling are integrated. No old shared-target implementation is retained.
- Added locked build/staging script, SDK configuration, NVIDIA license, and executable-adjacent SR/RR runtime DLLs. Publisher copies/hashes both DLLs plus all assets; the actual desktop launcher verifies executable and DLL hashes. Each release remains immutable.

Validation:
- 63 unit/regression tests passed (native-gpu-tests-final.log). Existing 124 compiler warnings remain.
- Hidden native renderer test passed SR -> Solari/RR -> native RR -> raster -> SR; each screenshot was captured after settling the mode, separately from mode changes (native-gpu-verified-errors.log). GPU diagnostic timings confirm actual Solari lighting/world-cache and RR execution, not just components: approximately 0.081 / 0.147 / 0.997 ms in the sampled scene. These are pass timings, not overall performance promises.
- Effects regression passed separately with DLSS raster (native-vfx-verified-errors.log) and Solari/RR (target/native-rt-validation/rt-vfx-errors.log), including moving labels, liquids, world portal generation/travel, relay doors, Desert traps and dragon hierarchy. Waterfall screenshots inspected in both modes.
- Real-pointer hidden UI test passed Escape, long pause, Resize, Resume, Tab, weather controls, Settings, dev toggle and Back; the native-resolution graphics panel and pause screenshots were inspected (native-ui-verified-errors.log).
- An earlier effects probe encountered Vulkan DeviceLost followed by invalid-buffer errors. That driver reset did not recur in the final renderer, separate raster-effects, ray-traced-effects or UI runs. The initial failure log is retained as native-water-vfx-errors.log; no claim is made that the driver can never reset.
- PowerShell scripts parsed, all 303 asset copies and both runtime DLL copies passed hash verification, and the actual Windows PowerShell desktop launcher resolved the new release successfully.

Installed version: 20261023-143851-5a304a9e4a
Executable SHA256: 5A304A9E4AB8FCFB4A63DB81B6B42AA22ED79B0FF28FB6831E12F5A79CCF8CE4

Limits: this targets the RTX 5070 Ti / Vulkan ray-query-capable setup. Solari is experimental. Transparent liquids/barriers and masked foliage are not physically ray traced; no frame generation is added. Native graphics do not create detail missing from the existing pixel textures. Full catalogue and architecture details are in GRAPHICS_EFFECTS.txt and NATIVE_RENDERER.md. No visible play session was launched.
Installed-release GPU validation also passed from its own executable, immutable asset snapshot and packaged DLLs (target/installed-gpu-validation/installed-gpu-errors.log), with no runtime/shader errors. All hidden validation processes exited.

## 2026-10-23 — attached torches, integrated settings and fluid/cutout corrections

Completed in source:
- Settings now has a single centred shell with General and Graphics navigation, a compact two-column graphics page and a shared Back action. Removed the detached graphics-root lifecycle and the old right-aligned general menu. One settings-owned style system handles hover/pressed/selected states and restores normal colours; the general action handler no longer recolours unrelated buttons.
- Torches have a bracket and angled shaft anchored to the actual left/right or front block face. They inherit the support entity's transform and lifetime, with world-size compensation for scaled blocks. Thin cactus and cutout vegetation are not torch supports. Removed the extra dark texture tint so the shaft remains visible. Smaller Solari light proxies retain emitted power without the old oversized circular shadows.
- Sideways fluid transfer remembers the spill-edge offset and propagates it down the stream. Support is resolved upward from solid ground; a full airborne packet cannot masquerade as a pool. Fluid contact uses the same edge-aligned rectangles as rendering.
- Fluid meshes now construct the exposed boundary of their rectangle union. Shared horizontal and vertical faces are omitted, including partially covered faces; stacked/adjacent water no longer has internal transparent caps that double-blend into seams.
- Lit alpha-mask StandardMaterials now write the primary deferred G-buffer in Solari mode, giving Ray Reconstruction matching depth/albedo/normal guides. Secondary rays still exclude cutout geometry. Unlit artwork and blended/custom effects retain forward rendering. An intermediate attempt to defer unlit artwork failed visual review and was corrected before publication.

Validation so far:
- 69 unit/regression tests pass (surface-final-tests.log), including spill-edge propagation, unsupported full packets, hidden interior liquid faces, torch support ownership/lifetime/scale, cutout primary-versus-secondary eligibility, and settings navigation/hover reset.
- Real-pointer hidden UI test passes both tabs, pause, resize, Resume, Tab/dev controls, weather, Settings and Back (surface-ui-native-errors.log). The new centred Graphics page screenshot was inspected.
- Ray-traced effects run passed the corrected cutout path, liquid gallery, labels, portal, relay/trap and dragon scenarios (target/native-rt-validation/surface-final-vfx-errors.log). This run preceded only the final brighter torch tint and exclusion of cactus/vegetation mounts.

Final delivery validation:
- Final binary passed the native renderer-switch test with real Solari/RR GPU timings and dense cutout foliage (surface-gpu-errors.log).
- Installed-release ray-traced effects test passed from the immutable asset snapshot and packaged NVIDIA DLLs (target/native-rt-validation/surface-installed-errors.log). The final torch supports/visible shafts, connected water and preserved heads/backdrop were inspected in its capture. No shader/runtime errors appeared in these final logs.
- Publisher verified 303 assets, executable and both runtime DLL hashes. Windows PowerShell launcher resolved the new version. No visible session was launched or terminated.

Installed version: 20261023-150811-6d4a3a7b9b
Executable SHA256: 6D4A3A7B9B84C3C33A3E1F546720373DB18A19EE1E41EB250F767D8D7103C343

The simulation remains a finite-volume block grid; water is not a continuous fluid solver. Solari remains experimental; cutout primary visibility is supported here but secondary rays still omit masked geometry. Existing 124 compiler warnings remain.


## 2026-10-23 — reactive encounters, safe freezing and final lighting corrections

Installed version: 20261023-162722-4d9d6d7865
Executable SHA256: 4D9D6D78651F69BF90EBF4CDEECC353F034BDD52F55B3008A1B58C22528FB540

Completed:
- Portals accept E nearby as well as touching the opening; decorative frames and teleport effects do not cast shadows. Generated redstone mechanisms were removed; ore/resources remain.
- Dive/rise controls and phased autonomous swimming/escape strokes. Snow and frost shots create temporary breakable Ice, skipping occupied cells and partial puddles. Ice and snow surfaces preserve lateral sliding.
- Stable actor depths, duplicate damage-border prevention and foreground ghasts. The dragon has persistent circling/reactive/perching behaviour, 40 HP, animated-part hit regions and a clipped continuous fire cone that damages and ignites terrain.
- Compact animated torch flames, restrained thresholded bloom, explicit day/night Solari environment fill and correctly converted emissive light power. Lava emitters are inside opaque lava, avoiding floating proxy dots. The textured night backdrop retains detail.
- Spill origins survive final landings/respills; thin water/lava contact creates obsidian. Desert cave-floor cactus height varies from absent to three segments, with rate-limited contact damage; sand remains two hits. Optional biome mist is controlled by Atmospheric haze.

Validation:
- 75 tests passed in encounter-tests.log. Final native rendering transitions passed with actual GPU timings in encounter-gpu-errors.log. Final RT gameplay passed portal travel, moving labels, dragon persistence, continuous breath and damage in encounter-final-native-errors.log.
- Captures inspected after the final lighting correction. No runtime/shader errors in final native logs; existing 129 compiler warnings remain.
- Publisher verified 305 asset copies, executable and both NVIDIA runtime DLLs. Windows PowerShell launcher resolved the immutable installed build successfully. No user session was launched or terminated.

Full controls, completion checklist and practical limitations: ENCOUNTER_FIX_CHECKLIST.md. This supersedes earlier pending boss/DLSS status; dedicated dragon art, long-session balance, persistent world edits, volumetric fog and full transparent ray transport remain future work.


## 2026-10-23 — high-FPS movement regression correction

Installed version: 20261023-164136-26c1cb2932
Executable SHA256: 26C1CB2932A873ED1D214650F71529C17539FFD2FBDD9CC4FF11D2B6F5CE7ED3

The previous overlap-depth fix wrote Transform every PostUpdate, after interpolation. That caused Avian to treat the older smoothed XY coordinates as authoritative movement on following physics ticks. A controlled two-second, 300-units/second head test reproduced 392.812 units instead of 600 at 90 FPS (about 35% slower), while 60 FPS travelled normally.

Depth assignment now runs before fixed physics preparation and updates authoritative Position.z only when necessary. It leaves interpolated XY untouched; non-physics entities update Transform depth only when needed. Stable overlap ordering and foreground ghasts remain.

Validation: all 76 tests passed (target/motion-full-tests.log). The new regression uses actual Avian integration/interpolation at 30, 60, 90, 120 and 144 FPS; physical distance is 595.312–600 units over two seconds, within one fixed tick, with matching rendered distance. The build passed with the existing 129 warnings. Publisher verified executable, both runtime DLLs and 305 assets; the Windows PowerShell launcher passed integrity checks. The previous native GPU/effects validation is retained; it was not repeated for this simulation scheduling-only change. No user's running session was terminated.

Frame generation remains unimplemented. Current Bevy integration exposes Super Resolution and Ray Reconstruction. NVIDIA's Vulkan Streamline sample demonstrates a possible future FG route, requiring separate presentation/Reflex integration: https://github.com/nvpro-samples/vk_streamline . Generated display frames do not accelerate physics simulation. This correction confirms a head-motion regression; it does not independently prove the cause of any reported non-physics weather slowdown.

## 2026-10-23 — recovered glowstone, bush, dragon depth and surface fire work

Completed the interrupted changes from chat 01a12c82-6a54-7490-b996-63ffb327b280. Glowstone contributes steady warm point lighting to the existing nearest-source light pool, with textured emission controlled by lighting/local-light settings. Dead bushes now use a shared branching mesh and textured brown material instead of the plain tan fallback cube. The dragon rig compresses world depth after rotation to keep its geometry in front of terrain while preserving XY motion and turning. Breath impacts add animated surface flames around multiple nearby blocks; the block faces remain readable. Existing flammable-block damage, spread and extinguishing behavior remains.

The interrupted test had used Hero as a unit struct. Corrected its fixture to supply Hero fields, then completed validation:
- cargo test --offline: 90 passed, 0 failed (target/surface-fixes-tests-out.log).
- Build succeeded with 129 warnings; no compiler errors (target/visual-build-err.log).
- tools/Test-FallingHeadsVisual.ps1 -Tag surface-fixes: exit 0, no reported shader/runtime errors, clean Streamline shutdown, all required fresh captures present (target/surface-fixes-run-out.log and target/surface-fixes-err.log).
- Inspected target/surface-fixes-dragon-fire.png and target/surface-fixes-dragon-turn-middle.png: surface flames, block textures, branch silhouette and intermediate dragon pose render correctly.
- The visual probe uses DLSS/native raster modes with frame generation and ray tracing disabled. This run does not establish that earlier FG glare/shadow flicker is resolved, or independently validate this update in ray tracing.

Installed version: 20261023-214843-f243ada06a. SHA256: F243ADA06AEC5C514846069EC096540AAD56092307C2BCA2CE6A9F746CD717E0. Publisher verified 306 assets and runtime DLL copies; launcher ResolveOnly passed. Restart Falling Heads - Play to load this build.

## 2026-10-23 — dragon turning animation and flight momentum

Replaced constant-rate yaw with an exact critically damped angular response. Reversing the requested heading preserves angular velocity and eases into the new turn. Flight velocity now accelerates and decelerates smoothly, slows during banking and adds a small upward arc; free flight faces its travel direction while attacks track the target. Body pitch follows the continuous heading instead of switching with the facing flag. Local roll, asymmetric wing strokes, outer-wing folding, tucked legs, an articulated leading neck and delayed tail segments respond to turn effort. Fire origin and direction use the same neck transform as the visible head.

Validation: 91 unit tests passed, including equal turn response at 30/60/120/144 FPS and preserved angular momentum when retargeted. Final build succeeded with 129 warnings. The hidden renderer probe exited 0 with clean Streamline shutdown; fresh quarter/middle/completed-turn and fire captures were inspected. Throughout the sampled turn, actual transformed hit-mesh vertices were checked to remain finite and in front of terrain. Evidence: target/organic-turn-tests-out.log, target/organic-turn-run-out.log, target/organic-turn-err.log, target/organic-turn-dragon-turn-*.png and target/organic-turn-dragon-fire.png. This probe does not test ray tracing or generated frames.

Installed version: 20261023-220119-d475b0cf56. SHA256: D475B0CF5656A5D872EE9E9143FAF1176402AD10BE3BFCD190EDD303C4633F8A. Launcher integrity check passed. Restart Falling Heads - Play to use the new animation.

## 2026-10-23 — replace outlined block fire with transient breath wash

Supersedes the earlier block-edge fire presentation. Dragon breath now drives a separate per-frame impact surface map, sampled across nine cone rays that stop at the first solid block. Hit faces receive a world-space flowing flame wash; neighboring heated faces share the pattern, occupied neighbors mask spill, and boundaries beside unheated blocks erode softly. The intense wash exists only during contact and dissipates in place within a fraction of a second. No flame mesh shrinks during expiry or owner destruction.

GroundFire remains separate gameplay state for combustible blocks only, retaining damage, spread, water/rain extinguishing and a weaker lingering visual. Direct breath impact is visual-only except for the existing independently timed breath damage/ignition logic. Stone no longer receives the five-second lingering fire. Materials are removed when their transient visual expires.

Validation: 93 tests passed in target/fire-wash-tests-out.log, including first-surface occlusion, neighbor exposure, visual/gameplay separation, fixed transforms during fade, destroyed-owner cleanup and material cleanup over repeated impacts. Final build succeeded with 128 warnings. The focused connected-wall probe passed burst-on/release/off checks and captured target/fire-wash-final-fire-*.png. The final full renderer probe passed (target/fire-wash-dragon-run-out.log, target/fire-wash-dragon-err.log); inspected the actual dragon breath capture at target/fire-wash-dragon-dragon-fire.png. Both render runs exited 0 with clean Streamline shutdown and no reported shader/runtime errors. Focused scene uses native raster; full probe includes native and DLSS modes. Ray tracing and generated frames were not covered in this validation.

Installed version: 20261023-221635-8fdf004fc1. SHA256: 8FDF004FC11DD8413DE4FFEC3B31CD3C9A5657CC39F4FC9BCDB8168F47D575CC. Publisher and launcher integrity checks passed. Restart Falling Heads - Play to load the update.

## 2026-10-23 — biome atmosphere and supported plants; next-feature designs

Installed version: 20261023-223206-44c67594d6.
SHA256: 44C67594D6FC8069BD691AAD628F54712B02B096263262764D6025A44BFFB7AA.

Eight biome-specific lighting profiles now control sun, ambient fill, backdrop, distance fog and environment-map lighting. Nether/End/Underground illumination does not brighten with the outdoor day. Forest has subdued green fill and world-anchored irregular mist behind foreground blocks. Mist has bounded resident tiles/materials and no camera-snapping transforms or periodic global on/off switch.

Vegetation no longer randomly replaces solid terrain. Bushes and mushrooms require a supported three-wide/two-high cave opening; cactus clearance includes the complete stalk and headroom. Mushroom geometry has a stem and stepped cap. New placement applies to generated chunks, not existing entities in a previously running build.

Validation:
- cargo test --offline: 95 passed, zero failed (target/biome-atmosphere-tests-out.log). New tests cover constant enclosed-biome day/night profiles, plant support/clearance and exclusion from random solid-wall replacement.
- Final build passed with 130 warnings (target/visual-build-err.log). Existing mass/inertia and empty-mesh warnings appeared in the probe; no reported shader validation errors, runtime ERRORs or panics.
- Hidden native/RT atmosphere probe exited 0 with clean Streamline shutdown (target/biome-atmosphere-final-run-out.log, target/biome-atmosphere-final-err.log). Inspected fresh Nether day/night and forest mist/clear/RT captures. Forest screenshots contain naturally generated mushrooms on open ledges.
- An initial fog-off screenshot coincided with switching to RT and captured a transient black world. Separated asynchronous capture requests from setting changes by one second and reran the full comparison; the final fog-off capture renders normally. This does not claim mode switches have no transient blank frames.
- Frame generation is disabled in this probe; it does not validate FG or extended performance/balance. Mist is a layered shader effect, not volumetric light scattering.
- Publisher verified 306 assets, executable and nine runtime DLLs; launcher ResolveOnly passed. No user's running game was stopped or relaunched.

Per the user's choice, biome structures and dragon background breach are designed only in BIOME_DEVELOPMENT_PLAN.md. The rectangular fire presentation remains unresolved and is explicitly listed for a separate visual revision. Restart Falling Heads - Play to load this update.

## 2026-10-23 — replace blanket fog with actual local forest banks

Installed: 20261023-225556-295b51c26d.
SHA256: 295B51C26DD65FDC52EAF4ED0D63181C6E26B3A89EEE412A2685D50642396FE6.

The user rejected the scene-wide fog. Inspected official Gothic Remake forest/camp/cave screenshots (linked in BIOME_DEVELOPMENT_PLAN.md). They show clear foreground contrast, local pale ground haze and softened distance; they do not establish the precise shader implementation.

Correction to the previous release note: BiomeMistPlugin had never been registered. The earlier mist shader was not rendering, so the previous statement that it was visually validated was incorrect. The visible haze was DistanceFog across the shallow play plane. Removed camera DistanceFog and registered the new local mist plugin. The runtime probe now asserts that real mist entities exist, have bounded dimensions/count, and no camera DistanceFog remains. Two earlier probe attempts correctly failed this new check before registration was fixed.

Fog now appears in sparse forest pockets above actual floors, allows foliage and uneven edges, and requires open air above. Stable seed-based placement, minimum twelve-tile center separation, at most six banks inside a 2200-unit radius, 448–576 by 112–160 unit bounds. Banks sit behind solid foreground, ease opacity at fixed size, and clean up materials when retiring. The haze toggle hides them. Other biomes have no generic fog banks; their lighting palettes remain. Final density uses smooth warped waves in small local coordinates; replaced the visibly banded hash-noise iteration after inspecting captures.

Validation: 96 unit tests passed (target/local-fog-tests-out.log). Final code build succeeded with 123 warnings. Final shader/renderer probe exited 0, no reported shader/runtime errors or panics, clean Streamline shutdown (target/local-fog-release-run-out.log, target/local-fog-release-err.log). Inspected fresh native fog-on/off and RT images in target/local-fog-release-forest-*.png: local soft banks, clear surrounding air and successful toggle. Existing physics mass/inertia warnings remain. Frame generation and long-session performance were not covered. This is local ground mist, not full volumetric lighting or a distant 3D forest.

Publisher verified 306 assets, executable and nine runtime DLLs; launcher ResolveOnly passed. No user's game was stopped. Restart Falling Heads - Play for this correction.

## Forest trees and reduced glare — verified build 20261023-231629-8867c85af2

Implemented oak, birch and spruce with matching vanilla 16x16 bark, end-grain and cutout leaves. TreeSpecies is separate from Wood/Leaves gameplay types, preserving damage, fire and sound behavior. Shapes vary by seed and use a front-facing block-tree silhouette with foliage covering the upper trunk. Textures were extracted from the SHA-1-verified official Mojang 1.21.5 client; provenance is recorded in assets/textures/blocks/forest/species/source.json. Existing oak assets elsewhere remain unchanged.

Forest terrain is soil with grass-capped exposed surfaces, replacing the melon walls. Trees choose existing openings over intact three-cell footings and reserve stepped crown clearance. If necessary, at most 12 cells are cleared to fit the crown; preferred candidates minimize this excavation. Whole trees and clearances remain in one owning chunk, avoid spawn/portal areas, and keep enemy spawns and generated springs out. Placement tests sampled 525 forest chunks across three seeds and found 28 oak, 31 birch and 16 spruce sites. Trees generate mature; timed sapling growth and new inventory wood categories are not part of this pass.

Fixed update_block_visuals overwriting species and grassy-ground materials with the generic BlockType material on spawn/damage. It now supplies a default only when a material is missing. Added a regression test for custom shared-material preservation after damage. The first visual iteration exposed this integration issue and is not the accepted result.

The backdrop now has full roughness and zero specular reflectance; terrain reflectance is reduced. This removes the broad upper-left highlight while preserving biome-specific sun and fill colors. Fog support excludes wood and leaves, so tree crowns do not generate floating clouds.

Validation: cargo test --offline passed all 99 tests. Final hidden tree probe passed natural generation, material-handle identity for all three species, native rendering and RT rendering. Final biome probe passed Nether day/night, forest mist on/off, and RT environment. Both exited 0 with clean Streamline shutdown and no rendering validation errors. All eight final captures were inspected before local publication. Probes retain live combat/liquids, so arrows can destroy leaves and water can run over crowns during comparisons. Existing build warnings and Avian mass/inertia warnings remain; frame generation was disabled for these comparisons.

Accepted evidence:
- target/forest-trees-final-tests-out.log
- target/forest-trees-final-run-out.log and target/forest-trees-final-err.log
- target/forest-trees-final-trees-natural.png
- target/forest-trees-final-trees-species.png
- target/forest-trees-final-trees-rt.png
- target/forest-lighting-final-run-out.log and target/forest-lighting-final-err.log
- target/forest-lighting-final-{nether-day,nether-night,forest-mist,forest-clear,forest-rt}.png

Installed locally through Publish-FallingHeads.ps1, with 319 verified assets and 9 verified runtime DLLs. Launcher resolution verified:
- Build: 20261023-231629-8867c85af2
- SHA-256: 8867C85AF23C8FE0F2D7E1CB1BD33472BC99D71686FD93310FDBB0066973F6D7
- Executable: builds/20261023-231629-8867c85af2/FallingHeads.exe
- Restart Falling Heads - Play to use it. No existing user game process was stopped or relaunched.

Structures and the dragon background breach remain design-only proposals. The previously reported rectangular dragon-fire effect is still a separate unresolved visual issue.

## Softer upper-left glow restored — 20261023-232813-64c58744c1

User clarified that the upper-left light should be reduced, not removed. Restored the backdrop's directional sheen using shared BACKDROP_ROUGHNESS=0.62 and BACKDROP_REFLECTANCE=0.30 for both initial creation and per-frame lighting. This supersedes the previous full-roughness/zero-reflectance correction. Terrain reflectance and biome sun/fill settings remain as before.

Built offline and visually checked the real hidden forest probe in native and ray-traced rendering. Inspected all three captures: target/soft-corner-glow-trees-natural.png, target/soft-corner-glow-trees-species.png, and target/soft-corner-glow-trees-rt.png. The upper-left gradient is visible again with a softer falloff and readable background texture. Probe exit 0, completion assertions passed, clean Streamline shutdown, no rendering validation errors. Existing compiler and Avian warnings remain. This material-value adjustment was verified by build and render checks; the full unit suite was not rerun.

Installed locally and verified launcher resolution. Build 20261023-232813-64c58744c1; SHA-256 64C58744C1D7DFF74D669AD3BF4F5D474102537C98170A37E67CB9311F885C25. All 319 assets and 9 runtime DLLs verified. Restart Falling Heads - Play to use the update; no running user game was stopped.

## Forest density, precipitation lighting, and intermittent RT patches (completed)

- Forest bands now span 20 chunks. Other bands retain 10 chunks; the authoritative biome mapping uses a repeating, negative-coordinate-safe sequence. Biome teleport targets enter the upper third so falling leaves most of the destination ahead.
- Forest terrain uses rolling soil shelves and staggered descending gullies. Tree plans reserve grounded clearings for up to two spaced trees per chunk; oak, birch, and spruce selection is per tree. The three-seed, 525-chunk placement sample produces 249 oak, 234 birch, and 217 spruce trees, compared with 75 trees in the earlier forest sample. Footing, clearance, spawn/portal exclusions, and chunk ownership checks pass.
- Rain reduces the key light by at most 16%, snow by 8% (previously both 55%). Diffuse fill is preserved or slightly increased. Snow no longer applies wet material roughness; precipitation no longer darkens the backdrop tint.
- Intermittent RT patches were reproduced in sampled sequences with frame generation OFF. An early isolated screenshot after toggling FG was not sufficient evidence of FG causation. Local-light removal, ReSTIR, single-bounce tracing, and sun removal did not eliminate the original issue.
- A runtime A/B probe increased Solari's ray epsilon from 0.001 to 0.064 (64 world units per block); the large blotches disappeared in the inspected corrected sequences and returned when restoring 0.001. Bevy Solari 0.20.0 is now vendored with this one shader calibration and its original licenses. Camera cuts also reset Solari lighting history.
- Evidence so far: target/forest-weather-pass1-forest-density-{clear,snow,rain}.png, target/rt-flicker-before/, target/rt-flicker-bias/. Metric masks from the first probe include camera movement and are screening aids, not definitive pass/fail pixel counts.

Final validation: all 101 unit tests passed (target/forest-weather-final-tests-out.log). The final executable passed two continuous RT probes at large positive and negative world coordinates: 255 and 256 captures respectively in target/rt-verified-positive/ and target/rt-verified-negative/. All 511 frames were inspected in contact sheets, with suspect frames inspected at full resolution. No large intermittent black patches appeared in these checks. Stages cover stationary/moving cameras, Quality/DLAA, and toggling the FG setting; hidden windows do not validate actual interpolated display frames. Both probes exited cleanly without renderer validation errors and with successful Streamline shutdown. Existing compiler and physics warnings remain. The reproducible runner is tools/Test-FallingHeadsRayTracing.ps1.

Installed build: 20261024-000502-be133d8229. SHA-256: BE133D8229DAE2DBE633D3A3540284E129F7A92DEF060468DD2368849AB7CDEC. Published 319 assets and nine runtime DLLs, including the vendored Solari license notices. Active manifest and launcher resolution verified. Restart Falling Heads - Play to use the update. No existing user game was stopped or relaunched. Structures and the dragon breach remain design-only.

## Camera-aware torch coverage and DLSS 4.5 Performance — 24 October

Installed build 20261024-002148-41463cdb74 (SHA-256 41463CDB746CE7FDDCFB989B0300360F407C53A9E23CAFD84855799CE4C12EFD). Replaced the nearest-12-to-hero light assignment with up to 128 stable source assignments across the actual camera footprint, including offscreen light reach and smooth retirement. Torch lifetime and lava light sampling follow this footprint, so wide views no longer leave visible torches unlit merely because the hero is far away. Existing 440-unit physical light reach is unchanged.

Added an optional persisted DLSS 4.5 Performance setting for raster Super Resolution. Quality remains default. Ray Reconstruction retains its own Quality/DLAA behavior; the UI explains that 4.5 Performance requires DLSS ON and RT OFF. Official DLSS 5 exists, but no public NR SDK/runtime was found in NVIDIA's public developer downloads; no fake 4.5/5 switch or leaked/injected DLLs were installed. Research and primary sources are in MODLOG.md.

103 tests passed. Six fresh scene captures and the graphics-settings capture inspected; native, wide/standard viewport sizes, camera motion, SR Performance and RT all passed the gallery's visible-light assertions. Probe exited 0 with successful Streamline shutdown. 319 assets and nine runtime DLLs verified by publisher, active launcher resolution confirmed. Restart Falling Heads - Play. Witch, biome landmarks, dragon background breach and egg reward are still proposals, not implemented features.
