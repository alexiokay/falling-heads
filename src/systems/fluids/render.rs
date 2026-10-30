use super::*;
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct FluidMaterial {
    #[uniform(0)]
    effect: Vec4,
}
impl Material for FluidMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/liquid.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
#[derive(Resource)]
pub(super) struct VisualAssets {
    meshes: [Handle<Mesh>; 3],
    materials: [Handle<FluidMaterial>; 3],
}
pub(super) fn empty_mesh() -> Mesh {
    Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    // Keep the vertex layout valid even before the first fluid simulation tick.
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new())
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<[f32; 2]>::new())
    .with_inserted_indices(bevy::mesh::Indices::U32(Vec::new()))
}
pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FluidMaterial>>,
) {
    let assets = VisualAssets {
        meshes: [
            meshes.add(empty_mesh()),
            meshes.add(empty_mesh()),
            meshes.add(empty_mesh()),
        ],
        materials: [
            materials.add(FluidMaterial {
                effect: Vec4::new(0., 0., 0.72, 0.),
            }),
            materials.add(FluidMaterial {
                effect: Vec4::new(0., 1., 0., 0.),
            }),
            materials.add(FluidMaterial {
                effect: Vec4::new(0., 2., 0., 0.),
            }),
        ],
    };
    for i in 0..3 {
        let mut visual = commands.spawn((
            Mesh3d(assets.meshes[i].clone()),
            MeshMaterial3d(assets.materials[i].clone()),
            Transform::IDENTITY,
            bevy::camera::visibility::NoFrustumCulling,
        ));
        // The shadow pass cannot see the custom alpha mask. Steam would cast
        // the whole rectangular billboard as a black square onto new obsidian.
        if i != 1 {
            visual.insert(bevy::light::NotShadowCaster);
        }
    }
    commands.insert_resource(assets);
}

pub(super) fn fluid_sections(
    world: &FluidWorld,
    p: Cell,
    kind: Liquid,
) -> Vec<(f32, f32, f32, f32)> {
    if world.cooled.contains(&p) || world.ice.contains_key(&p) {
        return Vec::new();
    }
    let volume = world
        .cells
        .get(&p)
        .filter(|v| v.kind == kind)
        .map_or(0, |v| v.amount);
    let flux = world
        .falling
        .get(&p)
        .filter(|v| v.0 == kind)
        .map_or(0, |v| v.1);
    if volume == 0 && flux == 0 {
        return Vec::new();
    }
    let bottom = p.1 as f32 * 64. - 32.;
    let center = p.0 as f32 * 64.;
    let settled = world.supported.contains(&p) && volume > 0;
    let top = bottom + volume as f32 * 8.;
    let mut rects = Vec::new();
    if settled {
        rects.push((center - 32., center + 32., bottom, top));
    }
    let above = (p.0, p.1 + 1);
    let incoming = world
        .cells
        .get(&above)
        .is_some_and(|v| v.kind == kind && v.amount > 0)
        || world
            .falling
            .get(&above)
            .is_some_and(|v| v.0 == kind && v.1 > 0);
    let side_height = [-1, 1]
        .into_iter()
        .filter_map(|dx| {
            let q = (p.0 + dx, p.1);
            world
                .cells
                .get(&q)
                .filter(|v| v.kind == kind && world.supported.contains(&q))
                .map(|v| v.amount)
        })
        .max()
        .unwrap_or(0);
    // A side-fed spill starts at its feeder's surface, not the top of an
    // otherwise empty cell. Only an incoming stream extends above a pool.
    if !settled || (flux > 0 && incoming) {
        let high = if !incoming && side_height > 0 {
            bottom + side_height as f32 * 8.
        } else {
            bottom + 64.
        };
        // A transported cell's transient volume is not a free-surface width.
        // Falling ribbons keep their cross-section until they actually meet a pool.
        let width = match kind {
            Liquid::Water => 12.,
            Liquid::Lava => 18.,
        };
        let offset = world.fall_offset.get(&p).copied().unwrap_or_else(|| {
            [-1, 1]
                .into_iter()
                .find(|dx| {
                    world
                        .cells
                        .get(&(p.0 + dx, p.1))
                        .is_some_and(|v| v.kind == kind && v.amount > 0)
                        && world.supported.contains(&(p.0 + dx, p.1))
                })
                .map_or(0., |dx| dx as f32 * (32. - width * 0.5))
        });
        let stream_center = center + offset;
        let low = if settled { top } else { bottom };
        // A pool spilling sideways meets the falling sheet at the shared
        // cell boundary. These non-overlapping necks also participate in contact.
        for dx in [-1, 1] {
            let q = (p.0 + dx, p.1);
            if let Some(v) = world
                .cells
                .get(&q)
                .filter(|v| v.kind == kind && v.amount > 0 && world.supported.contains(&q))
            {
                let join_top = bottom + v.amount as f32 * 8.;
                if join_top > low {
                    let (left, right) = if dx < 0 {
                        (center - 32., stream_center - width * 0.5)
                    } else {
                        (stream_center + width * 0.5, center + 32.)
                    };
                    if right > left {
                        rects.push((left, right, low, join_top));
                    }
                }
            }
        }
        if low < high {
            rects.push((
                stream_center - width * 0.5,
                stream_center + width * 0.5,
                low,
                high,
            ));
        }
    }
    rects
}

// Build the boundary of the fluid union: internal transparent caps cause dark seams.
pub(super) fn exposed_segments(low: f32, high: f32, covers: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut result = vec![(low, high)];
    for &(a, b) in covers {
        result = result
            .into_iter()
            .flat_map(|(l, h)| {
                if b <= l || a >= h {
                    vec![(l, h)]
                } else {
                    let mut r = Vec::new();
                    if a > l {
                        r.push((l, a.min(h)));
                    }
                    if b < h {
                        r.push((b.max(l), h));
                    }
                    r
                }
            })
            .collect();
    }
    result
}
pub(super) fn liquid_mesh(world: &FluidWorld, kind: Liquid, loaded: impl Fn(Cell) -> bool) -> Mesh {
    // One connected volume at terrain-front depth. A detached foreground sheet
    // creates a second silhouette under perspective and must not cover the world.
    let front = crate::systems::actor_depth::WATER_FRONT;
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    let mut indices = Vec::<u32>::new();
    let cells: BTreeSet<_> = world
        .cells
        .keys()
        .chain(world.falling.keys())
        .copied()
        .collect();
    let rects: Vec<_> = cells
        .into_iter()
        .filter(|p| loaded(*p))
        .flat_map(|p| fluid_sections(world, p, kind))
        .collect();
    let key = |v: f32| (v * 1024.).round() as i64;
    let mut lefts: HashMap<i64, Vec<(f32, f32)>> = HashMap::new();
    let mut rights = lefts.clone();
    let mut bottoms = lefts.clone();
    let mut tops = lefts.clone();
    for &(l, r, b, t) in &rects {
        lefts.entry(key(l)).or_default().push((b, t));
        rights.entry(key(r)).or_default().push((b, t));
        bottoms.entry(key(b)).or_default().push((l, r));
        tops.entry(key(t)).or_default().push((l, r));
    }
    let mut quad = |v: [[f32; 3]; 4], n: [f32; 3]| {
        let base = positions.len() as u32;
        positions.extend(v);
        normals.extend([n; 4]);
        uvs.extend([[0., 1.], [1., 1.], [1., 0.], [0., 0.]]);
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    };
    for (l, r, b, t) in rects {
        quad(
            [[l, b, front], [r, b, front], [r, t, front], [l, t, front]],
            [0., 0., 1.],
        );
        for (a, z) in exposed_segments(l, r, bottoms.get(&key(t)).map_or(&[], Vec::as_slice)) {
            quad(
                [[a, t, front], [z, t, front], [z, t, -40.], [a, t, -40.]],
                [0., 1., 0.],
            );
        }
        for (a, z) in exposed_segments(l, r, tops.get(&key(b)).map_or(&[], Vec::as_slice)) {
            quad(
                [[a, b, -40.], [z, b, -40.], [z, b, front], [a, b, front]],
                [0., -1., 0.],
            );
        }
        for (a, z) in exposed_segments(b, t, rights.get(&key(l)).map_or(&[], Vec::as_slice)) {
            quad(
                [[l, a, -40.], [l, a, front], [l, z, front], [l, z, -40.]],
                [-1., 0., 0.],
            );
        }
        for (a, z) in exposed_segments(b, t, lefts.get(&key(r)).map_or(&[], Vec::as_slice)) {
            quad(
                [[r, a, front], [r, a, -40.], [r, z, -40.], [r, z, front]],
                [1., 0., 0.],
            );
        }
    }
    empty_mesh()
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}

pub(super) fn steam_opacity(age: f32) -> f32 {
    let rise = (age / 0.15).clamp(0., 1.);
    let fade = ((1. - age) / 0.65).clamp(0., 1.);
    rise * rise * (3. - 2. * rise) * fade * fade * (3. - 2. * fade)
}
pub(super) fn steam_mesh(world: &FluidWorld) -> Mesh {
    let mut p = Vec::new();
    let mut n = Vec::new();
    let mut uv = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    for (&cell, &ttl) in &world.steam {
        let age = (30 - ttl.min(30)) as f32;
        let x = cell.0 as f32 * 64.;
        let progress = age / 30.;
        let y = cell.1 as f32 * 64. + age * 0.9;
        let r = 13. + progress * 10.;
        let alpha = steam_opacity(progress);
        colors.extend([[1., 1., 1., alpha]; 4]);
        let b = p.len() as u32;
        p.extend([
            [x - r, y - r, 40.],
            [x + r, y - r, 40.],
            [x + r, y + r, 40.],
            [x - r, y + r, 40.],
        ]);
        n.extend([[0., 0., 1.]; 4]);
        uv.extend([[0., 1.], [1., 1.], [1., 0.], [0., 0.]]);
        indices.extend([b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    empty_mesh()
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, p)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, n)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}
pub(super) fn render(
    settings: Option<Res<crate::resources::graphics_settings::GraphicsSettings>>,
    world: Res<FluidWorld>,
    revision: Res<FluidRenderRevision>,
    loaded: Res<LoadedChunks>,
    assets: Res<VisualAssets>,
    time: Res<Time>,
    mut materials: ResMut<Assets<FluidMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for handle in &assets.materials {
        if let Some(mut material) = materials.get_mut(handle) {
            material.effect.x = if settings.as_ref().is_none_or(|s| s.liquid_animation) {
                time.elapsed_secs()
            } else {
                0.
            };
        }
    }
    if !revision.is_changed() && !loaded.is_changed() {
        return;
    }
    if let Some(mut mesh) = meshes.get_mut(&assets.meshes[2]) {
        let next = steam_mesh(&world);
        // Avoid resubmitting an unchanged empty mesh (and allocator warnings) every frame.
        if mesh.count_vertices() != 0 || next.count_vertices() != 0 {
            *mesh = next;
        }
    }
    let is_loaded = |p: Cell| {
        let c = chunk(p);
        loaded.is_loaded(c.0, c.1)
    };
    for (i, kind) in [Liquid::Water, Liquid::Lava].into_iter().enumerate() {
        if let Some(mut mesh) = meshes.get_mut(&assets.meshes[i]) {
            let next = liquid_mesh(&world, kind, is_loaded);
            if mesh.count_vertices() != 0 || next.count_vertices() != 0 {
                *mesh = next;
            }
        }
    }
}

/// Renderer observes only topology/appearance changes, not simulation clocks,
/// source reserves, occupancy, damage cooldowns or bookkeeping.
#[derive(Resource, Default)]
pub(super) struct FluidRenderRevision(u64);
#[derive(Default)]
pub(super) struct VisualSnapshot {
    cells: Vec<(Cell, Liquid, u8)>,
    falling: BTreeMap<Cell, (Liquid, u8)>,
    cooled: BTreeSet<Cell>,
    ice: Vec<Cell>,
    offsets: BTreeMap<Cell, f32>,
    supported: HashSet<Cell>,
    steam: BTreeMap<Cell, u8>,
}
pub(super) fn update_visual_revision(
    world: Res<FluidWorld>,
    mut snapshot: Local<VisualSnapshot>,
    mut revision: ResMut<FluidRenderRevision>,
) {
    if !world.is_changed() {
        return;
    }
    let equal = snapshot
        .cells
        .iter()
        .copied()
        .eq(world.cells.iter().map(|(&p, v)| (p, v.kind, v.amount)))
        && snapshot.falling == world.falling
        && snapshot.offsets == world.fall_offset
        && snapshot.supported == world.supported
        && snapshot.steam == world.steam
        && snapshot.cooled == world.cooled
        && snapshot.ice.iter().eq(world.ice.keys());
    if equal {
        return;
    }
    snapshot.cells.clear();
    snapshot
        .cells
        .extend(world.cells.iter().map(|(&p, v)| (p, v.kind, v.amount)));
    snapshot.falling.clone_from(&world.falling);
    snapshot.offsets.clone_from(&world.fall_offset);
    snapshot.supported.clone_from(&world.supported);
    snapshot.steam.clone_from(&world.steam);
    snapshot.cooled.clone_from(&world.cooled);
    snapshot.ice.clear();
    snapshot.ice.extend(world.ice.keys().copied());
    revision.0 = revision.0.wrapping_add(1);
}

#[cfg(test)]
mod revision_tests {
    use super::*;
    #[test]
    fn occupancy_and_simulation_bookkeeping_do_not_dirty_geometry() {
        let mut app = App::new();
        app.init_resource::<FluidWorld>()
            .init_resource::<FluidOccupancy>()
            .init_resource::<FluidRenderRevision>()
            .add_systems(Update, (update_occupancy, update_visual_revision).chain());
        app.update();
        assert_eq!(app.world().resource::<FluidRenderRevision>().0, 0);
        app.world_mut()
            .spawn((Health::new(6), Transform::default()));
        app.update();
        assert!(!app.world().resource::<FluidOccupancy>().0.is_empty());
        assert_eq!(app.world().resource::<FluidRenderRevision>().0, 0);
        app.world_mut()
            .resource_mut::<FluidWorld>()
            .deposit((0, 0), Liquid::Water, 8);
        app.update();
        let revision = app.world().resource::<FluidRenderRevision>().0;
        assert!(revision > 0);
        {
            let mut w = app.world_mut().resource_mut::<FluidWorld>();
            w.tick += 1;
            w.cells.get_mut(&(0, 0)).unwrap().reserve = 40;
        }
        app.update();
        assert_eq!(app.world().resource::<FluidRenderRevision>().0, revision);
        app.world_mut()
            .resource_mut::<FluidWorld>()
            .steam
            .insert((1, 0), 10);
        app.update();
        assert_eq!(
            app.world().resource::<FluidRenderRevision>().0,
            revision + 1
        );
        app.world_mut()
            .resource_mut::<FluidWorld>()
            .cooled
            .insert((0, 0));
        app.update();
        assert_eq!(
            app.world().resource::<FluidRenderRevision>().0,
            revision + 2
        );
    }
}
