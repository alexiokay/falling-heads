//! Combo streaks. A friendly head that keeps breaking blocks, each within
//! [`COMBO_WINDOW`] seconds of the last, builds a combo: a growing counter for
//! the hero, break notes that climb in pitch, milestones (x10 ON FIRE, x25
//! UNSTOPPABLE, x50 FRENZY) and capped XP and drop bonuses. Viewer heads keep
//! their own combos and big ones are announced. A split twin's breaks count
//! for its Splitter head.
use crate::components::block::BlockType;
use crate::components::hero::Hero;
use crate::components::play_style::SplitTwin;
use crate::components::username_label::UsernameLabel;
use crate::events::block_events::{BlockDestroyed, BonusDrop, HeadHitBlock};
use crate::events::xp_events::{XpGained, XpSource};
use bevy::prelude::*;
use rand::RngExt;
use std::collections::HashMap;

/// Seconds allowed between breaks before the combo ends.
pub const COMBO_WINDOW: f32 = 1.5;

/// A head's running combo and its best this session.
#[derive(Component, Default, Debug, Clone)]
pub struct Combo {
    pub count: u32,
    pub last: f32,
    pub best: u32,
    /// XP earned during the current combo (base plus bonus), for the summary.
    pub xp: u32,
    xp_fraction: f32,
}

/// XP multiplier for a combo of `count` breaks (capped at x3).
pub fn multiplier(count: u32) -> f32 {
    match count {
        25.. => 3.0,
        10.. => 2.0,
        5.. => 1.5,
        _ => 1.0,
    }
}

/// Milestone name and colour reached exactly at `count`.
fn milestone(count: u32) -> Option<&'static str> {
    match count {
        10 => Some("ON FIRE"),
        25 => Some("UNSTOPPABLE"),
        50 => Some("FRENZY"),
        _ => None,
    }
}

/// White, then yellow, orange and red-hot as the combo grows.
fn heat_color(count: u32) -> Color {
    match count {
        50.. => Color::srgb(1.0, 0.25, 0.2),
        25.. => Color::srgb(1.0, 0.5, 0.15),
        10.. => Color::srgb(1.0, 0.8, 0.2),
        5.. => Color::srgb(1.0, 0.95, 0.6),
        _ => Color::srgb(1.0, 1.0, 1.0),
    }
}

/// The head that last hit each block, so its destruction can be credited.
#[derive(Resource, Default)]
struct Hitters(HashMap<Entity, (Entity, f32)>);

/// Counters the combo probe reads.
#[derive(Resource, Default, Debug)]
pub struct ComboStats {
    pub head_hits: u32,
    pub credited: u32,
    pub hero_best: u32,
    /// Lengths of the hero's finished combos, oldest first.
    pub hero_ended: Vec<u32>,
}

#[derive(Resource)]
struct ComboSounds {
    tick: Handle<AudioSource>,
    milestone: Handle<AudioSource>,
    end: Handle<AudioSource>,
}

/// HUD: the hero's live combo, milestone bursts and the end summary.
#[derive(Component)]
pub struct ComboHud;

#[derive(Resource, Default)]
struct HudState {
    /// Seconds left showing a milestone or summary line.
    flash: f32,
    flash_text: String,
    flash_color: Color,
    /// Pulse after each break (font grows, then settles).
    pulse: f32,
}

#[derive(Component)]
struct ComboSpark {
    velocity: Vec3,
    life: f32,
}

pub struct CombosPlugin;

impl Plugin for CombosPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hitters>()
            .init_resource::<ComboStats>()
            .init_resource::<HudState>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    record_hits,
                    credit_breaks,
                    end_combos,
                    combo_hud,
                    combo_trail,
                )
                    .chain()
                    .run_if(in_state(crate::states::InGame)),
            );
    }
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(ComboSounds {
        tick: server.load("audio/mechanisms/note_harp.ogg"),
        milestone: server.load("audio/mechanisms/bell_use1.ogg"),
        end: server.load("audio/mechanisms/note_bass.ogg"),
    });
    commands.spawn((
        ComboHud,
        bevy::picking::Pickable::IGNORE,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(34.),
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(41.),
            width: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            ..default()
        },
        GlobalZIndex(60),
    ));
}

fn record_hits(
    time: Res<Time>,
    mut hits: MessageReader<HeadHitBlock>,
    mut hitters: ResMut<Hitters>,
    mut stats: ResMut<ComboStats>,
) {
    let now = time.elapsed_secs();
    for hit in hits.read() {
        hitters.0.insert(hit.block, (hit.head, now));
        stats.head_hits += 1;
    }
    // Forget hits on blocks that survived long ago.
    if hitters.0.len() > 512 {
        hitters.0.retain(|_, (_, at)| now - *at < 5.0);
    }
}

/// Random ore for the biggest combos (common to rare).
fn random_ore(rng: &mut impl RngExt) -> BlockType {
    match rng.random_range(0..100) {
        0..40 => BlockType::Coal,
        40..70 => BlockType::Iron,
        70..85 => BlockType::Gold,
        85..95 => BlockType::Redstone,
        95..99 => BlockType::Diamond,
        _ => BlockType::Emerald,
    }
}

fn is_ore(kind: BlockType) -> bool {
    matches!(
        kind,
        BlockType::Coal
            | BlockType::Iron
            | BlockType::Gold
            | BlockType::Diamond
            | BlockType::Emerald
            | BlockType::Redstone
            | BlockType::Netherite
            | BlockType::AncientDebris
    )
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn credit_breaks(
    mut commands: Commands,
    time: Res<Time>,
    mut destroyed: MessageReader<BlockDestroyed>,
    mut hitters: ResMut<Hitters>,
    mut heads: Query<(Option<&mut Combo>, Has<Hero>, Option<&SplitTwin>)>,
    labels: Query<&UsernameLabel>,
    mut xp: MessageWriter<XpGained>,
    mut drops: MessageWriter<BonusDrop>,
    mut stats: ResMut<ComboStats>,
    mut hud: ResMut<HudState>,
    mut notice: Option<ResMut<crate::systems::world_portals::PowerNotice>>,
    mut shake: Option<ResMut<crate::systems::extra_powers::CameraShake>>,
    sounds: Option<Res<ComboSounds>>,
) {
    let now = time.elapsed_secs();
    let mut rng = rand::rng();
    for event in destroyed.read() {
        let Some((mut head, _)) = hitters.0.remove(&event.entity) else {
            continue;
        };
        // A split twin's breaks build its Splitter head's combo.
        if let Ok((_, _, Some(twin))) = heads.get(head) {
            head = twin.parent;
        }
        let Ok((combo, hero, _)) = heads.get_mut(head) else {
            continue;
        };
        stats.credited += 1;
        let mut c = combo.map(|c| c.clone()).unwrap_or_default();
        if now - c.last > COMBO_WINDOW {
            c.count = 0;
            c.xp = 0;
            c.xp_fraction = 0.0;
        }
        c.count += 1;
        c.last = now;
        c.best = c.best.max(c.count);
        // The multiplier's share above x1 becomes extra XP (the base 1 XP per
        // block is awarded elsewhere).
        c.xp_fraction += multiplier(c.count) - 1.0;
        let bonus = c.xp_fraction.floor() as u32;
        c.xp_fraction -= bonus as f32;
        c.xp += 1 + bonus;
        if bonus > 0 {
            xp.write(XpGained {
                amount: bonus,
                source: XpSource::Combo,
            });
        }
        // Drop bonuses grow with the combo.
        let extra = match c.count {
            50.. => Some(if is_ore(event.block_type) {
                event.block_type
            } else {
                random_ore(&mut rng)
            }),
            25.. if is_ore(event.block_type) || rng.random_bool(0.1) => {
                Some(if is_ore(event.block_type) {
                    event.block_type
                } else {
                    random_ore(&mut rng)
                })
            }
            10.. if is_ore(event.block_type) && rng.random_bool(0.25) => Some(event.block_type),
            _ => None,
        };
        if let Some(block_type) = extra {
            drops.write(BonusDrop {
                block_type,
                position: event.position,
            });
        }
        let count = c.count;
        if hero {
            stats.hero_best = stats.hero_best.max(count);
            hud.pulse = 1.0;
            if let Some(sounds) = &sounds {
                // Each break a semitone higher, up to an octave.
                let pitch = 2f32.powf((count.saturating_sub(1).min(12)) as f32 / 12.0);
                commands.spawn((
                    AudioPlayer::new(sounds.tick.clone()),
                    PlaybackSettings::DESPAWN
                        .with_volume(bevy::audio::Volume::Linear(0.3))
                        .with_speed(pitch),
                ));
            }
        }
        if let Some(name) = milestone(count) {
            if hero {
                hud.flash = 1.4;
                hud.flash_text = name.to_string();
                hud.flash_color = heat_color(count);
                if let Some(shake) = shake.as_mut() {
                    shake.start(now, 0.35, 5.0 + count.min(50) as f32 * 0.12);
                }
                if let Some(sounds) = &sounds {
                    commands.spawn((
                        AudioPlayer::new(sounds.milestone.clone()),
                        PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.6)),
                    ));
                }
            } else if count >= 25
                && let Some(notice) = notice.as_mut()
                && let Some(label) = labels.iter().find(|l| l.owner == head)
            {
                notice.text = format!("{} hit a x{count} COMBO!", label.username);
                notice.remaining = 3.0;
            }
        }
        commands.entity(head).try_insert(c);
    }
}

/// Combos that ran out of time end: the hero gets a summary, viewers with a
/// big one get a shout-out.
#[allow(clippy::too_many_arguments)]
fn end_combos(
    mut commands: Commands,
    time: Res<Time>,
    mut stats: ResMut<ComboStats>,
    mut combos: Query<(Entity, &mut Combo, Has<Hero>)>,
    labels: Query<&UsernameLabel>,
    mut hud: ResMut<HudState>,
    mut notice: Option<ResMut<crate::systems::world_portals::PowerNotice>>,
    sounds: Option<Res<ComboSounds>>,
) {
    let now = time.elapsed_secs();
    for (head, mut c, hero) in &mut combos {
        if c.count == 0 || now - c.last <= COMBO_WINDOW {
            continue;
        }
        if hero {
            stats.hero_ended.push(c.count);
        }
        if hero && c.count >= 3 {
            hud.flash = 2.2;
            hud.flash_text = format!("x{} COMBO!  +{} XP", c.count, c.xp);
            hud.flash_color = heat_color(c.count);
            if let Some(sounds) = &sounds {
                commands.spawn((
                    AudioPlayer::new(sounds.end.clone()),
                    PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.45)),
                ));
            }
        } else if !hero
            && c.count >= 10
            && let Some(notice) = notice.as_mut()
            && let Some(label) = labels.iter().find(|l| l.owner == head)
        {
            notice.text = format!("{}: x{} COMBO", label.username, c.count);
            notice.remaining = 2.5;
        }
        c.count = 0;
        c.xp = 0;
        c.xp_fraction = 0.0;
    }
}

fn combo_hud(
    time: Res<Time>,
    mut hud_state: ResMut<HudState>,
    heroes: Query<&Combo, With<Hero>>,
    mut hud: Query<(&mut Text, &mut TextColor, &mut TextFont), With<ComboHud>>,
) {
    let dt = time.delta_secs();
    hud_state.flash = (hud_state.flash - dt).max(0.0);
    hud_state.pulse = (hud_state.pulse - dt * 4.0).max(0.0);
    let Ok((mut text, mut color, mut font)) = hud.single_mut() else {
        return;
    };
    let live = heroes.iter().next().map_or(0, |c| c.count);
    let (line, tint, size) = if live >= 2 {
        let tag = match live {
            50.. => "  FRENZY",
            25.. => "  UNSTOPPABLE",
            10.. => "  ON FIRE",
            _ => "",
        };
        (
            format!("x{live} COMBO{tag}"),
            heat_color(live),
            30.0 + live.min(50) as f32 * 0.3 + hud_state.pulse * 10.0,
        )
    } else if hud_state.flash > 0.0 {
        let fade = (hud_state.flash / 0.6).min(1.0);
        (
            hud_state.flash_text.clone(),
            hud_state.flash_color.with_alpha(fade),
            32.0,
        )
    } else {
        (String::new(), Color::WHITE, 30.0)
    };
    if text.0 != line {
        text.0 = line;
    }
    color.0 = tint;
    font.font_size = FontSize::Px(size);
}

/// A glowing trail behind the hero from x10, brighter and denser as it grows.
fn combo_trail(
    mut commands: Commands,
    time: Res<Time>,
    heroes: Query<(&Combo, &Transform), With<Hero>>,
    mut sparks: Query<
        (
            Entity,
            &mut ComboSpark,
            &mut Transform,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Without<Hero>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mesh: Local<Option<Handle<Mesh>>>,
) {
    let dt = time.delta_secs();
    for (e, mut s, mut t, material) in &mut sparks {
        s.life -= dt;
        if s.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        t.translation += s.velocity * dt;
        t.scale *= 1.0 - dt * 1.5;
        if let Some(mut m) = materials.get_mut(&material.0) {
            m.base_color.set_alpha((s.life / 0.4).min(1.0));
        }
    }
    let Ok((combo, hero)) = heroes.single() else {
        return;
    };
    if combo.count < 10 {
        return;
    }
    let mut rng = rand::rng();
    let mesh = mesh
        .get_or_insert_with(|| meshes.add(Rectangle::new(1.0, 1.0)))
        .clone();
    let per_frame = if combo.count >= 50 {
        3
    } else if combo.count >= 25 {
        2
    } else {
        1
    };
    for _ in 0..per_frame {
        let color = heat_color(combo.count);
        commands.spawn((
            ComboSpark {
                velocity: Vec3::new(
                    rng.random_range(-40.0..40.0),
                    rng.random_range(-20.0..50.0),
                    0.0,
                ),
                life: 0.4,
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                emissive: color.to_linear() * 2.5,
                unlit: true,
                alpha_mode: AlphaMode::Add,
                ..default()
            })),
            Transform::from_translation(
                hero.translation
                    + Vec3::new(
                        rng.random_range(-20.0..20.0),
                        rng.random_range(-20.0..20.0),
                        25.0,
                    ),
            )
            .with_scale(Vec3::splat(rng.random_range(6.0..12.0))),
            bevy::light::NotShadowCaster,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplier_steps_up_and_caps() {
        assert_eq!(multiplier(1), 1.0);
        assert_eq!(multiplier(5), 1.5);
        assert_eq!(multiplier(10), 2.0);
        assert_eq!(multiplier(25), 3.0);
        assert_eq!(multiplier(500), 3.0);
        assert_eq!(milestone(10), Some("ON FIRE"));
        assert_eq!(milestone(11), None);
    }
}
