//! Seeded, grounded block trees. Wood/Leaves retain the existing mining and fire rules.
use crate::{
    components::{
        block::{BiomeType, Block, BlockType},
        chunk::{CHUNK_SIZE, ChunkMember},
    },
    plugins::gameplay_plugin::SpawnArea,
    resources::{material_cache::MaterialCache, mesh_cache::MeshCache, world_seed::WorldSeed},
};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TreeSpecies {
    Oak,
    Birch,
    Spruce,
}
#[derive(Component, Clone, Copy)]
pub struct TreeBlock {
    pub root: IVec2,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreePlan {
    pub root: IVec2,
    pub species: TreeSpecies,
    pub cells: Vec<(IVec2, bool)>,
}
impl TreePlan {
    pub fn reserved(&self, p: IVec2) -> bool {
        let p = p - self.root;
        // A complete air corridor around the crown; random cave clutter and springs
        // cannot occupy it after the tree placement has been validated.
        p.y >= 0 && p.y <= self.height() + 1 && p.x.abs() <= self.clearance_radius(p.y)
    }
    fn height(&self) -> i32 {
        self.cells.iter().map(|(p, _)| p.y).max().unwrap_or(0)
    }
    fn clearance_radius(&self, y: i32) -> i32 {
        // Follow the stepped crown with one tile of breathing room, tapering at
        // its top and trunk instead of cutting a rectangular room into the wall.
        self.cells
            .iter()
            .filter(|(p, _)| (p.y - y).abs() <= 1)
            .map(|(p, _)| p.x.abs() + 1)
            .max()
            .unwrap_or(1)
    }
}
pub struct ForestTreesPlugin;
impl Plugin for ForestTreesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load)
            .add_systems(Update, prepare_materials);
    }
}
#[derive(Resource)]
struct TreeTextures(Vec<Handle<Image>>);
fn load(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(TreeTextures(
        [
            "birch_log",
            "birch_log_top",
            "birch_leaves",
            "spruce_log",
            "spruce_log_top",
            "spruce_leaves",
            "grass_block_top",
            "grass_block_side",
            "dirt",
            "oak_log",
            "oak_log_top",
            "oak_leaves",
        ]
        .map(|name| server.load(format!("textures/blocks/forest/species/{name}.png")))
        .to_vec(),
    ));
}
fn prepare_materials(
    textures: Res<TreeTextures>,
    mut images: ResMut<Assets<Image>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<MaterialCache>,
) {
    if cache.materials.is_empty() || cache.forest_floor.is_some() {
        return;
    }
    let Some(source) = textures
        .0
        .iter()
        .map(|h| images.get(h).cloned())
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    use super::atlas_generator::create_atlas_4x1 as atlas;
    for (species, i, tint) in [
        (TreeSpecies::Oak, 9, Color::srgb(0.53, 0.80, 0.40)),
        (TreeSpecies::Birch, 0, Color::srgb(0.70, 0.86, 0.45)),
        (TreeSpecies::Spruce, 3, Color::srgb(0.47, 0.68, 0.53)),
    ] {
        let bark = images.add(atlas(
            &source[i + 1],
            &source[i + 1],
            &source[i],
            &source[i],
        ));
        let leaves = images.add(atlas(
            &source[i + 2],
            &source[i + 2],
            &source[i + 2],
            &source[i + 2],
        ));
        for (is_leaf, texture, color) in [(false, bark, Color::WHITE), (true, leaves, tint)] {
            cache.tree_materials.insert(
                (species, is_leaf),
                mats.add(StandardMaterial {
                    base_color: color,
                    base_color_texture: Some(texture),
                    perceptual_roughness: 0.95,
                    reflectance: 0.08,
                    alpha_mode: if is_leaf {
                        AlphaMode::Mask(0.5)
                    } else {
                        AlphaMode::Opaque
                    },
                    ..default()
                }),
            );
        }
    }
    // Grass top is a greyscale biome-tinted texture. Tint its pixels in the atlas,
    // leaving the dirt sides brown rather than tinting the entire ground cube.
    let mut top = source[6].clone();
    if let Some(data) = top.data.as_mut() {
        for rgba in data.chunks_exact_mut(4) {
            rgba[0] = (rgba[0] as f32 * 0.48) as u8;
            rgba[1] = (rgba[1] as f32 * 0.66) as u8;
            rgba[2] = (rgba[2] as f32 * 0.30) as u8;
        }
    }
    let floor = images.add(atlas(&top, &source[8], &source[7], &source[7]));
    cache.forest_floor = Some(mats.add(StandardMaterial {
        base_color_texture: Some(floor),
        perceptual_roughness: 0.95,
        reflectance: 0.08,
        ..default()
    }));
    info!("Forest tree materials ready: oak, birch, spruce, grass-topped soil");
}

fn hash(p: IVec2, seed: u32) -> u32 {
    let mut h =
        (p.x as u32).wrapping_mul(0x9e3779b9) ^ (p.y as u32).wrapping_mul(0x85ebca6b) ^ seed;
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb352d);
    h ^ h >> 15
}
/// A front projection of block trees: the crown hides the upper trunk, as it
/// would in a full 3D tree. Every occupied gameplay cell remains unique.
pub fn shape(species: TreeSpecies, variant: u32) -> Vec<(IVec2, bool)> {
    let mut cells = HashMap::new();
    let extra = (variant % 2) as i32;
    let (trunk, layers): (i32, Vec<(i32, i32)>) = match species {
        TreeSpecies::Oak => (
            1 + extra,
            vec![
                (2 + extra, 2),
                (3 + extra, 2),
                (4 + extra, 2),
                (5 + extra, 1),
            ],
        ),
        TreeSpecies::Birch => (
            2 + extra,
            vec![
                (3 + extra, 1),
                (4 + extra, 2),
                (5 + extra, 1),
                (6 + extra, 1),
            ],
        ),
        TreeSpecies::Spruce => (
            1 + extra,
            vec![
                (2 + extra, 2),
                (3 + extra, 1),
                (4 + extra, 2),
                (5 + extra, 1),
                (6 + extra, 1),
                (7 + extra, 0),
            ],
        ),
    };
    for (y, radius) in layers {
        for x in -radius..=radius {
            // One clipped oak corner breaks the square silhouette without holes.
            if species == TreeSpecies::Oak
                && y == 4 + extra
                && x == if variant % 4 < 2 { -2 } else { 2 }
            {
                continue;
            }
            cells.insert(IVec2::new(x, y), true);
        }
    }
    for y in 0..=trunk {
        cells.insert(IVec2::new(0, y), false);
    }
    let mut result: Vec<_> = cells.into_iter().collect();
    result.sort_by_key(|(p, _)| (p.y, p.x));
    result
}
/// Both chunks compute the same tree; each emits only its own occupied cells.
/// Crowns can cross the upper boundary, so a perfectly valid forest floor is
/// no longer rejected just because it sits high in a streaming chunk.
pub fn plan(
    chunk: IVec2,
    seed: &WorldSeed,
    spawn: &SpawnArea,
    _portal: Option<IVec2>,
) -> Vec<TreePlan> {
    let candidates: Vec<_> = (-2..=1)
        .flat_map(|dy| plan_owned(chunk + IVec2::Y * dy, seed, spawn))
        .collect();
    let rank = |t: &TreePlan| (hash(t.root, seed.seed), t.root.x, t.root.y);
    candidates
        .iter()
        .filter(|t| {
            t.root.y <= chunk.y * CHUNK_SIZE + CHUNK_SIZE - 1
                && t.root.y + t.height() + 1 >= chunk.y * CHUNK_SIZE
        })
        .filter(|t| {
            !candidates.iter().any(|other| {
                rank(other) < rank(t)
                    && (t.cells.iter().any(|(p, _)| other.reserved(t.root + *p))
                        || other.cells.iter().any(|(p, _)| t.reserved(other.root + *p)))
            })
        })
        .cloned()
        .collect()
}
fn plan_owned(chunk: IVec2, seed: &WorldSeed, spawn: &SpawnArea) -> Vec<TreePlan> {
    let biome = crate::utils::biome_utils::determine_biome_at_chunk(chunk.x, chunk.y);
    if !matches!(biome, BiomeType::Forest | BiomeType::Plains) {
        return Vec::new();
    }
    if biome == BiomeType::Plains && hash(chunk, seed.seed) % 3 != 0 {
        return Vec::new();
    }
    let blocked = |p: IVec2| {
        let c = p.div_euclid(IVec2::splat(CHUNK_SIZE));
        let portal = super::world_portals::site(c.x, c.y, seed.seed)
            .filter(|s| (s.as_vec2() * 64.).distance(spawn.center.xy()) > spawn.radius + 400.);
        if portal.is_some_and(|s| super::world_portals::reserved(p, s)) {
            return true;
        }
        let b = crate::utils::biome_utils::determine_biome_at_chunk(c.x, c.y);
        super::biome_encounters::site(c.x, c.y, b, seed.seed)
            .filter(|s| {
                portal.is_none()
                    && (s.origin.as_vec2() * 64.).distance(spawn.center.xy())
                        > spawn.radius + s.width() as f32 * 64. + 100.
            })
            .is_some_and(|s| s.reserved(p))
    };
    let species = TreeSpecies::Spruce;
    let cells = shape(species, 1);
    let mut envelope = cells.clone();
    for kind in [TreeSpecies::Oak, TreeSpecies::Birch] {
        for v in 0..2 {
            envelope.extend(shape(kind, v));
        }
    }
    let template = TreePlan {
        root: IVec2::ZERO,
        species,
        cells: envelope,
    };
    let height = cells.iter().map(|(p, _)| p.y).max().unwrap();
    let cave = |p: IVec2| super::world_generation::should_generate_cave(p.x, p.y, biome, seed);
    let mut candidates = Vec::new();
    for x in 3..CHUNK_SIZE - 3 {
        for y in 0..CHUNK_SIZE {
            let root = chunk * CHUNK_SIZE + IVec2::new(x, y);
            if (-1..=1).any(|dx| {
                let floor = root + IVec2::new(dx, -1);
                cave(floor)
                    || blocked(floor)
                    || (floor.as_vec2() * 64.).distance(spawn.center.xy()) <= spawn.radius
            }) {
                continue;
            }
            // Start in an existing opening with an intact three-cell footing.
            if !(-1..=1).all(|dx| (0..=1).all(|dy| cave(root + IVec2::new(dx, dy)))) {
                continue;
            }
            let mut carved = 0;
            let clear = (-3..=3).all(|dx| {
                (0..=height + 1).all(|dy| {
                    if !template.reserved(IVec2::new(dx, dy)) {
                        return true;
                    }
                    let p = root + IVec2::new(dx, dy);
                    if !cave(p) {
                        carved += 1;
                    }
                    carved <= 12
                        && (p.as_vec2() * 64.).distance(spawn.center.xy()) > spawn.radius + 64.
                        && crate::utils::biome_utils::determine_biome_at_chunk(
                            p.x.div_euclid(CHUNK_SIZE),
                            p.y.div_euclid(CHUNK_SIZE),
                        ) == biome
                        && !blocked(p)
                })
            });
            if clear {
                candidates.push((carved, root));
            }
        }
    }
    candidates.sort_by_key(|(carved, p)| (*carved, hash(*p, seed.seed)));
    let mut trees: Vec<TreePlan> = Vec::new();
    // Separate crowns leave a full air tile between their rooms.
    for (_, root) in candidates {
        if trees.iter().any(|t| (t.root.x - root.x).abs() < 7) {
            continue;
        }
        let species = if biome == BiomeType::Plains {
            TreeSpecies::Oak
        } else {
            match hash(root, seed.seed) % 3 {
                0 => TreeSpecies::Oak,
                1 => TreeSpecies::Birch,
                _ => TreeSpecies::Spruce,
            }
        };
        let tree = TreePlan {
            root,
            species,
            cells: shape(species, hash(root, seed.seed ^ 9301)),
        };
        trees.push(tree);
        if trees.len() == if biome == BiomeType::Plains { 1 } else { 2 } {
            break;
        }
    }
    trees
}
pub fn spawn(
    commands: &mut Commands,
    tree: &TreePlan,
    chunk: IVec2,
    mesh: &MeshCache,
    cache: &MaterialCache,
) {
    for (offset, leaf) in &tree.cells {
        if (tree.root + *offset).div_euclid(IVec2::splat(CHUNK_SIZE)) != chunk {
            continue;
        }
        let kind = if *leaf {
            BlockType::Leaves
        } else {
            BlockType::Wood
        };
        let p = (tree.root + *offset).as_vec2() * 64.;
        commands.spawn((
            Block {
                health: kind.starting_health(),
                max_health: kind.starting_health(),
            },
            kind,
            tree.species,
            TreeBlock { root: tree.root },
            ChunkMember {
                chunk_x: chunk.x,
                chunk_y: chunk.y,
            },
            Mesh3d(if *leaf {
                mesh.leaves_mesh.clone()
            } else {
                mesh.wood_mesh.clone()
            }),
            MeshMaterial3d(cache.tree_materials[&(tree.species, *leaf)].clone()),
            Transform::from_xyz(p.x, p.y, -10.),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streamed_crowns_cross_boundaries_without_duplicate_blocks() {
        let spawn = SpawnArea {
            center: Vec3::ZERO,
            radius: 500.,
        };
        let seed = WorldSeed::new(921);
        let cy = (crate::utils::biome_utils::nearest_biome_y(0., BiomeType::Forest) / 1024.).floor()
            as i32;
        let mut occupied = std::collections::HashSet::new();
        let mut crossings = 0;
        for x in 4..12 {
            for y in cy - 4..cy + 4 {
                let chunk = IVec2::new(x, y);
                for tree in plan(chunk, &seed, &spawn, None) {
                    for (p, _) in &tree.cells {
                        let cell = tree.root + *p;
                        if cell.div_euclid(IVec2::splat(CHUNK_SIZE)) != chunk {
                            continue;
                        }
                        assert!(occupied.insert(cell), "Two streamed trees occupy {cell:?}");
                        crossings += (cell.y.div_euclid(CHUNK_SIZE)
                            != tree.root.y.div_euclid(CHUNK_SIZE))
                            as usize;
                    }
                }
            }
        }
        assert!(
            crossings > 0,
            "The fixture must actually exercise boundary crowns"
        );
    }
    #[test]
    fn trunks_and_crowns_are_connected_without_duplicate_cells() {
        for species in [TreeSpecies::Oak, TreeSpecies::Birch, TreeSpecies::Spruce] {
            for variant in 0..4 {
                let cells = shape(species, variant);
                let map: HashMap<_, _> = cells.iter().copied().collect();
                assert_eq!(map.len(), cells.len());
                assert_eq!(map[&IVec2::ZERO], false);
                let mut seen = std::collections::HashSet::from([IVec2::ZERO]);
                loop {
                    let old = seen.len();
                    for (p, _) in &cells {
                        if [IVec2::X, -IVec2::X, IVec2::Y, -IVec2::Y]
                            .iter()
                            .any(|d| seen.contains(&(*p + *d)))
                        {
                            seen.insert(*p);
                        }
                    }
                    if seen.len() == old {
                        break;
                    }
                }
                assert_eq!(seen.len(), cells.len(), "Disconnected {species:?}");
            }
        }
    }
    #[test]
    fn natural_trees_are_seeded_supported_clear_and_partitioned() {
        let spawn = SpawnArea {
            center: Vec3::ZERO,
            radius: 500.,
        };
        let mut counts = [0; 3];
        let cy_center = (crate::utils::biome_utils::nearest_biome_y(0., BiomeType::Forest) / 1024.)
            .floor() as i32;
        for seed in [7, 81, 921] {
            for cy in cy_center - 3..=cy_center + 3 {
                for cx in -12..=12 {
                    let chunk = IVec2::new(cx, cy);
                    let seed = WorldSeed::new(seed);
                    let portal = super::super::world_portals::site(cx, cy, seed.seed);
                    let trees = plan(chunk, &seed, &spawn, portal);
                    assert_eq!(trees, plan(chunk, &seed, &spawn, portal));
                    for a in &trees {
                        for b in &trees {
                            if a.root != b.root
                                && a.root.y.div_euclid(CHUNK_SIZE)
                                    == b.root.y.div_euclid(CHUNK_SIZE)
                            {
                                assert!((a.root.x - b.root.x).abs() >= 7);
                            }
                        }
                    }
                    for tree in trees {
                        counts[tree.species as usize] += 1;
                        assert!(!super::super::world_generation::should_generate_cave(
                            tree.root.x,
                            tree.root.y - 1,
                            BiomeType::Forest,
                            &seed
                        ));
                        for (p, _) in &tree.cells {
                            let p = *p + tree.root;
                            assert_eq!(p.x.div_euclid(CHUNK_SIZE), chunk.x);
                            assert!(
                                (p.y.div_euclid(CHUNK_SIZE) - tree.root.y.div_euclid(CHUNK_SIZE))
                                    .abs()
                                    <= 1
                            );
                            assert!(tree.reserved(p));
                            assert!(
                                !portal
                                    .is_some_and(|s| super::super::world_portals::reserved(p, s))
                            );
                        }
                        let carved = (-3..=3)
                            .flat_map(|x| {
                                (0..=tree.height() + 1).map(move |y| IVec2::new(x, y) + tree.root)
                            })
                            .filter(|p| {
                                tree.reserved(*p)
                                    && !super::super::world_generation::should_generate_cave(
                                        p.x,
                                        p.y,
                                        BiomeType::Forest,
                                        &seed,
                                    )
                            })
                            .count();
                        assert!(carved <= 12);
                        assert!(
                            plan(
                                chunk,
                                &seed,
                                &SpawnArea {
                                    center: (tree.root.as_vec2() * 64.).extend(0.),
                                    radius: 2000.
                                },
                                portal
                            )
                            .is_empty()
                        );
                    }
                }
            }
        }
        eprintln!("Natural forest tree counts {counts:?}");
        assert!(
            counts.iter().all(|n| *n >= 30),
            "Forests need frequent trees of every species"
        );
        assert!(
            counts.iter().sum::<i32>() >= 100,
            "Too many treeless chunks"
        );
    }
}
