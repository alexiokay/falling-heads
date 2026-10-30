use crate::{
    components::{
        block::{Block, BlockType},
        hero::Hero,
        snow_layer::SnowLayer,
        weather::{ActiveWeather, WeatherType},
    },
    utils::constants::BLOCK_SIZE,
};
use bevy::prelude::*;
use std::collections::HashSet;

pub struct SnowAccumulationPlugin;
impl Plugin for SnowAccumulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SnowVisuals>().add_systems(
            Update,
            (
                wash_submerged_snow,
                accumulate_snow,
                melt_snow,
                sync_snow_caps,
            )
                .chain()
                .after(super::weather_core::WeatherConditions)
                .before(crate::plugins::physics_plugin::PhysicsSet::Collision)
                .run_if(in_state(crate::states::InGame)),
        );
    }
}
#[derive(Component)]
struct SnowCap;
#[derive(Resource, Default)]
struct SnowVisuals {
    mesh: Option<Handle<Mesh>>,
    material: Option<Handle<StandardMaterial>>,
    mushroom_meshes: Vec<Handle<Mesh>>,
}
fn cell(pos: Vec3) -> (i32, i32) {
    (
        (pos.x / BLOCK_SIZE).round() as i32,
        (pos.y / BLOCK_SIZE).round() as i32,
    )
}
fn wet_surface(
    fluids: &crate::systems::fluids::FluidWorld,
    t: &Transform,
    kind: BlockType,
) -> bool {
    let top = if kind == BlockType::Mushroom {
        -1.5
    } else {
        BLOCK_SIZE * 0.5
    };
    fluids.covers_surface(t.translation.xy() + Vec2::Y * (top + 0.1))
}
fn wash_submerged_snow(
    mut commands: Commands,
    fluids: Res<crate::systems::fluids::FluidWorld>,
    blocks: Query<(Entity, &Transform, &BlockType), With<SnowLayer>>,
) {
    for (e, t, k) in &blocks {
        if wet_surface(&fluids, t, *k) {
            commands.entity(e).remove::<SnowLayer>();
        }
    }
}
fn accumulate_snow(
    mut commands: Commands,
    fluids: Res<crate::systems::fluids::FluidWorld>,
    weather: Res<ActiveWeather>,
    time: Res<Time>,
    hero: Query<&Transform, With<Hero>>,
    mut blocks: Query<
        (
            Entity,
            &Transform,
            &BlockType,
            Option<&mut SnowLayer>,
            Option<&crate::systems::biome_content::Tile>,
        ),
        With<Block>,
    >,
    mut timer: Local<f32>,
) {
    if !weather.is_snowing() {
        *timer = 0.0;
        return;
    }
    *timer += time.delta_secs();
    if *timer < 3.0 {
        return;
    }
    *timer %= 3.0;
    let Ok(hero) = hero.single() else {
        return;
    };
    // Weather is local to the scrolling playfield. Remote loaded roofs must not
    // suppress snow in every chamber below them. A ledge needs two clear cells
    // above it; narrow covered gaps and solid interior cells stay snow-free.
    let occupied: HashSet<_> = blocks
        .iter()
        .filter(|(_, _, kind, _, tile)| {
            **kind != BlockType::Cobweb && tile.is_none_or(|t| !t.decorative())
        })
        .map(|(_, t, _, _, _)| cell(t.translation))
        .collect();
    for (entity, transform, kind, layer, tile) in &mut blocks {
        if tile.is_some_and(|t| t.decorative()) {
            continue;
        }
        if *kind == BlockType::Cobweb || wet_surface(&fluids, transform, *kind) {
            continue;
        }
        if transform.translation.distance_squared(hero.translation) > 2000.0_f32.powi(2) {
            continue;
        }
        let (x, y) = cell(transform.translation);
        if (1..=2).any(|dy| occupied.contains(&(x, y + dy))) {
            continue;
        }
        if let Some(mut layer) = layer {
            if layer.stage < 4 {
                layer.add_snow();
            }
        } else {
            commands.entity(entity).insert(SnowLayer { stage: 1 });
        }
    }
}
fn melt_snow(
    mut commands: Commands,
    weather: Res<ActiveWeather>,
    time: Res<Time>,
    hero: Query<&Transform, With<Hero>>,
    terrain: Query<(&Transform, &BlockType), With<Block>>,
    mut blocks: Query<(Entity, &Transform, &mut SnowLayer), With<Block>>,
    mut fluids: ResMut<crate::systems::fluids::FluidWorld>,
    mut timer: Local<f32>,
) {
    if weather.is_snowing() {
        *timer = 0.0;
        return;
    }
    let fiery = weather.weather_type == WeatherType::FireRain
        && weather.duration_remaining > 0.
        && weather.intensity >= 0.1;
    let interval = if fiery { 0.75 } else { 5.0 };
    *timer += time.delta_secs();
    if *timer < interval {
        return;
    }
    *timer %= interval;
    let occupied: HashSet<_> = terrain
        .iter()
        .filter(|(_, k)| **k != BlockType::Cobweb)
        .map(|(t, _)| cell(t.translation))
        .collect();
    for (entity, transform, mut layer) in &mut blocks {
        let biome = crate::utils::biome_utils::determine_biome_at_position(
            transform.translation.x,
            transform.translation.y,
        );
        if !fiery && crate::systems::biome_climate::climate(biome).temperature_c <= 0. {
            continue;
        }
        if fiery {
            let Ok(hero) = hero.single() else {
                continue;
            };
            let p = cell(transform.translation);
            if transform.translation.distance_squared(hero.translation) > 2000_f32.powi(2)
                || (1..=2).any(|dy| occupied.contains(&(p.0, p.1 + dy)))
            {
                continue;
            }
            // Four thin cap stages are less than one full liquid cell.
            fluids.melt_cap((p.0, p.1 + 1));
        }
        layer.melt();
        if !layer.has_snow() {
            commands.entity(entity).remove::<SnowLayer>();
        }
    }
}
fn cap_transform(parent_scale: Vec3, thickness: f32) -> Transform {
    Transform::from_translation(Vec3::new(
        0.0,
        (BLOCK_SIZE + thickness) * 0.5 / parent_scale.y,
        0.0,
    ))
    .with_scale(Vec3::new(BLOCK_SIZE, thickness, BLOCK_SIZE) / parent_scale)
}
/// Coat the stepped cap, not the empty upper half of the mushroom's tile.
fn mushroom_snow_mesh(stage: u8) -> Mesh {
    let thickness = 0.4 + stage.min(4) as f32 * 0.4;
    let shelves = [
        (0., -1.5, 20., 22., 18.),
        (-13.5, -6., 20., 5., 26.),
        (13.5, -6., 20., 5., 26.),
        (0., -6., 9., 22., 4.),
        (0., -6., 31., 22., 4.),
    ];
    let mut mesh: Option<Mesh> = None;
    for (x, y, z, w, d) in shelves {
        let part = Mesh::from(Cuboid::new(w, thickness, d)).transformed_by(Transform::from_xyz(
            x,
            y + thickness * 0.5,
            z,
        ));
        if let Some(m) = mesh.as_mut() {
            m.merge(&part).unwrap();
        } else {
            mesh = Some(part);
        }
    }
    mesh.unwrap()
}
fn snow_shape(
    kind: Option<&BlockType>,
    scale: Vec3,
    stage: u8,
    visuals: &mut SnowVisuals,
    meshes: &mut Assets<Mesh>,
) -> (Handle<Mesh>, Transform) {
    if kind == Some(&BlockType::Mushroom) {
        if visuals.mushroom_meshes.is_empty() {
            for stage in 1..=4 {
                visuals
                    .mushroom_meshes
                    .push(meshes.add(mushroom_snow_mesh(stage)));
            }
        }
        (
            visuals.mushroom_meshes[stage.clamp(1, 4) as usize - 1].clone(),
            Transform::from_scale(scale.recip()),
        )
    } else {
        (
            visuals
                .mesh
                .get_or_insert_with(|| meshes.add(Cuboid::new(1., 1., 1.)))
                .clone(),
            cap_transform(scale, SnowLayer { stage }.thickness()),
        )
    }
}
fn sync_snow_caps(
    mut commands: Commands,
    blocks: Query<
        (
            Entity,
            &Transform,
            Option<&SnowLayer>,
            Option<&Children>,
            Option<&BlockType>,
        ),
        (With<Block>, Without<SnowCap>),
    >,
    mut caps: Query<(Entity, &ChildOf, &mut Transform, Option<&mut Mesh3d>), With<SnowCap>>,
    mut visuals: ResMut<SnowVisuals>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
) {
    // Physics uses the same SnowLayer::thickness() for its top contact surface.
    for (entity, parent, mut transform, mesh) in &mut caps {
        match blocks.get(parent.parent()) {
            Ok((_, block_transform, Some(layer), _, kind)) if layer.has_snow() => {
                let (next_mesh, next) = snow_shape(
                    kind,
                    block_transform.scale,
                    layer.stage,
                    &mut visuals,
                    &mut meshes,
                );
                if let Some(mut mesh) = mesh {
                    if mesh.0 != next_mesh {
                        mesh.0 = next_mesh;
                    }
                }
                if *transform != next {
                    *transform = next;
                }
            }
            _ => {
                commands.entity(entity).try_despawn();
            }
        }
    }
    for (entity, transform, layer, children, kind) in &blocks {
        let Some(layer) = layer.filter(|layer| layer.has_snow()) else {
            continue;
        };
        if children.is_some_and(|children| children.iter().any(|child| caps.contains(child))) {
            continue;
        }
        let (mesh, snow_transform) = snow_shape(
            kind,
            transform.scale,
            layer.stage,
            &mut visuals,
            &mut meshes,
        );
        let material = visuals
            .material
            .get_or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color_texture: Some(assets.load("textures/blocks/iceland/snow.png")),
                    unlit: true,
                    ..default()
                })
            })
            .clone();
        commands.entity(entity).with_child((
            SnowCap,
            Mesh3d(mesh),
            MeshMaterial3d(material),
            snow_transform,
        ));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn water_washes_existing_caps_and_prevents_underwater_snow() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<crate::systems::fluids::FluidWorld>()
            .insert_resource(ActiveWeather {
                weather_type: WeatherType::Snow,
                intensity: 1.,
                duration_remaining: 100.,
                ..default()
            })
            .add_systems(Update, (wash_submerged_snow, accumulate_snow).chain());
        app.world_mut().spawn((
            Hero {
                move_direction: 1.,
                direction_change_timer: Timer::default(),
            },
            Transform::IDENTITY,
        ));
        let shore = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                BlockType::Stone,
                Transform::IDENTITY,
                SnowLayer { stage: 3 },
            ))
            .id();
        let mushroom = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                BlockType::Mushroom,
                Transform::from_xyz(64., 64., 0.),
                SnowLayer { stage: 2 },
            ))
            .id();
        let dry = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                BlockType::Stone,
                Transform::from_xyz(256., 0., 0.),
            ))
            .id();
        {
            let mut water = app
                .world_mut()
                .resource_mut::<crate::systems::fluids::FluidWorld>();
            water.deposit((0, 1), crate::systems::fluids::Liquid::Water, 1);
            water.deposit((1, 1), crate::systems::fluids::Liquid::Water, 8);
        }
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(4));
        app.update();
        assert!(app.world().get::<SnowLayer>(shore).is_none());
        assert!(app.world().get::<SnowLayer>(mushroom).is_none());
        assert!(app.world().get::<SnowLayer>(dry).is_some());
    }
    #[test]
    fn only_active_snow_grows_on_local_exposed_surfaces() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(ActiveWeather::default())
            .init_resource::<crate::systems::fluids::FluidWorld>()
            .add_systems(Update, accumulate_snow);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(4));
        app.world_mut().spawn((
            Hero {
                move_direction: 1.,
                direction_change_timer: Timer::default(),
            },
            Transform::IDENTITY,
        ));
        let covered = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                BlockType::Stone,
                Transform::IDENTITY,
            ))
            .id();
        let roof = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                BlockType::Stone,
                Transform::from_xyz(0., 128., 0.),
            ))
            .id();
        let distant = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                BlockType::Stone,
                Transform::from_xyz(0., 640., 0.),
            ))
            .id();
        let web = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                BlockType::Cobweb,
                Transform::from_xyz(128., 64., 0.),
            ))
            .id();
        app.update();
        assert!(app.world().get::<SnowLayer>(roof).is_none());
        {
            let mut weather = app.world_mut().resource_mut::<ActiveWeather>();
            weather.weather_type = WeatherType::Snow;
            weather.intensity = 0.4;
            weather.duration_remaining = 100.;
        }
        app.update();
        assert_eq!(app.world().get::<SnowLayer>(roof).unwrap().stage, 1);
        assert!(app.world().get::<SnowLayer>(distant).is_some());
        assert!(
            app.world().get::<SnowLayer>(web).is_none(),
            "Non-solid cobwebs cannot support a snow cap"
        );
        assert!(
            app.world().get::<SnowLayer>(covered).is_none(),
            "A cave ledge is not exposed just because its immediate neighbour is empty"
        );
        app.world_mut().resource_mut::<ActiveWeather>().weather_type = WeatherType::Clear;
        app.update();
        assert_eq!(app.world().get::<SnowLayer>(roof).unwrap().stage, 1);
        {
            let mut weather = app.world_mut().resource_mut::<ActiveWeather>();
            weather.weather_type = WeatherType::Snow;
            weather.duration_remaining = 0.;
        }
        app.update();
        assert_eq!(app.world().get::<SnowLayer>(roof).unwrap().stage, 1);
    }
    #[test]
    fn fire_rain_melts_exposed_local_caps_into_finite_water() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(ActiveWeather {
                weather_type: WeatherType::FireRain,
                intensity: 1.,
                duration_remaining: 20.,
                ..default()
            })
            .init_resource::<crate::systems::fluids::FluidWorld>()
            .add_systems(Update, melt_snow);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.8));
        app.world_mut().spawn((
            Hero {
                move_direction: 1.,
                direction_change_timer: Timer::default(),
            },
            Transform::IDENTITY,
        ));
        let exposed = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                BlockType::Stone,
                Transform::IDENTITY,
                SnowLayer { stage: 4 },
            ))
            .id();
        let covered = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                BlockType::Stone,
                Transform::from_xyz(128., 0., 0.),
                SnowLayer { stage: 4 },
            ))
            .id();
        app.world_mut().spawn((
            Block {
                health: 3,
                max_health: 3,
            },
            BlockType::Stone,
            Transform::from_xyz(128., 128., 0.),
        ));
        let far = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                BlockType::Stone,
                Transform::from_xyz(4000., 0., 0.),
                SnowLayer { stage: 4 },
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<SnowLayer>(exposed).unwrap().stage, 3);
        assert_eq!(app.world().get::<SnowLayer>(covered).unwrap().stage, 4);
        assert_eq!(app.world().get::<SnowLayer>(far).unwrap().stage, 4);
        assert!(
            app.world()
                .resource::<crate::systems::fluids::FluidWorld>()
                .has_liquid_at(0, 1)
        );
        assert!(
            !app.world()
                .resource::<crate::systems::fluids::FluidWorld>()
                .has_liquid_at(2, 1)
        );
    }
    #[test]
    fn caps_stay_above_both_block_mesh_scales() {
        for scale in [Vec3::ONE, Vec3::splat(BLOCK_SIZE)] {
            for stage in 1..=4 {
                let height = SnowLayer { stage }.thickness();
                let cap = cap_transform(scale, height);
                let center = cap.translation * scale;
                let size = cap.scale * scale;
                assert!((center.y - size.y / 2.0 - BLOCK_SIZE / 2.0).abs() < 0.001);
                assert_eq!(size, Vec3::new(BLOCK_SIZE, height, BLOCK_SIZE));
            }
        }
    }
    #[test]
    fn cover_uses_real_block_spacing_including_negative_coordinates() {
        let lower = cell(Vec3::new(-64.0, -64.0, 0.0));
        let upper = cell(Vec3::new(-64.0, 0.0, 0.0));
        assert_eq!((lower.0, lower.1 + 1), upper);
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[test]
    fn mushroom_coating_hugs_cap_and_never_becomes_a_floating_tile() {
        for stage in 1..=4 {
            let mesh = mushroom_snow_mesh(stage);
            let bevy::mesh::VertexAttributeValues::Float32x3(vertices) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!("positions");
            };
            for p in vertices {
                assert!(p[0].abs() <= 16.1);
                assert!(p[1] >= -6.01 && p[1] <= 0.51);
                assert!((7. ..=33.).contains(&p[2]));
            }
        }
    }
    #[test]
    fn cap_resizes_and_disappears_when_snow_melts() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<StandardMaterial>::default())
            .init_resource::<SnowVisuals>()
            .add_systems(Update, sync_snow_caps);
        let block = app
            .world_mut()
            .spawn((
                Block {
                    health: 3,
                    max_health: 3,
                },
                Transform::from_scale(Vec3::splat(64.)),
                SnowLayer { stage: 3 },
            ))
            .id();
        let cap = app
            .world_mut()
            .spawn((SnowCap, Transform::IDENTITY, ChildOf(block)))
            .id();
        app.update();
        assert_eq!(app.world().get::<Transform>(cap).unwrap().scale.y, 6. / 64.);
        app.world_mut()
            .entity_mut(block)
            .insert(SnowLayer { stage: 1 });
        app.update();
        assert_eq!(app.world().get::<Transform>(cap).unwrap().scale.y, 2. / 64.);
        app.world_mut().entity_mut(block).remove::<SnowLayer>();
        app.update();
        assert!(app.world().get_entity(cap).is_err());
    }
}
