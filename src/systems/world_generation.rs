use bevy::prelude::*;
use rand::Rng;
use crate::components::block::{Block, BlockType, BiomeType};
use crate::components::chunk::{Chunk, ChunkMember, LoadedChunks, CHUNK_SIZE, world_to_chunk_coords};
use crate::components::hero::Hero;
use crate::components::enemy::{Enemy, EnemyType, Health};
use crate::components::physics::{Velocity, Gravity, Bouncy};
use crate::resources::asset_handles::BlockTextures;
use crate::resources::material_cache::MaterialCache;
use crate::resources::mesh_cache::MeshCache;

const BLOCK_SIZE: f32 = 1.0;  // 3D cube size
const BLOCK_SPACING: f32 = 0.0;  // No spacing - blocks touch
const TOTAL_BLOCK_SIZE: f32 = BLOCK_SIZE + BLOCK_SPACING;

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
) {
    // Wait for caches to be initialized before spawning blocks
    if material_cache.materials.is_empty() || mesh_cache.is_none() || enemy_textures.is_none() {
        return; // Caches not ready yet, wait for next frame
    }

    let mesh_cache = mesh_cache.unwrap();
    let enemy_textures = enemy_textures.unwrap();

    let Ok(hero_transform) = hero_query.single() else {
        return;
    };

    let hero_pos = hero_transform.translation;
    let (hero_chunk_x, hero_chunk_y) = world_to_chunk_coords(hero_pos.x, hero_pos.y, TOTAL_BLOCK_SIZE * 64.0);

    // Generate chunks in a radius around the hero
    let chunk_radius = 3;

    for chunk_x in (hero_chunk_x - chunk_radius)..=(hero_chunk_x + chunk_radius) {
        for chunk_y in (hero_chunk_y - chunk_radius)..=(hero_chunk_y + chunk_radius) {
            if !loaded_chunks.is_loaded(chunk_x, chunk_y) {
                generate_chunk(&mut commands, &mut meshes, &mut materials, chunk_x, chunk_y, &block_textures, &material_cache, &mesh_cache, &enemy_textures);
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
    let (hero_chunk_x, hero_chunk_y) = world_to_chunk_coords(hero_pos.x, hero_pos.y, TOTAL_BLOCK_SIZE * 64.0);

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
    for (chunk_entity, chunk_x, chunk_y) in chunks_to_despawn {
        // Despawn all blocks in this chunk
        for (block_entity, member) in block_query.iter() {
            if member.chunk_x == chunk_x && member.chunk_y == chunk_y {
                commands.entity(block_entity).despawn();
            }
        }

        // Despawn chunk entity
        commands.entity(chunk_entity).despawn();
        loaded_chunks.remove(chunk_x, chunk_y);
    }
}

/// Generate a single chunk with blocks
fn generate_chunk(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    chunk_x: i32,
    chunk_y: i32,
    _block_textures: &Res<crate::resources::asset_handles::BlockTextures>,
    material_cache: &Res<MaterialCache>,
    mesh_cache: &MeshCache,
    enemy_textures: &crate::resources::asset_handles::EnemyTextures,
) {
    let mut rng = rand::thread_rng();

    // Determine biome based on chunk position
    let biome = determine_biome(chunk_x, chunk_y, &mut rng);

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

    // Generate blocks in this chunk
    for local_x in 0..CHUNK_SIZE {
        for local_y in 0..CHUNK_SIZE {
            let world_block_x = chunk_x * CHUNK_SIZE + local_x;
            let world_block_y = chunk_y * CHUNK_SIZE + local_y;

            // Cave generation check (biome-specific)
            if should_generate_cave(world_block_x, world_block_y, biome) {
                continue; // Skip this block, it's part of a cave
            }

            // Determine block type
            let block_type = determine_block_type(biome, world_block_x, world_block_y, &mut rng);

            let world_x = world_block_x as f32 * TOTAL_BLOCK_SIZE * 64.0;  // Scale to 2D coordinates
            let world_y = world_block_y as f32 * TOTAL_BLOCK_SIZE * 64.0;  // Scale to 2D coordinates
            let world_z = 0.0;

            // Render as 3D cubes scaled to match 2D
            let mut transform = Transform::from_xyz(world_x, world_y, world_z);
            transform.scale = Vec3::splat(64.0);  // Scale cube to 64x64x64 to match 2D block size

            // Get cached material for this block type
            let material = material_cache.get(block_type)
                .expect("Material should be cached for all block types");

            commands.spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                block_type,
                ChunkMember { chunk_x, chunk_y },
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(material),
                transform,
            ));
        }
    }
}

/// Determine biome based on chunk coordinates
fn determine_biome(chunk_x: i32, chunk_y: i32, rng: &mut impl Rng) -> BiomeType {
    // Use simple noise-like pattern based on coordinates
    let biome_seed = (chunk_x * 73 + chunk_y * 179) % 100;

    // Nether - very deep chunks
    if chunk_y < -20 {
        return BiomeType::Nether;
    }

    // End - specific pattern deep underground
    if chunk_y < -15 && biome_seed % 3 == 0 {
        return BiomeType::End;
    }

    // Underground biome for deep chunks
    if chunk_y < -10 {
        return BiomeType::Underground;
    }

    // Mountains for high altitude
    if chunk_y > 5 {
        return BiomeType::Mountains;
    }

    // Surface biomes based on X coordinate patterns
    match biome_seed.abs() {
        0..=25 => BiomeType::Plains,
        26..=45 => BiomeType::Forest,
        46..=65 => BiomeType::Desert,
        _ => BiomeType::Mountains,
    }
}

/// Check if a block should be a cave (empty space) - Minecraft-style organic caves with biome-specific patterns
fn should_generate_cave(block_x: i32, block_y: i32, biome: BiomeType) -> bool {
    // Minecraft-style cave generation using layered noise
    // Combines multiple frequencies for organic, natural-looking caves

    let x = block_x as f32;
    let y = block_y as f32;

    // Biome-specific cave parameters
    let (frequency_mult, threshold, worm_chance) = match biome {
        BiomeType::Nether => (1.5, 0.45, 0.6),  // More frequent, larger caves with many tunnels
        BiomeType::End => (0.8, 0.25, 0.2),     // Smaller, rarer caves
        BiomeType::Underground => (1.2, 0.4, 0.4), // Moderate caves
        _ => (1.0, 0.35, 0.3),                  // Default
    };

    // Primary cave noise - creates main cave structure
    let cave_noise_1 = ((x * 0.02 * frequency_mult).sin() * (y * 0.03 * frequency_mult).cos()
        + (x * 0.015 * frequency_mult + y * 0.025 * frequency_mult).sin()).abs();

    // Secondary cave noise - adds variation
    let cave_noise_2 = ((x * 0.04 * frequency_mult + 100.0).cos() * (y * 0.05 * frequency_mult + 50.0).sin()).abs();

    // Tertiary noise - creates smaller pockets and variation
    let cave_noise_3 = ((x * 0.08 * frequency_mult).sin() + (y * 0.07 * frequency_mult).cos()).abs();

    // Combine noises with different weights
    let combined_noise = cave_noise_1 * 0.6 + cave_noise_2 * 0.3 + cave_noise_3 * 0.1;

    // Cave exists if noise is below threshold
    let is_cave = combined_noise < threshold;

    // Add worm-like tunnels for variety (horizontal meandering paths)
    let worm_center_y = ((x * 0.01).sin() * 40.0) as i32;
    let distance_from_worm = (y as i32 - worm_center_y).abs();
    let worm_thickness = 2 + ((x * 0.05).cos().abs() * 3.0) as i32; // 2-5 blocks thick
    let is_worm_tunnel = distance_from_worm < worm_thickness && ((x / 80.0).fract() < worm_chance);

    is_cave || is_worm_tunnel
}

/// Determine block type based on biome and position
fn determine_block_type(biome: BiomeType, block_x: i32, block_y: i32, rng: &mut impl Rng) -> BlockType {
    // Chance for decorative blocks
    if rng.gen_bool(0.05) {
        let decorative = biome.decorative_blocks();
        if !decorative.is_empty() {
            return decorative[rng.gen_range(0..decorative.len())];
        }
    }

    // Ore generation (more common in deeper areas)
    let depth_factor = (-block_y).max(0) as u32;
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
                let world_x = world_block_x as f32 * TOTAL_BLOCK_SIZE * 64.0;
                let world_y = world_block_y as f32 * TOTAL_BLOCK_SIZE * 64.0;

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
                    _ => enemy_textures.skeleton.clone(), // Fallback for other types
                };

                commands.spawn((
                    Enemy::new(enemy_type),
                    Health::new(max_health),
                    ChunkMember { chunk_x, chunk_y },
                    Velocity { x: 0.0, y: 0.0 },
                    Gravity { force: -500.0 },
                    Bouncy { restitution: 0.6 },
                    Mesh3d(mesh_cache.enemy_mesh.clone()),  // Use shared mesh - PERFORMANCE FIX
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        base_color_texture: Some(texture),
                        unlit: true,
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    })),
                    Transform::from_xyz(world_x, world_y, 10.0),  // Z=10 same as other entities
                ));

                info!("Spawned {:?} at world({}, {}) chunk({}, {}) in {:?} biome", enemy_type, world_x, world_y, chunk_x, chunk_y, biome);
            }
        }
    }
}
