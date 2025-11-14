use bevy::prelude::*;
use rand::Rng;
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
                generate_chunk(&mut commands, &mut meshes, &mut materials, chunk_x, chunk_y, &block_textures, &material_cache, &mesh_cache, &enemy_textures, &spawn_area);
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
    spawn_chunk_enemies(commands, meshes, materials, chunk_x, chunk_y, biome, &mesh_cache, enemy_textures, &mut rng);

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
            if should_generate_cave(world_block_x, world_block_y, biome) {
                continue; // Skip this block, it's part of a cave
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

/// Check if a block should be a cave (empty space) - Corridor-based caves with biome-specific shapes
fn should_generate_cave(block_x: i32, block_y: i32, biome: BiomeType) -> bool {
    let x = block_x as f32;
    let y = block_y as f32;

    // Biome-specific corridor patterns
    match biome {
        BiomeType::Nether => {
            // Nether: Vertical wavy corridors (lava tubes)
            let corridor_center = ((y * 0.015).sin() * 20.0) as i32;
            let distance_from_corridor = (x as i32 - corridor_center).abs();
            let corridor_width = 2 + ((y * 0.05).cos().abs() * 2.0) as i32; // 2-4 blocks wide

            // Multiple parallel vertical corridors
            let is_vertical_corridor = distance_from_corridor < corridor_width;

            // Add some small pocket rooms
            let pocket_noise = ((x * 0.12).sin() * (y * 0.1).cos()).abs();
            let is_pocket = pocket_noise < 0.08;

            is_vertical_corridor || is_pocket
        },

        BiomeType::End => {
            // End: Straight diagonal corridors (crystalline geometry)
            let diagonal1 = ((x + y) % 100.0).abs();
            let diagonal2 = ((x - y) % 100.0).abs();
            let is_diagonal = diagonal1 < 3.0 || diagonal2 < 3.0;

            // Very sparse - only show corridors sometimes
            let sparse_check = ((x / 80.0).sin() * (y / 80.0).cos()).abs() > 0.6;

            is_diagonal && sparse_check
        },

        BiomeType::Underground => {
            // Underground: Winding horizontal and vertical corridors (classic dungeon)
            // Horizontal corridors
            let h_corridor_center = ((x * 0.012).sin() * 15.0) as i32;
            let h_distance = (y as i32 - h_corridor_center).abs();
            let is_h_corridor = h_distance < 2; // 2-3 blocks tall

            // Vertical corridors
            let v_corridor_center = ((y * 0.01).cos() * 15.0) as i32;
            let v_distance = (x as i32 - v_corridor_center).abs();
            let is_v_corridor = v_distance < 2; // 2-3 blocks wide

            // Larger rooms at intersections
            let room_noise = ((x * 0.08).sin() * (y * 0.09).cos()).abs();
            let is_room = room_noise < 0.12;

            is_h_corridor || is_v_corridor || is_room
        },

        BiomeType::Mountains => {
            // Mountains: Horizontal mining-shaft corridors at different heights
            let shaft_height_1 = ((x * 0.008).sin() * 10.0) as i32;
            let shaft_height_2 = ((x * 0.015 + 100.0).cos() * 12.0) as i32 + 30;

            let dist1 = (y as i32 - shaft_height_1).abs();
            let dist2 = (y as i32 - shaft_height_2).abs();

            let is_shaft = dist1 < 2 || dist2 < 2; // 2-3 blocks tall horizontal shafts

            // Small caverns
            let cavern_noise = ((x * 0.1).sin() * (y * 0.11).cos()).abs();
            let is_cavern = cavern_noise < 0.09;

            is_shaft || is_cavern
        },

        BiomeType::Iceland => {
            // Iceland: Jagged irregular corridors (ice cracks)
            let crack_center = ((x * 0.02).sin() * 8.0) as i32;
            let distance_from_crack = (y as i32 - crack_center).abs();

            // Thin jagged corridors (1-2 blocks)
            let crack_width = 1 + ((x * 0.1).cos().abs() * 1.0) as i32;
            let is_crack = distance_from_crack < crack_width;

            // Very small pockets
            let pocket_noise = ((x * 0.15).sin() * (y * 0.16).cos()).abs();
            let is_pocket = pocket_noise < 0.07;

            is_crack || is_pocket
        },

        BiomeType::Desert => {
            // Desert: Sparse horizontal sandstone corridors (tomb-like)
            let tomb_center = ((x * 0.01).cos() * 12.0) as i32;
            let distance_from_tomb = (y as i32 - tomb_center).abs();

            // Sparse distribution
            let sparse = ((x / 120.0).fract() * (y / 80.0).fract()).abs() > 0.7;
            let is_corridor = distance_from_tomb < 2 && sparse;

            // Small tomb rooms
            let room_noise = ((x * 0.11).sin() * (y * 0.12).cos()).abs();
            let is_tomb_room = room_noise < 0.08 && sparse;

            is_corridor || is_tomb_room
        },

        BiomeType::Forest => {
            // Forest: Twisting organic root corridors (irregular and curved)
            let root_center_1 = ((x * 0.018).sin() * 10.0) as i32;
            let root_center_2 = ((y * 0.016).cos() * 10.0) as i32;

            let dist_h = (y as i32 - root_center_1).abs();
            let dist_v = (x as i32 - root_center_2).abs();

            // Thin twisting corridors (1-2 blocks)
            let is_root = dist_h < 2 || dist_v < 2;

            // Root cavities (small rooms)
            let cavity_noise = ((x * 0.13).sin() * (y * 0.14).cos()).abs();
            let is_cavity = cavity_noise < 0.1;

            is_root || is_cavity
        },

        BiomeType::Plains => {
            // Plains: Rare shallow horizontal burrows
            let burrow_center = ((x * 0.009).sin() * 8.0) as i32;
            let distance = (y as i32 - burrow_center).abs();

            // Very sparse
            let sparse = ((x / 150.0).fract()).abs() > 0.85;

            distance < 2 && sparse
        },
    }
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
                if should_generate_cave(world_block_x, world_block_y, biome) {
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
