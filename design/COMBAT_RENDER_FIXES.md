# Combat and rendering corrections — 27 October 2026

## Contact experiment

Developer controls expose **Final hit: BOUNCE (default)** / **Final hit: PASS THROUGH**.
This is an in-memory experiment, reset to the original bounce on launch.
Only a real contact that can deal the final damage point removes that surface from
the current sweep. The sweep runs again against remaining solids. Damage cooldowns
and normal destruction/loot events still apply. Enemies and surviving blocks retain
their existing collision response. Crowd contact geometry is unchanged.

## Projectiles and enemy identity

Normal and tipped arrows share ARROW_SPEED (1,200 world units/second); tipped arrows
also use the live global gravity. Terrain and line-of-sight queries share the same
projectile silhouette routine. Dripstone and crystal silhouettes come from opaque
pixel runs in their bundled Faithful textures. Regenerate with
`tools/Generate-TerrainSilhouettes.py` after replacing those textures, then rustfmt
the generated Rust file. Plants remain pass-through, and cactus retains its narrower
body. This does not change head collision geometry.

Blazes lock their aim during a short charge, stop to fire three shots with slight
spread, then drift during recovery. Breezes now have ordinary terrain contacts and
gravity: lateral leaps, braking near the apex, and wind knockback. They no longer
share the blaze's continuous hover. Ghasts retain their separate fireball behavior.
Reference for breeze movement: https://www.minecraft.net/nl-nl/article/minecraft-java-edition-1-21

Enemy face materials are private where per-actor tint/emission is applied. Teleport
compression restores geometry and re-registers the current material with the renderer,
without restoring a stale material snapshot; species
art attaches before the compression pass. This also prevents one damaged specialist
from tinting every other enemy of that species.

## Teleport and depth ownership

Teleport restarts charging even if the same destination is requested during reveal.
A request gets a charge frame before the timer advances. The destination commit
updates physics Position, rendered Transform, action center and camera together.
Camera follow is ordered after action-center calculation. Background reconciliation
runs after the final teleport cut.

Background, structure backing and mist use separate named depth bands (-100, -30,
-5 respectively). Fire and ice overlays use distinct offsets and update after
teleport, before transform propagation. This removes coplanar surfaces and stale
overlay positions; it does not disable fog or ray tracing.

## Verification

Unit regressions cover repeated teleport requests, late species material assignment,
opaque sprite hit regions, shared arrow speed and both lethal-contact modes. The
native content probe checks species art, blaze volley, teleport charge/reveal and
camera cut, species art after streaming, and overlapping fire/ice captures. The
architecture gate must pass without adding warning-baseline entries. See MODLOG.md
for the verified executable and final native results.
