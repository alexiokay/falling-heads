# MineHeads performance notes — 23 October 2026

## Measurements and limits

Hidden 720×1280 fluid/torch scene, raster DLSS Quality, FG disabled, shadows enabled, no render FPS cap. Samples use real frame deltas after warm-up; they are not controlled GPU-clock benchmarks.

| Run | Clear median / p95 ms | Snow median / p95 ms | Live particles | Meshes / materials |
|---|---|---|---|---|
| Baseline | 12.135 / 13.590 | 12.214 / 13.257 | 180 | 234 / 258 |
| Shared weather assets | 11.552 / 13.040 | 11.690 / 13.159 | 178 | 43 / 67 |
| Later dragon-assets build | 12.675 / 13.770 | 12.875 / 14.508 | 178 | 49 / 75 |
| Restored backdrop, empty-mesh update guard | 10.769 / 12.143 | 11.694 / 12.988 | 178 | 52 / 78 |

Asset reduction is repeatable; timing varies between runs. The first comparison improved median snow time by about 4%, but the later run was slower overall. This does not establish a reliable FPS gain or reproduce the user's large snowfall drop. Do not mark that report solved. Test a heavier representative scene at the user's output resolution and separate CPU simulation, render preparation, GPU transparency and ray-tracing cost.

## Existing and newly applied work

- Terrain/background materials and meshes are cached, chunks stream around the player, and engine visibility culls ordinary meshes.
- Nearby local lights use a bounded pool; secondary-ray scene representation is restricted to supported opaque geometry.
- Shared weather mesh/material handles replace per-particle asset allocations. Random size is preserved with transform scale. Particle count, textures, color and lifetime are unchanged.
- Unchanged lighting material values are not rewritten, and unrelated settings no longer rebuild the SR/RR configuration. Renderer-mode inspection now also uses immutable iteration: Bevy Assets::iter_mut emitted Modified for every material each frame even when no fields changed. Only actual method switches now trigger GPU material re-preparation. This additional optimization was discovered after the measurements above; final results are recorded separately below.
- Empty fluid geometry is not repeatedly submitted to the mesh allocator. Fluid geometry still needs a dedicated visual-change revision to avoid rebuilding after occupancy-only changes.
- Native-resolution UI is composited separately. DLSS SR reduces world rendering resolution when selected; this is an optional reconstruction tradeoff, not a substitute for CPU optimization.
- Frame Generation increases presented frames, not simulation speed. It is not used when measuring rendered frame time above.

## Next profiling targets without reducing visual quality

1. Measure system spans and GPU passes in the same reproducible crowded scene. Identify the limiting stage before selecting an optimization.
2. Rebuild fluid geometry only when visible fluid/steam state or loaded chunks change; reuse scratch buffers.
3. Update snow-cap terrain data on block changes instead of scanning every block each frame.
4. Check draw batching for shared weather assets; alpha sorting can still split otherwise compatible draws.
5. Inspect transparent overdraw. Tight particle geometry can avoid shading invisible texture margins without reducing the visible effect. Low-resolution particle rendering is a separate quality tradeoff and has not been enabled.
6. Consider pooled/GPU particle simulation only if CPU spawn/update costs are measured as a bottleneck. LOD, occlusion culling, virtual textures and VRS are not automatically improvements for this side-view scene.

## Primary references

- [Bevy profiling](https://github.com/bevyengine/bevy/blob/main/docs/profiling.md): CPU system spans and GPU profiling guidance.
- [Bevy instancing](https://bevy.org/examples/shaders/custom-shader-instancing/): shared mesh/material handles permit built-in batching and instancing.
- [Epic VFX optimization guide](https://dev.epicgames.com/documentation/en-us/unreal-engine/vfx-optimization-guide?application_version=4.27): distinguish overdraw, tick cost and render-thread draw preparation.
- [NVIDIA particle rendering](https://developer.nvidia.com/gpugems/gpugems3/part-iv-image-effects/chapter-23-high-speed-screen-particles): transparent overdraw and the artifacts/tradeoffs of lower-resolution particle buffers.


## Final material-update correction result

`target/visual-release-err.log`, published build `20261023-195826-06c27aa322`:

| Weather | Frames sampled | Median ms | p95 ms |
|---|---:|---:|---:|
| Clear | 1507 | 5.261 | 5.897 |
| Snow | 1811 | 5.460 | 6.141 |

The snow sample had 180 live particles, 63 meshes and 90 materials. Compared with the original 12.214 ms snow median, the final 5.460 ms is about 55% lower frame time in this fixture. FG was off and particle appearance/count was retained. Natural scene contents and GPU clocks vary, so treat this as strong evidence of removed overhead rather than a universal FPS guarantee. The final renderer regression confirms stable frames emit no material Modified events, and an RT switch updates only the eligible lit materials.


## Crowd work — 24 October 2026

Code audit found that each heart had its own mesh/material, every actor scanned all hearts to check whether its hearts existed, and every heart material was rewritten every frame. Villager spawning also allocated a face mesh/material per actor. Healthy crowd rendering now shares geometry and immutable materials; hit faces temporarily fork a private material and rejoin the shared material when the flash ends. A test with 100 villagers verifies 300 hearts use one mesh and two materials, including damage/healing visibility changes. The villager face adds one shared mesh/material.

Terrain collision candidate enumeration now borrows an iterator instead of allocating a vector for each actor. Crowd stall recovery uses the existing terrain spatial hash. Bats previously ran a full terrain line-of-sight scan for multiple possible targets; they now select the nearest candidate first, gather terrain in its local rectangle, and use the same nearby set for avoidance.

Primary engine references reviewed for this request:
- [Unity 6.3 batching](https://docs.unity.com/en-us/engine/6000.3/manual/analysis/graphics-performance-profiling/reduce-draw-calls/draw-call-batching/draw-call-batching): compatible shared meshes/materials reduce render-state work; CPU dynamic batching is not universally beneficial.
- [Unreal MassGameplay](https://dev.epicgames.com/documentation/unreal-engine/overview-of-mass-gameplay-in-unreal-engine?lang=en-US): data-oriented processing, instanced representations, visibility relevance and variable simulation frequency. Simulation LOD is a future option for distant actors; this change keeps the on-screen crowd's physics active.

Measurement protocol: separate hidden process; i7-13700K / RTX 5070 Ti; 1360x900; Frame Generation and frame cap off; production villager spawning; enclosed collision fixture; five-second warm-up and eight-second sampling windows at 10 and 100 villagers; runtime assertions require every villager to be visible. A final 100-villager phase enables Solari/Ray Reconstruction. Random events are reset to avoid changing the requested count. Timings are real frame intervals, not isolated GPU timings.

Discarded exploratory runs: the first fixture omitted terrain membership, allowing actors to fall away; a later run was interrupted by a random event adding three villagers. Neither establishes a valid before/after FPS improvement. Accepted measurements are appended after the corrected probe passes.


### Accepted crowd run (target/crowd-2560-accepted.log)

Executable before the final structure-only cosmetic finish pass, 2560x1440 capture verified, RTX 5070 Ti / i7-13700K. Clear weather, FG off, no FPS cap. Raster uses native resolution; RT uses Solari with DLSS Ray Reconstruction Quality. Five seconds warm-up and eight seconds sampling per phase. All 100 actors remain within the camera viewport; the enclosure has a ceiling to prevent physics launching actors outside the sampled view. No compiler or second verification game ran during sampling.

| Phase | Actors visible | Frames | Median ms | p95 ms | Approx. median FPS |
|---|---:|---:|---:|---:|---:|
| Raster | 10/10 | 1334 | 5.203 | 13.475 | 192 |
| Raster | 100/100 | 1445 | 5.328 | 6.948 | 188 |
| RT / RR Quality | 100/100 | 543 | 15.051 | 18.707 | 66 |
| RT / RR Quality | 10/10 | 671 | 11.936 | 14.320 | 84 |

The raster crowd adds little median cost in this scene. RT remains substantially more expensive and has a larger crowd-related difference. This identifies the next profiling direction, but the timings do not separate GPU work from render preparation and simulation. There is no accepted same-scene pre-change baseline, so these results do not establish a percentage improvement. Mesh/material totals changed only from 52/201 to 53/202 between the first 10 and 100 actor phases, rather than hundreds of face/heart assets. Four fresh screenshots and clean process/Streamline shutdown were verified. Existing engine warnings remain.

Further rejected exploratory run: an open-top enclosure allowed one actor off screen. That run failed the 100-visible assertion and was not accepted. The corrected enclosed run above passed all four visibility/population assertions.


## Terrain sweep validation — 26 October 2026

The terrain handler now runs on the physics clock and queries a swept rectangle through the same spatial index. Local candidate/contact buffers are retained; it does not scan all world blocks per actor or increase global Avian substeps. This change addresses collision correctness.

Final full-size (2560x1440) native crowd fixture, frame generation off: 100/100 villagers visible; raster median 4.583 ms, p95 5.294 ms (1,681 samples); RT/RR median 11.363 ms, p95 12.658 ms (703 samples). Ten-villager controls: raster median 4.426 ms, RT median 11.543 ms. Source: target/crowd-run-err.log. These are current fixture measurements, not a controlled before/after speedup claim or a guarantee for every combat/weather scene.


## Collision cost clarification — 26 October 2026

For the terrain pipeline, let B be loaded terrain blocks, H simulated non-flying heads and K nearby block candidates per head. In ordinary short fixed-step movement, cost is approximately O(B + H*K): the Changed/Added query used to decide whether to rebuild the index itself has a linear worst-case scan even when it finds nothing; an actual rebuild is also O(B). The per-head broad phase enumerates intersected spatial bins and sweeps only their candidates. Swept movement has a bounded number of contact iterations; exceptional initial-overlap recovery can cost O(K^2) per embedded actor. Very long paths enumerate more spatial bins. Loaded chunk lookups are bounded by the streaming radius. This is an account of terrain collision, not a single complexity claim for all Avian actor interactions, rendering, fluids or the entire game.

The potion pass now uses the same local index for terrain instead of scanning every world block for each segment. It still tests friendly actor targets, and longer-lasting clouds increase the number of active hazards. No new whole-game FPS speedup is claimed for this release.


## Packed 370-villager comparison — 27 October 2026

Two native runs, 2560×1440, raster, DLSS/RT/FG off, clear weather, uncapped rendering; 370 live production villagers, all verified inside the viewport. Identical initial positions and alternating movement directions; original rounded-cuboid phase then sphere phase. Normal terrain sweep remains square in both. The test arena deliberately holds a crowded group and uses durable walls; it does not reproduce mass terrain destruction, combat, names, long sessions or the user's entire world. Natural fluid/world state and simulation divergence are not reset between phases. Fixed order and normal desktop variability limit conclusions about small frame-time differences.

| Run / contact shape | Frame median / p95 ms | Contact detection median ms | Selected solver stages median ms |
|---|---:|---:|---:|
| 1 / original rounded cuboids | 5.538 / 7.687 | 0.353 | 0.828 |
| 1 / spheres | 5.393 / 6.725 | 0.146 | 0.540 |
| 2 / original rounded cuboids | 4.958 / 6.852 | 0.333 | 0.770 |
| 2 / spheres | 4.982 / 6.323 | 0.130 | 0.487 |

Contact detection decreased about 59–61%, selected solver stages about 35–37%, and frame p95 about 8–13%. Average frame time did not consistently improve (run 2 was slightly worse); do not claim a general FPS improvement or that the user's lag is resolved. These are last-physics-tick diagnostic snapshots sampled alongside frames, not additive whole-frame CPU totals. Solver sample includes preparation, constraints, relaxation and restitution, not all solver stages. All heads/heart assets stay enabled; no actor count cap, visibility suppression or skipped terrain collision was introduced. Crowded head contact becomes spherical, changing diagonal contact/corner overlap, while sprite sizes, normal bounce and square swept terrain bounds remain intact.

Logs: `target/crowd-dense-first-run.log`, `target/crowd-run-err.log`; four fresh captures per run. Shared mesh/material counts stayed approximately 55–57 / 218–220 rather than allocating per villager. The separate fluid occupancy fix eliminates per-render-frame mesh regeneration caused only by head occupancy changes, but its standalone FPS effect was not isolated in this comparison.

## Architecture refactor: named crowd — 27 October 2026

The crowd fixture now includes actual username labels and asserts the health-row entity count. 370 healthy villagers have 370 shared-mesh rows rather than 1,110 individual heart entities, with the same number of visible heart quads. Names, physical contacts and terrain sweeps remain active. Mass owner removal and health changes are covered by automated tests.

Default application optimization (opt-level=1, dependencies=3), 2560×1440, raster, RT/DLSS/FG off, no concurrent compiler or verification game during sampling:

| Contact shape | Visible villagers | Heart rows / labels including hero | Median / p95 frame ms | Contact median ms | Selected solver median ms |
|---|---:|---:|---:|---:|---:|
| Rounded cuboid control | 370 | 370 / 371 | 4.748 / 6.597 | 0.330 | 0.742 |
| Sphere crowd contacts | 370 | 370 / 371 | 4.697 / 6.216 | 0.147 | 0.519 |

Source: `target/architecture-crowd-opt1.log`; four fresh captures and clean shutdown. The old unnamed fixture is not a controlled before/after baseline for this refactor. Fixed phase order, naturally diverging simulation and desktop variability still limit conclusions. These are fixture measurements, not a claim that severe lag in the user's complete world is resolved.


### Sphere experiment withdrawn

The user rejected the change in diagonal crowd packing on 27 October. Production now uses the original rounded cuboids again; the sphere override and benchmark resource are removed. Earlier sphere numbers remain historical evidence, not the shipped default or a claimed architecture improvement. In the default-build named fixture, 4.748 versus 4.697 ms is approximately 210.6 versus 212.9 FPS from medians: about 2.3 FPS, within ordinary run variability. A second optimized-application run gave 4.872 versus 4.716 ms, also too small and uncontrolled to establish a universal gain. Faster isolated contact detection was real, but did not justify the visual/behavioral tradeoff.

The opt-level=3 candidate did not show a reliable whole-frame improvement over opt-level=1 in this workload. Both retain optimized dependencies and crash symbols. No percentage speedup from the compiler setting is claimed.

The crowd probe accepts `-Dense -Named -Count 2000`. Counts above 370 use a larger physical arena and wider test-camera field of view, retaining full-size actors and simulation. All actors must project inside the viewport. This is a scale workload, not evidence that 2,000 individually readable heads fit at the regular camera zoom or that arbitrary pile density is cheap.


### Final restored-contact build: 370 and 2,000 heads

Binary SHA-256 `B6E29650ED51A09BEFE8DE2757178EA89A9416C50BB1C2EC481562512A1546C0`, application opt-level=3 with PDBs; RTX 5070 Ti / i7-13700K. Native 2560×1440 raster, RT/DLSS/FG off, clear weather, uncapped rendering. No compiler or second game ran during sampling. Both measured phases use the restored rounded cuboids; they are repeated spawns, not a collider comparison.

| Workload / sample | Frame samples | Median ms | p95 ms | Contact median ms | Selected solver median ms |
|---|---:|---:|---:|---:|---:|
| 370, normal camera, 1 | 1,539 | 4.766 | 6.558 | 0.350 | 0.800 |
| 370, normal camera, 2 | 1,524 | 4.805 | 6.514 | 0.341 | 0.794 |
| 2,000, wider test camera, 1 | 992 | 8.671 | 10.365 | 0.601 | 1.427 |
| 2,000, wider test camera, 2 | 974 | 8.849 | 10.432 | 0.592 | 1.437 |

All 370 / 2,000 actor centers remained inside the viewport; all corresponding heart rows and 371 / 2,001 username labels (including hero) were present. The 2,000-head fixture uses a 56×36-block arena and wider camera field of view. It has a different physical density from the tighter 370-head fixture; do not extrapolate these as identical-density scaling. Names naturally overlap at this zoom; none are hidden for speed. The 2,000-head median corresponds to approximately 113–115 FPS on this machine. This demonstrates the restored contacts can run this workload; it does not establish that the cleanup caused that speed, or guarantee full-world combat/weather/RT performance or arbitrarily dense piles.

Logs: `target/architecture-crowd-370-restored.log` and `target/architecture-crowd-2000-restored.log`. Both produced four fresh captures and clean shutdowns. The first 10-head warm-up control had a 12–17 ms p95, while the ending control was about 4.8–5.1 ms; startup/streaming work is another reason not to infer a universal speedup from small differences.
