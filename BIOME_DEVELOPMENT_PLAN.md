# Falling Heads: deepen the existing eight biomes

## Implemented in this pass

- Falling-stream contact uses the same front-face sections as its mesh. A head slows on overlap, including in the middle of a falling stream; empty space beside a narrow stream has no drag. Swimming requires meaningful immersion. Lava's first hit occurs on overlap, not only at the bottom of a cell.
- Arrow tip length matches arrow2.glb's transformed local X extent (1.6 times entity scale). Terrain penetration varies deterministically from three to eight world units, capped by actual arrow tip length; actor penetration varies from seven to thirteen units. Arrows that begin inside terrain are moved back to the entry face. Terrain still takes no arrow damage.
- Deterministic portal ruins occur in approximately one out of 127 chunks, outside the initial spawn clearance. Each reserved chamber and frame fits inside one chunk. Generation excludes ordinary terrain, decorations, fluid seeds and enemy spawn points from that chamber.
- Ruins use a four-by-five decorative, non-colliding obsidian frame behind the play plane. The two-by-three opening uses an original continuous animated shader. Frame damage cannot disable it; it is not a mineable Block.
- Portals are always active, without a plate, wire circuit or destructible frame dependency. Redstone circuits remain used by separate traps/relay doors.
- A hero inside an active portal for 0.08 seconds requests safe travel. Frame contact alone does not teleport. The existing charge/reveal animation and loaded, biome-checked landing search handle the move.
- TELEPORT is available through the developer power button, command/API route and a 5% lootbox reward. All use the same safe travel request. Travel advances through Plains -> Forest -> Desert -> Mountains -> Underground -> Nether -> End -> Iceland -> Plains. Repeated requests share a twenty-second cooldown; rejected requests show feedback.
- API: POST http://127.0.0.1:3029/command with JSON {"action":"TELEPORT","username":"player"}. The HTTP success means queued; runtime cooldown and landing checks still apply. Existing dev-panel biome selection remains the way to choose a specific biome.

## Asset audit

Verified PNG headers: `assets/textures/resources/redstone.png`, `assets/textures/blocks/general/redstone_ore.png`, and `assets/textures/blocks/nether/obsidian.png` are all 64 by 64 pixels.

The Falling Heads and adjacent communityGaming search did not find `pack.mcmeta` or dedicated redstone-wire, repeater, comparator, piston, lever or portal textures. This does not establish that there is no resource pack anywhere else on the computer. Current wires/plate are procedural geometry; portal fill is a shader. Dedicated mechanism art should be added as an internally consistent set before expanding to a building interface.

## Biome direction (implemented slices listed below)

Preserve the current bouncing core. Differentiate what heads encounter, the available routes and how hazards are triggered; avoid globally retuning gravity, restitution or head speed for each biome.

| Existing biome | Intended play rhythm | Objects and mechanisms to add | Enemy role to develop |
|---|---|---|---|
| Plains | Fast, readable recovery space between harder zones | Broad tunnels, small ore caches, simple plate/lamp/door tutorial ruins | Sparse skeleton sentries; clearly telegraphed shots |
| Forest | Fast clearing interrupted by deliberate route choices | Fragile leaf shortcuts, hollow logs, hanging reward pods; vine doors opened by two separated plates | Short-range ambusher with a visible wind-up; not another rapid shooter |
| Desert | Timing and risk/reward | Sandstone tombs, breakable sand bridges, marked pressure-plate arrow dispensers, dry wells | Slow ranged guardian with a distinct aiming tell; trap arrows must still lodge without mining terrain |
| Mountains | Vertical route planning | Ledges, ore seams, falling rubble pockets, counterweighted lifts and pressure-plate bridges | Infrequent high-ground sentry with knockback; generous recovery shelves |
| Underground | Read and manipulate machinery | Mineshaft supports, redstone ore rooms, wire/repeater/door circuits, finite pumps and drainage gates | Armoured miner requiring several hits; expose a weakness rather than merely adding health |
| Nether | Short, dangerous routes with cooling opportunities | Basalt-like platforms using suitable new art, lava reservoirs, controllable sluices, cooling-water rewards | Rare Ghast encounters in sufficiently open chambers; reuse existing Ghast logic only after adapting its natural spawn path |
| End | Spatial puzzles and brief precision encounters | Linked portal pairs, timed bridges, chorus clusters, delayed redstone relays | Proper End guardian with a new asset and teleport tell; rare articulated dragon flyovers now occur; a full boss remains future work |
| Iceland | Changing routes with predictable freeze/thaw timing | Thin ice sheets, snow-capped ledges, thaw valves, sheltered recovery pockets | Existing Yeti as the signature foe; tune freeze duration and projectile tells before adding another enemy |

### Why these first

The current natural generator uses skeletons in several biomes, Yeti in Iceland, and a separately scheduled dragon flyover in the End. Other enemy enum variants do not imply a complete usable spawning/art/behaviour pipeline: the current chunk spawner falls back to skeleton textures for unhandled types. Do not enable those enum variants indiscriminately.

**Desert trap rooms** and **Underground relay doors** now implement the first encounter slice. Next develop controlled Nether sluices and richer End encounter balance, after validating long-session encounter balance.

## Architecture for the next mechanisms

1. Separate inputs (plate, lever, sensor), signal nodes (wire, delayed repeater), and outputs (door, dispenser, pump, portal). Circuit evaluation should use bounded fixed ticks, not depend on draw frames.
2. All current receivers use the explicit local circuit graph in redstone.rs. Encounter layouts construct directed graphs; branching and attenuating feedback are tested. Player editing and delayed repeater components remain future work.
3. Bind device coordinates to stable chunk/cell IDs, not raw entity IDs in save data. Despawned chunks must release render/collider entities while preserving intended circuit state in the save model.
4. Generate structures as reserved footprints with checked access routes, separate from ordinary cave decoration. Include receiver/plate locations in the same structure specification.
5. Keep mineable solid parts in the normal Block/spatial-hash pipeline; render wires and portal planes as non-colliding visuals. A plate senses head overlap and does not add a hidden wall.
6. Portal requests, command powers and loot rewards must keep using one travel coordinator, including cooldown, destination validation and failure feedback.
7. Future pumps/sluices must transfer finite volume through the existing fluid simulation; they must not spawn endless decorative streams unrelated to contact.

## Remaining work and acceptance checks

- [ ] Player placement/editing of circuits, levers, repeaters, dispensers and pistons.
- [ ] Persistent circuit state and changed encounter structures across chunk unload/reload and game restarts. Current world regeneration can reconstruct a ruin on a later visit.
- [x] First authored Desert trap and Underground relay rooms with open exits and a visible trap warning.
- [ ] Extend authored encounters to the other six biomes.
- [ ] Dedicated mechanism icons/textures and distinct enemy assets; no silent skeleton fallback for new enemy types.
- [ ] Extended balance testing for portal encounter frequency, cooldown, leaf-clearing pace and fluid resistance.
- [x] Cap trap projectiles, prevent held-plate volley spam and enforce shared teleport cooldown.
- [ ] Long-session check for interacting encounters that could create unavoidable trap chains.
- [ ] Measure twenty-minute runs: area dwell time, stuck duration, biome transitions, deaths per hazard and resource income. Tune encounter layout before altering head physics.

## Weather, fluid continuity and encounter update (2026-10-23)

Implemented in code; validation results are recorded in FALLING_HEADS_UPGRADE_2026-10-23.md:

- [x] Falling fluid spans complete cell height; pool lips join adjoining falling sheets. Short-lived transport traces bridge simulation ticks. Rendering and immersion use identical sections.
- [x] Rain collects finite water on local exposed ledges (maximum twelve deposits of two volume units per weather reaction; no infinite source).
- [x] Active fire rain melts exposed snow caps faster, creates a small amount of meltwater and steam, and accelerates thawing of frozen fluid blocks. Natural terrain Snow blocks are not all converted by this system.
- [x] Water/lava bucket powers through dev buttons and the same external command path. Each pours at most sixteen volume units into clear compatible cells; shared eight-second cooldown.
- [x] Teleport minimum charge/reveal 0.55/0.40 seconds, with stronger chromatic separation and lens distortion. Real destination loading can extend preparation.
- [x] Small steam wisps with bounded size and smooth opacity envelope.
- [x] Desert trap rooms: pressure plate, visible warning, delayed three-arrow volley, eight-second rearm limit; at most 24 trap arrows alive globally. Held plates do not repeat continuously. Arrows lodge in terrain without mining it.
- [x] Underground relay doors: pressure-plate latch, propagation delay, mineable control/door blocks, open while heads, drops or fluid occupy the doorway. Breaking the control opens the door; mined doors do not rebuild within the same loaded room.
- [x] One directed signal graph implementation for portals and biome mechanisms. The old fixed-chain evaluator was removed.
- [x] Versioned desktop launcher/publication scripts. Installed releases contain an executable and matching immutable asset snapshot; publishing selects the next launch without replacing a running executable.

These encounters are deterministic sparse structures in newly generated Desert/Underground chunks. They do not alter dry head restitution, gravity or baseline movement. The same reserved footprint is used by terrain and enemy generation.

Still pending: persistent changed structures across unloading/restarts, dedicated mechanism artwork, levers/player wiring/pistons, Nether sluices, further End encounters beyond the articulated dragon flyover, and long play-session balance measurements. Current circuit state is intentionally session/chunk-local; do not describe it as a completed persistence system.

## Underwater biome proposal: SpongeBob / Bikini Bottom direction (not implemented)

Working biome title: **Coral Commons**. Readable, playful ocean neighbourhoods with flower silhouettes in distant water, warm sand, turquoise coral and occasional industrial ruins. It can later carry explicit SpongeBob locations/characters, but the first asset set should have a consistent authored pixel-art style rather than mixing screenshots and unrelated packs.

### Five connected areas and their gameplay

1. **Sponge neighbourhood** — porous sponge blocks, pineapple-shaped homes, shell paths, mailboxes and safe air-pocket rooms. Soft, quick-breaking material gives a recovery area between hazards.
2. **Jellyfish meadow** — tall seagrass, coral arches, drifting jellyfish. Jellyfish show a bright charge before a short electrical pulse; bubble columns offer a clearly marked escape route.
3. **Kelp forest** — fragile kelp shortcuts, pearl pods and alternating current gates. Currents add a bounded, visible force while preserving head-to-head collisions; they must never teleport or silently accelerate actors.
4. **Sunken workshop** — anchor doors, redstone-driven pumps, pressure plates and scrap-robot patrols. Reuse the circuit graph and finite fluid simulation, including safe door closing.
5. **Deep trench** — darker basalt/coral, anglerfish-like sentry with a lure tell, rare treasure vault and a portal ruin back to a dry biome. Always include a recovery route.

### Initial content budget

- Sixteen 64x64 tile textures: pale sand, dark wet sand, sponge front/top/side, three coral families, kelp, seagrass, shell stone, rusty metal, anchor panel, pearl ore, pineapple rind and cut pineapple.
- Eight prop sprites/meshes: pineapple house entrance, shell lamp, mailbox, bubble vent, anchor, barrel, treasure chest and sea-flower background shapes.
- Three enemies with full art/AI pipelines: jellyfish (charge/pulse), scrap crab (armour opening/closing), lure sentry (telegraphed projectile). No unimplemented enemy enum with skeleton fallback.
- Two powers to consider after buckets: bubble shield (temporary buoyancy/projectile deflection) and sponge absorb (removes a bounded quantity of nearby water, with a visible drying effect).
- Two room templates: bubble-column shortcut and pump/anchor-door treasure room. One optional larger landmark once those work.

### Architecture and acceptance before shipping

Do not fill every cell with a fake static water sprite. Ocean ambience and playable fluid have separate responsibilities: background tint/caustics establish the ocean, while actual underwater spaces use a tested medium controller and coherent fluid/contact geometry. Define underwater movement deliberately; do not globally change dry bouncing to make a new biome work. Prototype one chamber with adjustable buoyancy, readable current direction and safe exits first.

Acceptance: no head/loot embeds in coral or doors; a jellyfish pulse is visible before damage; bubbles do not trap heads indefinitely; biome teleport reaches a valid dry or aquatic landing as appropriate; all named props/enemies have matching assets. No ocean content is enabled merely by this proposal.

## Current graphics, portals and climate

See GRAPHICS_EFFECTS.md (also supplied as .txt) for the current rendering implementation, eleven settings, climate table, rare background portals, arrow embedding and End dragon. Water has no age-based drain. This section supersedes earlier date-stamped implementation notes where behaviour changed.


## Current implementation record — 23 October 2026 follow-up

Older proposal tables above are design history, not a list of shipped features. Redstone machinery remains removed at the user's request. The End dragon is now a persistent reactive encounter with articulated collision and continuous terrain-igniting breath. Graphics has fourteen switches, including verified NVIDIA 2× Frame Generation; higher MFG modes are not exposed. Follow FG_IMPLEMENTATION.md, GRAPHICS_EFFECTS.md and PERFORMANCE_NOTES.md for current status and outstanding validation. Coral Commons and other proposed biome expansion are not implemented.


## Next content direction — user clarification, 2026-10-23

The user's “what next” means richer playable content: mastered biomes, distinct animated enemies, different projectiles and interactive blocks. It does not mean more renderer toggles. The following is a proposal, not shipped content. It supersedes old redstone-machine priorities above: the user removed those mechanisms from the current scope.

### Recommended next complete pack: Nether

Upgrade the existing Ghast into an articulated creature with independently lagging tentacles, breathing/body squash, head tracking, charge/release/recovery, hurt and death poses. The current implementation floats and swaps normal/shooting textures; an enum entry or two textures is not a completed new animation pipeline. Keep one readable charged fireball as its core attack; introduce a slow ember cluster as a rarer variant with a distinct warning. Add a ground enemy (Cinderling) that hops between safe patches and lobs short-range embers, so every foe is not another floating shooter.

Add basalt (hard, nonflammable), magma crust (visibly cracks before releasing a small bounded lava pocket), and steam vents (water cools hot vents; periodic warning before pressure release). Generate open Ghast chambers with cover and a dry recovery exit. Reuse accepted burning and finite-water/lava reactions without silently retuning them.

### Follow-on biome packs (proposed)

| Biome | Distinct enemy and animated attack | Blocks and encounter geometry |
|---|---|---|
| Forest | Root crawler: limbs brace, body winds up, then a thorn fan; spore moth: wing pulse before a slow spore cloud | Hollow logs, fragile vine bridges, spore pods, burnable roots; optional leaf shortcuts |
| Iceland | Ice wraith with flowing body and a visible shard wind-up; improve Yeti's arm wind-up/recovery | Thin ice that cracks under sustained contact, packed snow, thaw pockets; occupied-cell-safe freezing |
| Desert | Armored scarab: antenna/body tell, short charge, exposed rear during recovery | Crumbling sandstone, buried chambers, breakable sand shelves, cactus clusters |
| Mountains | Crystal bat: wingbeat, banking dive and recovery rather than instant direction changes | Brittle crystal, loose scree, wind vents, generous landing shelves |
| Underground | Burrowing grub that visibly disturbs soil before emerging; crystal sentry with a delayed ricochet shard | Clay pockets, support beams, resonant crystal, finite flooded caverns |
| End | Shell guardian that opens before firing a slow seeking orb; dragon encounter phases and rewards | Chorus growth, phase stone, short-lived fading platforms with recovery paths |
| Plains | Low-pressure beetle/hopper with readable crouch/leap/landing | Soft earth, seed pods, broad safe routes between difficult packs |

Every new enemy needs its own art, idle/move/turn/wind-up/attack/recovery/hurt/death poses, collider/projectile matching, spawn rules, sounds, drops and tests. Projectiles should differ mechanically (arc, bounce, pierce, freeze or lingering cloud) and visually; avoid recolored arrow variants. New terrain needs break feedback, loot, fire/water/ice behavior and persistence decisions. Do not enable unhandled EnemyType values through the current skeleton-texture fallback.

Ship one complete pack at a time, then tune encounter frequency and measure stuck time. Underwater/Coral Commons remains a later ninth-biome proposal, after existing biome loops are distinct. Changed-world persistence is a prerequisite for long-term progression and revisiting mined areas.

## Proposed next work: structures and dragon breach (design only, 2026-10-23)

User decision: design both before choosing the next implementation. Neither feature below is implemented by the atmosphere/vegetation update. Earlier completed encounter experiments above are history, not a claim that the structures proposed here exist.

### A. Biome structures: playable landmarks in the falling route

Use compact, partly ruined rooms embedded in the cave network, rather than buildings pasted into solid terrain. A structure should change the route, offer a readable reward and have an escape. Start at roughly 12–24 tiles wide and 10–18 tall (one or two screens, depending on zoom). Preserve bouncing and mining controls.

| Prototype | Space and silhouette | Decision for the head | Reward and exit |
|---|---|---|---|
| Forest root sanctuary | Broken log arch, irregular clearing, three broad root shelves, mushrooms on supported pockets | Safe descending route around the edge; break a leaf curtain for a shorter central route | Small ore cache below the arch; two open lower outlets |
| Desert sunken tomb | Sandstone lintel, offset rooms and eroded exterior; no rectangular cleared border | Visible pressure plate with a preview of the arrow direction; side passage avoids it | Ore alcove on the risky route; breakable sand exit and permanent open bypass |
| Underground abandoned shaft | Timber supports, broken platforms, lamp-lit pockets | Choose between broad shelves and a narrow resource-rich descent | Separated ore pockets; no mechanism required to leave |
| Nether cooling shrine | Cracked dark platforms around a finite lava basin, glowstone markers | Cross above the basin or take a longer cool route | Small cooling-water cache; outlet protected from the basin's initial spill |
| End fractured observatory | Obsidian crescents and a broken central ring | Bounce between broad fragments; optional route toward portal reward | Clear return/exit routes; reserve a separate open boss stage |

First build recommendation: Forest root sanctuary. It makes the new mist and vegetation useful together and needs fewer new mechanisms. Desert tomb follows when projectile trap readability and fair timing are approved.

Generation contract:
1. Choose rare candidates from a stable hash of world seed and a coarse region ID. Reject overlaps with portals, spawn clearance, existing encounter reservations and biome boundaries. Choose a template variant and mirror from the same hash.
2. Reserve the whole structure and its approach/exit buffers before emitting ordinary terrain, decorations, fluid springs or enemies. Derive every intersecting structure from nearby region IDs so chunk load order cannot cut a room in half.
3. Each template describes solid cells, mandatory empty cells, supported plant sockets, reward sockets, hazards and entry/exit sockets. Distinguish foreground blocks from decorative background beams.
4. Carve an irregular transition pocket around the authored outline, with deterministic erosion and rubble. Connect at least two sockets to real caves. Reject a candidate if a bounded corridor search cannot reach them without cutting another reserved feature.
5. Validate with a conservative head-sized occupancy grid: a clear entrance, a reachable reward or clearly signposted mining route, and at least one always-open descending exit. Do not rely on a locked door or destructible circuit for the only escape.
6. Spawn structures in the normal Block/spatial-hash pipeline. Cosmetics have no hidden collision. Initial fluid volumes are finite; validate the flooded route as well as the empty layout.
7. Keep a stable structure ID and changed-cell/reward state across chunk unload. The current game can regenerate mined terrain on revisit; persistent edits must be added before calling structures permanently looted or destroyed. MVP can explicitly remain session-only, but must preserve it within that session.

Acceptance: same seed gives the same footprint regardless of chunk load order; no plants in walls; no sealed head traps; no duplication at chunk edges; no overlapping portal chambers; visible hazard tell; bounded generation cost. Record repeated visits and a descent through each layout before tuning spawn frequency.

### B. Dragon entrance: a breach through the background

The dragon should fracture the rear wall, briefly reveal another realm, then emerge into the play plane. A ragged hole about 3–5 tiles in radius reads better than a circular decal. Use the current biome's background texture on the broken pieces and a dark violet moving depth layer behind the opening.

Sequence (initial art timings, to tune in motion):
- 0.0–0.6 s: a low rumble, small dust slips and branching cracks announce the location. Rim light grows unevenly along the cracks. Leave the player an unobscured escape route.
- 0.6–0.85 s: the center breaks first; neighboring background cells fracture with small radial delays. An irregular silhouette opens, with a brief restrained light pulse.
- 0.85–2.0 s: the dragon's head and shoulders emerge first; wings unfold after clearing the rim. Render the body behind the rim initially, then move it into the established foreground depth. Head, wing and tail motion follow the turn rig instead of enlarging one flat sprite.
- 2.0–2.8 s: the tail clears, the dragon banks into its first orbit, loose chips fall under gravity and dust trails disperse. Breathing fire and contact damage begin only after the emergence stage completes.
- Afterwards: keep the breach visible for the encounter, or slowly seal it with small inward settling fragments after the dragon leaves. Do not shrink the entire hole or all debris toward its center.

Debris should be actual textured shards: varied size, angular velocity and outward impulse followed by gravity, with near/far layers. Start with a pool capped around 48–80 visible fragments and fewer major slabs. Most fragments are visual only; avoid surprise collision damage. Fade dust and tiny fragments independently. Large fragments can disappear below the camera. A small, optional camera impulse and delayed sound sell weight without a sustained screen shake.

Implementation contract:
- Pick a world-anchored background location in a readable chamber. Reject positions that hide the head or place most of the hole behind dense foreground blocks; delay the entrance if no suitable location exists.
- The hole cuts only the rear wall. Foreground Blocks, mining health, collision, resources and liquids remain intact. A separate explicit gameplay attack would be needed to destroy those.
- Maintain a breach mask keyed by background cells. The streaming background pool must apply that mask to newly loaded tiles and must not redraw intact tiles over the hole on camera movement.
- Add an entering boss phase. Disable normal steering/attacks until the rig clears the rim, and handle interruption, defeat, teleport and chunk unload without leaking materials or leaving an unowned mask.
- Use stable world positions and full-size debris throughout decay. Test foreground/rim occlusion during emergence and banking in native raster, DLSS and RT. Use depth-tested effects and exclude transient cosmetic shards from expensive ray tracing if needed.

Acceptance: no popping intact tiles inside the hole; no square mask boundary; all debris follows continuous trajectories; no shrink-to-center exit; no accidental terrain damage; no attack before the tell completes; animation survives camera motion; resource counts return to baseline after cleanup.

Recommendation: build this breach after the fire presentation is satisfactory, because it is another prominent boss effect and should share coherent dust, embers and lighting. Choose it first instead if a dramatic boss entrance matters more than exploration variety.

### Fire presentation still needs revision

The current transient impact wash separates dragon contact from persistent burning correctly, but the user still sees the individual rectangular cards. The next visual iteration should use a connected impact plume across contacted surfaces, with irregular flame tongues and released ember particles, plus per-face depth/neighbor occlusion. Fade and advect particles rather than scale the full effect. Validate a connected wall, isolated block, corner and mid-wall impact before replacing the installed effect. This atmosphere pass does not claim that criticism is resolved.

### What the Gothic reference means here

THQ confirms Gothic 1 Remake uses Unreal Engine 5, but its public release page does not document exact atmosphere settings: https://thqnordic.com/news/gothic-1-remake-is-out-now-welcome-to-the-colony . Unreal supports locally blended exposure/color-grading/bloom settings through post-process volumes, and localized height/radial fog or more expensive lit volumetric fog. These are relevant documented techniques, not a verified breakdown of Gothic's particular implementation:
- https://dev.epicgames.com/documentation/en-us/unreal-engine/add-post-process-volumes
- https://dev.epicgames.com/documentation/en-us/unreal-engine/local-fog-volumes-in-unreal-engine

Falling Heads' current update uses per-biome sun/fill/background/fog colors, smooth lighting transitions and inexpensive world-anchored mist behind foreground terrain. It is not volumetric light scattering and does not add light shafts. A stronger forest look would also need foreground/background vegetation layers and a more distinctive forest tile palette; fog alone cannot make stone corridors into a forest.

## Atmosphere and supported vegetation update (2026-10-23)

- Eight independent sun, ambient fill, background, fog and environment-light palettes. Forest is subdued green; Desert is warm and bright; mountain/ice areas are cooler. Nether, Underground and End retain their own illumination across the outdoor day/night cycle. Camera lighting transitions ease over time.
- The Atmospheric haze setting controls both distance fog and sparse, world-anchored mist. Wisps use irregular procedural density and fade at the tile boundaries, sit behind the foreground terrain, and no longer snap their entire transform to follow the player or switch on/off on a fixed 90-second schedule.
- Bushes, mushrooms, vines and cacti no longer replace solid terrain through the random decorative-block selection. Bushes/mushrooms require a floor with a 3-wide, 2-high clear cave pocket; cacti require side clearance and headroom around their whole stalk. This pass chooses existing pockets rather than excavating new ones.
- Mushrooms use a shared colored stem/cap mesh rather than a full red cube. Fluid springs do not start in the same cell as a newly generated plant.
- New placement rules apply when chunks are generated. They do not move vegetation in an already running older build.
- The structures and breach above remain proposals. Fire card presentation remains a known visual issue.

## Fog direction correction after Gothic reference review (2026-10-23)

This supersedes the previous broad mist coverage. The user rejected fog spread throughout the scene. Inspected official THQ screenshots directly in a browser:
- https://gothic.thqnordic.com/game-sites/gothic/content/screenshots/screenshot-14.png — pale, low haze around vegetation; nearby dark rocks and creatures keep contrast.
- https://gothic.thqnordic.com/game-sites/gothic/content/screenshots/screenshot-07.png — sharply defined foreground and soft distant forest, with localized low haze.
- https://gothic.thqnordic.com/game-sites/gothic/content/screenshots/screenshot-03.jpg — illuminated haze in a cave interior with dark foreground silhouettes.
These are visual observations, not claims about Gothic's exact shader implementation.

Correction: removed camera DistanceFog, since foreground and backdrop are almost at the same camera distance in this shallow side-view scene. Removed the continuous 25-tile mist field. Fog banks now require real forest floor support, a clear central opening and seven-cell-wide open air above it (allowing uneven ground and foliage), deterministic sparse placement and at least twelve tiles between centers. At most six banks exist within the 2200-unit active radius, each only 448–576 units wide and 112–160 high. They stay behind foreground terrain, use pale gray-green irregular wisps, ease in/out at fixed size, and disappear when their supporting space ceases to qualify. Other biomes receive no generic mist from this system. Biome lighting remains separate and still applies.

The Atmospheric haze toggle now controls these local banks. This remains a side-view approximation, not a recreation of Gothic's volumetric rendering or distant 3D forest layers.


Integration finding: the previous BiomeMistPlugin module was not registered in the application. Its shader did not contribute to the earlier screenshots; those showed camera distance fog. The statement above about earlier broad mist coverage describes the unconnected implementation, not what rendered. The correction registers the localized plugin in main.rs, removes camera fog, and adds a render-probe assertion requiring actual live mist entities with small dimensions. Two pre-registration probe attempts failed that assertion; they are not accepted validation.

## Forest trees and glare correction (2026-10-23)

Added oak, birch and spruce as connected Wood/Leaves block assemblies with species-specific bark, end grain and leaf textures. The three tree species use a consistent vanilla 16-pixel texture scale, retaining existing assets elsewhere. Species material identity persists through damage. The front-view crown hides the upper trunk, giving broad oak crowns, tall pale birches and tiered evergreen silhouettes; trunk heights vary by seed.

Forest terrain is now soil with grass-capped exposed ledges, replacing melon walls. Generation chooses three-cell solid footings at existing openings and prefers the site needing the least excavation. Where necessary it clears at most 12 terrain cells in a stepped outline following the crown, with one tile of surrounding air. Whole trees and clearances stay inside their owning chunks. Spawn clearances, portal footprints, enemy placement and springs respect the tree reservation. At most one tree is planned per forest chunk; unsuitable sites stay empty. Trees generate mature with the terrain; no timed sapling simulation or new inventory/crafting categories are introduced in this pass.

The upper-left wash was the specular highlight of the repeated backdrop material. The backdrop now has full roughness and zero specular reflectance; terrain also has lower reflectance. Biome sun/fill colors remain in place. Local forest fog uses terrain support, never tree crowns or branches.

Structures and the dragon background breach remain the earlier design proposals pending the user's choice.

Follow-up lighting preference: the user wanted the upper-left glow softened, not removed. The accepted follow-up restores moderate backdrop reflectance (0.30) with roughness 0.62, keeping the directional gradient while reducing the original wash. Verified in native and RT forest captures, build 20261023-232813-64c58744c1. This supersedes the zero-reflectance backdrop values described above.

### Implemented: denser forest layout and gentler weather (23 October follow-up)
Forest terrain now keeps broad soil footing with rolling tops and staggered gullies, with up to two spaced, grounded trees per chunk. Forest bands are 20 chunks deep and biome entry selects the upper third. This replaces the previous high-fracture cave layout that left mostly empty space and very few viable tree sites. Rain/snow preserve diffuse fill and backdrop brightness; only rain wets materials. See FALLING_HEADS_UPGRADE_2026-10-23.md for test evidence and installed build status.

Structures and the dragon's background breach remain design proposals awaiting the user's choice; this update does not implement either.

## Next additions proposed on 24 October 2026 (not implemented)

The implementation record at the end of this document supersedes the proposal's enemy/landmark availability claims. Unlisted proposal features remain future work.

The strongest next step is recognizable places and encounters, while preserving the bouncing/falling core. Keep one clear main descent route; optional side pockets should reconnect below instead of requiring platformer controls the player does not have. Reserve structures before ore, plants, enemies and liquid placement, and validate exits/supports. Start with small rooms that fit within a chunk; use a deterministic owner and cross-chunk reservation only when larger landmarks are justified.

| Biome | First landmark and gameplay addition | Visual identity to develop |
|---|---|---|
| Plains | Broken watchtower and safe ore/reward pockets | Grassy overhangs, wildflowers, layered distant hills |
| Forest | Witch clearing, hollow logs, leaf shortcuts and hanging caches | Tree groups of varied ages, roots/moss, distant trunks, localized ground mist |
| Desert | Sandstone tomb with marked arrow traps and breakable sand bridges | Strata, dunes behind the play plane, sparse dust and worn masonry |
| Mountains | Ore ledges, rope bridge ruins and readable rubble hazards | Angular silhouettes, snow only on upward exposed surfaces, distant ridges |
| Underground | Mineshafts, small machinery rooms, drainage-gate puzzles | Supports, rock strata, localized dampness and mineral glints |
| Nether | Basalt shrine, lava reservoir and cooling-route choice | Dark basalt contrast, localized heat shimmer and embers, emissive cracks |
| End | Broken obelisks and crystal chambers before the dragon arena | Floating rear fragments, restrained violet energy and deeper void layers |
| Iceland | Frozen ruins, ice bridges and yeti reward alcoves | Windward snow drifts, icicles, blue ice translucency and restrained drifting powder |

Enemy audit: Zombie, Skeleton, Pigman, Enderman, Yeti, Ghast and Dragon exist; Witch does not. Add one encounter with a new role before multiplying recolored enemies. Proposed Witch: visible wind-up, lobbed potion with a clearly marked landing area, brief slowing cloud, occasional healing with an interrupt window. Since the hero is currently invincible, poison damage alone is ineffective; target route control, temporary resource-yield effects or other existing vulnerable actors, and state the effect clearly. Avoid unavoidable long freezes. Place her in a spacious clearing/ruin, not random solid terrain. Reuse common projectile/collision rules and make drops collectible by the current movement model.

Dragon audit: the boss currently spawns directly near the hero, automatically during End encounters or through DRAGON. The requested background breach is still the design above. Death awards 100 XP through the common death event, with no dedicated egg/item drop. A proposed guaranteed first-kill egg should unlock a small cosmetic hatchling with modest automatic collection of already-dropped resources, not free mining or combat that trivializes the game. Subsequent kills could yield essence for hatchling appearance/collection-radius progression with a strict cap. Implement inventory persistence, a once-per-boss reward guard, pickup attraction and an explicit hatch/unlock UI before claiming this reward exists. The shared damage handler can currently emit repeated death events after health reaches zero; fix this before adding valuable guaranteed drops.

Graphical priorities after lighting correctness: coherent material roughness/normal detail at the current texture scale; stronger near/mid/far scenery; grounded contact shadows around trees/props; neighbor-aware flame plumes and textured debris; restrained local mist/light shafts; smooth exposure and biome transitions without losing foreground readability. Measure frame-time costs per feature, especially RT with more lights. DLSS alone does not add the missing environmental composition.

## Implemented encounter and landmark pass — 24 October 2026

Eight deterministic landmarks now reserve their footprints before normal terrain, plants, trees, springs and ordinary enemies. Sites occur in about one of nine eligible chunks, outside the initial clearance and separate from portal rooms. Every site fits in its owning chunk, has a clear central/right approach, and contains a biome-appropriate mineable cache:

| Biome | Landmark | Encounter |
|---|---|---|
| Plains | Timber wayfarer camp | Recovery/cache pocket |
| Forest | Canopied witch grove with mushroom and grass footing | Witch |
| Desert | Broken sandstone sanctuary | Ranged Sentinel |
| Mountains | Broken watchpost with separated ledges | Ore/recovery space |
| Underground | Supported miner's vault | Armoured Miner |
| Nether | Raised obsidian forge and glowstone | Ember-coloured Sentinel projectile |
| End | Purpur/obsidian observatory | Sentinel with clear-space blink |
| Iceland | Ice-roofed shelter | Slowing Sentinel and recovery cache |

The three new enemy roles use modeled heads; the Witch also uses the existing bundled Minecraft-style face texture. Attacks lock a target and show a 0.85-second wind-up; ranged attacks also mark the intended impact point. Witches lob potions, while Sentinels fire straight projectiles. Impact areas expire after 2.4 seconds and tick at bounded intervals. Forest/ice hits apply a short Slowness effect without replacing an existing different buff. The hero's existing invulnerability remains; vulnerable villagers take damage normally. Miners commit to a charge, visibly open their guard afterward and take extra damage during that recovery. Attacks require line of sight and remain within an 850-unit activation range. Ordinary random groups were reduced to one or two enemies to leave room for these encounters. Existing Yeti, Skeleton and other enemies remain.

Ore uses normal mining/pickup behavior; specialist deaths award XP through the existing system. Death events are now emitted once, avoiding repeated rewards after zero health. No new currency or unimplemented item inventory is advertised.

This pass does not implement every proposal above. Witch healing, trap machinery, persistent structure changes across unloading, multi-chunk buildings, the dragon's background breach and egg progression remain pending. New world generation is seen in newly generated chunks after restarting into the updated build.


2026-10-24 refinement: landmark candidates now use 1/41 chunk eligibility with neighboring candidates excluded, biome-dependent floor silhouettes and seeded mirrored layouts. Purpur now uses its Minecraft block texture. Added cave bats (2 HP, 8 XP, bounded swoops with wall avoidance). Specialist enemy heads now use the normal flat sprite/damage-material path; target warning markers removed. Changes apply on chunk generation.


## Implemented terrain and landmark follow-up — 24 October 2026

Installed 20261024-035006-4bb3b7c7b7: global rolling Plains shelves, horizontal Underground chambers and staggered descents, eight larger biome landmarks with three position-based variants, recessed non-solid walls/windows and original plank/masonry textures. Local forest mist now fades below ledges. 136 tests and final rendered gallery passed; natural terrain verified away from spawn. Multi-chunk structures, persistent world edits, dragon breach/egg progression and crafting remain pending. Current authoritative request audit: ENCOUNTER_FIX_CHECKLIST.md.
