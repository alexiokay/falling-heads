//! Character play styles: viewers choose how their heads play "the ball"
//! (`STYLE` / `VILLAGERS` with `"style"`), the streamer cycles the hero's style
//! with C. Bounce and block-damage rules live in the terrain contact solver.
use crate::components::hero::Hero;
use crate::components::play_style::{
    PlayStyle, SPLIT_SCALE, SPLIT_SECONDS, Split, SplitCooldown, SplitRequest, SplitTwin,
    TwinReturning, WallCling,
};
use crate::components::username_label::UsernameLabel;
use crate::events::power_events::PowerActivated;
use crate::resources::command_queue::PowerCommandType;
#[cfg(test)]
use crate::states::GameState;
use avian3d::prelude::{GravityScale, LinearVelocity};
use bevy::prelude::*;
use std::collections::HashMap;

/// Each viewer's chosen style, so heads they spawn later get it too.
#[derive(Resource, Default)]
pub struct ViewerStyles(pub HashMap<String, PlayStyle>);

#[derive(Component)]
struct StyleToast(Timer);

/// Dev panel button that sets the hero's style.
#[derive(Component)]
pub struct HeroStyleButton(pub PlayStyle);

/// HUD line showing the hero's current style.
#[derive(Component)]
pub struct HeroStyleText;

fn style_color(style: PlayStyle) -> Color {
    match style {
        PlayStyle::Bouncy => Color::srgb(0.6, 0.9, 1.0),
        PlayStyle::Heavy => Color::srgb(1.0, 0.55, 0.35),
        PlayStyle::Floaty => Color::srgb(0.75, 0.7, 1.0),
        PlayStyle::Sticky => Color::srgb(0.55, 1.0, 0.45),
        PlayStyle::Splitter => Color::srgb(1.0, 0.55, 0.85),
    }
}

pub struct PlayStylePlugin;

impl Plugin for PlayStylePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewerStyles>().add_systems(
            Update,
            (
                style_commands,
                hero_style_hotkey,
                apply_gravity,
                wall_cling,
                split_heads,
                merge_twins,
                fade_puffs,
                label_style_names,
                hero_style_buttons,
                hero_style_hud,
                fade_toasts,
            )
                .chain()
                .run_if(in_state(crate::states::InGame)),
        );
    }
}

fn viewer_key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Records viewer choices and applies them to that viewer's existing heads and
/// to any head that gets their name label later.
fn style_commands(
    mut commands: Commands,
    mut powers: MessageReader<PowerActivated>,
    mut styles: ResMut<ViewerStyles>,
    labels: Query<&UsernameLabel>,
    new_labels: Query<&UsernameLabel, Added<UsernameLabel>>,
) {
    for power in powers.read() {
        let command = &power.command;
        let Some(style) = command.style else { continue };
        if !matches!(
            command.command_type,
            PowerCommandType::Style | PowerCommandType::Villagers
        ) {
            continue;
        }
        let key = viewer_key(&command.username);
        styles.0.insert(key.clone(), style);
        for label in &labels {
            if viewer_key(&label.username) == key {
                commands.entity(label.owner).try_insert(style);
            }
        }
        info!("{} chose the {} play style", command.username, style.name());
    }
    for label in &new_labels {
        if let Some(style) = styles.0.get(&viewer_key(&label.username)) {
            commands.entity(label.owner).try_insert(*style);
        }
    }
}

/// C cycles the hero through the styles and shows the new one briefly.
fn hero_style_hotkey(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    heroes: Query<(Entity, Option<&PlayStyle>), With<Hero>>,
    toasts: Query<Entity, With<StyleToast>>,
) {
    if !keys.just_pressed(KeyCode::KeyC) {
        return;
    }
    for (hero, current) in &heroes {
        let next = current.copied().unwrap_or_default().next();
        commands.entity(hero).insert(next);
        for old in &toasts {
            commands.entity(old).despawn();
        }
        commands.spawn((
            StyleToast(Timer::from_seconds(2.0, TimerMode::Once)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(18.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            children![(
                Text::new(format!("Hero style: {}", next.name())),
                TextFont {
                    font_size: FontSize::Px(28.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            )],
        ));
    }
}

fn apply_gravity(mut heads: Query<(&PlayStyle, &mut GravityScale), Changed<PlayStyle>>) {
    for (style, mut gravity) in &mut heads {
        gravity.0 = style.gravity_scale();
    }
}

/// Holds a Sticky head on the wall (slow slide), then wall-jumps away from it.
#[allow(clippy::type_complexity)] // Bevy query tuple
fn wall_cling(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut clinging: Query<(
        Entity,
        &WallCling,
        &mut LinearVelocity,
        Option<&PlayStyle>,
        Option<&mut Hero>,
        Option<&mut crate::components::villager::Villager>,
    )>,
) {
    let now = time.elapsed_secs();
    for (entity, cling, mut velocity, style, hero, villager) in &mut clinging {
        if style != Some(&PlayStyle::Sticky) {
            commands.entity(entity).remove::<WallCling>();
        } else if now < cling.release_at {
            velocity.0 = Vec3::new(0.0, -40.0, 0.0);
        } else {
            velocity.0 = cling.hop().extend(0.0);
            // Walk away from the wall after the jump instead of straight back into it.
            if let Some(mut hero) = hero {
                hero.move_direction = cling.side;
            }
            if let Some(mut villager) = villager {
                villager.move_direction = cling.side;
            }
            commands.entity(entity).remove::<WallCling>();
        }
    }
}

/// Shows a non-default style next to the viewer's name: "bob · Heavy".
fn label_style_names(
    changed: Query<(Entity, &PlayStyle), Changed<PlayStyle>>,
    mut labels: Query<(&UsernameLabel, &mut Text2d)>,
) {
    if changed.is_empty() {
        return;
    }
    let styles: HashMap<Entity, PlayStyle> = changed.iter().map(|(e, s)| (e, *s)).collect();
    for (label, mut text) in &mut labels {
        if let Some(style) = styles.get(&label.owner) {
            text.0 = match style {
                PlayStyle::Bouncy => label.username.clone(),
                _ => format!("{} · {}", label.username, style.name()),
            };
        }
    }
}

/// Dev panel style buttons: click sets the hero's style; the active one is lit.
fn hero_style_buttons(
    mut commands: Commands,
    mut buttons: Query<(
        &bevy::picking::hover::PickingInteraction,
        &HeroStyleButton,
        &mut BackgroundColor,
    )>,
    heroes: Query<(Entity, Option<&PlayStyle>), With<Hero>>,
) {
    use bevy::picking::hover::PickingInteraction;
    let Ok((hero, current)) = heroes.single() else {
        return;
    };
    let mut active = current.copied().unwrap_or_default();
    for (interaction, button, _) in &buttons {
        if *interaction == PickingInteraction::Pressed && button.0 != active {
            commands.entity(hero).insert(button.0);
            active = button.0;
        }
    }
    for (interaction, button, mut color) in &mut buttons {
        let wanted = if button.0 == active {
            style_color(button.0).darker(0.35)
        } else if *interaction == PickingInteraction::Hovered {
            crate::systems::ui::dev_panel::HERO_STYLE_COLOR.lighter(0.1)
        } else {
            crate::systems::ui::dev_panel::HERO_STYLE_COLOR
        };
        if color.0 != wanted {
            color.0 = wanted;
        }
    }
}

/// Keeps the HUD "Style: ..." line in sync with the hero.
fn hero_style_hud(
    heroes: Query<Option<&PlayStyle>, With<Hero>>,
    mut texts: Query<(&mut Text, &mut TextColor), With<HeroStyleText>>,
) {
    let style = heroes.iter().next().flatten().copied().unwrap_or_default();
    let label = format!("Style: {}", style.name());
    for (mut text, mut color) in &mut texts {
        if text.0 != label {
            text.0 = label.clone();
            color.0 = style_color(style);
        }
    }
}

fn fade_toasts(
    mut commands: Commands,
    time: Res<Time>,
    mut toasts: Query<(Entity, &mut StyleToast)>,
) {
    for (entity, mut toast) in &mut toasts {
        if toast.0.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Puff of the split and merge: small coloured squares that drift and fade.
#[derive(Component)]
struct SplitPuff {
    velocity: Vec3,
    life: f32,
    max: f32,
}

fn puff(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    at: Vec3,
) {
    let mesh = meshes.add(Rectangle::new(1.0, 1.0));
    for i in 0..12 {
        let a = i as f32 * std::f32::consts::TAU / 12.0;
        let color = if i % 2 == 0 {
            Color::srgba(1.0, 0.55, 0.85, 0.9)
        } else {
            Color::srgba(1.0, 0.95, 1.0, 0.9)
        };
        commands.spawn((
            SplitPuff {
                velocity: Vec3::new(a.cos(), a.sin(), 0.0) * 260.0,
                life: 0.4,
                max: 0.4,
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_translation(at + Vec3::Z * 30.0).with_scale(Vec3::splat(9.0)),
            bevy::light::NotShadowCaster,
        ));
    }
}

fn fade_puffs(
    mut commands: Commands,
    time: Res<Time>,
    mut puffs: Query<(
        Entity,
        &mut SplitPuff,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    for (e, mut p, mut t, material) in &mut puffs {
        p.life -= dt;
        if p.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        t.translation += p.velocity * dt;
        if let Some(mut m) = materials.get_mut(&material.0) {
            m.base_color.set_alpha(p.life / p.max);
        }
    }
}

/// A Splitter's hard landing: the head shrinks and a small twin with the same
/// face pops out the other way. Both bounce and dig until they merge again.
#[allow(clippy::type_complexity)]
fn split_heads(
    mut commands: Commands,
    time: Res<Time>,
    mut requests: Query<(
        Entity,
        &Transform,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &avian3d::prelude::LinearVelocity,
        Has<Split>,
        Option<&SplitCooldown>,
        Option<&PlayStyle>,
        &SplitRequest,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let now = time.elapsed_secs();
    for (head, t, mesh, material, v, split, cooldown, style, request) in &mut requests {
        commands.entity(head).remove::<SplitRequest>();
        let ready = !split
            && cooldown.is_none_or(|c| now >= c.0)
            && style.copied() == Some(PlayStyle::Splitter);
        if !ready {
            continue;
        }
        // The twin heads the opposite way to the head's current drift.
        let side = if v.x >= 0.0 { -1.0 } else { 1.0 };
        let at = t.translation + Vec3::new(side * 30.0, 12.0, 0.0);
        let twin = commands
            .spawn((
                SplitTwin { parent: head },
                PlayStyle::Bouncy,
                crate::components::bundles::avian_physics::character(),
                crate::components::bundles::SpriteBundle3d::with_scale(
                    mesh.0.clone(),
                    material.0.clone(),
                    at,
                    Vec3::new(SPLIT_SCALE, SPLIT_SCALE, 1.0),
                ),
            ))
            .id();
        commands
            .entity(twin)
            .insert(avian3d::prelude::LinearVelocity(Vec3::new(
                side * 320.0,
                (request.speed * 0.55).clamp(260.0, 460.0),
                0.0,
            )));
        commands.entity(head).insert(Split {
            twin,
            until: now + SPLIT_SECONDS,
        });
        puff(&mut commands, &mut meshes, &mut materials, t.translation);
    }
}

/// After [`SPLIT_SECONDS`] the twin flies straight back (through terrain) and
/// merges with a puff; the head grows back to full size.
#[allow(clippy::type_complexity)]
fn merge_twins(
    mut commands: Commands,
    time: Res<Time>,
    heads: Query<(&Transform, &Split), Without<SplitTwin>>,
    mut twins: Query<
        (
            Entity,
            &SplitTwin,
            &mut Transform,
            Option<&mut avian3d::prelude::Position>,
            Option<&TwinReturning>,
        ),
        Without<Split>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let now = time.elapsed_secs();
    for (twin, link, mut t, position, returning) in &mut twins {
        // The head is gone (died, despawned) or no longer split: the twin goes too.
        let Ok((head_t, split)) = heads.get(link.parent) else {
            commands.entity(twin).despawn();
            continue;
        };
        if split.twin != twin {
            commands.entity(twin).despawn();
            continue;
        }
        if now < split.until {
            continue;
        }
        let Some(returning) = returning else {
            commands.entity(twin).insert((
                TwinReturning { since: now },
                avian3d::prelude::RigidBody::Kinematic,
                avian3d::prelude::LinearVelocity::ZERO,
            ));
            continue;
        };
        let target = head_t.translation;
        let to_head = target - t.translation;
        let step = 900.0 * time.delta_secs();
        let next = if to_head.length() <= step {
            target
        } else {
            t.translation + to_head.normalize() * step
        };
        t.translation = next;
        if let Some(mut p) = position {
            p.0 = next;
        }
        if to_head.length() < 28.0 || now - returning.since > 1.5 {
            commands.entity(twin).despawn();
            commands
                .entity(link.parent)
                .remove::<Split>()
                .insert(SplitCooldown(now + 2.5));
            puff(&mut commands, &mut meshes, &mut materials, target);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::command_queue::PowerCommand;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin))
            .insert_state(GameState::Playing)
            .add_computed_state::<crate::states::InGame>()
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(100),
            ))
            .add_message::<PowerActivated>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_plugins(PlayStylePlugin);
        app
    }

    fn head(app: &mut App, viewer: &str) -> Entity {
        let head = app
            .world_mut()
            .spawn((GravityScale(1.5), LinearVelocity::ZERO))
            .id();
        app.world_mut().spawn((
            UsernameLabel::with_offset(viewer.into(), head, 40.0),
            Text2d::new(viewer),
        ));
        head
    }

    fn style_command(app: &mut App, viewer: &str, kind: PowerCommandType, style: PlayStyle) {
        let mut command = PowerCommand::new(kind, viewer.into());
        command.style = Some(style);
        app.world_mut().write_message(PowerActivated { command });
    }

    #[test]
    fn style_reaches_existing_and_later_heads_of_that_viewer_only() {
        let mut app = app();
        let bob = head(&mut app, "Bob");
        let ann = head(&mut app, "ann");
        app.update();
        style_command(&mut app, "bob", PowerCommandType::Style, PlayStyle::Heavy);
        app.update();
        app.update();
        assert_eq!(app.world().get::<PlayStyle>(bob), Some(&PlayStyle::Heavy));
        assert_eq!(app.world().get::<PlayStyle>(ann), None);
        assert_eq!(app.world().get::<GravityScale>(bob).unwrap().0, 2.4);

        // A head Bob spawns later gets his remembered style and a labelled name.
        let later = head(&mut app, "BOB");
        app.update();
        app.update();
        assert_eq!(app.world().get::<PlayStyle>(later), Some(&PlayStyle::Heavy));
        let mut labels = app.world_mut().query::<(&UsernameLabel, &Text2d)>();
        let text = labels
            .iter(app.world())
            .find(|(l, _)| l.owner == later)
            .map(|(_, t)| t.0.clone());
        assert_eq!(text.as_deref(), Some("BOB · Heavy"));
    }

    #[test]
    fn sticky_heads_hold_the_wall_then_jump_away_from_it() {
        let mut app = app();
        let head = app
            .world_mut()
            .spawn((
                PlayStyle::Sticky,
                GravityScale(1.5),
                LinearVelocity(Vec3::new(-300.0, -200.0, 0.0)),
                WallCling {
                    side: 1.0,
                    release_at: 0.2,
                },
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<LinearVelocity>(head).unwrap().x, 0.0);
        for _ in 0..3 {
            app.update();
        }
        let v = app.world().get::<LinearVelocity>(head).unwrap().0;
        assert!(
            v.x > 0.0 && v.y > 0.0,
            "hop goes up and away from the wall: {v}"
        );
        assert!(app.world().get::<WallCling>(head).is_none());
    }
}
