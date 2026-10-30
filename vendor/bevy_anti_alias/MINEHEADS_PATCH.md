# MineHeads patch — Bevy anti-alias 0.20.0

Copied from the Cargo registry with upstream MIT/Apache licenses intact.

`Dlss` exposes `super_resolution_preset` (0 SDK default, 11 K, 13 M). Preparation translates it into internal `dlss_wgpu` context-key bits, so changing the model invalidates and recreates the cached feature even when render/output resolution stay identical. The matching vendored `dlss_wgpu` patch strips these bits from NGX feature flags and writes the six official SR render-preset hints under the SDK mutex before feature creation. Every creation writes the hints, including reset to zero, because the NGX parameter object is shared.

MineHeads sets K/M only on Super Resolution cameras. Ray Reconstruction retains its independent model. The quality enum continues to control render resolution independently. NVIDIA driver/App overrides may supersede these application hints.

Reference: https://raw.githubusercontent.com/NVIDIA/DLSS/main/include/nvsdk_ngx_defs.h and https://www.nvidia.com/en-us/geforce/forums/geforce-graphics-cards/5/580689/dlss-45-super-resolution-faq/

Regression coverage: `switching_renderer_cleans_temporal_components`, graphics settings migration/independence tests, and `tools/Test-MineHeadsLauncher.ps1` logging all four K/M × Quality/Performance feature creations plus RR roundtrip.
