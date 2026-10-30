use super::*;

pub fn site(cx: i32, cy: i32, biome: BiomeType, seed: u32) -> Option<Site> {
    let rx = cx.div_euclid(3);
    let ry = cy.div_euclid(2);
    SITES.with(|sites| {
        let mut sites = sites.borrow_mut();
        let key = (rx, ry, biome, seed);
        if let Some(site) = sites.get(&key) {
            return *site;
        }
        if sites.len() > 2048 {
            sites.clear();
        }
        let site = ground_site(rx, ry, biome, seed);
        sites.insert(key, site);
        site
    })
}

pub(super) fn ground_site(rx: i32, ry: i32, biome: BiomeType, seed: u32) -> Option<Site> {
    let h = hash((rx as u32).wrapping_mul(73856093) ^ (ry as u32).wrapping_mul(19349663) ^ seed);
    if h % 100 >= 30 {
        return None;
    }
    // Avoid splitting a landmark across a biome boundary or a portal reservation.
    for x in 0..3 {
        for y in 0..2 {
            if crate::systems::world_portals::site(rx * 3 + x, ry * 2 + y, seed).is_some() {
                return None;
            }
        }
    }
    let width = if biome == BiomeType::Desert { 14 } else { 6 };
    let mut best = None;
    let mut score_best = i32::MIN;
    for lx in (width + 2..48 - width - 2).step_by(3) {
        for ly in 11..=24 {
            let o = IVec2::new(rx * 48 + lx, ry * 32 + ly);
            let air = |x, y| crate::systems::world_shapes::is_open(x, y, biome, seed);
            if !(-1..=1).all(|dx| (0..=2).all(|dy| air(o.x + dx, o.y + dy))) {
                continue;
            }
            let mut support = 0;
            let mut depth = 0;
            for dx in -width..=width {
                if let Some(d) = (0..=5).find(|d| !air(o.x + dx, o.y - 6 - d)) {
                    support += 1;
                    depth += d;
                }
            }
            if support < (width * 2 + 1) * 9 / 10 {
                continue;
            }
            let score =
                support * 30 - depth * 3 + ((hash(h ^ ((lx * 37 + ly) as u32))) % 17) as i32;
            if score > score_best {
                score_best = score;
                best = Some(Site {
                    origin: o,
                    biome,
                    seed,
                });
            }
        }
    }
    best
}

pub fn title(b: BiomeType) -> &'static str {
    match b {
        BiomeType::Plains => "WAYFARER CAMP",
        BiomeType::Forest => "WITCH GROVE",
        BiomeType::Desert => "ANCIENT AQUEDUCT",
        BiomeType::Mountains => "BROKEN WATCHPOST",
        BiomeType::Underground => "MINER'S VAULT",
        BiomeType::Nether => "EMBER FORGE",
        BiomeType::End => "VOID OBSERVATORY",
        BiomeType::Iceland => "FROZEN SHELTER",
    }
}

pub(super) fn palette(b: BiomeType) -> (BlockType, BlockType, BlockType) {
    match b {
        BiomeType::Plains => (BlockType::Wood, BlockType::Grass, BlockType::Coal),
        BiomeType::Forest => (BlockType::Wood, BlockType::Dirt, BlockType::Emerald),
        BiomeType::Desert => (BlockType::Sandstone, BlockType::Sand, BlockType::Gold),
        BiomeType::Mountains => (BlockType::Stone, BlockType::Snow, BlockType::Iron),
        BiomeType::Underground => (BlockType::Wood, BlockType::Stone, BlockType::Redstone),
        BiomeType::Nether => (
            BlockType::Obsidian,
            BlockType::Netherrack,
            BlockType::AncientDebris,
        ),
        BiomeType::End => (BlockType::Purpur, BlockType::EndStone, BlockType::Diamond),
        BiomeType::Iceland => (BlockType::Ice, BlockType::Snow, BlockType::Emerald),
    }
}

pub fn layout(site: Site) -> Vec<(IVec2, BlockType)> {
    let (frame, floor, ore) = palette(site.biome);
    let variant = hash(site.origin.x as u32 ^ (site.origin.y as u32).rotate_left(13)) % 3;
    let mut cells = std::collections::BTreeMap::new();
    let mut put = |x: i32, y: i32, k: BlockType| {
        cells.insert((x, y), k);
    };
    match site.biome {
        BiomeType::Plains => {
            // Timber cottage: stone footing, stepped roof, loft and a side well.
            for x in -6..=1 {
                put(x, -4, floor);
                put(x, 3, frame);
            }
            for y in -3..=2 {
                put(-6, y, frame);
                if y != 0 && y != 1 {
                    put(1, y, frame);
                }
            }
            for row in 0..=3 {
                for x in [-6 + row, 1 - row] {
                    put(
                        x,
                        4 + row,
                        if variant == 1 {
                            BlockType::Stone
                        } else {
                            frame
                        },
                    );
                }
            }
            for x in [3, 6] {
                for y in -4..=-2 {
                    put(x, y, BlockType::Stone);
                }
            }
            for x in 3..=6 {
                put(x, -4, BlockType::Stone);
                if variant != 2 {
                    put(x, 2, frame);
                }
            }
            for (x, y) in [(-4, -2), (-2, -1), (0, 1)] {
                put(x, y, frame);
            }
            if variant == 2 {
                put(-5, 7, BlockType::Stone);
                put(-5, 6, BlockType::Stone);
            }
        }
        BiomeType::Forest => {
            // Hollow old tree with a canopy walkway and scattered roots.
            for y in -5..=4 {
                put(-5, y, frame);
                if y != 0 && y != 1 {
                    put(-2, y, frame);
                }
            }
            for y in 4..=7 {
                for x in -6i32..=2 {
                    if (x + 2).abs() + y < 10 {
                        put(x, y, BlockType::Leaves);
                    }
                }
            }
            for x in -6..=0 {
                put(x, -5, floor);
            }
            for x in 0..=6 {
                if variant != 1 || x != 4 {
                    put(x, 2, frame);
                }
            }
            for y in -4..=1 {
                if y != 0 {
                    put(6, y, frame);
                }
            }
            put(4, -4, BlockType::Mushroom);
            put(3, -5, floor);
            put(4, -5, floor);
            if variant == 2 {
                for x in 2..=5 {
                    put(x, 5, BlockType::Leaves);
                }
            }
        }
        BiomeType::Desert => {
            // Selected concept: three arches, upper crossing, buried galleries.
            for x in -14..=14 {
                put(x, -5, frame);
                if !(-2..=1).contains(&x) {
                    put(x, -1, frame);
                }
                if x != 3 && x != 4 {
                    put(x, 7, frame);
                }
            }
            for x in [-14, -6, 6, 14] {
                for y in 0..=6 {
                    put(x, y, frame);
                }
            }
            // Step each haunch out from a pier; every block has an orthogonal
            // connection, rather than isolated diagonal "arch" pixels.
            for (left, right) in [(-14, -6), (-6, 6), (6, 14)] {
                for (depth, y) in [(1, 5), (2, 6)] {
                    for x in left + 1..=left + depth {
                        put(x, y, frame);
                    }
                    for x in right - depth..right {
                        put(x, y, frame);
                    }
                }
            }
            for x in [-14, -5, 5, 14] {
                for y in -4..=-2 {
                    if !(x == 5 && y == -3) {
                        put(x, y, frame);
                    }
                }
            }
            for (x, y) in [(-1, -2), (0, -3), (1, -4)] {
                put(x, y, frame);
            }
            for (x, top) in [(8, 1), (9, 2), (10, 3), (11, 4)] {
                for y in 0..=top {
                    put(x, y, frame);
                }
            }
            put(11, -4, BlockType::Gold);
            put(12, -4, BlockType::Gold);
            if variant == 1 {
                put(-11, 6, BlockType::Sand);
            }
            if variant == 2 {
                put(12, -4, BlockType::Sand);
            }
        }
        BiomeType::Mountains => {
            // Broken watchtower and two offset stone balconies.
            for x in -6..=-1 {
                put(x, -5, frame);
                put(x, 2, frame);
                put(x, 6, frame);
            }
            for y in -4..=5 {
                put(-6, y, frame);
                if y != 0 && y != 1 && y != 4 {
                    put(-1, y, frame);
                }
            }
            for x in [-6, -4, -2] {
                put(x, 7, frame);
            }
            for x in 2..=6 {
                put(x, -3, frame);
                if x > 3 {
                    put(x, 3, floor);
                }
            }
            if variant != 1 {
                put(2, 1, frame);
                put(3, 1, frame);
            }
            if variant == 2 {
                put(-5, 5, floor);
            }
        }
        BiomeType::Underground => {
            // Braced mine galleries with a broken bridge and ore store below.
            for x in -6..=6 {
                put(x, -5, floor);
                put(x, 6, frame);
                if x.abs() > 1 {
                    put(x, 1, frame);
                }
            }
            for x in [-6, -2, 6] {
                for y in -4..=5 {
                    if y != 0 && y != 1 {
                        put(x, y, frame);
                    }
                }
            }
            for x in -5..=-3 {
                put(x, -4, BlockType::Coal);
            }
            for x in 2..=5 {
                if variant != 1 || x != 4 {
                    put(x, 4, frame);
                }
            }
            put(5, -4, BlockType::Iron);
            if variant == 2 {
                put(-4, 5, BlockType::Cobweb);
            }
        }
        BiomeType::Nether => {
            // Obsidian furnace, glowing hearth and two raised access platforms.
            for x in -6..=6 {
                put(x, -5, frame);
                if x.abs() >= 3 {
                    put(x, 2, frame);
                }
            }
            for x in [-6, 6] {
                for y in -4..=6 {
                    put(x, y, frame);
                }
            }
            for x in -2..=2 {
                put(x, -4, frame);
                put(x, 4, frame);
            }
            put(-2, -3, frame);
            put(2, -3, frame);
            put(0, -3, BlockType::Glowstone);
            for x in [-5, 5] {
                put(x, 3, BlockType::Glowstone);
            }
            for x in -5i32..=5 {
                if x.abs() > 1 && (variant != 1 || x % 2 == 0) {
                    put(x, 6, frame);
                }
            }
            if variant == 2 {
                put(-3, 5, BlockType::AncientDebris);
            }
        }
        BiomeType::End => {
            // Floating observatory: split platforms, tall pylons and crown.
            for x in -6i32..=6 {
                if x.abs() > 1 {
                    put(x, -4, frame);
                }
            }
            for x in [-5, 5] {
                for y in -3..=5 {
                    if y != 0 {
                        put(x, y, frame);
                    }
                }
            }
            for x in -4i32..=4 {
                put(
                    x,
                    6,
                    if x.abs() < 2 {
                        BlockType::Obsidian
                    } else {
                        frame
                    },
                );
            }
            put(0, 7, BlockType::Glowstone);
            for x in [-3, -2, 2, 3] {
                put(x, 2, frame);
            }
            if variant == 1 {
                put(4, 4, BlockType::Obsidian);
            }
            if variant == 2 {
                put(-4, 4, BlockType::Obsidian);
            }
        }
        BiomeType::Iceland => {
            // Layered ice shelter with a snow crown and stepped entry.
            for x in -6..=6 {
                put(x, -5, floor);
            }
            for row in 0..=3 {
                let span = 6 - row;
                put(-span, 3 + row, frame);
                put(span, 3 + row, frame);
                for x in -span..=span {
                    if row == 3 {
                        put(x, 7, floor);
                    }
                }
            }
            for x in [-6, 6] {
                for y in -4..=2 {
                    if y != 0 && y != 1 {
                        put(x, y, frame);
                    }
                }
            }
            for x in -5..=-2 {
                put(x, -2, frame);
            }
            for x in 3..=5 {
                put(x, 2, frame);
            }
            if variant != 1 {
                put(-3, 3, BlockType::Wood);
            }
            if variant == 2 {
                put(3, -4, BlockType::Ice);
            }
        }
    }
    // Clear horizontal access for bouncing heads, retain irregular vertical routes.
    cells.retain(|&(x, y), _| {
        !(y == 0 && x >= 0 && !(site.biome == BiomeType::Desert && matches!(x, 6 | 8..=11 | 14)))
            && (x, y) != (2, -2)
    });
    cells.insert((2, -2), ore);
    // The continuous footing joins raised floors to the natural ground. In
    // particular, the cottage floor is at -4 rather than the other sites' -5.
    for x in -site.width()..=site.width() {
        cells.entry((x, -5)).or_insert(floor);
        for y in -10..=-6 {
            let p = site.origin + IVec2::new(x, y);
            if crate::systems::world_shapes::is_open(p.x, p.y, site.biome, site.seed) {
                cells.insert((x, y), floor);
            }
        }
    }
    cells
        .into_iter()
        .map(|((x, y), kind)| (IVec2::new(x, y), kind))
        .collect()
}

pub(crate) fn finish_style(biome: BiomeType, p: IVec2, kind: BlockType) -> Option<usize> {
    if biome == BiomeType::Desert && kind == BlockType::Sandstone {
        if p.y == 7 || p.y == -5 {
            return Some(5);
        }
        if matches!(p.x, -14 | -6 | 6 | 14) && matches!(p.y, 2 | 5) {
            return Some(6);
        }
    }
    if kind == BlockType::Stone {
        return Some(2);
    }
    if kind == BlockType::Wood {
        if biome == BiomeType::Plains && p.y >= 4 {
            return Some(1);
        }
        if matches!(p.y, 1 | 2 | 3 | 6) && !matches!(p.x, -6 | -2 | 6) {
            return Some(0);
        }
    }
    None
}

pub(crate) fn backdrop(site: Site) -> Vec<(IVec2, usize)> {
    let mut out = Vec::new();
    for y in -4i32..=6 {
        for x in -site.width() + 1..site.width() {
            let style = match site.biome {
                BiomeType::Plains
                    if x <= 0
                        && y >= -3
                        && y <= 2
                        && !(x == 0 && y < 0)
                        && !((-4..=-3).contains(&x) && (0..=1).contains(&y)) =>
                {
                    Some(0)
                }
                BiomeType::Plains if y >= 4 && x > -6 + (y - 4) && x < 1 - (y - 4) => Some(0),
                BiomeType::Desert if y <= -2 => Some(4),
                BiomeType::Forest if (-4..=-3).contains(&x) && y < 4 => Some(1),
                BiomeType::Mountains if x <= -2 && y < 6 && !(x == -4 && (3..=4).contains(&y)) => {
                    Some(2)
                }
                BiomeType::Underground if y == 5 || (x == -5 && y < 5) || (x >= 3 && y <= -3) => {
                    Some(2)
                }
                BiomeType::Nether if x.abs() <= 2 && (-3..=3).contains(&y) => Some(3),
                _ => None,
            };
            if let Some(style) = style {
                out.push((IVec2::new(x, y), style));
            }
        }
    }
    out
}

pub(crate) fn soil_surface(
    site: Site,
    p: IVec2,
    kind: BlockType,
    occupied: &std::collections::HashSet<IVec2>,
) -> bool {
    let above = site.origin + p + IVec2::Y;
    matches!(kind, BlockType::Dirt | BlockType::Grass)
        && matches!(site.biome, BiomeType::Forest | BiomeType::Plains)
        && !occupied.contains(&(p + IVec2::Y))
        && (site.reserved(above)
            || crate::systems::world_shapes::is_open(above.x, above.y, site.biome, site.seed))
}

pub fn spawn(commands: &mut Commands, site: Site, meshes: &MeshCache, materials: &MaterialCache) {
    spawn_partition(commands, site, None, meshes, materials);
}

pub fn spawn_chunk(
    commands: &mut Commands,
    site: Site,
    chunk: IVec2,
    meshes: &MeshCache,
    materials: &MaterialCache,
) {
    spawn_partition(commands, site, Some(chunk), meshes, materials);
}

pub(super) fn spawn_partition(
    commands: &mut Commands,
    site: Site,
    partition: Option<IVec2>,
    meshes: &MeshCache,
    materials: &MaterialCache,
) {
    let owner = site.origin.div_euclid(IVec2::splat(16));
    let cells = layout(site);
    let occupied: std::collections::HashSet<_> = cells
        .iter()
        .filter(|(_, k)| {
            !matches!(
                k,
                BlockType::Mushroom | BlockType::DeadBush | BlockType::Vines | BlockType::Cobweb
            )
        })
        .map(|(p, _)| *p)
        .collect();
    for (p, mut kind) in cells {
        let grass_floor = soil_surface(site, p, kind, &occupied);
        if kind == BlockType::Grass && !grass_floor {
            kind = BlockType::Dirt;
        }
        let Some(material) = materials.materials.get(&kind) else {
            continue;
        };
        let cell = site.origin + p;
        let chunk = cell.div_euclid(IVec2::splat(16));
        if partition.is_some_and(|p| p != chunk) {
            continue;
        }
        let material = if grass_floor {
            materials.forest_floor.as_ref().unwrap_or(material)
        } else {
            material
        };
        let (mesh, scale) = if grass_floor {
            (meshes.wood_mesh.clone(), 1.)
        } else {
            match kind {
                BlockType::Wood => (meshes.wood_mesh.clone(), 1.),
                BlockType::Leaves | BlockType::Grass => (meshes.leaves_mesh.clone(), 1.),
                BlockType::Sandstone => (meshes.sandstone_mesh.clone(), 1.),
                BlockType::Mushroom => (meshes.mushroom_mesh.clone(), 1.),
                _ => (meshes.block_mesh.clone(), 64.),
            }
        };
        let mut e = commands.spawn((
            Block {
                health: kind.starting_health(),
                max_health: kind.starting_health(),
            },
            kind,
            ChunkMember {
                chunk_x: chunk.x,
                chunk_y: chunk.y,
            },
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_translation((cell.as_vec2() * 64.).extend(-10.))
                .with_scale(Vec3::splat(scale)),
        ));
        if let Some(tile) = crate::systems::biome_content::terrain::structure(site.biome, p, kind) {
            e.insert((tile, tile.base()));
        } else if let Some(style) = finish_style(site.biome, p, kind) {
            e.insert(StructureFinish(style));
        }
        if p == IVec2::new(2, -2) {
            e.insert(RewardCache { biome: site.biome });
        }
    }
    if partition.is_some_and(|p| p != owner) {
        return;
    }
    let chunk = owner;
    commands.spawn((
        PendingLandmark(site),
        ChunkMember {
            chunk_x: chunk.x,
            chunk_y: chunk.y,
        },
        Transform::from_translation((site.origin.as_vec2() * 64.).extend(0.)),
    ));
}
