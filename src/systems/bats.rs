//! Small cave fliers: source-textured heads and hinged flat wings, bounded swoops.
use crate::components::{
    block::Block,
    damage_visual::DeathAnimation,
    enemy::{Enemy, EnemyType, Health},
    hero::Hero,
    villager::Villager,
};
use avian3d::prelude::*;
use bevy::prelude::*;
#[derive(Component)]
pub struct Bat {
    home: Vec2,
    age: f32,
    phase: f32,
}
#[derive(Component)]
struct Wing(f32);
#[derive(Resource)]
struct Art {
    head: Handle<Mesh>,
    wing: Handle<Mesh>,
    face: Handle<Image>,
    membrane: Handle<Image>,
}
pub struct BatPlugin;
impl Plugin for BatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (attach, fly, flap)
                .chain()
                .before(super::teleport_vfx::world_transition)
                .run_if(in_state(crate::states::InGame)),
        );
    }
}
fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, server: Res<AssetServer>) {
    commands.insert_resource(Art {
        head: meshes.add(Rectangle::new(32., 32.)),
        wing: meshes.add(Rectangle::new(42., 24.)),
        face: server
            .load_builder()
            .with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
                s.sampler = bevy::image::ImageSampler::nearest()
            })
            .load("textures/entities/enemy/bat_face.png"),
        membrane: server
            .load_builder()
            .with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
                s.sampler = bevy::image::ImageSampler::nearest()
            })
            .load("textures/entities/enemy/bat_wing.png"),
    });
}
fn attach(
    mut commands: Commands,
    q: Query<(Entity, &Enemy, &Transform), Added<Enemy>>,
    art: Res<Art>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    for (e, enemy, t) in &q {
        if enemy.enemy_type != EnemyType::Bat {
            continue;
        }
        let material = mats.add(StandardMaterial {
            base_color_texture: Some(art.face.clone()),
            alpha_mode: AlphaMode::Mask(0.1),
            unlit: true,
            ..default()
        });
        commands.entity(e).insert((
            Bat {
                home: t.translation.xy(),
                age: 0.,
                phase: (e.index_u32() % 19) as f32,
            },
            GravityScale(0.),
            Collider::cuboid(32., 28., 32.),
            Mesh3d(art.head.clone()),
            MeshMaterial3d(material),
        ));
        for side in [-1., 1.] {
            commands.spawn((
                ChildOf(e),
                Wing(side),
                Mesh3d(art.wing.clone()),
                MeshMaterial3d(mats.add(StandardMaterial {
                    base_color_texture: Some(art.membrane.clone()),
                    alpha_mode: AlphaMode::Mask(0.1),
                    unlit: true,
                    cull_mode: None,
                    ..default()
                })),
                Transform::from_xyz(side * 31., 0., -1.),
            ));
        }
    }
}
fn clear(from: Vec2, to: Vec2, terrain: &[Vec2]) -> bool {
    !terrain
        .iter()
        .any(|p| super::projectile_system::segment_box(from, to, *p, Vec2::splat(49.)).is_some())
}
fn steering(p: Vec2, goal: Vec2, phase: f32, terrain: &[Vec2]) -> Vec2 {
    let desired = (goal - p).normalize_or_zero();
    for turn in [0., 0.8, -0.8, 1.6, -1.6, std::f32::consts::PI] {
        let dir = Vec2::from_angle(turn + phase.sin() * 0.1).rotate(desired);
        if clear(p, p + dir * 90., terrain) {
            return dir * 125.;
        }
    }
    Vec2::ZERO
}
fn fly(
    time: Res<Time>,
    mut bats: Query<(&Transform, &mut LinearVelocity, &mut Bat, &Health), Without<DeathAnimation>>,
    targets: Query<
        &Transform,
        (
            Or<(With<Hero>, With<Villager>)>,
            Without<super::launcher::LauncherPassenger>,
        ),
    >,
    blocks: Query<&Transform, With<Block>>,
    grid: Res<crate::resources::spatial_hash::SpatialHash>,
) {
    if bats.is_empty() {
        return;
    }
    for (t, mut v, mut bat, hp) in &mut bats {
        if hp.current == 0 {
            continue;
        }
        bat.age += time.delta_secs();
        let a = bat.age + bat.phase;
        let patrol = bat.home + Vec2::new((a * 0.7).sin() * 150., (a * 1.1).cos() * 80.);
        let target = targets
            .iter()
            .filter(|q| q.translation.xy().distance(bat.home) < 400.)
            .min_by(|a, b| {
                a.translation
                    .distance_squared(t.translation)
                    .total_cmp(&b.translation.distance_squared(t.translation))
            });
        let p = t.translation.xy();
        let goal = if a.rem_euclid(5.) < 1.3 {
            target.map(|q| q.translation.xy()).unwrap_or(patrol)
        } else {
            patrol
        };
        let terrain: Vec<_> = grid
            .rect_iter(
                p.min(goal) - Vec2::splat(150.),
                p.max(goal) + Vec2::splat(150.),
            )
            .filter_map(|e| blocks.get(e).ok())
            .map(|t| t.translation.xy())
            .collect();
        let goal = if clear(p, goal, &terrain) {
            goal
        } else {
            patrol
        };
        let wanted = steering(t.translation.xy(), goal, a, &terrain);
        v.0 =
            v.0.lerp(wanted.extend(0.), 1. - (-time.delta_secs() * 5.).exp());
    }
}
fn flap(time: Res<Time>, bats: Query<&Bat>, mut wings: Query<(&ChildOf, &Wing, &mut Transform)>) {
    for (p, w, mut t) in &mut wings {
        if let Ok(b) = bats.get(p.parent()) {
            let wave = (time.elapsed_secs() * 22. + b.phase).sin();
            t.rotation = Quat::from_rotation_z(w.0 * (0.2 + wave * 0.55));
            t.scale.x = w.0 * (0.72 + wave.abs() * 0.28);
            t.translation.y = wave * 5.;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn teleport_captures_initialized_bat_face_and_mesh() {
        use super::super::biome_teleport::{BiomeTeleport, CHARGE_SECONDS, TeleportPhase};
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .insert_resource(Time::<()>::default());
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Rectangle::new(32., 32.));
        let fallback = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Rectangle::new(64., 64.));
        let mat = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        app.insert_resource(Art {
            head: mesh.clone(),
            wing: default(),
            face: default(),
            membrane: default(),
        });
        let mut tp = BiomeTeleport::default();
        tp.phase = TeleportPhase::Charging;
        tp.elapsed = CHARGE_SECONDS * 0.8;
        app.insert_resource(tp);
        app.add_systems(
            Update,
            (attach, super::super::teleport_vfx::world_transition).chain(),
        );
        let e = app
            .world_mut()
            .spawn((
                Enemy::new(EnemyType::Bat),
                Health::new(2),
                Transform::default(),
                Mesh3d(fallback),
                MeshMaterial3d(mat),
            ))
            .id();
        app.update();
        app.world_mut().resource_mut::<BiomeTeleport>().phase = TeleportPhase::Idle;
        app.update();
        assert_eq!(app.world().get::<Mesh3d>(e).unwrap().0, mesh);
        assert!(app.world().get::<Bat>(e).is_some());
    }
    #[test]
    fn bat_steers_around_wall_instead_of_flying_through_it() {
        let walls = [Vec2::new(80., 0.)];
        let v = steering(Vec2::ZERO, Vec2::new(200., 0.), 0., &walls);
        assert!(v.y.abs() > 30.);
        assert!(clear(Vec2::ZERO, v.normalize() * 90., &walls));
    }
}
