use super::catalog::{TILES, Tile};
use bevy::prelude::*;
use std::collections::HashMap;
#[derive(Resource)]
pub(super) struct Art {
    pub cube: Handle<Mesh>,
    pub face: Handle<Mesh>,
    pub plant: Handle<Mesh>,
    pub small_face: Handle<Mesh>,
    pub quad: Handle<Mesh>,
    pub tiles: HashMap<Tile, Handle<StandardMaterial>>,
    pub faces: Vec<Handle<StandardMaterial>>,
    pub shots: Vec<Handle<StandardMaterial>>,
    pub drops: Vec<Handle<StandardMaterial>>,
    magma: [Handle<Image>; 3],
    pub sounds: Vec<Handle<AudioSource>>,
}
pub(super) fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let load = |name: &str| {
        server
            .load_builder()
            .with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
                s.sampler = bevy::image::ImageSampler::nearest()
            })
            .load(format!("textures/biome_content/{name}.png"))
    };
    let mut tiles = HashMap::new();
    for &tile in TILES {
        let texture = if tile.decorative() {
            tile.texture().into()
        } else {
            format!("{}_atlas", tile.texture())
        };
        tiles.insert(
            tile,
            mats.add(StandardMaterial {
                base_color_texture: Some(load(&texture)),
                base_color: if matches!(tile, Tile::Grass | Tile::Fern) {
                    Color::srgb(0.48, 0.75, 0.28)
                } else {
                    Color::WHITE
                },
                alpha_mode: if tile.decorative() || tile == Tile::Azalea {
                    AlphaMode::Mask(0.4)
                } else {
                    AlphaMode::Opaque
                },
                cull_mode: None,
                perceptual_roughness: 0.85,
                emissive: if tile == Tile::Magma {
                    LinearRgba::rgb(1.5, 0.3, 0.01)
                } else {
                    LinearRgba::BLACK
                },
                ..default()
            }),
        );
    }
    let faces = super::enemies::SPECIES
        .iter()
        .map(|s| {
            mats.add(StandardMaterial {
                base_color_texture: Some(load(s.asset())),
                unlit: true,
                alpha_mode: AlphaMode::Mask(0.3),
                ..default()
            })
        })
        .collect();
    let shots = [
        "poison_arrow",
        "frost_arrow",
        "wind_charge",
        "shulker_spark",
        "amethyst_shard",
        "fire_charge",
    ]
    .iter()
    .enumerate()
    .map(|(i, n)| {
        mats.add(StandardMaterial {
            base_color_texture: Some(load(n)),
            base_color: match i {
                0 | 1 => Color::WHITE,
                3 => Color::srgb(0.9, 0.55, 1.),
                _ => Color::WHITE,
            },
            unlit: true,
            alpha_mode: AlphaMode::Mask(0.1),
            cull_mode: None,
            ..default()
        })
    })
    .collect();
    let drops = [
        "sweet_berries",
        "blaze_rod",
        "slime_ball",
        "shulker_shell",
        "amethyst_shard",
    ]
    .iter()
    .map(|n| {
        mats.add(StandardMaterial {
            base_color_texture: Some(load(n)),
            unlit: true,
            alpha_mode: AlphaMode::Mask(0.2),
            ..default()
        })
    })
    .collect();
    commands.insert_resource(Art {
        cube: meshes.add(crate::resources::cube_mesh::create_uv_cube(
            64.,
            crate::resources::cube_mesh::UVLayout::oak_log(),
        )),
        face: meshes.add(Rectangle::new(64., 64.)),
        plant: meshes.add(
            Mesh::from(Rectangle::new(64., 64.)).transformed_by(Transform::from_xyz(0., 0., 24.)),
        ),
        small_face: meshes.add(Rectangle::new(42., 28.)),
        quad: meshes.add(Rectangle::new(1., 1.)),
        tiles,
        faces,
        shots,
        drops,
        sounds: [
            "blaze_shoot",
            "blaze_hurt",
            "blaze_death",
            "shulker_shoot",
            "creeper_fuse",
            "slime_jump",
            "magma_jump",
        ]
        .iter()
        .map(|s| server.load(format!("audio/biome_content/{s}.ogg")))
        .collect(),
        magma: std::array::from_fn(|i| load(&format!("magma_{i}_atlas"))),
    });
}
pub(super) fn finish(
    mut commands: Commands,
    art: Res<Art>,
    tiles: Query<(Entity, &Tile, &Transform), Added<Tile>>,
) {
    for (e, tile, t) in &tiles {
        // Keep the owner at the grid center; visual scale must not alter collision extent.
        let transform = Transform::from_translation(t.translation);
        let mesh = if tile.decorative() {
            art.plant.clone()
        } else {
            art.cube.clone()
        };
        commands.entity(e).insert((
            Mesh3d(mesh),
            MeshMaterial3d(art.tiles[tile].clone()),
            transform,
        ));
    }
}
pub(super) fn animate_magma(
    time: Res<Time>,
    art: Res<Art>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut last: Local<usize>,
) {
    let frame = (time.elapsed_secs() * 5.) as usize % 3;
    if *last == frame {
        return;
    }
    *last = frame;
    if let Some(mut m) = mats.get_mut(&art.tiles[&Tile::Magma]) {
        m.base_color_texture = Some(art.magma[frame].clone());
    }
}

/// Variant tiles follow Graphics > Lighting like ordinary blocks: with lighting
/// off they render flat at full colour instead of going dark at night and in caves.
pub(super) fn follow_lighting(
    settings: Option<Res<crate::resources::graphics_settings::GraphicsSettings>>,
    art: Option<Res<Art>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let (Some(settings), Some(art)) = (settings, art) else {
        return;
    };
    if !settings.is_changed() && !art.is_added() {
        return;
    }
    for handle in art.tiles.values() {
        if let Some(mut m) = mats.get_mut(handle)
            && m.unlit == settings.lighting
        {
            m.unlit = !settings.lighting;
        }
    }
}
