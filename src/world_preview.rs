//! Exact world geometry export. Runs before Bevy/window/audio/GPU initialization.
use crate::{
    components::block::{BiomeType, BlockType},
    resources::world_seed::WorldSeed,
    systems::{biome_encounters as sites, forest_trees as trees, world_generation as world_gen},
    utils::biome_utils::*,
};
use bevy::prelude::*;
use serde_json::json;
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let seed = WorldSeed::new(arg("--seed").unwrap_or("921".into()).parse()?);
    let biomes = [
        BiomeType::Plains,
        BiomeType::Forest,
        BiomeType::Desert,
        BiomeType::Mountains,
        BiomeType::Underground,
        BiomeType::Nether,
        BiomeType::End,
        BiomeType::Iceland,
    ];
    let name = arg("--biome").unwrap_or("Plains".into());
    let biome = biomes
        .into_iter()
        .find(|b| format!("{b:?}").eq_ignore_ascii_case(&name))
        .ok_or("Unknown biome")?;
    let cy = (nearest_biome_y(0., biome) / 1024.).floor() as i32;
    let mut lo = cy;
    let mut hi = cy;
    while determine_biome_at_chunk(0, lo - 1) == biome {
        lo -= 1;
    }
    while determine_biome_at_chunk(0, hi + 1) == biome {
        hi += 1;
    }
    let width: i32 = arg("--width")
        .unwrap_or("160".into())
        .parse::<i32>()?
        .clamp(32, 512);
    let height: i32 = arg("--height")
        .unwrap_or("112".into())
        .parse::<i32>()?
        .clamp(32, 512);
    let center_x: i32 = arg("--x").unwrap_or("96".into()).parse()?;
    let center_y: i32 = arg("--y")
        .map(|a| a.parse())
        .transpose()?
        .unwrap_or((lo + hi + 1) * 8);
    let min = IVec2::new(center_x - width / 2, center_y - height / 2);
    let max = min + IVec2::new(width, height);
    let spawn = crate::plugins::gameplay_plugin::SpawnArea::default();
    let mut cells = std::collections::BTreeMap::new();
    let mut landmarks = std::collections::BTreeMap::new();
    let mut walls = std::collections::BTreeMap::new();
    let mut portals = Vec::new();
    let start = std::time::Instant::now();
    for cx in min.x.div_euclid(16)..=(max.x - 1).div_euclid(16) {
        for cy in min.y.div_euclid(16)..=(max.y - 1).div_euclid(16) {
            let b = determine_biome_at_chunk(cx, cy);
            let chunk = IVec2::new(cx, cy);
            let portal = crate::systems::world_portals::site(cx, cy, seed.seed)
                .filter(|p| (p.as_vec2() * 64.).distance(spawn.center.xy()) > spawn.radius + 400.);
            let site = sites::site(cx, cy, b, seed.seed).filter(|s| {
                portal.is_none()
                    && (s.origin.as_vec2() * 64.).distance(spawn.center.xy())
                        > spawn.radius + s.width() as f32 * 64. + 100.
            });
            let tree = trees::plan(chunk, &seed, &spawn, portal);
            if let Some(p) = portal {
                portals.push(json!([p.x, p.y]));
            }
            for lx in 0..16 {
                for ly in 0..16 {
                    let p = chunk * 16 + IVec2::new(lx, ly);
                    if (p.as_vec2() * 64.).distance(spawn.center.xy()) < spawn.radius {
                        continue;
                    }
                    if portal.is_some_and(|s| crate::systems::world_portals::reserved(p, s))
                        || site.is_some_and(|s| s.reserved(p))
                        || tree.iter().any(|t| t.reserved(p))
                    {
                        continue;
                    }
                    let is_open = world_gen::should_generate_cave(p.x, p.y, b, &seed);
                    let tile = if is_open {
                        crate::systems::biome_content::terrain::feature(p.x, p.y, b, seed.seed)
                            .or_else(|| {
                                crate::systems::biome_content::terrain::decoration(
                                    p.x, p.y, b, seed.seed,
                                )
                            })
                    } else {
                        crate::systems::biome_content::terrain::solid(
                            p.x,
                            p.y,
                            b,
                            seed.seed,
                            world_gen::seeded_block(b, p.x, p.y, seed.seed),
                        )
                    };
                    if let Some(tile) = tile {
                        cells.insert(
                            (p.x, p.y),
                            (
                                format!("{:?}", tile.base()),
                                false,
                                Some(format!("Biome_{}", tile.texture())),
                            ),
                        );
                        continue;
                    }
                    let open = world_gen::should_generate_cave(p.x, p.y, b, &seed);
                    let kind = if open {
                        world_gen::decoration(p.x, p.y, b, &seed)
                    } else {
                        Some(world_gen::seeded_block(b, p.x, p.y, seed.seed))
                    };
                    if let Some(k) = kind {
                        let grass = matches!(b, BiomeType::Forest | BiomeType::Plains)
                            && k == BlockType::Dirt
                            && world_gen::should_generate_cave(p.x, p.y + 1, b, &seed);
                        cells.insert((p.x, p.y), (format!("{k:?}"), grass, None::<String>));
                    }
                }
            }
            for t in tree {
                for (p, leaf) in t.cells {
                    let p = t.root + p;
                    if p.div_euclid(IVec2::splat(16)) != chunk {
                        continue;
                    }
                    cells.insert(
                        (p.x, p.y),
                        (
                            if leaf { "Leaves" } else { "Wood" }.into(),
                            false,
                            Some(format!(
                                "{:?}_{}",
                                t.species,
                                if leaf { "leaves" } else { "log" }
                            )),
                        ),
                    );
                }
            }
            if let Some(s) = site {
                landmarks.insert((s.origin.x,s.origin.y),json!({"x":s.origin.x,"y":s.origin.y,"name":sites::title(b),"width":s.width()*2+1}));
                for (p, style) in sites::backdrop(s) {
                    let p = s.origin + p;
                    walls.insert((p.x, p.y), style);
                }
                let layout = sites::layout(s);
                let occupied = layout
                    .iter()
                    .filter(|(_, k)| {
                        !matches!(
                            k,
                            BlockType::Mushroom
                                | BlockType::DeadBush
                                | BlockType::Vines
                                | BlockType::Cobweb
                        )
                    })
                    .map(|(p, _)| *p)
                    .collect();
                for (p, mut k) in layout {
                    let grass = sites::soil_surface(s, p, k, &occupied);
                    if k == BlockType::Grass && !grass {
                        k = BlockType::Dirt;
                    }
                    let texture = crate::systems::biome_content::terrain::structure(b, p, k)
                        .map(|t| format!("Biome_{}", t.texture()))
                        .or_else(|| sites::finish_style(b, p, k).map(|i| format!("Finish_{i}")));
                    let p = s.origin + p;
                    if p.div_euclid(IVec2::splat(16)) == chunk {
                        cells.insert((p.x, p.y), (format!("{k:?}"), grass, texture));
                    }
                }
            }
        }
    }
    let cells: Vec<_> = cells
        .into_iter()
        .filter(|((x, y), _)| *x >= min.x && *x < max.x && *y >= min.y && *y < max.y)
        .map(|((x, y), (kind, grass, texture))| json!([x, y, kind, grass, texture]))
        .collect();
    let walls: Vec<_> = walls
        .into_iter()
        .map(|((x, y), style)| json!([x, y, format!("Finish_{style}")]))
        .collect();
    let output = json!({"seed":seed.seed,"biome":name,"min":[min.x,min.y],"width":width,"height":height,"cells":cells,"walls":walls,"portals":portals,"landmarks":landmarks.values().collect::<Vec<_>>(),"generation_ms":start.elapsed().as_secs_f64()*1000.,"notes":"Shared terrain, seeded blocks, plants, trees, structure placement and wall finishes. Lighting, enemies, dynamic fluids and runtime VFX are not simulated; gateway openings are marked schematically."});
    let path = arg("--output").ok_or("--output is required")?;
    std::fs::write(path, serde_json::to_vec(&output)?)?;
    Ok(())
}
