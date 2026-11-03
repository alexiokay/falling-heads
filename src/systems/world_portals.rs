//! Rare background gateways. Decoration never participates in mining or collision;
//! world portals, powers and loot share one safe teleport coordinator.
use crate::{
    components::{
        block::{BiomeType, BlockType},
        chunk::ChunkMember,
        hero::Hero,
    },
    events::power_events::PowerActivated,
    resources::{
        command_queue::PowerCommandType, material_cache::MaterialCache, mesh_cache::MeshCache,
    },
    states::GameState,
    systems::biome_teleport::{BiomeTeleport, TeleportPhase},
    utils::biome_utils::{determine_biome_at_position, nearest_biome_y},
};
use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};

pub struct WorldPortalPlugin;
impl Plugin for WorldPortalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TravelLock>()
            .init_resource::<PowerNotice>()
            .add_plugins(MaterialPlugin::<GateMaterial>::default())
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (power_requests, attach, update, update_notice)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            // Messages stay live in the space prison too (guards, escapes).
            .add_systems(Update, update_notice.run_if(in_state(GameState::Prison)));
    }
}

#[derive(Resource, Default)]
struct TravelLock {
    remaining: f32,
}

#[derive(Resource, Default)]
pub(crate) struct PowerNotice {
    pub text: String,
    pub remaining: f32,
}
#[derive(Component)]
struct NoticeLabel;
fn update_notice(
    time: Res<Time>,
    mut notice: ResMut<PowerNotice>,
    mut labels: Query<&mut Text, With<NoticeLabel>>,
) {
    notice.remaining = (notice.remaining - time.delta_secs()).max(0.);
    for mut text in &mut labels {
        **text = if notice.remaining > 0. {
            notice.text.clone()
        } else {
            String::new()
        };
    }
}

pub fn next_biome(biome: BiomeType) -> BiomeType {
    match biome {
        BiomeType::Plains => BiomeType::Forest,
        BiomeType::Forest => BiomeType::Desert,
        BiomeType::Desert => BiomeType::Mountains,
        BiomeType::Mountains => BiomeType::Underground,
        BiomeType::Underground => BiomeType::Nether,
        BiomeType::Nether => BiomeType::End,
        BiomeType::End => BiomeType::Iceland,
        BiomeType::Iceland => BiomeType::Plains,
    }
}

fn request_travel(
    position: Vec3,
    destination: BiomeType,
    teleport: &mut BiomeTeleport,
    lock: &mut TravelLock,
) -> bool {
    if lock.remaining > 0. || teleport.target.is_some() || teleport.phase != TeleportPhase::Idle {
        return false;
    }
    teleport.target = Some(Vec3::new(
        position.x,
        nearest_biome_y(position.y, destination),
        position.z,
    ));
    lock.remaining = 20.;
    true
}

fn power_requests(
    mut powers: MessageReader<PowerActivated>,
    hero: Query<&Transform, With<Hero>>,
    mut teleport: ResMut<BiomeTeleport>,
    mut lock: ResMut<TravelLock>,
    mut notice: ResMut<PowerNotice>,
) {
    for event in powers.read() {
        if event.command.command_type != PowerCommandType::Teleport {
            continue;
        }
        if let Ok(hero) = hero.single() {
            let biome = determine_biome_at_position(hero.translation.x, hero.translation.y);
            // The ruins' travel lock does not hold back the TELEPORT power (its wait
            // is the Teleport cooldown under Settings > Cooldowns); a trip still
            // sets the lock so landing beside a ruin does not bounce straight back.
            let held = std::mem::take(&mut lock.remaining);
            let accepted = request_travel(
                hero.translation,
                next_biome(biome),
                &mut teleport,
                &mut lock,
            );
            if !accepted {
                lock.remaining = held;
                notice.text = "TELEPORT ALREADY IN PROGRESS".into();
                notice.remaining = 3.;
            }
        }
    }
}

/// One deterministic ruin per 127 chunks on average; its whole footprint
/// stays inside that chunk so streaming never creates a half-built structure.
pub fn site(cx: i32, cy: i32, seed: u32) -> Option<IVec2> {
    let h = (cx as u32).wrapping_mul(73856093) ^ (cy as u32).wrapping_mul(19349663) ^ seed;
    ((h ^ (h >> 13)) % 127 == 0).then_some(IVec2::new(cx * 16 + 8, cy * 16 + 7))
}
pub fn reserved(cell: IVec2, origin: IVec2) -> bool {
    let d = cell - origin;
    (-4..=3).contains(&d.x) && (-2..=4).contains(&d.y)
}

#[derive(Component)]
pub struct PortalRuin {
    origin: Vec2,
    destination: BiomeType,
    dwell: f32,
}
#[derive(Component)]
struct GateFace;
#[derive(Component)]
struct GateLabel(Entity);

pub fn spawn_ruin(
    commands: &mut Commands,
    origin: IVec2,
    biome: BiomeType,
    meshes: &MeshCache,
    materials: &MaterialCache,
) {
    let member = ChunkMember {
        chunk_x: origin.x.div_euclid(16),
        chunk_y: origin.y.div_euclid(16),
    };
    let root = commands
        .spawn((
            PortalRuin {
                origin: origin.as_vec2() * 64.,
                destination: next_biome(biome),
                dwell: 0.,
            },
            member,
            Transform::from_xyz(origin.x as f32 * 64. + 32., origin.y as f32 * 64. + 64., 0.),
            Visibility::Inherited,
        ))
        .id();
    // Frame and opening share lifetime/transform; decoration never enters collision.
    for x in -1..=2 {
        for y in -1..=3 {
            if x != -1 && x != 2 && y != -1 && y != 3 {
                continue;
            }
            commands.spawn((
                ChildOf(root),
                bevy::light::NotShadowCaster,
                Mesh3d(meshes.block_mesh.clone()),
                MeshMaterial3d(materials.get(BlockType::Obsidian).expect("portal stone")),
                Transform::from_xyz(x as f32 * 64. - 32., y as f32 * 64. - 64., -3.)
                    .with_scale(Vec3::new(64., 64., 8.)),
            ));
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
struct GateMaterial {
    #[uniform(0)]
    effect: Vec4,
}
impl Material for GateMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/world_portal.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
#[derive(Resource)]
struct GateAssets {
    quad: Handle<Mesh>,
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        NoticeLabel,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(18.),
            ..default()
        },
        TextColor(Color::srgb(0.88, 0.76, 1.)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(72.),
            left: Val::Percent(15.),
            ..default()
        },
        GlobalZIndex(490),
        Pickable::IGNORE,
    ));
    commands.insert_resource(GateAssets {
        quad: meshes.add(Rectangle::new(1., 1.)),
    });
}
fn attach(
    mut commands: Commands,
    ruins: Query<(Entity, &PortalRuin), Added<PortalRuin>>,
    assets: Res<GateAssets>,
    mut materials: ResMut<Assets<GateMaterial>>,
) {
    for (entity, _ruin) in &ruins {
        let p = Vec2::new(-32., -64.);
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                GateFace,
                bevy::light::NotShadowCaster,
                Mesh3d(assets.quad.clone()),
                MeshMaterial3d(materials.add(GateMaterial { effect: Vec4::ZERO })),
                Transform::from_xyz(p.x + 32., p.y + 64., 5.).with_scale(Vec3::new(128., 192., 1.)),
            ));
        });
        commands.spawn((
            GateLabel(entity),
            crate::components::username_label::UsernameLabel::with_offset(
                String::new(),
                entity,
                172.,
            ),
            Text2d::new(""),
            TextFont {
                font_size: FontSize::Px(15.),
                ..default()
            },
            TextColor(Color::srgb(0.84, 0.73, 1.)),
            Transform::IDENTITY,
        ));
    }
}

fn update(
    time: Res<Time>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut lock: ResMut<TravelLock>,
    mut teleport: ResMut<BiomeTeleport>,
    mut ruins: Query<&mut PortalRuin>,
    hero: Query<&Transform, (With<Hero>, Without<super::launcher::LauncherPassenger>)>,
    faces: Query<&MeshMaterial3d<GateMaterial>, With<GateFace>>,
    mut labels: Query<(&GateLabel, &mut Text2d)>,
    mut materials: ResMut<Assets<GateMaterial>>,
) {
    lock.remaining = (lock.remaining - time.delta_secs()).max(0.);
    if let Ok(hero) = hero.single() {
        for mut ruin in &mut ruins {
            let d = (hero.translation.xy() - (ruin.origin + Vec2::new(32., 64.))).abs();
            ruin.dwell = if d.x < 72. && d.y < 104. {
                ruin.dwell + time.delta_secs()
            } else {
                0.
            };
            let nearby = d.x < 260. && d.y < 420.;
            let interact = nearby && keys.as_ref().is_some_and(|k| k.just_pressed(KeyCode::KeyE));
            if (ruin.dwell >= 0.08 || interact)
                && request_travel(hero.translation, ruin.destination, &mut teleport, &mut lock)
            {
                ruin.dwell = 0.;
            }
        }
    }
    for handle in &faces {
        if let Some(mut mat) = materials.get_mut(&handle.0) {
            mat.effect = Vec4::new(time.elapsed_secs(), 1., 0., 0.);
        }
    }
    for (owner, mut label) in &mut labels {
        if let Ok(ruin) = ruins.get(owner.0) {
            **label = if lock.remaining > 0. {
                format!("PORTAL RECHARGE {:.0}s", lock.remaining)
            } else {
                format!("{:?} - TOUCH OR E NEARBY", ruin.destination).to_uppercase()
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn power_and_loot_messages_share_safe_travel_and_rejection_feedback() {
        let mut app = App::new();
        app.init_resource::<TravelLock>()
            .init_resource::<PowerNotice>()
            .init_resource::<BiomeTeleport>()
            .add_message::<PowerActivated>()
            .add_systems(Update, power_requests);
        app.world_mut().spawn((
            Hero {
                move_direction: 1.,
                direction_change_timer: Timer::from_seconds(1., TimerMode::Repeating),
            },
            Transform::IDENTITY,
        ));
        for sender in ["API", "Lootbox"] {
            app.world_mut().write_message(PowerActivated {
                command: crate::resources::command_queue::PowerCommand::new(
                    PowerCommandType::Teleport,
                    sender.into(),
                ),
            });
            app.update();
        }
        let target = app.world().resource::<BiomeTeleport>().target.unwrap();
        assert_eq!(
            determine_biome_at_position(target.x, target.y),
            BiomeType::Forest
        );
        assert!(
            app.world()
                .resource::<PowerNotice>()
                .text
                .contains("IN PROGRESS")
        );
        assert!(
            crate::components::lootbox::LootboxReward::all_rewards()
                .contains(&crate::components::lootbox::LootboxReward::Teleport)
        );
    }
    #[test]
    fn portal_works_without_plate_or_destructible_frame() {
        let mut app = App::new();
        app.init_resource::<TravelLock>()
            .init_resource::<BiomeTeleport>()
            .init_resource::<Assets<GateMaterial>>()
            .insert_resource(Time::<()>::default())
            .add_systems(Update, update);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.1));
        app.world_mut().spawn((
            Hero {
                move_direction: 1.,
                direction_change_timer: Timer::default(),
            },
            Transform::from_xyz(32., 64., 0.),
        ));
        app.world_mut().spawn(PortalRuin {
            origin: Vec2::ZERO,
            destination: BiomeType::Forest,
            dwell: 0.,
        });
        app.update();
        assert!(app.world().resource::<BiomeTeleport>().target.is_some());
        let count = (-100..100)
            .flat_map(|x| (-100..100).map(move |y| (x, y)))
            .filter(|&(x, y)| site(x, y, 42).is_some())
            .count();
        assert!(
            (200..450).contains(&count),
            "Portals must be rare: {count}/40000"
        );
    }
    #[test]
    fn e_activates_a_gateway_above_the_head_but_not_distant_gateways() {
        let mut app = App::new();
        app.init_resource::<TravelLock>()
            .init_resource::<BiomeTeleport>()
            .init_resource::<Assets<GateMaterial>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(Time::<()>::default())
            .add_systems(Update, update);
        let hero = app
            .world_mut()
            .spawn((
                Hero {
                    move_direction: 1.,
                    direction_change_timer: Timer::default(),
                },
                Transform::from_xyz(32., -290., 0.),
            ))
            .id();
        app.world_mut().spawn(PortalRuin {
            origin: Vec2::ZERO,
            destination: BiomeType::Forest,
            dwell: 0.,
        });
        app.update();
        assert!(app.world().resource::<BiomeTeleport>().target.is_none());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyE);
        app.update();
        assert!(app.world().resource::<BiomeTeleport>().target.is_some());
        app.world_mut().resource_mut::<BiomeTeleport>().target = None;
        app.world_mut().resource_mut::<TravelLock>().remaining = 0.;
        app.world_mut()
            .get_mut::<Transform>(hero)
            .unwrap()
            .translation
            .y = -1000.;
        app.update();
        assert!(app.world().resource::<BiomeTeleport>().target.is_none());
    }
    #[test]
    fn every_ruin_footprint_stays_in_its_chunk() {
        for x in -20..20 {
            for y in -20..20 {
                if let Some(p) = site(x, y, 42) {
                    for dx in -4..=3 {
                        for dy in -2..=4 {
                            assert_eq!(
                                ((p.x + dx).div_euclid(16), (p.y + dy).div_euclid(16)),
                                (x, y)
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn travel_rejects_overlap_and_cooldown() {
        let mut travel = BiomeTeleport::default();
        let mut lock = TravelLock::default();
        assert!(request_travel(
            Vec3::ZERO,
            BiomeType::Nether,
            &mut travel,
            &mut lock
        ));
        assert_eq!(
            determine_biome_at_position(travel.target.unwrap().x, travel.target.unwrap().y),
            BiomeType::Nether
        );
        assert!(!request_travel(
            Vec3::ZERO,
            BiomeType::End,
            &mut travel,
            &mut lock
        ));
        travel.target = None;
        assert!(!request_travel(
            Vec3::ZERO,
            BiomeType::End,
            &mut travel,
            &mut lock
        ));
    }
}
