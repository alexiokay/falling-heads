# Falling Heads Solari patch

Base: bevy_solari 0.20.0 from the Cargo registry, MIT/Apache-2.0.

`src/scene/bindings.wesl`: scale RAY_T_MIN from 0.001 to 0.064 for the
64-unit block world. The same epsilon protects primary/secondary ray origins
and visibility ray intervals. At the 1500-unit camera distance and ~190000-unit
world heights, the original epsilon allowed reconstructed surfaces to intersect
their own ray geometry. This produced intermittent large black patches.

Reproduced with frame generation off. Runtime A/B sequences in
`target/rt-flicker-bias` restored the artifacts when returning to 0.001.
No lighting, denoiser, shadow, or ray-tracing features are disabled by this patch.
Keep this calibration when updating Solari; revalidate across camera distances.
