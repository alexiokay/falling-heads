use bevy::prelude::*;
use rand::Rng;
use noise::{NoiseFn, Perlin, Simplex};
use crate::components::block::{Block, BlockType, BiomeType, BiomeBehavior};
use crate::components::chunk::{Chunk, ChunkMember, LoadedChunks, CHUNK_SIZE, world_to_chunk_coords};
use crate::components::hero::Hero;
use crate::components::enemy::{Enemy, Health, EnemyBehavior};
use crate::components::bundles::{SpriteBundle3d, avian_physics};
use crate::resources::asset_handles::BlockTextures;
use crate::resources::material_cache::MaterialCache;
use crate::resources::mesh_cache::MeshCache;
use crate::utils::biome_utils::determine_biome_at_chunk;
use crate::utils::constants::BLOCK_SIZE;
// avian3d not needed for world generation (blocks don't use Avian physics)

// World generation constants - can be adjusted independently from rendering
const WORLD_BLOCK_SIZE: f32 = BLOCK_SIZE;  // World units per block (matches BLOCK_SIZE from constants.rs = 64.0)
const BLOCK_SPACING: f32 = 0.0;  // No spacing - blocks touch
const TOTAL_BLOCK_SIZE: f32 = WORLD_BLOCK_SIZE + BLOCK_SPACING;  // Total size including spacing (64.0 + 0.0 = 64.0)

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
) {
    // Wait for caches to be initialized before spawning blocks
    // MaterialCache now includes atlas generation, so when materials are ready, atlases are too
    if material_cache.materials.is_empty() || mesh_cache.is_none() || enemy_textures.is_none() {
        return; // Caches not ready yet, wait for next frame
    }

    let mesh_cache = mesh_cache.unwrap();
    let enemy_textures = enemy_textures.unwrap();

    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let hero_pos = hero_transform.translation;
    let (hero_chunk_x, hero_chunk_y) = world_to_chunk_coords(hero_pos.x, hero_pos.y, TOTAL_BLOCK_SIZE);

    // Generate chunks in a radius around the hero
    let chunk_radius = 3;

    for chunk_x in (hero_chunk_x - chunk_radius)..=(hero_chunk_x + chunk_radius) {
        for chunk_y in (hero_chunk_y - chunk_radius)..=(hero_chunk_y + chunk_radius) {
            if !loaded_chunks.is_loaded(chunk_x, chunk_y) {
                generate_chunk(&mut commands, &mut meshes, &mut materials, chunk_x, chunk_y, &block_textures, &material_cache, &mesh_cache, &enemy_textures, &spawn_area, &world_seed);
                loaded_chunks.add(chunk_x, chunk_y);
            }
        }
    }
}

/// Despawn chunks that are too far from the hero
pub fn despawn_distant_chunks(
    mut commands: Commands,
    hero_query: Query<&Transform, With<Hero>>,
    chunk_query: Query<(Entity, &Chunk)>,
    block_query: Query<(Entity, &ChunkMember)>,
    mut loaded_chunks: ResMut<LoadedChunks>,
) {
    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let hero_pos = hero_transform.translation;
    let (hero_chunk_x, hero_chunk_y) = world_to_chunk_coords(hero_pos.x, hero_pos.y, TOTAL_BLOCK_SIZE);

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
        let mut total_blocks_despawned = 0;
        let num_chunks = chunks_to_despawn.len();

        for (chunk_entity, chunk_x, chunk_y) in chunks_to_despawn {
            // Despawn all blocks in this chunk
            let mut blocks_in_chunk = 0;
            for (block_entity, member) in block_query.iter() {
                if member.chunk_x == chunk_x && member.chunk_y == chunk_y {
                    commands.entity(block_entity).despawn(); // Despawn children too (damage overlays, borders)
                    blocks_in_chunk += 1;
                }
            }
            total_blocks_despawned += blocks_in_chunk;

            // Despawn chunk entity
            commands.entity(chunk_entity).despawn();
            loaded_chunks.remove(chunk_x, chunk_y);
        }

        info!("Despawned {} distant chunks ({} blocks total)", num_chunks, total_blocks_despawned);
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
    let mut rng = rand::thread_rng();

    // Determine biome based on chunk position
    let biome = determine_biome_at_chunk(chunk_x, chunk_y);

    // Spawn chunk entity marker
    commands.spawn(Chunk {
        chunk_x,
        chunk_y,
        biome,
    });

    // Spawn enemies based on biome
    spawn_chunk_enemies(commands, meshes, materials, chunk_x, chunk_y, biome, &mesh_cache, enemy_textures, &mut rng, world_seed);

    // Use shared cube mesh from cache - CRITICAL PERFORMANCE FIX
    let cube_mesh = mesh_cache.block_mesh.clone();
    let pumpkin_mesh = mesh_cache.pumpkin_mesh.clone();
    let wood_mesh = mesh_cache.wood_mesh.clone();
    let leaves_mesh = mesh_cache.leaves_mesh.clone();
    let melon_mesh = mesh_cache.melon_mesh.clone();
    let cactus_mesh = mesh_cache.cactus_mesh.clone();
    let sandstone_mesh = mesh_cache.sandstone_mesh.clone();

    // Generate blocks in this chunk
    for local_x in 0..CHUNK_SIZE {
        for local_y in 0..CHUNK_SIZE {
            let world_block_x = chunk_x * CHUNK_SIZE + local_x;
            let world_block_y = chunk_y * CHUNK_SIZE + local_y;

            let world_x = world_block_x as f32 * TOTAL_BLOCK_SIZE;
            let world_y = world_block_y as f32 * TOTAL_BLOCK_SIZE;

            // Skip blocks in the spawn area (keep spawn area clear)
            let distance_to_spawn = ((world_x - spawn_area.center.x).powi(2) + (world_y - spawn_area.center.y).powi(2)).sqrt();
            if distance_to_spawn < spawn_area.radius {
                continue; // Skip this block, it's in the spawn area
            }

            // Cave generation check (biome-specific)
            let is_cave = should_generate_cave(world_block_x, world_block_y, biome, world_seed);

            if is_cave {
                // Try to spawn cave decorations (stalactites, columns, etc) in empty cave spaces
                if let Some(decoration_type) = get_cave_decoration(world_block_x, world_block_y, biome, world_seed) {
                    // Spawn decoration block at this position
                    let world_z = -10.0;
                    let mut transform = Transform::from_xyz(world_x, world_y, world_z);
                    transform.scale = Vec3::splat(64.0);

                    let material = material_cache.get(decoration_type)
                        .expect("Material should be cached for all block types");

                    commands.spawn((
                        Block {
                            health: 2,  // Decorations are more fragile
                            max_health: 2,
                        },
                        decoration_type,
                        ChunkMember { chunk_x, chunk_y },
                        Mesh3d(cube_mesh.clone()),
                        MeshMaterial3d(material),
                        transform,
                    ));
                }
                continue; // Skip normal block, it's part of a cave
            }

            // Determine block type
            let block_type = determine_block_type(biome, world_block_x, world_block_y, &mut rng);

            let world_z = -10.0;  // Behind entities so they render in front of blocks

            // Render as 3D cubes scaled to match 2D
            let mut transform = Transform::from_xyz(world_x, world_y, world_z);

            // Choose mesh based on block type (directional blocks use UV-mapped mesh)
            let block_mesh = match block_type {
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
                BlockType::Cobweb => {
                    // Cobweb is a flat 64x64 rectangle (billboard facing forward)
                    meshes.add(Rectangle::new(64.0, 64.0))
                }
                _ => {
                    // Uniform blocks use standard cube (scaled to 64x64x64)
                    transform.scale = Vec3::splat(64.0);
                    cube_mesh.clone()
                }
            };

            // Get cached material for this block type
            let material = material_cache.get(block_type)
                .expect("Material should be cached for all block types");

            let mut entity_commands = commands.spawn((
                Block {
                    health: 3,
                    max_health: 3,
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
}

/// Get cave decoration block type if this position should have decorations
/// Returns None if no decoration, Some(BlockType) for stalactites, columns, ore veins, etc.
fn get_cave_decoration(block_x: i32, block_y: i32, biome: BiomeType, world_seed: &crate::resources::world_seed::WorldSeed) -> Option<BlockType> {
    let x = block_x as f64;
    let y = block_y as f64;

    let simplex = Simplex::new(world_seed.simplex_seed());
    let perlin = Perlin::new(world_seed.perlin_seed());

    // Check if adjacent blocks form ceiling/floor (for stalactites/stalagmites)
    let has_ceiling_above = !should_generate_cave(block_x, block_y + 1, biome, world_seed);
    let has_floor_below = !should_generate_cave(block_x, block_y - 1, biome, world_seed);

    // Stalactites (hang from ceiling)
    if has_ceiling_above {
        let stalactite_noise = simplex.get([x * 0.2, y * 0.2, 5000.0]);
        if stalactite_noise > 0.75 {
            return Some(match biome {
                BiomeType::Iceland => BlockType::Snow,  // Icicles
                BiomeType::Nether => BlockType::Netherrack,  // Lava drips
                BiomeType::End => BlockType::Obsidian,  // Void crystals
                BiomeType::Desert => BlockType::Sandstone,  // Sandstone formations
                _ => BlockType::Stone,  // Stone stalactites
            });
        }
    }

    // Stalagmites (grow from floor)
    if has_floor_below {
        let stalagmite_noise = simplex.get([x * 0.2, y * 0.2, 6000.0]);
        if stalagmite_noise > 0.75 {
            return Some(match biome {
                BiomeType::Iceland => BlockType::Snow,  // Ice spikes
                BiomeType::Nether => BlockType::Glowstone,  // Glowing formations
                BiomeType::End => BlockType::Purpur,  // End pillars
                _ => BlockType::Stone,  // Stone stalagmites
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
                ores.len() - 1  // Pick rarest ore
            } else if ores.len() > 1 {
                1  // Pick second ore
            } else {
                0  // Pick only ore
            };
            return Some(ores[rare_ore_index]);
        }
    }

    // Glowing mushrooms/crystals in dark caves
    if matches!(biome, BiomeType::Underground | BiomeType::Forest | BiomeType::Nether) {
        let glow_noise = simplex.get([x * 0.3, y * 0.3, 9000.0]);
        if glow_noise > 0.85 {
            return Some(match biome {
                BiomeType::Nether => BlockType::Glowstone,
                BiomeType::Forest => BlockType::Mushroom,
                _ => BlockType::Redstone,  // Glowing redstone ore
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

    None  // No decoration at this position
}

/// Check if a block should be a cave (empty space) - Natural caves using Perlin noise, worm carving, and multi-layer approach
fn should_generate_cave(block_x: i32, block_y: i32, biome: BiomeType, world_seed: &crate::resources::world_seed::WorldSeed) -> bool {
    let x = block_x as f64;
    let y = block_y as f64;

    // Create noise generators with random seed for unique worlds
    let perlin = Perlin::new(world_seed.perlin_seed());
    let simplex = Simplex::new(world_seed.simplex_seed());

    // === DEPTH-BASED CAVE DENSITY (Proposal 2) ===
    // Caves get more common deeper underground
    let depth = (-block_y).max(0) as f64;
    let depth_factor = (depth / 500.0).min(1.0); // 0.0 at surface, 1.0 at depth 500+

    // === LAYER 1: LARGE CAVERNS (3D Perlin noise for vast chambers) ===
    let cavern_scale = match biome {
        BiomeType::Nether => 0.015,  // Huge lava chambers
        BiomeType::Forest => 0.04,   // Small twisty caves
        _ => 0.02,  // Medium sized features
    };

    let cavern_noise = perlin.get([x * cavern_scale, y * cavern_scale, 0.0]);

    // Base thresholds - RAISED for smaller caves (Proposal 5)
    let base_cavern_threshold = match biome {
        BiomeType::Nether => 0.6,           // Was 0.35 (65% cave) → Now 0.6 (40% cave)
        BiomeType::Underground => 0.65,     // Was 0.4 (60%) → Now 0.65 (35%)
        BiomeType::Mountains => 0.7,        // Was 0.4 (60%) → Now 0.7 (30%)
        BiomeType::Forest => 0.68,          // Was 0.75 (25%) → Now 0.68 (32% for more root caves)
        BiomeType::Desert => 0.8,           // Was 0.55 (45%) → Now 0.8 (20%)
        BiomeType::Plains => 0.85,          // Was 0.65 (35%) → Now 0.85 (15%)
        BiomeType::Iceland => 0.85,         // Was 0.6 (40%) → Now 0.85 (15%)
        BiomeType::End => 0.65,             // Was 0.8 (20%) → Now 0.65 (35% for more caves)
    };

    // Adjust threshold based on depth (easier to carve at depth)
    let cavern_threshold = base_cavern_threshold - (depth_factor * 0.3);

    // === BIOME-SPECIFIC LAYER ENABLE/DISABLE (Proposal 1) ===
    let enable_large_caverns = !matches!(biome, BiomeType::Iceland | BiomeType::Plains);
    let is_large_cavern = enable_large_caverns && cavern_noise > cavern_threshold;

    // === LAYER 2: MEDIUM TUNNELS (Worm-based carving simulation) ===
    // Biome-specific number of worms (Proposal 1)
    let num_worms = match biome {
        BiomeType::Desert => 1,      // 1 worm for tomb corridors (mixed with rectangular rooms)
        BiomeType::End => 1,         // 1 worm for some organic tunnels mixed with geometry
        BiomeType::Plains => 1,      // Simple burrow
        _ => 2,                      // Most biomes get 2 worms (Forest, Underground, Nether, Mountains, Iceland)
    };

    let enable_worm_tunnels = num_worms > 0;
    let mut is_medium_tunnel = false;

    if enable_worm_tunnels {
        // Simulate "worms" that carve paths through terrain using noise to create winding tunnels
        let worm_scale = 0.05;

        // Variable tunnel width based on noise (creates narrow squeezes and wide passages)
        let width_variation = (simplex.get([x * 0.03, y * 0.03, 500.0]) + 1.0) * 0.5; // 0.0 to 1.0

        // Tunnel width - same for both horizontal and vertical (balanced exploration)
        let tunnel_width = match biome {
            BiomeType::Nether => 3.0 + width_variation * 5.0,  // 3-8 blocks (lava tubes)
            BiomeType::Underground => 2.0 + width_variation * 4.0,  // 2-6 blocks
            BiomeType::Mountains => 2.0 + width_variation * 3.0,  // 2-5 blocks (mine shafts)
            BiomeType::Forest => 1.5 + width_variation * 2.5,  // 1.5-4 blocks (root caves)
            BiomeType::Iceland => 1.0 + width_variation * 2.0,  // 1-3 blocks (ice cracks)
            BiomeType::Plains => 1.5 + width_variation * 2.0,  // 1.5-3.5 blocks (burrows)
            BiomeType::End => 2.0 + width_variation * 2.5,  // 2-4.5 blocks (void tunnels)
            _ => 2.0 + width_variation * 3.0,
        };

        // === HORIZONTAL Worm 1 (always active - varies Y based on X) ===
        // Biome-independent for cross-biome connectivity
        let worm_h1_y = simplex.get([x * worm_scale, 5000.0, 0.0]) * 20.0;
        let dist_worm_h1 = ((y - worm_h1_y).powi(2)).sqrt();
        is_medium_tunnel = dist_worm_h1 < tunnel_width;

        // === VERTICAL Worm 1 (always active - varies X based on Y) ===
        let worm_v1_x = simplex.get([y * worm_scale, 1000.0, 0.0]) * 15.0;
        let dist_worm_v1 = ((x - worm_v1_x).powi(2)).sqrt();
        is_medium_tunnel = is_medium_tunnel || dist_worm_v1 < tunnel_width;

        // === Additional worms if num_worms >= 2 (1 horizontal + 1 vertical for balance) ===
        if num_worms >= 2 {
            // HORIZONTAL Worm 2
            let worm_h2_y = simplex.get([x * worm_scale * 0.8, 6000.0, 0.0]) * 18.0;
            let dist_worm_h2 = ((y - worm_h2_y).powi(2)).sqrt();
            is_medium_tunnel = is_medium_tunnel || dist_worm_h2 < tunnel_width;

            // VERTICAL Worm 2
            let worm_v2_x = simplex.get([y * worm_scale * 0.7, 2000.0, 0.0]) * 20.0;
            let dist_worm_v2 = ((x - worm_v2_x).powi(2)).sqrt();
            is_medium_tunnel = is_medium_tunnel || dist_worm_v2 < tunnel_width;
        }
    }

    // === LAYER 3: SMALL CREVICES (High-frequency noise for fine detail) ===
    // Emphasize in Iceland and Mountains, disable in Plains/Desert/Nether
    let enable_small_crevices = matches!(biome, BiomeType::Iceland | BiomeType::End | BiomeType::Mountains | BiomeType::Forest);
    let mut is_small_crevice = false;

    if enable_small_crevices {
        let detail_scale = 0.15;
        let crevice_noise = simplex.get([x * detail_scale, y * detail_scale, 4000.0]);
        let crevice_threshold = match biome {
            BiomeType::Iceland => 0.7,  // Lots of cracks in ice
            BiomeType::End => 0.72,  // Crystalline fractures
            BiomeType::Mountains => 0.75,  // Rocky crevices
            BiomeType::Forest => 0.8,  // Some root cracks
            _ => 0.85,  // Minimal small cracks
        };
        is_small_crevice = crevice_noise > crevice_threshold;
    }

    // === LAYER 4: BIOME-SPECIFIC FEATURES ===
    let biome_feature = match biome {
        BiomeType::Nether => {
            // Vertical lava tubes using directional noise
            let vertical_noise = perlin.get([x * 0.03, 0.0, y * 0.08]);
            vertical_noise > 0.5
        },
        BiomeType::End => {
            // Diagonal void corridors (crystalline geometry)
            let diagonal_noise = simplex.get([(x + y) * 0.04, (x - y) * 0.04, 0.0]);
            diagonal_noise > 0.4  // Was 0.6 → Now 0.4 (more diagonal caves)
        },
        BiomeType::Forest => {
            // Organic root-like branching
            let branch_noise = perlin.get([x * 0.06, y * 0.05, 1000.0]);
            branch_noise > 0.65
        },
        BiomeType::Desert => {
            // Rectangular tomb chambers
            let tomb_x = ((x / 50.0).fract() - 0.5).abs();
            let tomb_y = ((y / 40.0).fract() - 0.5).abs();
            tomb_x < 0.3 && tomb_y < 0.3 && simplex.get([x * 0.01, y * 0.01, 0.0]) > 0.4
        },
        BiomeType::Iceland => {
            // Jagged ice fractures using absolute noise
            let fracture = simplex.get([x * 0.08, y * 0.08, 0.0]).abs();
            fracture < 0.15
        },
        _ => false,
    };

    // === COMBINE ALL LAYERS ===
    // Cave exists if ANY layer creates an opening (OR logic for natural variation)
    is_large_cavern || is_medium_tunnel || is_small_crevice || biome_feature
}

/// Determine block type based on biome and position
fn determine_block_type(biome: BiomeType, _block_x: i32, block_y: i32, rng: &mut impl Rng) -> BlockType {
    // Chance for decorative blocks (only near surface)
    if block_y > -100 && rng.gen_bool(0.05) {
        let decorative = biome.decorative_blocks();
        if !decorative.is_empty() {
            return decorative[rng.gen_range(0..decorative.len())];
        }
    }

    // Desert: Mix of sand and sandstone throughout (not depth-dependent)
    if matches!(biome, BiomeType::Desert) {
        // Random mix of sand and sandstone blocks
        let terrain_chance = rng.gen_range(0..100);

        if terrain_chance < 60 {
            // 60% Sand
            return BlockType::Sand;
        } else {
            // 40% Sandstone or ores
            let ore_chance = rng.gen_range(0..1000);
            let ores = biome.ore_blocks();

            return match ore_chance {
                0..=850 => BlockType::Sandstone, // Most common
                851..=920 => if !ores.is_empty() { ores[0] } else { BlockType::Sandstone },
                921..=970 => if ores.len() > 1 { ores[rng.gen_range(0..2.min(ores.len()))] } else { BlockType::Sandstone },
                971..=995 => if ores.len() > 2 { ores[rng.gen_range(1..ores.len())] } else { BlockType::Sandstone },
                _ => if ores.len() > 3 { ores[ores.len() - 1] } else { BlockType::Sandstone },
            };
        }
    }

    // Ore generation (more common in deeper areas)
    let _depth_factor = (-block_y).max(0) as u32;
    let ore_chance = rng.gen_range(0..1000);

    // Use biome-specific ores
    let ores = biome.ore_blocks();

    let block_type = match ore_chance {
        0..=900 => biome.terrain_block(), // 90% terrain block
        901..=950 => if !ores.is_empty() { ores[0] } else { biome.terrain_block() }, // 5% common ore (coal/iron)
        951..=980 => if ores.len() > 1 { ores[rng.gen_range(0..2.min(ores.len()))] } else { biome.terrain_block() }, // 3% uncommon ore
        981..=995 => if ores.len() > 2 { ores[rng.gen_range(1..ores.len())] } else { biome.terrain_block() }, // 1.5% rare ore
        _ => if ores.len() > 3 { ores[ores.len() - 1] } else { biome.terrain_block() }, // 0.5% very rare (gold/diamond/ancient debris)
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
        BlockType::Cobweb => Color::srgb(0.9, 0.9, 0.9),  // Light gray/white

        // Nether blocks
        BlockType::Netherrack => Color::srgb(0.6, 0.2, 0.2),      // Dark red
        BlockType::Netherite => Color::srgb(0.3, 0.25, 0.35),     // Dark purple-gray
        BlockType::AncientDebris => Color::srgb(0.4, 0.3, 0.25),  // Brown/dark (rare)
        BlockType::SoulSand => Color::srgb(0.35, 0.25, 0.2),      // Dark brown
        BlockType::Glowstone => Color::srgb(1.0, 0.9, 0.5),       // Bright yellow

        // End blocks
        BlockType::EndStone => Color::srgb(0.9, 0.9, 0.7),        // Pale yellow
        BlockType::Obsidian => Color::srgb(0.1, 0.0, 0.15),       // Very dark purple
        BlockType::ChorusFruit => Color::srgb(0.7, 0.3, 0.7),     // Purple
        BlockType::Purpur => Color::srgb(0.6, 0.4, 0.6),          // Light purple
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
    rng: &mut impl Rng,
    world_seed: &crate::resources::world_seed::WorldSeed,
) {
    let spawn_rate = biome.enemy_spawn_rate();
    let enemy_types = biome.enemy_types();

    if enemy_types.is_empty() {
        return;
    }

    // Spawn enemies per chunk based on spawn rate - ONLY in caves/empty spaces
    if rng.gen_bool(spawn_rate as f64) {
        let num_enemies = rng.gen_range(3..=6); // Spawn many enemies

        for _ in 0..num_enemies {
            // Try to find a cave/empty position (not inside a block)
            let mut spawn_pos = None;
            for _ in 0..50 {
                let local_x = rng.gen_range(0..CHUNK_SIZE);
                let local_y = rng.gen_range(0..CHUNK_SIZE);

                let world_block_x = chunk_x * CHUNK_SIZE + local_x;
                let world_block_y = chunk_y * CHUNK_SIZE + local_y;

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
                let enemy_type = enemy_types[rng.gen_range(0..enemy_types.len())];
                let max_health = enemy_type.max_health();

                // Get texture for enemy type
                let texture = match enemy_type {
                    crate::components::enemy::EnemyType::Skeleton => {
                        if rng.gen_bool(0.5) {
                            enemy_textures.skeleton.clone()
                        } else {
                            enemy_textures.skeleton2.clone()
                        }
                    },
                    crate::components::enemy::EnemyType::Yeti => enemy_textures.yeti.clone(),
                    _ => enemy_textures.skeleton.clone(), // Fallback for other types
                };

                let material = materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(texture),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                });

                // Give enemies initial velocity to bounce around - upward boost to escape caves
                let initial_velocity = Vec3::new(
                    rng.gen_range(-100.0..100.0),  // Random horizontal velocity
                    rng.gen_range(100.0..200.0),   // Upward velocity to pop out of spawn area
                    0.0
                );

                // Use flying physics for dragon, enemy physics with initial velocity for others
                let physics_components = if enemy_type == crate::components::enemy::EnemyType::Dragon {
                    avian_physics::flying_enemy()
                } else {
                    avian_physics::enemy_with_velocity(initial_velocity)
                };

                commands.spawn((
                    Enemy::new(enemy_type),
                    Health::new(max_health),
                    crate::components::buff::Buffable,
                    ChunkMember { chunk_x, chunk_y },
                    physics_components,  // Avian physics with initial velocity
                    SpriteBundle3d::new(
                        mesh_cache.enemy_mesh.clone(),
                        material,
                        Vec3::new(world_x, world_y, 10.0),
                    ),
                ));

                info!("Spawned {:?} at world({}, {}) chunk({}, {}) in {:?} biome", enemy_type, world_x, world_y, chunk_x, chunk_y, biome);
            }
        }
    }
}
