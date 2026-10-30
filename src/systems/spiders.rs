//! Underground ambush spiders: readable crouch, physical pounce and temporary silk.
use crate::{
    components::{
        block::Block,
        buff::{Buff, BuffType},
        damage_visual::DeathAnimation,
        enemy::{Enemy, EnemyType, Health},
        hero::Hero,
        villager::Villager,
    },
    events::damage_events::{EntityDamaged, EntityDied},
};
use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;

#[derive(Component)]
pub struct Spider {
    cooldown: f32,
    windup: f32,
    target: Vec3,
    bite: f32,
    silk: f32,
    age: f32,
}
#[derive(Component)]
struct Leg {
    side: f32,
    index: usize,
}
#[derive(Component)]
pub(crate) struct Web {
    life: f32,
    tick: f32,
}
#[derive(Resource)]
struct Art {
    head: Handle<Mesh>,
    cube: Handle<Mesh>,
    web: Handle<Mesh>,
    shell: Handle<StandardMaterial>,
    joint: Handle<StandardMaterial>,
    eyes: Handle<StandardMaterial>,
    silk: Handle<StandardMaterial>,
    hiss: Handle<AudioSource>,
    death: Handle<AudioSource>,
}
pub struct SpiderPlugin;
impl Plugin for SpiderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (attach, think, animate, webs, sounds)
                .chain()
                .before(super::teleport_vfx::world_transition)
                .run_if(in_state(crate::states::InGame)),
        );
    }
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    server: Res<AssetServer>,
) {
    commands.insert_resource(Art {
        head: meshes.add(Rectangle::new(64., 64.)),
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        web: meshes.add(Rectangle::new(120., 120.)),
        shell: materials.add(StandardMaterial {
            base_color: Color::srgb(0.13, 0.105, 0.095),
            perceptual_roughness: 0.9,
            ..default()
        }),
        joint: materials.add(StandardMaterial {
            base_color: Color::srgb(0.23, 0.18, 0.13),
            ..default()
        }),
        eyes: materials.add(StandardMaterial {
            base_color_texture: Some(
                server
                    .load_builder()
                    .with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
                        s.sampler = bevy::image::ImageSampler::nearest()
                    })
                    .load("textures/entities/enemy/CaveSpiderFace.png"),
            ),
            unlit: true,
            alpha_mode: AlphaMode::Mask(0.1),
            ..default()
        }),
        silk: materials.add(StandardMaterial {
            base_color: Color::srgba(0.8, 0.83, 0.86, 0.48),
            base_color_texture: Some(server.load("textures/blocks/coweb/cobweb.png")),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        hiss: server.load("audio/sfx/mobs/spider/say4.ogg"),
        death: server.load("audio/sfx/mobs/spider/death.ogg"),
    });
}
fn part(
    commands: &mut Commands,
    root: Entity,
    art: &Art,
    position: Vec3,
    size: Vec3,
    material: Handle<StandardMaterial>,
) -> Entity {
    commands
        .spawn((
            ChildOf(root),
            Mesh3d(art.cube.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(position).with_scale(size),
        ))
        .id()
}
fn attach(
    mut commands: Commands,
    enemies: Query<(Entity, &Enemy), Added<Enemy>>,
    art: Res<Art>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let head_mesh = art.head.clone();
    for (e, enemy) in &enemies {
        if enemy.enemy_type != EnemyType::Spider {
            continue;
        }
        // Flat pack face, matching other heads; three hinged legs on each side.
        let face_source = materials.get(&art.eyes).unwrap().clone();
        let face = materials.add(face_source);
        commands.entity(e).insert((
            Mesh3d(art.web.clone()),
            MeshMaterial3d(face),
            Spider {
                cooldown: 1.2,
                windup: 0.,
                target: Vec3::ZERO,
                bite: 0.,
                silk: 0.,
                age: 0.,
            },
        ));
        // Use a 64-unit head mesh, without scaling the physics owner or children.
        commands.entity(e).insert(Mesh3d(head_mesh.clone()));
        for side in [-1., 1.] {
            for index in 0..3 {
                let root = commands
                    .spawn((
                        ChildOf(e),
                        Leg { side, index },
                        Transform::from_xyz(side * 27., (index as f32 - 1.) * 18., -1.),
                        Visibility::Inherited,
                    ))
                    .id();
                part(
                    &mut commands,
                    root,
                    &art,
                    Vec3::new(side * 14., 0., 0.),
                    Vec3::new(28., 5., 3.),
                    art.shell.clone(),
                );
                let foot = part(
                    &mut commands,
                    root,
                    &art,
                    Vec3::new(side * 34., -7., 0.),
                    Vec3::new(24., 4., 3.),
                    art.shell.clone(),
                );
                commands.entity(foot).insert(
                    Transform::from_xyz(side * 34., -7., 0.)
                        .with_rotation(Quat::from_rotation_z(-side * 0.55))
                        .with_scale(Vec3::new(24., 4., 3.)),
                );
            }
        }
    }
}
fn sight(from: Vec2, to: Vec2, blocks: &[Vec2]) -> bool {
    !blocks
        .iter()
        .any(|p| super::projectile_system::segment_box(from, to, *p, Vec2::splat(29.)).is_some())
}
#[allow(clippy::too_many_arguments)] // Bevy system parameters
fn think(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<Art>,
    _settings: Res<crate::resources::settings::GameSettings>,
    mut spiders: Query<
        (&Transform, &mut Spider, &mut LinearVelocity, &Health),
        Without<DeathAnimation>,
    >,
    actors: Query<
        (Entity, &Transform, Option<&Buff>),
        (
            Or<(With<Hero>, With<Villager>)>,
            Without<super::launcher::LauncherPassenger>,
        ),
    >,
    blocks: Query<&Transform, With<Block>>,
    grid: Res<crate::resources::spatial_hash::SpatialHash>,
    mut damage: MessageWriter<EntityDamaged>,
    mut terrain: Local<Vec<Vec2>>,
) {
    if spiders.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    for (t, mut spider, mut velocity, hp) in &mut spiders {
        if hp.current == 0 {
            continue;
        }
        spider.age += dt;
        spider.bite = (spider.bite - dt).max(0.);
        spider.cooldown = (spider.cooldown - dt).max(0.);
        if spider.silk > 0. {
            spider.silk -= dt;
            if spider.silk <= 0. {
                spawn_web(&mut commands, &art, t.translation);
            }
        }
        let target = actors
            .iter()
            .filter(|(_, a, _)| a.translation.xy().distance(t.translation.xy()) < 560.)
            .min_by(|(_, a, _), (_, b, _)| {
                a.translation
                    .distance_squared(t.translation)
                    .total_cmp(&b.translation.distance_squared(t.translation))
            });
        let Some((entity, target, buff)) = target else {
            spider.windup = 0.;
            continue;
        };
        let delta = target.translation - t.translation;
        // Line of sight against blocks around the segment only, not the whole world
        let (from, to) = (t.translation.xy(), target.translation.xy());
        terrain.clear();
        terrain.extend(
            grid.rect_iter(
                from.min(to) - Vec2::splat(64.),
                from.max(to) + Vec2::splat(64.),
            )
            .filter_map(|e| blocks.get(e).ok())
            .map(|b| b.translation.xy()),
        );
        if !sight(from, to, &terrain) {
            spider.windup = 0.;
            continue;
        }
        if delta.xy().length() < 68. && spider.bite == 0. {
            damage.write(EntityDamaged { entity, damage: 1 });
            spider.bite = 1.4;
            if buff.is_none() {
                commands
                    .entity(entity)
                    .try_insert(Buff::new(BuffType::Poisoned, 1.25));
            }
        }
        if spider.windup > 0. {
            spider.windup -= dt;
            velocity.x *= (-12. * dt).exp();
            if spider.windup <= 0. {
                let aim = (spider.target - t.translation).xy().normalize_or_zero();
                velocity.0 = Vec3::new(aim.x * 380., (aim.y * 180. + 230.).clamp(120., 360.), 0.);
                spider.cooldown = 2.8;
                spider.silk = 0.65;
                commands.spawn((
                    AudioPlayer::new(art.hiss.clone()),
                    PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.25)),
                ));
            }
        } else if spider.cooldown == 0. {
            spider.windup = 0.65;
            spider.target = target.translation;
        } else if spider.cooldown < 1.8 {
            velocity.x = delta.x.signum() * 85.;
        }
    }
}
fn spawn_web(commands: &mut Commands, art: &Art, p: Vec3) {
    let root = commands
        .spawn((
            Web { life: 7., tick: 0. },
            crate::components::block::CobwebBlock,
            Transform::from_translation(Vec3::new(p.x, p.y, 54.)),
            Visibility::Inherited,
        ))
        .id();
    let angle = (p.x * 0.013 + p.y * 0.007).sin() * 0.3;
    commands.spawn((
        ChildOf(root),
        Mesh3d(art.web.clone()),
        MeshMaterial3d(art.silk.clone()),
        Transform::from_rotation(Quat::from_rotation_z(angle)),
        bevy::light::NotShadowCaster,
    ));
}
fn animate(
    time: Res<Time>,
    parents: Query<&Spider>,
    mut legs: Query<(&ChildOf, &Leg, &mut Transform)>,
) {
    for (parent, leg, mut t) in &mut legs {
        let Ok(s) = parents.get(parent.parent()) else {
            continue;
        };
        let phase = time.elapsed_secs() * 9. + leg.index as f32 * std::f32::consts::PI + leg.side;
        let crouch = if s.windup > 0. { 0.65 } else { 0. };
        t.rotation = Quat::from_rotation_z(
            leg.side * ((leg.index as f32 - 1.) * 0.26 + crouch * 0.25) + phase.sin() * 0.16,
        );
    }
}
fn webs(
    mut commands: Commands,
    time: Res<Time>,
    mut webs: Query<(Entity, &mut Web, &Transform, &mut Visibility)>,
    _actors: Query<(Entity, &Transform, Option<&Buff>), Or<(With<Hero>, With<Villager>)>>,
) {
    for (entity, mut web, _t, mut vis) in &mut webs {
        web.life -= time.delta_secs();
        web.tick -= time.delta_secs();
        if web.life <= 0. {
            commands.entity(entity).despawn();
            continue;
        }
        // Last half second is a brief warning blink; no persistent terrain edits.
        *vis = if web.life < 0.5 && (web.life * 20.) as i32 % 2 == 0 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        // Movement is handled by the shared cobweb system, independently of
        // poison/fire buffs. Spider silk and terrain webs now have one contract.
    }
}
fn sounds(
    mut commands: Commands,
    mut deaths: MessageReader<EntityDied>,
    spiders: Query<(), With<Spider>>,
    art: Res<Art>,
    _settings: Res<crate::resources::settings::GameSettings>,
) {
    for event in deaths.read() {
        if spiders.contains(event.entity) {
            commands.spawn((
                AudioPlayer::new(art.death.clone()),
                PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.5)),
            ));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ambush_requires_open_line_and_not_a_wall() {
        assert!(sight(Vec2::ZERO, Vec2::new(200., 0.), &[]));
        assert!(!sight(
            Vec2::ZERO,
            Vec2::new(200., 0.),
            &[Vec2::new(100., 0.)]
        ));
    }
    #[test]
    fn crouch_precedes_pounce_bite_has_cooldown_and_silk_expires() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .init_resource::<crate::resources::settings::GameSettings>()
            .insert_resource(Art {
                head: default(),
                cube: default(),
                web: default(),
                shell: default(),
                joint: default(),
                eyes: default(),
                silk: default(),
                hiss: default(),
                death: default(),
            })
            .insert_resource(crate::resources::spatial_hash::SpatialHash::new(256.))
            .add_message::<EntityDamaged>()
            .add_systems(Update, (think, webs).chain());
        let foe = app
            .world_mut()
            .spawn((
                Spider {
                    cooldown: 0.,
                    windup: 0.,
                    target: Vec3::ZERO,
                    bite: 0.,
                    silk: 0.,
                    age: 0.,
                },
                Health::new(4),
                Transform::IDENTITY,
                LinearVelocity::ZERO,
            ))
            .id();
        let target = app
            .world_mut()
            .spawn((
                Hero {
                    move_direction: 1.,
                    direction_change_timer: Timer::default(),
                },
                Transform::from_xyz(48., 0., 0.),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.1));
        app.update();
        assert!(app.world().get::<Spider>(foe).unwrap().windup > 0.);
        assert_eq!(
            app.world().get::<LinearVelocity>(foe).unwrap().0,
            Vec3::ZERO
        );
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<EntityDamaged>>()
                .drain()
                .count(),
            1
        );
        assert_eq!(
            app.world().get::<Buff>(target).unwrap().buff_type,
            BuffType::Poisoned
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.7));
        app.update();
        assert!(app.world().get::<LinearVelocity>(foe).unwrap().x > 300.);
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<EntityDamaged>>()
                .drain()
                .count(),
            0
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.7));
        app.update();
        assert_eq!(app.world_mut().query::<&Web>().iter(app.world()).count(), 1);
        app.world_mut().get_mut::<Health>(foe).unwrap().current = 0;
        app.world_mut().entity_mut(target).remove::<Buff>();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.31));
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&crate::components::block::CobwebBlock>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(8.));
        app.update();
        assert_eq!(app.world_mut().query::<&Web>().iter(app.world()).count(), 0);
    }
}
