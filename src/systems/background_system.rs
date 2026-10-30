use crate::components::background::{Background, BackgroundCache};
use crate::components::block::BiomeType;
use crate::components::hero::Hero;
use crate::resources::asset_handles::BlockTextures;
use crate::utils::biome_utils::determine_biome_at_position;
use bevy::prelude::*;

// Retain a soft directional sheen without the original bright corner wash.
pub(crate) const BACKDROP_ROUGHNESS: f32 = 0.62;
pub(crate) const BACKDROP_REFLECTANCE: f32 = 0.30;

/// Get material for a specific biome
fn get_biome_material(
    biome: BiomeType,
    block_textures: &BlockTextures,
    materials: &mut Assets<StandardMaterial>,
    bg_cache: &mut BackgroundCache,
) -> Handle<StandardMaterial> {
    // Return cached material if it exists
    if let Some(material) = bg_cache.materials.get(&biome) {
        return material.clone();
    }

    // Create new material for this biome
    // All backgrounds darkened to 0.4 for consistent visibility
    info!("Creating background material for biome: {:?}", biome);
    let (bg_color, bg_texture) = match biome {
        BiomeType::Nether => (Color::srgb(0.4, 0.4, 0.4), block_textures.nether.clone()),
        BiomeType::End => (Color::srgb(0.4, 0.4, 0.4), block_textures.obsidian.clone()),
        BiomeType::Underground => (
            Color::srgb(0.4, 0.4, 0.4),
            block_textures.stone.clone(), // Use stone for underground
        ),
        BiomeType::Desert => (Color::srgb(0.4, 0.4, 0.4), block_textures.sandstone.clone()),
        BiomeType::Forest => (Color::srgb(0.4, 0.4, 0.4), block_textures.oak_log.clone()),
        BiomeType::Plains => (
            Color::srgb(0.4, 0.4, 0.4),
            block_textures.dirt.clone(), // Use dirt for plains
        ),
        BiomeType::Mountains => (
            Color::srgb(0.4, 0.4, 0.4),
            block_textures.stone.clone(), // Use stone for mountains
        ),
        BiomeType::Iceland => (
            Color::srgb(0.4, 0.4, 0.4),
            block_textures.packed_ice.clone(), // Use packed ice for iceland
        ),
    };

    let material = materials.add(StandardMaterial {
        base_color: bg_color,
        base_color_texture: Some(bg_texture),
        unlit: true,
        alpha_mode: AlphaMode::Opaque,
        perceptual_roughness: BACKDROP_ROUGHNESS,
        reflectance: BACKDROP_REFLECTANCE,
        ..default()
    });

    bg_cache.materials.insert(biome, material.clone());
    material
}

const TILE_SIZE: f32 = 64.;

fn grid_origin(center: IVec2, size: IVec2) -> IVec2 {
    center - size / 2
}

/// Keep every resident tile fixed in world space. Moving the whole grid makes
/// temporal renderers interpret stationary wallpaper as moving geometry.
pub fn update_background(
    mut commands: Commands,
    hero_query: Query<&Transform, With<Hero>>,
    camera_query: Query<&Transform, (With<Camera3d>, Without<Hero>, Without<Background>)>,
    background_query: Query<(Entity, &Background)>,
    block_textures: Res<BlockTextures>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut bg_cache: ResMut<BackgroundCache>,
    windows: Query<&Window>,
    mut last_grid: Local<Option<(IVec2, IVec2)>>,
) {
    if hero_query.single().is_err() {
        return;
    }
    let Ok(camera) = camera_query.single() else {
        return;
    };
    let Some(window) = windows.iter().next() else {
        return;
    };
    // Preserve the existing generous off-screen padding and visual scale.
    let size = (Vec2::new(window.width(), window.height()) * 6. / TILE_SIZE)
        .ceil()
        .as_ivec2()
        .max(IVec2::ONE);
    let center = (camera.translation.truncate() / TILE_SIZE)
        .round()
        .as_ivec2();
    if *last_grid == Some((center, size)) && !background_query.is_empty() {
        return;
    }
    let origin = grid_origin(center, size);
    let end = origin + size;
    let mut resident = std::collections::HashSet::new();
    for (entity, tile) in &background_query {
        if tile.cell.cmpge(origin).all() && tile.cell.cmplt(end).all() {
            resident.insert(tile.cell);
        } else {
            // Only discard out-of-range tiles. A new edge tile starts with new
            // transform history instead of inheriting a distant tile's velocity.
            commands.entity(entity).despawn();
        }
    }
    let mesh = bg_cache
        .mesh
        .get_or_insert_with(|| meshes.add(Rectangle::new(TILE_SIZE, TILE_SIZE)))
        .clone();
    for y in origin.y..end.y {
        for x in origin.x..end.x {
            let cell = IVec2::new(x, y);
            if resident.contains(&cell) {
                continue;
            }
            let position = cell.as_vec2() * TILE_SIZE;
            let biome = determine_biome_at_position(position.x, position.y);
            let material =
                get_biome_material(biome, &block_textures, &mut materials, &mut bg_cache);
            commands.spawn((
                Background { biome, cell },
                bevy::light::NotShadowCaster,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(position.x, position.y, super::actor_depth::BACKGROUND),
            ));
        }
    }
    *last_grid = Some((center, size));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn fixture() -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<BlockTextures>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<BackgroundCache>()
            .add_systems(Update, update_background);
        app.world_mut().spawn((
            Hero {
                move_direction: 1.,
                direction_change_timer: Timer::default(),
            },
            Transform::default(),
        ));
        app.world_mut().spawn(Window {
            resolution: (96, 96).into(),
            ..default()
        });
        let camera = app
            .world_mut()
            .spawn((Camera3d::default(), Transform::from_xyz(17., 13., 1500.)))
            .id();
        (app, camera)
    }

    fn snapshot(app: &mut App) -> HashMap<IVec2, (Entity, Transform)> {
        app.world_mut()
            .query::<(Entity, &Background, &Transform)>()
            .iter(app.world())
            .map(|(e, b, t)| (b.cell, (e, *t)))
            .collect()
    }

    #[test]
    fn camera_movement_keeps_overlapping_tiles_and_their_transform_history() {
        let (mut app, camera) = fixture();
        app.update();
        let before = snapshot(&mut app);
        assert_eq!(before.len(), 81); // Odd grid dimensions must not lose a row/column.
        // Change archetype order, as native-renderer attachment does in game.
        for (i, (entity, _)) in before.values().enumerate() {
            if i % 2 == 0 {
                app.world_mut()
                    .entity_mut(*entity)
                    .insert(Name::new("reordered"));
            }
        }
        app.world_mut()
            .get_mut::<Transform>(camera)
            .unwrap()
            .translation
            .x = 65.;
        app.update();
        let after = snapshot(&mut app);
        assert_eq!(after.len(), 81);
        let mut retained = 0;
        for (cell, (entity, transform)) in &after {
            assert_eq!(transform.translation.truncate(), cell.as_vec2() * TILE_SIZE);
            if let Some((old_entity, old_transform)) = before.get(cell) {
                retained += 1;
                assert_eq!(entity, old_entity);
                assert_eq!(transform, old_transform);
            }
        }
        assert_eq!(retained, 72); // Only the off-screen edge column was replaced.
        app.update();
        assert_eq!(snapshot(&mut app), after);
    }

    #[test]
    fn resizing_and_teleporting_leave_a_complete_unique_grid() {
        let (mut app, camera) = fixture();
        app.update();
        app.world_mut()
            .query::<&mut Window>()
            .single_mut(app.world_mut())
            .unwrap()
            .resolution
            .set(160., 96.);
        app.world_mut()
            .get_mut::<Transform>(camera)
            .unwrap()
            .translation = Vec3::new(-3200., 6400., 1500.);
        app.update();
        let tiles = snapshot(&mut app);
        assert_eq!(tiles.len(), 15 * 9);
        let origin = grid_origin(IVec2::new(-50, 100), IVec2::new(15, 9));
        for y in origin.y..origin.y + 9 {
            for x in origin.x..origin.x + 15 {
                assert!(tiles.contains_key(&IVec2::new(x, y)));
            }
        }
    }
}

/// Opt-in moving-camera test: exercise real renderer archetype changes and
/// assert that resident backdrop entities never acquire artificial motion.
pub fn motion_probe(
    time: Res<Time<Real>>,
    mut camera: Query<&mut Transform, (With<Camera3d>, Without<Background>)>,
    tiles: Query<(Entity, &Transform), With<Background>>,
    mut previous: Local<std::collections::HashMap<Entity, Vec3>>,
    mut anchor: Local<Option<Vec3>>,
    mut checks: Local<u64>,
    mut reported: Local<bool>,
) {
    let t = time.elapsed_secs();
    if t < 8.5 {
        return;
    }
    let current: std::collections::HashMap<_, _> =
        tiles.iter().map(|(e, t)| (e, t.translation)).collect();
    for (entity, position) in &current {
        if let Some(old) = previous.get(entity) {
            assert_eq!(
                position, old,
                "Resident background tile moved; invalid temporal motion"
            );
            *checks += 1;
        }
    }
    *previous = current;
    if let Ok(mut camera) = camera.single_mut() {
        let base = *anchor.get_or_insert(camera.translation);
        camera.translation = base
            + Vec3::new(
                ((t - 8.5) * 0.8).sin() * 250.,
                ((t - 8.5) * 0.6).sin() * 180.,
                0.,
            );
    }
    if t > 39. && !*reported {
        assert!(
            *checks > 1000,
            "Background movement probe did not exercise resident tiles"
        );
        eprintln!(
            "[BACKGROUND MOTION] completed {} resident-position checks",
            *checks
        );
        *reported = true;
    }
}
