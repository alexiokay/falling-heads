//! Sparse low fog banks attached to open forest floors, never a screen-wide veil.
use crate::{
    components::{
        block::{BiomeType, Block, BlockType},
        hero::Hero,
    },
    resources::{graphics_settings::GraphicsSettings, world_seed::WorldSeed},
};
use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};
use std::collections::{HashMap, HashSet};
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
struct MistMaterial {
    #[uniform(0)]
    effect: Vec4,
    #[uniform(1)]
    tint: Vec4,
}
impl Material for MistMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/biome_mist.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
#[derive(Component)]
struct Mist {
    floor: IVec2,
    opacity: f32,
    last_wanted: f32,
}
#[derive(Resource)]
struct MistMesh(Handle<Mesh>);
#[derive(Resource, Default)]
struct Pockets {
    wanted: HashMap<IVec2, f32>,
    next_scan: f32,
}
const MAX_POCKETS: usize = 10;
fn rank(p: IVec2, seed: u32) -> u32 {
    let mut h = (p.x as u32).wrapping_mul(73856093) ^ (p.y as u32).wrapping_mul(19349663) ^ seed;
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb352d);
    h ^= h >> 15;
    h
}
fn select_pockets(
    occupied: &HashSet<IVec2>,
    floors: &HashSet<IVec2>,
    center: Vec2,
    seed: u32,
) -> Vec<IVec2> {
    let mut candidates: Vec<_> = floors
        .iter()
        .copied()
        .filter(|p| {
            let pos = p.as_vec2() * 64.;
            pos.distance_squared(center) < 2200_f32.powi(2)
                && rank(*p, seed) % 3 == 0
                && crate::utils::biome_utils::determine_biome_at_position(pos.x, pos.y)
                    == BiomeType::Forest
                && (floors.contains(&(*p + IVec2::X)) || floors.contains(&(*p - IVec2::X)))
                && (-1..=1).all(|dx| !occupied.contains(&(*p + IVec2::new(dx, 1))))
                && (-3..=3)
                    .all(|dx| (2..=3).all(|dy| !occupied.contains(&(*p + IVec2::new(dx, dy)))))
        })
        .collect();
    candidates.sort_by_key(|p| {
        (
            ((p.as_vec2() * 64. - center).length() / 400.) as i32,
            rank(*p, seed),
            p.x,
            p.y,
        )
    });
    let mut selected: Vec<IVec2> = Vec::new();
    for candidate in candidates {
        if selected
            .iter()
            .all(|p| (candidate - *p).as_vec2().length_squared() > 9_f32.powi(2))
        {
            selected.push(candidate);
            if selected.len() == MAX_POCKETS {
                break;
            }
        }
    }
    selected
}
pub struct BiomeMistPlugin;
impl Plugin for BiomeMistPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<MistMaterial>::default())
            .init_resource::<Pockets>()
            .add_systems(Startup, setup)
            .add_systems(Update, update);
    }
}
fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(MistMesh(meshes.add(Rectangle::new(1., 1.))));
}
fn update(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<GraphicsSettings>,
    mesh: Res<MistMesh>,
    seed: Res<WorldSeed>,
    hero: Query<&Transform, With<Hero>>,
    blocks: Query<(&Transform, &BlockType), With<Block>>,
    mut pockets: ResMut<Pockets>,
    mut mist: Query<(
        Entity,
        &mut Mist,
        &mut Visibility,
        &MeshMaterial3d<MistMaterial>,
    )>,
    mut materials: ResMut<Assets<MistMaterial>>,
) {
    let Ok(hero) = hero.single() else {
        return;
    };
    let now = time.elapsed_secs();
    if now >= pockets.next_scan {
        pockets.next_scan = now + 0.65;
        let mut occupied = HashSet::new();
        let mut floors = HashSet::new();
        for (t, kind) in &blocks {
            if t.translation.xy().distance_squared(hero.translation.xy()) > 2500_f32.powi(2) {
                continue;
            }
            let p = (t.translation.xy() / 64.).round().as_ivec2();
            // Fog can flow through foliage; only solid terrain defines the pocket.
            if !matches!(
                kind,
                BlockType::DeadBush
                    | BlockType::Mushroom
                    | BlockType::Vines
                    | BlockType::Cobweb
                    | BlockType::Cactus
                    | BlockType::Leaves
            ) {
                occupied.insert(p);
                // Crowns and branches must not produce floating clouds above trees.
                if !matches!(kind, BlockType::Wood | BlockType::Leaves) {
                    floors.insert(p);
                }
            }
        }
        pockets.wanted = select_pockets(&occupied, &floors, hero.translation.xy(), seed.seed)
            .into_iter()
            .map(|p| (p, 0.40))
            .collect();
    }
    let mut retained = HashSet::new();
    let mut live = 0;
    for (entity, mut bank, mut visibility, handle) in &mut mist {
        let nearby =
            (bank.floor.as_vec2() * 64.).distance_squared(hero.translation.xy()) < 2600_f32.powi(2);
        if pockets.wanted.contains_key(&bank.floor) {
            bank.last_wanted = now;
        }
        // Keep an established bank while crossing candidate/chunk boundaries.
        let target = if settings.fog && nearby && now - bank.last_wanted < 12. {
            0.40
        } else {
            0.
        };
        bank.opacity += (target - bank.opacity)
            * (1. - (-time.delta_secs() * if target > 0. { 0.8 } else { 0.28 }).exp());
        if target == 0. && bank.opacity < 0.005 {
            commands.entity(entity).despawn();
            materials.remove(handle.0.id());
            continue;
        }
        live += 1;
        retained.insert(bank.floor);
        *visibility = if settings.fog {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.effect.x = now;
            material.effect.y = bank.opacity;
        }
    }
    if !settings.fog {
        return;
    }
    let mut wanted: Vec<_> = pockets.wanted.keys().copied().collect();
    wanted.sort_by_key(|p| (p.x, p.y));
    for floor in wanted {
        if retained.contains(&floor) || live >= MAX_POCKETS {
            continue;
        }
        let h = rank(floor, seed.seed);
        let height = 448. + (h % 97) as f32;
        let width = 960. + ((h >> 8) % 321) as f32;
        let position = floor.as_vec2() * 64. + Vec2::new(0., 64.);
        let (color, _) = super::biome_climate::mist_style(BiomeType::Forest);
        commands.spawn((
            Mist {
                floor,
                opacity: 0.,
                last_wanted: now,
            },
            Mesh3d(mesh.0.clone()),
            MeshMaterial3d(materials.add(MistMaterial {
                effect: Vec4::new(now, 0., 0., (h % 1000) as f32 * 0.013),
                tint: color.extend(1.),
            })),
            Transform::from_translation(position.extend(super::actor_depth::MIST))
                .with_scale(Vec3::new(width, height, 1.)),
            Visibility::Visible,
            bevy::light::NotShadowCaster,
        ));
        live += 1;
    }
}
/// Focus the existing natural bank for a reproducible visual check; never spawn test fog.
pub(crate) fn smoke_focus(world: &mut World) -> Option<Vec3> {
    let hero = world
        .query_filtered::<&Transform, With<Hero>>()
        .single(world)
        .ok()?
        .translation;
    world
        .query_filtered::<&Transform, With<Mist>>()
        .iter(world)
        .min_by(|a, b| {
            a.translation
                .distance_squared(hero)
                .total_cmp(&b.translation.distance_squared(hero))
        })
        .map(|t| Vec3::new(t.translation.x, t.translation.y + 120., 90.))
}
pub(crate) fn smoke_verify(world: &mut World) {
    assert_eq!(
        world.query::<&DistanceFog>().iter(world).count(),
        0,
        "No camera-wide fog in a shallow 2.5D scene"
    );
    let banks: Vec<_> = world
        .query_filtered::<&Transform, With<Mist>>()
        .iter(world)
        .collect();
    assert!(!banks.is_empty() && banks.len() <= MAX_POCKETS);
    for t in banks {
        assert!(t.scale.y <= 544. && t.scale.x <= 1280. && t.translation.z < 0.);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn banks_require_forest_floors_clear_air_and_separation() {
        let y = (crate::utils::biome_utils::nearest_biome_y(0., BiomeType::Forest) / 64.) as i32;
        let floors: HashSet<_> = (-50..=50).map(|x| IVec2::new(x, y)).collect();
        let center = Vec2::new(0., y as f32 * 64.);
        let selected = select_pockets(&floors, &floors, center, 42);
        assert!(!selected.is_empty() && selected.len() <= MAX_POCKETS);
        for (i, p) in selected.iter().enumerate() {
            for q in selected.iter().skip(i + 1) {
                assert!((*p - *q).as_vec2().length() > 9.);
            }
        }
        assert!(select_pockets(&HashSet::new(), &HashSet::new(), center, 42).is_empty());
        let mut blocked = floors.clone();
        for p in &floors {
            blocked.insert(*p + IVec2::Y);
        }
        assert!(select_pockets(&blocked, &floors, center, 42).is_empty());
        let desert_y =
            (crate::utils::biome_utils::nearest_biome_y(0., BiomeType::Desert) / 64.) as i32;
        let desert: HashSet<_> = (-50..=50).map(|x| IVec2::new(x, desert_y)).collect();
        assert!(
            select_pockets(&desert, &desert, Vec2::new(0., desert_y as f32 * 64.), 42).is_empty()
        );
    }
}
