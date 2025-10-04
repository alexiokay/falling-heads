use bevy::prelude::*;
use rand::Rng;
use crate::components::block::{Block, BlockType, BiomeType};
use crate::components::chunk::{Chunk, ChunkMember, LoadedChunks, CHUNK_SIZE, world_to_chunk_coords};
use crate::components::hero::Hero;

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
) {
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
                generate_chunk(&mut commands, &mut meshes, &mut materials, chunk_x, chunk_y);
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

    let despawn_distance = 5; // Despawn chunks more than 5 chunks away

    for (chunk_entity, chunk) in chunk_query.iter() {
        let distance_x = (chunk.chunk_x - hero_chunk_x).abs();
        let distance_y = (chunk.chunk_y - hero_chunk_y).abs();

        if distance_x > despawn_distance || distance_y > despawn_distance {
            // Despawn all blocks in this chunk
            for (block_entity, member) in block_query.iter() {
                if member.chunk_x == chunk.chunk_x && member.chunk_y == chunk.chunk_y {
                    commands.entity(block_entity).despawn();
                }
            }

            // Despawn chunk entity
            commands.entity(chunk_entity).despawn();
            loaded_chunks.remove(chunk.chunk_x, chunk.chunk_y);
        }
    }
}

/// Generate a single chunk with blocks
fn generate_chunk(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    chunk_x: i32,
    chunk_y: i32,
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

    // Create cube mesh (reused for all blocks in this chunk)
    let cube_mesh = meshes.add(Cuboid::new(BLOCK_SIZE, BLOCK_SIZE, BLOCK_SIZE));

    // Generate blocks in this chunk
    for local_x in 0..CHUNK_SIZE {
        for local_y in 0..CHUNK_SIZE {
            let world_block_x = chunk_x * CHUNK_SIZE + local_x;
            let world_block_y = chunk_y * CHUNK_SIZE + local_y;

            // Cave generation check
            if should_generate_cave(world_block_x, world_block_y) {
                continue; // Skip this block, it's part of a cave
            }

            // Determine block type
            let block_type = determine_block_type(biome, world_block_x, world_block_y, &mut rng);
            let color = get_block_color(block_type);

            let world_x = world_block_x as f32 * TOTAL_BLOCK_SIZE * 64.0;  // Scale to 2D coordinates
            let world_y = world_block_y as f32 * TOTAL_BLOCK_SIZE * 64.0;  // Scale to 2D coordinates
            let world_z = 0.0;

            // Render as 3D cubes scaled to match 2D
            let mut transform = Transform::from_xyz(world_x, world_y, world_z);
            transform.scale = Vec3::splat(64.0);  // Scale cube to 64x64x64 to match 2D block size

            commands.spawn((
                Block {
                    health: 5,
                    max_health: 5,
                },
                block_type,
                ChunkMember { chunk_x, chunk_y },
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: color,
                    perceptual_roughness: 0.8,
                    metallic: 0.1,
                    ..default()
                })),
                transform,
            ));
        }
    }
}

/// Determine biome based on chunk coordinates
fn determine_biome(chunk_x: i32, chunk_y: i32, rng: &mut impl Rng) -> BiomeType {
    // Use simple noise-like pattern based on coordinates
    let biome_seed = (chunk_x * 73 + chunk_y * 179) % 100;

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
        0..=30 => BiomeType::Plains,
        31..=55 => BiomeType::Forest,
        56..=75 => BiomeType::Desert,
        _ => BiomeType::Mountains,
    }
}

/// Check if a block should be a cave (empty space)
fn should_generate_cave(block_x: i32, block_y: i32) -> bool {
    // Simple pseudo-noise cave generation
    // Using a combination of sine waves for cave patterns
    let x_f = block_x as f32 * 0.1;
    let y_f = block_y as f32 * 0.1;

    let noise1 = ((x_f * 1.5).sin() + (y_f * 1.7).cos()) * 0.5;
    let noise2 = ((x_f * 2.3).cos() + (y_f * 1.9).sin()) * 0.5;
    let combined = noise1 + noise2;

    // Cave threshold
    combined > 1.2
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
    let ore_chance = rng.gen_range(0..100) + (depth_factor / 10).min(30);

    let block_type = match ore_chance {
        0..=70 => biome.terrain_block(), // Terrain block
        71..=85 => BlockType::Coal,
        86..=93 => BlockType::Iron,
        94..=97 => BlockType::Gold,
        98 => BlockType::Redstone,
        99 => if depth_factor > 50 { BlockType::Diamond } else { BlockType::Iron },
        _ => if depth_factor > 80 { BlockType::Emerald } else { BlockType::Gold },
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
    }
}
