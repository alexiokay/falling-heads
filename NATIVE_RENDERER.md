# MineHeads native graphics

Bevy 0.20.0, Vulkan. This build uses the local SDK installation already used by Commonfield; no second SDK download is required. `.cargo/config.toml` points to `../../south-balcony-twin/vendor/{DLSS,Vulkan-Headers,clang-runtime/clang/native}`. Environment overrides may point to another installation. Commonfield's sources and binaries are not modified.

Build with `tools/Build-MineHeads.ps1`. It performs a locked Cargo build and stages `nvngx_dlss.dll` (SR) plus `nvngx_dlssd.dll` (Ray Reconstruction) beside the executable. The publisher copies and hashes both libraries into the immutable release. NVIDIA's licence is included under assets/licenses. `prettyplease` is locked to 0.2.37, matching Commonfield: bindgen 0.73.1 with prettyplease 0.3.0 mixed incompatible syn 2/3 types.

## Settings and actual rendering

Escape → Settings → Graphics tab:
- DLSS Quality upscaling ON: raster uses DLSS Super Resolution, with motion vectors and camera jitter.
- Ray tracing ON: lit opaque materials use Solari direct/indirect illumination and ray-traced shadows, denoised by DLSS Ray Reconstruction. Lighting must also be enabled.
- Ray tracing ON, DLSS upscaling OFF: RR runs at native resolution (DLAA quality mode); denoising remains required.
- Both OFF: normal native-resolution raster rendering; FXAA/SSAO may be used.

The status label reports the actual selected supported path, not only the saved request. Ray tracing replaces raster shadow maps/SSAO while active. NVIDIA Frame Generation is implemented separately through Streamline, with a saved 2× toggle and Reflex pacing. See FG_IMPLEMENTATION.md. This native build requests a Vulkan ray-query-capable device; it targets this computer's RTX 5070 Ti rather than promising compatibility with unsupported GPUs.

## Hybrid scene representation

Opaque lit StandardMaterial geometry gets an owned RT-only child with a validated position/normal/UV/tangent layout and u32 triangle indices. The child has no Mesh3d, so it does not duplicate visible drawing. Converted meshes are shared, updated on source changes and released when their source is removed; children follow transforms and parent despawn. The active RT scene is bounded around the hero, retaining offscreen nearby occluders. Material-handle changes replace the corresponding RT instance.

Lit opaque and alpha-tested StandardMaterials populate the primary deferred G-buffer. Unlit artwork retains its forward rendering path. This keeps the depth, albedo and normal guides consistent for Ray Reconstruction and prevents cutout pixels from referring to missing surface data. Alpha-tested foliage remains excluded from secondary-ray instances, so texture holes are not falsely treated as solid rectangles. Unlit or blended actor sprites, glass-like barriers and fluid/portal/flame custom materials remain forward rendered. The textured backdrop uses lit deferred rendering with low-level textured emission. Torch/lava point-light locations have bounded emissive sphere representations for Solari, which does not support point lights. These are small visible emissive cores with matching primary and secondary geometry; invisible opaque proxies previously produced reconstruction/shadow artifacts. Torch origins follow the flame, not the block centre. Emissive luminance converts PointLight lumens using the diffuse sphere surface area times PI, preserving power when its radius changes. Lava emitters sit inside the actual filled volume behind its opaque surface, avoiding visible light-source dots floating above lava. This is hybrid ray-traced lighting, not a claim that every transparent shader is ray traced or that water now has physical ray-traced refraction.

Teleport camera cuts reset DLSS temporal history. Switching rendering modes removes obsolete deferred/jitter/mip components; the HDR world renders into an independent image and a native LDR presentation camera draws it behind sharp text/UI. This replaces the old shared-target stack; both raster and DLSS use the same compositor.

## Validation

The opt-in hidden `--gpu-smoke-test` exercises SR → Solari/RR → native-resolution RR → native raster → SR, captures each mode and requires GPU diagnostic timings for actual Solari lighting/world-cache and RR execution. The final renderer, raster-effects, ray-traced-effects and UI runs passed; results and the earlier isolated driver-reset incident are recorded in MINEHEADS_UPGRADE_2026-10-23.md. The fluid regression covers constant airborne width at every row near a supported pool, independent of transient parcel volume.


## Encounter/lighting correction pass

The background now supplies a lit deferred surface and textured low-level emission, so its G-buffer agrees with the reconstructed image. Solari receives an explicit hemispherical environment light (day/night and storm adjusted); the raster-only GlobalAmbientLight is not a replacement for an environment light in this path. The source sphere and visible torch flame are aligned. Portal frames, portal effects and flame quads are excluded from shadow casting and secondary-ray attachment.

Bloom uses an additive 1.5 HDR threshold with 0.035 intensity. Ordinary terrain no longer contributes to bloom. Haze starts farther behind the gameplay plane. Optional drifting mist is a separate sparse alpha shader, not volumetric ray marching. Full transparent light transport remains outside this implementation.


## Local dependency patches

- `vendor/wgpu-hal` (30.0.1): Vulkan interposer loader, UNORM native swapchain selection, present markers and device-lifetime shutdown fallback.
- `vendor/dlss_wgpu` (6.0.0): shared NGX shutdown ownership; SR/RR features release before Streamline closes the shared SDK.
- `vendor/bevy_pbr` (0.20.0): clustered-light lookup factors honor MainPassResolutionOverride. Dividing fragment positions from an upscaled main pass by the full output resolution selected incorrect light clusters and clipped torch lighting.

These patches are scoped to this project through Cargo.toml; shared Cargo registry files and Commonfield are not changed.


Shadow-map re-enable recreates the owned directional sun with its custom CascadeShadowConfig preserved; default Bevy cascade distances do not reach the 1500-unit-distant play plane. The dedicated on/off/on native comparison verifies projected shadows return. Stable renderer-mode inspection uses immutable material iteration, avoiding the Modified events that Assets::iter_mut emits for every entry.
