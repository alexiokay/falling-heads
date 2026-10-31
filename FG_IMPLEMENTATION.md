# NVIDIA Frame Generation — implementation and verification

NVIDIA 2× Frame Generation was published as `20261023-180629-5432528f69`. The correction batch is now published as `20261023-195826-06c27aa322`. The authoritative installed version is `builds/active.json`.

## Controls and scope

Settings → Graphics → NVIDIA Frame Generation (2×). The preference is saved and defaults off; older settings retain all existing choices. Turn VSync off to use FG. The status explains unsupported hardware, focus requirements, suspension or runtime errors. The FPS display distinguishes rendered and presented frames.

One generated frame is inserted between rendered frames. Higher Multi Frame Generation multipliers are not exposed or claimed. Frame generation does not change simulation time or fix CPU simulation bottlenecks. Menus/teleport suspend generation; resize/camera cuts reset history. Reflex pacing and markers are integrated.

## Completed integration

- [x] Signed production Streamline 2.14.16 Vulkan loader, startup capability checks, runtime DLL staging and licenses.
- [x] Local wgpu-hal 30.0.1 interposer/present/device-lifetime hooks; no shared registry modifications.
- [x] Frame tokens, Reflex pacing/markers, depth/motion/HUD-free colour snapshots and camera constants.
- [x] Settings/status/FPS display and suspension on pause/resize/teleport.
- [x] SR/RR/native renderer switching, compositor resizing and shared NGX shutdown ownership through local dlss_wgpu 6.0.0 patch.
- [x] Focused native generation verification, clean device shutdown and immutable release publication.

## Verification evidence

- `target/fg-run9-err.log`: focused visible test, NVIDIA actual=2, 2368 presented / 1459 rendered = 909 generated presents. Pause, resize, SR/RR/native switches and FG off passed. Exit 0 and slShutdown result=0.
- `target/fg-final-err.log` / `target/fg-package-err.log`: hidden transition/package checks, clean shutdown. Hidden tests cannot establish active generation because NVIDIA suspends it without window focus.
- `target/release82-tests-out.log`: 82 regression tests passed, including real-time Avian movement at 30/60/90/120/144 FPS, settings migration, shallow-water freezing and ground-fire damage/spread.
- Earlier failed runs are retained as debugging evidence, not passing validation: a resize handle lifetime issue, shared NGX shutdown result24 and same-frame capture/toggle ordering were fixed before publication.

## Current follow-up batch

- [x] More dragon armor/scales/horns/teeth/wing ribs/claws/tail spines; distinct joint motion for flight, perch and breath.
- [x] Ground-fire regression proves wood takes damage and is destroyed, fire spreads to leaves, and nearby stone is not ignited. Head burning unchanged at user's request.
- [x] DLSS torch-cluster resolution correction in local Bevy PBR 0.20.0 patch; SR/native captures show the lights.
- [x] Restore previous background appearance after user rejected the brighter matte proposal.
- [x] Shadows on by default; preserve saved preferences.
- [x] Shared snow particle mesh/material resources, unchanged visible count/texture/motion.
- [x] Snowball shallow-water freeze and volume-preserving thaw; occupied/falling-cell protections retained.
- [x] Steam/water excluded from rectangular shadow casting; early daylight reaction capture shows soft steam without dark square silhouettes.
- [x] Final restored-background visual check, shadow toggle, FG/SR/RR regression and immutable publication.
- [ ] Reproduce the larger reported snowfall FPS drop in a representative heavier scene. Asset allocations improved, but repeated timings vary; see PERFORMANCE_NOTES.md.

## Remaining scope

Higher Multi Frame Generation modes, persistent world modifications and the broader biome expansion remain future work. This file does not certify every historical feature proposal as implemented. The user accepted current burning; do not retune it incidentally. Source assets, settings and per-run logs are retained so work can resume after chat errors.


### Final visual validation

`target/visual-restored3-*.png`: inspected daylight steam/obsidian, restored night backdrop, SR/native torch placement, snow and detailed dragon with continuous breath. Process exited 0 and Streamline shut down successfully. Empty meshes now retain a valid vertex layout; an extra zero-length color attribute was removed after the native test caught it truncating liquid geometry. The mesh-layout regression covers attribute lengths. The test helper also rejects rendering errors and stale captures rather than treating process exit 0 alone as success.

Existing entity-despawn race warnings and debris mass-property warnings remain visible in the native log. No rendering validation error occurred in this final visual run. Shadow-map re-enabling is now corrected: the game recreates its owned sun when returning to shadow maps and preserves the custom 900–2600-unit cascade range. `target/shadow-cascades-sr.png`, `-no-ao.png` and `-no-shadows.png` are the dedicated on/off/on comparison (historical capture suffixes); projected shadows return in the third image. Two unsuccessful approaches were removed before this check.


### Material upload optimization

A read-only renderer-mode inspection now replaces `Assets::iter_mut()`. The latter emits Modified for every material merely by iterating, regardless of whether its fields change. That caused repeated GPU uploads and preparation even in stable scenes. The new code only gets mutable access for materials whose opaque render method changes. A regression exercises stable frames and verifies that switching to ray tracing updates lit surfaces while leaving unlit artwork alone. Final release tests passed (82/82). The stable-material regression confirms zero repeated asset events on unchanged frames.


## Published correction release

Version: `20261023-195826-06c27aa322`

Executable SHA256: `06C27AA322C3BBAB7AA0C08B3D3CDA238F555C4A71F60F4635B216DC6A14F53E`

- `target/visual-release-err.log`: complete comparison/profile/dragon run, exit 0, clean Streamline shutdown; captures inspected.
- `target/fg-release-verified-err.log`: pause, resize, SR/RR/native/FG-off transitions passed, exit 0 and slShutdown result=0. Hidden run correctly generated zero extra presents; focused 2× proof remains run9.
- `target/release82-tests-out.log`: 82 passed, 0 failed.
- Publisher verified the executable, all 305 assets, all 9 NVIDIA runtime DLL hashes and licenses. Desktop launcher ResolveOnly points to this exact release.
- User's running game was not closed or replaced; next Falling Heads - Play launch selects the update.
- Final snow fixture: 180 particles, median 5.460 ms, p95 6.141 ms (FG off), versus baseline median 12.214 ms / p95 13.257 ms. This is a measured scene result, not a promise for every level/GPU load.

Open: higher Multi Frame Generation modes, profiling the user's exact crowded snow scenario, persistent world changes and proposed biome expansion. Current burning behavior is preserved.


## Motion-artifact audit (2026-10-23)

User reports trails after movement with Frame Generation. Official Bevy 0.20 source and NVIDIA Streamline 2.14.16 guidance were compared with the actual producer/copy/tag path.

### Confirmed defect and correction

Bevy `prepare_prepass_textures` allocates at `ExtractedCamera.physical_target_size`, while the prepass viewport uses `MainPassResolutionOverride`. Bevy's DLSS node explicitly supplies that override as `partial_texture_size`. Our FG copy previously used `ViewPrepassTextures.size`, including the unused allocation area, then tagged it as a full motion/depth field. Consequently the motion/depth positions did not correspond to the full HUD-free output image when upscaling was enabled.

The FG copy now allocates, copies and tags only the active render region, keeping HUD-free color at output resolution. Native rendering retains the full size. Resizing either extent recreates snapshots and resets history. Abrupt camera translation, rotation and projection changes now reset FG history as well; ordinary follow motion and temporal jitter do not.

### Checks and limits

- Bevy UV motion convention and NVIDIA sign conversion agree: Bevy's SR scales by negative render dimensions for pixel vectors; FG uses negative one for normalized vectors. Jitter is negated as in Bevy's own DLSS node. These were not changed.
- Matrix serialization matches NVIDIA's Vulkan sample; reversed depth and unjittered camera reprojection are retained.
- Opaque/cutout head meshes and rigid dragon parts participate in Bevy's previous-transform motion prepass.
- Alpha-blended water, flame, fog and overlays do not write the standard prepass. Their motion remains an image-quality limitation to investigate separately; do not force them opaque or claim zero artifacts.
- HUD-free color excludes the separate presentation/UI camera, with same output extent and stored color bytes as the presented world image. No explicit UI alpha buffer is currently supplied.
- Existing focused run9 proves generation was active, not that temporal image quality was correct. The new hidden test can validate inputs/transitions, but cannot prove generated-frame visual quality because NVIDIA suspends FG when unfocused.

### Sources

- https://bevy.org/news/bevy-0-20/
- https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_anti_alias/src/dlss/node.rs
- https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_core_pipeline/src/core_3d/mod.rs
- https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_core_pipeline/src/prepass/node.rs
- https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_core_pipeline/src/prepass/mod.rs
- https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS_G.md
- https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuide.md

Validation: the two new temporal-input regressions pass (`target/fg-motion-tests-out.log`). Runtime transition check and publication completed; see subsequent entry for final results.


### Motion-input correction release

Published `20261023-202042-390bcb20b6`, SHA256 `390BCB20B6412E2E92AE2968BDF1EE663614820E1CE55CE24E2D53AFF90F2D8A`. Launcher resolve-only confirms the new immutable executable. No existing game session was closed or focused.

- `target/fg-motion-tests-out.log`: both new regression tests passed (82 unrelated tests filtered).
- `target/fg-motion-run-out.log`: hidden native runtime test exited 0.
- `target/fg-motion-audit-err.log`: active depth/motion regions were 360x640 for 720x1280 SR output, 400x550 for resized 800x1100 SR, 533x733 for RR, and 800x1100 for native. Pause/resize/resume/SR/RR/native/FG-off transitions passed with status/result 0 and clean shutdown.
- Fresh SR/native screenshots inspected; scene and torches render correctly. The hidden run generated zero extra frames as expected; it validates input plumbing and stability, not artifact-free interpolated frames. Focused moving-scene visual confirmation remains outstanding.


## Background flicker follow-up

User comparison: FG off mostly removes the flicker, but the background glare/lighting can still jump; FG on adds blur and shadow flicker. The earlier extent fix was necessary but did not establish artifact-free output.

Confirmed second source defect: `update_background` repositioned the entire grid at snapped camera coordinates and derived each tile's destination from ECS query iteration order. Initial spawn used a different loop ordering, and render attachment can change archetypes/order. The movement guard compared the camera center against an arbitrary tile's position rather than the previous camera center. Stationary background surfaces therefore acquired artificial transforms and motion history. This also affects DLSS without FG.

Correction: background tiles carry stable integer world coordinates. On movement/resize the system retains every in-range tile entity and transform, discards only tiles outside the padded rectangle and spawns missing edge cells. The cache compares actual grid center/dimensions. Odd dimensions now produce the full requested grid, and initial tiles are world-aligned. Background textures, lighting, materials and shadow quality are unchanged. The update is explicitly ordered after camera following.

`target/bg-stability-tests-out.log`: two ECS regression tests passed, exercising camera movement plus archetype reordering, retained entity/transform identity, idle frames, odd grid dimensions, resizing and a distant teleport. `tools/Test-FallingHeadsFG.ps1 -BackgroundMotion` additionally pans the real camera and asserts resident-tile positions every frame across rendering-mode switches. Runtime result and publication follow below.


### Background stability release result

Published `20261023-203905-6d1b03be29`, SHA256 `6D1B03BE297D97E08A121442EE0D8791D8D4BF1D558AE1DD98D082B06620816C`. Launcher resolved to the new immutable release. Existing game windows were untouched.

- Two background ECS regressions passed (`target/bg-stability-tests-out.log`).
- Hidden moving-camera renderer run passed: `target/bg-motion-run-out.log`, `target/fg-background-stable-err.log`, exit 0, status/result 0, clean Streamline shutdown. 14,380,590 retained entity-position comparisons succeeded across SR, resize, RR and native rendering. This is a repeated invariant count, not a count of distinct tiles or visual frames.
- Fresh SR/native captures inspected for backdrop coverage, terrain and torch rendering. Static captures and unfocused testing cannot certify flicker-free generated frames. User confirmation of glare/shadow stability remains outstanding; no incidental change was made to background color, reflectance or lighting.
