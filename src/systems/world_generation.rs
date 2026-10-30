use crate::components::block::{BiomeBehavior, BiomeType, Block, BlockType};
use crate::components::bundles::{SpriteBundle3d, avian_physics};
use crate::components::chunk::{
    CHUNK_SIZE, Chunk, ChunkMember, LoadedChunks, world_to_chunk_coords,
};
use crate::components::enemy::{Enemy, EnemyBehavior, Health};
use crate::components::hero::Hero;
use crate::resources::asset_handles::BlockTextures;
use crate::resources::material_cache::MaterialCache;
use crate::resources::mesh_cache::MeshCache;
use crate::utils::biome_utils::determine_biome_at_chunk;
use crate::utils::constants::BLOCK_SIZE;
use bevy::prelude::*;
use noise::{NoiseFn, Perlin, Simplex};
use rand::RngExt;
// avian3d not needed for world generation (blocks don't use Avian physics)

// World generation constants - can be adjusted independently from rendering
const WORLD_BLOCK_SIZE: f32 = BLOCK_SIZE; // World units per block (matches BLOCK_SIZE from constants.rs = 64.0)
const BLOCK_SPACING: f32 = 0.0; // No spacing - blocks touch
const TOTAL_BLOCK_SIZE: f32 = WORLD_BLOCK_SIZE + BLOCK_SPACING; // Total size including spacing (64.0 + 0.0 = 64.0)

/// Generate chunks around the hero position
pub fn generate_chunks_around_hero(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    hero_query: Query<&Transform, With<Hero>>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    block_textures: Res<BlockTextures>,
    material_cache: Res<MaterialCache>,
    mesh_cache: Option<Res<MeshCache>>,
    enemy_textures: Option<Res<crate::resources::asset_handles::EnemyTextures>>,
    spawn_area: Res<crate::plugins::gameplay_plugin::SpawnArea>,
    world_seed: Res<crate::resources::world_seed::WorldSeed>,
    teleport: Res<crate::systems::biome_teleport::BiomeTeleport>,
    launcher: Res<crate::systems::launcher::LauncherState>,
) {
    // Wait for caches to be initialized before spawning blocks
    // MaterialCache now includes atlas generation, so when materials are ready, atlases are too
    if material_cache.materials.is_empty()
        || material_cache.forest_floor.is_none()
        || mesh_cache.is_none()
        || enemy_textures.is_none()
    {
        return; // Caches not ready yet, wait for next frame
    }

    let mesh_cache = mesh_cache.unwrap();
    let enemy_textures = enemy_textures.unwrap();

    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let hero_pos = teleport.target.unwrap_or(hero_transform.translation);
    let (hero_chunk_x, hero_chunk_y) =
        world_to_chunk_coords(hero_pos.x, hero_pos.y, TOTAL_BLOCK_SIZE);

    // Generate chunks in a radius around the hero
    let chunk_radius = 3;

    let mut pending = Vec::new();
    for dx in -chunk_radius..=chunk_radius {
        for dy in -chunk_radius..=chunk_radius {
            let (x, y) = (hero_chunk_x + dx, hero_chunk_y + dy);
            if !loaded_chunks.is_loaded(x, y) {
                pending.push((dx * dx + dy * dy, x, y));
            }
        }
    }
    if let Some(target) = launcher.preload {
        let (cx, cy) = world_to_chunk_coords(target.x, target.y, TOTAL_BLOCK_SIZE);
        for dx in -1..=1 {
            for dy in -1..=1 {
                let (x, y) = (cx + dx, cy + dy);
                if !loaded_chunks.is_loaded(x, y) {
                    pending.retain(|(_, px, py)| (*px, *py) != (x, y));
                    pending.push((-10 + dx * dx + dy * dy, x, y));
                }
            }
        }
    }
    pending.sort_unstable();
    // During teleport, prepare destination incrementally before moving the player.
    let budget = if teleport.target.is_some() { 3 } else { 4 };
    for (_, x, y) in pending.into_iter().take(budget) {
        generate_chunk(
            &mut commands,
            &mut meshes,
            &mut materials,
            x,
            y,
            &block_textures,
            &material_cache,
            &mesh_cache,
            &enemy_textures,
            &spawn_area,
            &world_seed,
        );
        loaded_chunks.add(x, y);
    }
}

/// Despawn chunks that are too far from the hero
pub fn despawn_distant_chunks(
    mut commands: Commands,
    hero_query: Query<&Transform, With<Hero>>,
    chunk_query: Query<(Entity, &Chunk)>,
    block_query: Query<(Entity, &ChunkMember)>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    teleport: Res<crate::systems::biome_teleport::BiomeTeleport>,
    launcher: Res<crate::systems::launcher::LauncherState>,
) {
    if teleport.target.is_some() || launcher.preload.is_some() {
        return;
    }
    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let hero_pos = hero_transform.translation;
    let (hero_chunk_x, hero_chunk_y) =
        world_to_chunk_coords(hero_pos.x, hero_pos.y, TOTAL_BLOCK_SIZE);

    let despawn_distance = 4; // Despawn chunks more than 4 chunks away (reduced from 5 for performance)

    // Collect chunks to despawn first to avoid nested iteration
    let mut chunks_to_despawn = Vec::new();

    for (chunk_entity, chunk) in chunk_query.iter() {
        let distance_x = (chunk.chunk_x - hero_chunk_x).abs();
        let distance_y = (chunk.chunk_y - hero_chunk_y).abs();

        if distance_x > despawn_distance || distance_y > despawn_distance {
            chunks_to_despawn.push((chunk_entity, chunk.chunk_x, chunk.chunk_y));
        }
    }

    // Now despawn blocks for all chunks that need to be despawned
    if !chunks_to_despawn.is_empty() {
        let num_chunks = chunks_to_despawn.len();
        let unloading: std::collections::HashSet<(i32, i32)> =
            chunks_to_despawn.iter().map(|&(_, x, y)| (x, y)).collect();

        // One pass over members for all unloading chunks (was one pass per chunk).
        let mut total_blocks_despawned = 0;
        for (block_entity, member) in block_query.iter() {
            if unloading.contains(&(member.chunk_x, member.chunk_y)) {
                commands.entity(block_entity).despawn(); // Despawn children too (damage overlays, borders)
                total_blocks_despawned += 1;
            }
        }

        for (chunk_entity, chunk_x, chunk_y) in chunks_to_despawn {
            commands.entity(chunk_entity).despawn();
            loaded_chunks.remove(chunk_x, chunk_y);
        }

        info!(
            "Despawned {} distant chunks ({} blocks total)",
            num_chunks, total_blocks_despawned
        );
    }
}

/// Generate a single chunk with blocks
pub fn generate_chunk(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    chunk_x: i32,
    chunk_y: i32,
    _block_textures: &Res<crate::resources::asset_handles::BlockTextures>,
    material_cache: &Res<MaterialCache>,
    mesh_cache: &MeshCache,
    enemy_textures: &crate::resources::asset_handles::EnemyTextures,
    spawn_area: &crate::plugins::gameplay_plugin::SpawnArea,
    world_seed: &crate::resources::world_seed::WorldSeed,
) {
    let mut rng = rand::rng();

    // Determine biome based on chunk position
    let biome = determine_biome_at_chunk(chunk_x, chunk_y);

    // Spawn chunk entity marker
    commands.spawn(Chunk {
        chunk_x,
        chunk_y,
        biome,
    });

    // Use shared cube mesh from cache - CRITICAL PERFORMANCE FIX
    let cube_mesh = mesh_cache.block_mesh.clone();
    let pumpkin_mesh = mesh_cache.pumpkin_mesh.clone();
    let wood_mesh = mesh_cache.wood_mesh.clone();
    let leaves_mesh = mesh_cache.leaves_mesh.clone();
    let melon_mesh = mesh_cache.melon_mesh.clone();
    let cactus_mesh = mesh_cache.cactus_mesh.clone();
    let sandstone_mesh = mesh_cache.sandstone_mesh.clone();

    let portal_site = crate::systems::world_portals::site(chunk_x, chunk_y, world_seed.seed)
        .filter(|p| {
            (p.as_vec2() * 64.).distance(spawn_area.center.xy()) > spawn_area.radius + 400.
        });
    let encounter =
        super::biome_encounters::site(chunk_x, chunk_y, biome, world_seed.seed).filter(|s| {
            portal_site.is_none()
                && (s.origin.as_vec2() * 64.).distance(spawn_area.center.xy())
                    > spawn_area.radius + s.width() as f32 * 64. + 100.
        });
    let tree = super::forest_trees::plan(
        IVec2::new(chunk_x, chunk_y),
        world_seed,
        spawn_area,
        portal_site,
    );
    spawn_chunk_enemies(
        commands,
        meshes,
        materials,
        chunk_x,
        chunk_y,
        biome,
        &mesh_cache,
        enemy_textures,
        &mut rng,
        world_seed,
        &tree,
    );
    // Generate blocks in this chunk
    for local_x in 0..CHUNK_SIZE {
        for local_y in 0..CHUNK_SIZE {
            let world_block_x = chunk_x * CHUNK_SIZE + local_x;
            let world_block_y = chunk_y * CHUNK_SIZE + local_y;

            let world_x = world_block_x as f32 * TOTAL_BLOCK_SIZE;
            let world_y = world_block_y as f32 * TOTAL_BLOCK_SIZE;

            // Skip blocks in the spawn area (keep spawn area clear)
            let distance_to_spawn = ((world_x - spawn_area.center.x).powi(2)
                + (world_y - spawn_area.center.y).powi(2))
            .sqrt();
            if distance_to_spawn < spawn_area.radius {
                continue; // Skip this block, it's in the spawn area
            }

            if portal_site.is_some_and(|p| {
                crate::systems::world_portals::reserved(IVec2::new(world_block_x, world_block_y), p)
            }) {
                continue;
            }
            if tree
                .iter()
                .any(|t| t.reserved(IVec2::new(world_block_x, world_block_y)))
            {
                continue;
            }
            if encounter.is_some_and(|s| s.reserved(IVec2::new(world_block_x, world_block_y))) {
                continue;
            }
            // Cave generation check (biome-specific)
            let is_cave = should_generate_cave(world_block_x, world_block_y, biome, world_seed);

            let tile = if is_cave {
                super::biome_content::terrain::feature(
                    world_block_x,
                    world_block_y,
                    biome,
                    world_seed.seed,
                )
                .or_else(|| {
                    super::biome_content::terrain::decoration(
                        world_block_x,
                        world_block_y,
                        biome,
                        world_seed.seed,
                    )
                })
            } else {
                super::biome_content::terrain::solid(
                    world_block_x,
                    world_block_y,
                    biome,
                    world_seed.seed,
                    seeded_block(biome, world_block_x, world_block_y, world_seed.seed),
                )
            };
            if let Some(tile) = tile {
                let kind = tile.base();
                commands.spawn((
                    tile,
                    kind,
                    Block {
                        health: kind.starting_health(),
                        max_health: kind.starting_health(),
                    },
                    ChunkMember { chunk_x, chunk_y },
                    Transform::from_xyz(world_x, world_y, -10.),
                    Visibility::Inherited,
                ));
                continue;
            }
            if is_cave {
                // Try to spawn cave decorations (stalactites, columns, etc) in empty cave spaces
                let plant = ground_plant_at(world_block_x, world_block_y, biome, world_seed);
                if let Some(decoration_type) =
                    cactus_at(world_block_x, world_block_y, biome, world_seed)
                        .or(plant)
                        .or_else(|| {
                            get_cave_decoration(world_block_x, world_block_y, biome, world_seed)
                        })
                {
                    // Spawn decoration block at this position
                    let world_z = -10.0;
                    let mut transform = Transform::from_xyz(world_x, world_y, world_z);
                    transform.scale = Vec3::splat(64.0);

                    let material = material_cache
                        .get(decoration_type)
                        .expect("Material should be cached for all block types");

                    commands.spawn((
                        Block {
                            health: decoration_type.starting_health().min(2),
                            max_health: decoration_type.starting_health().min(2),
                        },
                        decoration_type,
                        ChunkMember { chunk_x, chunk_y },
                        Mesh3d(match decoration_type {
                            BlockType::Cactus => cactus_mesh.clone(),
                            BlockType::DeadBush => mesh_cache.dead_bush_mesh.clone(),
                            BlockType::Mushroom => mesh_cache.mushroom_mesh.clone(),
                            _ => cube_mesh.clone(),
                        }),
                        MeshMaterial3d(material),
                        if matches!(
                            decoration_type,
                            BlockType::Cactus | BlockType::DeadBush | BlockType::Mushroom
                        ) {
                            Transform::from_translation(transform.translation)
                        } else {
                            transform
                        },
                    ));
                }
                if let Some(kind) = crate::systems::fluids::spring_kind(
                    world_block_x,
                    world_block_y,
                    world_seed.seed,
                    biome,
                ) {
                    if plant.is_none()
                        && cactus_at(world_block_x, world_block_y, biome, world_seed).is_none()
                        && get_cave_decoration(world_block_x, world_block_y, biome, world_seed)
                            .is_none()
                        && !should_generate_cave(
                            world_block_x,
                            world_block_y - 1,
                            biome,
                            world_seed,
                        )
                        && should_generate_cave(world_block_x, world_block_y + 1, biome, world_seed)
                    {
                        commands.spawn((
                            crate::systems::fluids::FluidSeed {
                                cell: (world_block_x, world_block_y),
                                kind,
                            },
                            ChunkMember { chunk_x, chunk_y },
                        ));
                    }
                }
                continue; // Skip normal block, it's part of a cave
            }

            // Determine block type
            let block_type = seeded_block(biome, world_block_x, world_block_y, world_seed.seed);

            let world_z = -10.0; // Behind entities so they render in front of blocks

            // Render as 3D cubes scaled to match 2D
            let mut transform = Transform::from_xyz(world_x, world_y, world_z);

            // Choose mesh based on block type (directional blocks use UV-mapped mesh)
            let forest_surface = matches!(biome, BiomeType::Forest | BiomeType::Plains)
                && block_type == BlockType::Dirt
                && should_generate_cave(world_block_x, world_block_y + 1, biome, world_seed);
            let block_mesh = if forest_surface {
                wood_mesh.clone()
            } else {
                match block_type {
                    BlockType::Pumpkin => {
                        // Pumpkins use custom UV-mapped mesh (no scaling needed, already 64x64x64)
                        pumpkin_mesh.clone()
                    }
                    BlockType::Wood => {
                        // Wood/logs use custom UV-mapped mesh (no scaling needed, already 64x64x64)
                        wood_mesh.clone()
                    }
                    BlockType::Leaves => {
                        // Leaves use custom UV-mapped mesh (no scaling needed, already 64x64x64)
                        leaves_mesh.clone()
                    }
                    BlockType::Melon => {
                        // Melons use custom UV-mapped mesh (no scaling needed, already 64x64x64)
                        melon_mesh.clone()
                    }
                    BlockType::Grass => {
                        // Grass uses leaves mesh (reusing leaves texture for grass blocks)
                        leaves_mesh.clone()
                    }
                    BlockType::Cactus => {
                        // Cactus uses custom UV-mapped mesh (no scaling needed, already 64x64x64)
                        cactus_mesh.clone()
                    }
                    BlockType::Sandstone => {
                        // Sandstone uses custom UV-mapped mesh (no scaling needed, already 64x64x64)
                        sandstone_mesh.clone()
                    }
                    BlockType::DeadBush => mesh_cache.dead_bush_mesh.clone(),
                    BlockType::Cobweb => {
                        // Cobweb is a flat 64x64 rectangle (billboard facing forward)
                        meshes.add(Rectangle::new(64.0, 64.0))
                    }
                    _ => {
                        // Uniform blocks use standard cube (scaled to 64x64x64)
                        transform.scale = Vec3::splat(64.0);
                        cube_mesh.clone()
                    }
                }
            };

            // Get cached material for this block type
            let material = if forest_surface {
                material_cache.forest_floor.clone().unwrap()
            } else {
                material_cache
                    .get(block_type)
                    .expect("Material should be cached for all block types")
            };

            let mut entity_commands = commands.spawn((
                Block {
                    health: block_type.starting_health(),
                    max_health: block_type.starting_health(),
                },
                block_type,
                ChunkMember { chunk_x, chunk_y },
                // Blocks do NOT use Avian - spatial hash is much faster for 15k+ static blocks
                Mesh3d(block_mesh),
                MeshMaterial3d(material),
                transform,
            ));

            // Add CobwebBlock marker for cobweb blocks
            if block_type == BlockType::Cobweb {
                entity_commands.insert(crate::components::block::CobwebBlock);
            }
        }
    }
    if let Some(site) = encounter {
        super::biome_encounters::spawn_chunk(
            commands,
            site,
            IVec2::new(chunk_x, chunk_y),
            mesh_cache,
            material_cache,
        );
    }
    if let Some(origin) = portal_site {
        crate::systems::world_portals::spawn_ruin(
            commands,
            origin,
            biome,
            mesh_cache,
            material_cache,
        );
    }
    for tree in tree {
        super::forest_trees::spawn(
            commands,
            &tree,
            IVec2::new(chunk_x, chunk_y),
            mesh_cache,
            material_cache,
        );
    }
}

/// Get cave decoration block type if this position should have decorations
/// Returns None if no decoration, Some(BlockType) for stalactites, columns, ore veins, etc.
fn get_cave_decoration(
    block_x: i32,
    block_y: i32,
    biome: BiomeType,
    world_seed: &crate::resources::world_seed::WorldSeed,
) -> Option<BlockType> {
    let x = block_x as f64;
    let y = block_y as f64;

    // Noise tables are built once per seed and thread, not for every cave cell
    // (same pattern as world_shapes).
    /// (simplex seed, perlin seed) and the generators built from them
    type SeededNoise = Option<((u32, u32), Simplex, Perlin)>;
    thread_local! {
        static NOISE: std::cell::RefCell<SeededNoise> = const { std::cell::RefCell::new(None) };
    }
    let seeds = (world_seed.simplex_seed(), world_seed.perlin_seed());
    let (simplex, perlin) = NOISE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.as_ref().is_none_or(|(s, _, _)| *s != seeds) {
            *cache = Some((seeds, Simplex::new(seeds.0), Perlin::new(seeds.1)));
        }
        let (_, simplex, perlin) = cache.as_ref().unwrap();
        (*simplex, *perlin)
    });

    // Check if adjacent blocks form ceiling/floor (for stalactites/stalagmites)
    let has_ceiling_above = !should_generate_cave(block_x, block_y + 1, biome, world_seed);
    let has_floor_below = !should_generate_cave(block_x, block_y - 1, biome, world_seed);

    // Stalactites (hang from ceiling)
    if has_ceiling_above {
        let stalactite_noise = simplex.get([x * 0.2, y * 0.2, 5000.0]);
        if stalactite_noise > 0.75 {
            return Some(match biome {
                BiomeType::Iceland => BlockType::Snow,      // Icicles
                BiomeType::Nether => BlockType::Netherrack, // Lava drips
                BiomeType::End => BlockType::Obsidian,      // Void crystals
                BiomeType::Desert => BlockType::Sandstone,  // Sandstone formations
                _ => BlockType::Stone,                      // Stone stalactites
            });
        }
    }

    // Stalagmites (grow from floor)
    if has_floor_below {
        let stalagmite_noise = simplex.get([x * 0.2, y * 0.2, 6000.0]);
        if stalagmite_noise > 0.75 {
            return Some(match biome {
                BiomeType::Iceland => BlockType::Snow,     // Ice spikes
                BiomeType::Nether => BlockType::Glowstone, // Glowing formations
                BiomeType::End => BlockType::Purpur,       // End pillars
                _ => BlockType::Stone,                     // Stone stalagmites
            });
        }
    }

    // Columns (stalactite meets stalagmite)
    if has_ceiling_above && has_floor_below {
        let column_noise = perlin.get([x * 0.15, y * 0.15, 7000.0]);
        if column_noise > 0.8 {
            return Some(match biome {
                BiomeType::Mountains => BlockType::Stone,
                BiomeType::Underground => BlockType::Stone,
                BiomeType::Iceland => BlockType::Snow,
                BiomeType::Nether => BlockType::Netherrack,
                _ => BlockType::Stone,
            });
        }
    }

    // Ore veins in cave walls (richer ore deposits exposed)
    let ore_vein_noise = simplex.get([x * 0.25, y * 0.25, 8000.0]);
    if ore_vein_noise > 0.82 {
        let ores = biome.ore_blocks();
        if !ores.is_empty() {
            // Bias towards rarer ores in caves (reward exploration)
            let rare_ore_index = if ores.len() > 2 {
                ores.len() - 1 // Pick rarest ore
            } else if ores.len() > 1 {
                1 // Pick second ore
            } else {
                0 // Pick only ore
            };
            return Some(ores[rare_ore_index]);
        }
    }

    // Glowing mushrooms/crystals in dark caves
    if matches!(biome, BiomeType::Underground | BiomeType::Nether) {
        let glow_noise = simplex.get([x * 0.3, y * 0.3, 9000.0]);
        if glow_noise > 0.85 {
            return Some(match biome {
                BiomeType::Nether => BlockType::Glowstone,
                _ => BlockType::Redstone, // Glowing redstone ore
            });
        }
    }

    // Cobwebs in forest/underground caves
    if matches!(biome, BiomeType::Forest | BiomeType::Underground) {
        let web_noise = simplex.get([x * 0.4, y * 0.4, 10000.0]);
        if web_noise > 0.88 {
            return Some(BlockType::Cobweb);
        }
    }

    None // No decoration at this position
}

/// Check if a block should be a cave (empty space) - Natural caves using Perlin noise, worm carving, and multi-layer approach
pub(crate) fn should_generate_cave(
    x: i32,
    y: i32,
    biome: BiomeType,
    seed: &crate::resources::world_seed::WorldSeed,
) -> bool {
    super::biome_content::terrain::open(x, y, biome, seed.seed)
}
pub(crate) fn seeded_block(biome: BiomeType, x: i32, y: i32, seed: u32) -> BlockType {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(super::world_shapes::hash(x, y, seed) as u64);
    determine_block_type(biome, x, y, &mut rng)
}
pub(crate) fn decoration(
    x: i32,
    y: i32,
    biome: BiomeType,
    seed: &crate::resources::world_seed::WorldSeed,
) -> Option<BlockType> {
    cactus_at(x, y, biome, seed)
        .or_else(|| ground_plant_at(x, y, biome, seed))
        .or_else(|| get_cave_decoration(x, y, biome, seed))
}
/// Determine block type based on biome and position
fn determine_block_type(
    biome: BiomeType,
    _block_x: i32,
    block_y: i32,
    rng: &mut impl RngExt,
) -> BlockType {
    // Chance for decorative blocks (only near surface)
    if block_y > -100 && rng.random_bool(0.05) {
        let decorative = biome.decorative_blocks();
        if !decorative.is_empty() {
            let kind = decorative[rng.random_range(0..decorative.len())];
            // Loose vegetation belongs on cave floors, never inside the solid wall.
            if !matches!(
                kind,
                BlockType::Cactus | BlockType::DeadBush | BlockType::Mushroom | BlockType::Vines
            ) {
                return kind;
            }
        }
    }

    // Desert: Mix of sand and sandstone throughout (not depth-dependent)
    if matches!(biome, BiomeType::Desert) {
        // Random mix of sand and sandstone blocks
        let terrain_chance = rng.random_range(0..100);

        if terrain_chance < 60 {
            // 60% Sand
            return BlockType::Sand;
        } else {
            // 40% Sandstone or ores
            let ore_chance = rng.random_range(0..1000);
            let ores = biome.ore_blocks();

            return match ore_chance {
                0..=850 => BlockType::Sandstone, // Most common
                851..=920 => {
                    if !ores.is_empty() {
                        ores[0]
                    } else {
                        BlockType::Sandstone
                    }
                }
                921..=970 => {
                    if ores.len() > 1 {
                        ores[rng.random_range(0..2.min(ores.len()))]
                    } else {
                        BlockType::Sandstone
                    }
                }
                971..=995 => {
                    if ores.len() > 2 {
                        ores[rng.random_range(1..ores.len())]
                    } else {
                        BlockType::Sandstone
                    }
                }
                _ => {
                    if ores.len() > 3 {
                        ores[ores.len() - 1]
                    } else {
                        BlockType::Sandstone
                    }
                }
            };
        }
    }

    // Ore generation (more common in deeper areas)
    let _depth_factor = (-block_y).max(0) as u32;
    let ore_chance = rng.random_range(0..1000);

    // Use biome-specific ores
    let ores = biome.ore_blocks();

    let block_type = match ore_chance {
        0..=900 => biome.terrain_block(), // 90% terrain block
        901..=950 => {
            if !ores.is_empty() {
                ores[0]
            } else {
                biome.terrain_block()
            }
        } // 5% common ore (coal/iron)
        951..=980 => {
            if ores.len() > 1 {
                ores[rng.random_range(0..2.min(ores.len()))]
            } else {
                biome.terrain_block()
            }
        } // 3% uncommon ore
        981..=995 => {
            if ores.len() > 2 {
                ores[rng.random_range(1..ores.len())]
            } else {
                biome.terrain_block()
            }
        } // 1.5% rare ore
        _ => {
            if ores.len() > 3 {
                ores[ores.len() - 1]
            } else {
                biome.terrain_block()
            }
        } // 0.5% very rare (gold/diamond/ancient debris)
    };

    block_type
}

pub fn get_block_color(block_type: BlockType) -> Color {
    match block_type {
        // Terrain blocks
        BlockType::Stone => Color::srgb(0.5, 0.5, 0.5),
        BlockType::Dirt => Color::srgb(0.55, 0.35, 0.2),
        BlockType::Grass => Color::srgb(0.2, 0.8, 0.2),
        BlockType::Sand => Color::srgb(0.95, 0.9, 0.6),
        BlockType::Sandstone => Color::srgb(0.85, 0.75, 0.5),
        BlockType::Ice => Color::srgb(0.65, 0.85, 1.),
        BlockType::Snow => Color::srgb(0.95, 0.95, 1.0),

        // Ore blocks
        BlockType::Coal => Color::srgb(0.3, 0.3, 0.3),
        BlockType::Iron => Color::srgb(0.7, 0.7, 0.7),
        BlockType::Gold => Color::srgb(1.0, 0.84, 0.0),
        BlockType::Diamond => Color::srgb(0.0, 0.8, 1.0),
        BlockType::Emerald => Color::srgb(0.0, 1.0, 0.3),
        BlockType::Redstone => Color::srgb(1.0, 0.0, 0.0),

        // Decorative blocks
        BlockType::Wood => Color::srgb(0.4, 0.25, 0.1),
        BlockType::Leaves => Color::srgb(0.1, 0.6, 0.1),
        BlockType::Pumpkin => Color::srgb(1.0, 0.5, 0.0),
        BlockType::Melon => Color::srgb(0.6, 1.0, 0.3),
        BlockType::Cactus => Color::srgb(0.2, 0.7, 0.2),
        BlockType::DeadBush => Color::srgb(0.6, 0.5, 0.3),
        BlockType::Mushroom => Color::srgb(0.8, 0.3, 0.3),
        BlockType::Vines => Color::srgb(0.15, 0.5, 0.15),
        BlockType::Cobweb => Color::srgb(0.9, 0.9, 0.9), // Light gray/white

        // Nether blocks
        BlockType::Netherrack => Color::srgb(0.6, 0.2, 0.2), // Dark red
        BlockType::Netherite => Color::srgb(0.3, 0.25, 0.35), // Dark purple-gray
        BlockType::AncientDebris => Color::srgb(0.4, 0.3, 0.25), // Brown/dark (rare)
        BlockType::SoulSand => Color::srgb(0.35, 0.25, 0.2), // Dark brown
        BlockType::Glowstone => Color::srgb(1.0, 0.9, 0.5),  // Bright yellow

        // End blocks
        BlockType::EndStone => Color::srgb(0.9, 0.9, 0.7), // Pale yellow
        BlockType::Obsidian => Color::srgb(0.1, 0.0, 0.15), // Very dark purple
        BlockType::ChorusFruit => Color::srgb(0.7, 0.3, 0.7), // Purple
        BlockType::Purpur => Color::srgb(0.6, 0.4, 0.6),   // Light purple
    }
}

/// Spawn enemies in a chunk based on biome - spawn in caves only
fn spawn_chunk_enemies(
    commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    chunk_x: i32,
    chunk_y: i32,
    biome: BiomeType,
    mesh_cache: &MeshCache,
    enemy_textures: &crate::resources::asset_handles::EnemyTextures,
    rng: &mut impl RngExt,
    world_seed: &crate::resources::world_seed::WorldSeed,
    tree: &[super::forest_trees::TreePlan],
) {
    let spawn_rate = biome.enemy_spawn_rate();
    let enemy_types = biome.enemy_types();

    if enemy_types.is_empty() {
        return;
    }

    // Spawn enemies per chunk based on spawn rate - ONLY in caves/empty spaces
    if rng.random_bool(spawn_rate as f64) {
        let num_enemies = rng.random_range(1..=2); // Small readable encounters; landmarks add specialist foes

        for _ in 0..num_enemies {
            // Try to find a cave/empty position (not inside a block)
            let mut spawn_pos = None;
            for _ in 0..50 {
                let local_x = rng.random_range(0..CHUNK_SIZE);
                let local_y = rng.random_range(0..CHUNK_SIZE);

                let world_block_x = chunk_x * CHUNK_SIZE + local_x;
                let world_block_y = chunk_y * CHUNK_SIZE + local_y;

                if tree
                    .iter()
                    .any(|t| t.reserved(IVec2::new(world_block_x, world_block_y)))
                {
                    continue;
                }
                if crate::systems::world_portals::site(chunk_x, chunk_y, world_seed.seed)
                    .is_some_and(|p| {
                        crate::systems::world_portals::reserved(
                            IVec2::new(world_block_x, world_block_y),
                            p,
                        )
                    })
                {
                    continue;
                }
                if super::biome_encounters::site(chunk_x, chunk_y, biome, world_seed.seed)
                    .is_some_and(|s| s.reserved(IVec2::new(world_block_x, world_block_y)))
                {
                    continue;
                }
                // Check if this position is a cave (empty space)
                if should_generate_cave(world_block_x, world_block_y, biome, world_seed) {
                    spawn_pos = Some((world_block_x, world_block_y));
                    break;
                }
            }

            // If we found an empty cave position, spawn the enemy
            if let Some((world_block_x, world_block_y)) = spawn_pos {
                let world_x = world_block_x as f32 * TOTAL_BLOCK_SIZE;
                let world_y = world_block_y as f32 * TOTAL_BLOCK_SIZE;

                // Pick random enemy type from biome
                let mut enemy_type = enemy_types[rng.random_range(0..enemy_types.len())];
                // Eight legs need an open pocket, not a one-cell shaft.
                if enemy_type == crate::components::enemy::EnemyType::Spider
                    && !(-1..=1).all(|dx| {
                        (0..=1).all(|dy| {
                            should_generate_cave(
                                world_block_x + dx,
                                world_block_y + dy,
                                biome,
                                world_seed,
                            )
                        })
                    })
                {
                    enemy_type = crate::components::enemy::EnemyType::Skeleton;
                }
                // Specialist encounters need their habitat, not arbitrary pockets in the biome.
                if enemy_type == crate::components::enemy::EnemyType::Bogged
                    && !super::biome_content::terrain::damp_grove(
                        world_block_x,
                        world_block_y,
                        world_seed.seed,
                    )
                {
                    enemy_type = crate::components::enemy::EnemyType::Skeleton;
                }
                if matches!(
                    enemy_type,
                    crate::components::enemy::EnemyType::Shulker
                        | crate::components::enemy::EnemyType::Breeze
                ) && !(-3..=3).any(|dx| {
                    !should_generate_cave(world_block_x + dx, world_block_y - 1, biome, world_seed)
                }) {
                    continue;
                }
                if super::biome_content::terrain::feature(
                    world_block_x,
                    world_block_y,
                    biome,
                    world_seed.seed,
                )
                .is_some()
                {
                    continue;
                }
                let max_health = enemy_type.max_health();

                // Get texture for enemy type
                let texture = match enemy_type {
                    crate::components::enemy::EnemyType::Bat => enemy_textures.bat.clone(),
                    crate::components::enemy::EnemyType::Spider => enemy_textures.spider.clone(),
                    crate::components::enemy::EnemyType::Enderman
                    | crate::components::enemy::EnemyType::Sentinel => {
                        enemy_textures.enderman.clone()
                    }
                    crate::components::enemy::EnemyType::Skeleton => {
                        if rng.random_bool(0.5) {
                            enemy_textures.skeleton.clone()
                        } else {
                            enemy_textures.skeleton2.clone()
                        }
                    }
                    crate::components::enemy::EnemyType::Yeti => enemy_textures.yeti.clone(),
                    _ => enemy_textures.skeleton.clone(), // Fallback for other types
                };

                let material = materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(texture),
                    unlit: true,
                    alpha_mode: AlphaMode::Mask(0.5),
                    ..default()
                });

                // Give enemies initial velocity to bounce around - upward boost to escape caves
                let initial_velocity = Vec3::new(
                    rng.random_range(-100.0..100.0), // Random horizontal velocity
                    rng.random_range(100.0..200.0),  // Upward velocity to pop out of spawn area
                    0.0,
                );

                // Use flying physics for dragon, enemy physics with initial velocity for others
                let physics_components = if matches!(
                    enemy_type,
                    crate::components::enemy::EnemyType::Dragon
                        | crate::components::enemy::EnemyType::Bat
                ) {
                    avian_physics::flying_enemy()
                } else {
                    avian_physics::enemy_with_velocity(initial_velocity)
                };

                commands.spawn((
                    Enemy::new(enemy_type),
                    Health::new(max_health),
                    crate::components::buff::Buffable,
                    ChunkMember { chunk_x, chunk_y },
                    physics_components, // Avian physics with initial velocity
                    SpriteBundle3d::new(
                        mesh_cache.enemy_mesh.clone(),
                        material,
                        Vec3::new(world_x, world_y, 10.0),
                    ),
                ));

                info!(
                    "Spawned {:?} at world({}, {}) chunk({}, {}) in {:?} biome",
                    enemy_type, world_x, world_y, chunk_x, chunk_y, biome
                );
            }
        }
    }
}

fn cactus_height(x: i32, base_y: i32, seed: u32) -> i32 {
    let h = (x as u32).wrapping_mul(73856093) ^ (base_y as u32).wrapping_mul(19349663) ^ seed;
    if h % 5 != 0 { 0 } else { ((h >> 8) % 4) as i32 }
}
fn cactus_at(
    x: i32,
    y: i32,
    biome: BiomeType,
    seed: &crate::resources::world_seed::WorldSeed,
) -> Option<BlockType> {
    if biome != BiomeType::Desert {
        return None;
    }
    for height in 1..=3 {
        let ground = y - height;
        let full_height = cactus_height(x, ground, seed.seed);
        if full_height >= height
            && !should_generate_cave(x, ground, biome, seed)
            && (-1..=1).all(|dx| {
                (1..=full_height + 1).all(|dy| {
                    should_generate_cave(x + dx, ground + dy, biome, seed)
                        && get_cave_decoration(x + dx, ground + dy, biome, seed).is_none()
                })
            })
        {
            return Some(BlockType::Cactus);
        }
    }
    None
}

/// Sparse supported plants with a three-cell-wide, two-cell-high clear pocket.
fn ground_plant_at(
    x: i32,
    y: i32,
    biome: BiomeType,
    seed: &crate::resources::world_seed::WorldSeed,
) -> Option<BlockType> {
    let kind = match biome {
        BiomeType::Desert => BlockType::DeadBush,
        BiomeType::Forest | BiomeType::Underground => BlockType::Mushroom,
        _ => return None,
    };
    let h = (x as u32).wrapping_mul(83492791) ^ (y as u32).wrapping_mul(2971215073) ^ seed.seed;
    if h % 4 != 0 || should_generate_cave(x, y - 1, biome, seed) {
        return None;
    }
    let clear = (-1..=1).all(|dx| {
        (0..=1).all(|dy| {
            should_generate_cave(x + dx, y + dy, biome, seed)
                && get_cave_decoration(x + dx, y + dy, biome, seed).is_none()
                && cactus_at(x + dx, y + dy, biome, seed).is_none()
        })
    });
    clear.then_some(kind)
}

#[cfg(test)]
mod cactus_tests {
    use super::*;
    #[test]
    fn stalk_heights_include_absent_and_one_to_three_segments() {
        let heights: std::collections::HashSet<_> =
            (-1000..1000).map(|x| cactus_height(x, 0, 42)).collect();
        assert_eq!(heights, std::collections::HashSet::from([0, 1, 2, 3]));
        assert_eq!(BlockType::Sand.starting_health(), 2);
    }
    #[test]
    fn plants_have_support_and_clearance_and_never_replace_solid_wall() {
        let seed = crate::resources::world_seed::WorldSeed::new(42);
        let mut count = 0;
        for biome in [BiomeType::Desert, BiomeType::Forest, BiomeType::Underground] {
            let mut rng = rand::rng();
            for x in -65..65 {
                for y in -65..65 {
                    assert!(!matches!(
                        determine_block_type(biome, x, y, &mut rng),
                        BlockType::DeadBush
                            | BlockType::Mushroom
                            | BlockType::Cactus
                            | BlockType::Vines
                    ));
                    if ground_plant_at(x, y, biome, &seed).is_some() {
                        count += 1;
                        assert!(!should_generate_cave(x, y - 1, biome, &seed));
                        for dx in -1..=1 {
                            for dy in 0..=1 {
                                assert!(should_generate_cave(x + dx, y + dy, biome, &seed));
                                assert!(
                                    get_cave_decoration(x + dx, y + dy, biome, &seed).is_none()
                                );
                            }
                        }
                    }
                }
            }
        }
        assert!(
            count > 10,
            "Clearance must still allow plants to generate: {count}"
        );
    }
}
