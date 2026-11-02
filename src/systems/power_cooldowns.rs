//! Per-power cooldowns, set under Settings > Cooldowns. Every power starts at
//! 0 (no cooldown). A value of N seconds blocks the same power for N seconds
//! after it runs, whoever sent it; `process_commands` checks it before a power
//! starts. Saved to `power-cooldowns.json` in the settings folder.
use crate::components::settings_menu::{SettingsPage, SettingsPageRoot};
use crate::resources::command_queue::PowerCommandType as P;
use crate::resources::settings::{persistence_enabled, read_settings_file, write_settings_file};
use crate::states::GameState;
use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use std::collections::HashMap;

const COOLDOWN_FILE: &str = "power-cooldowns.json";

/// Values a click steps through, in seconds; the last wraps back to 0.
pub const STEPS: [u32; 9] = [0, 5, 10, 15, 30, 60, 120, 300, 600];

/// Every power in the order the settings page lists them.
pub const POWERS: [P; 35] = [
    P::Bolt,
    P::Thunder,
    P::Tnt,
    P::Arrow,
    P::Laser,
    P::LaserBurst,
    P::Ghast,
    P::Anvil,
    P::Quake,
    P::Wind,
    P::BlackHole,
    P::IceAge,
    P::Bees,
    P::Dragon,
    P::Golem,
    P::Villagers,
    P::Mirror,
    P::Magnet,
    P::Box,
    P::Lootbox,
    P::RandomBox,
    P::GrowBuff,
    P::Shrink,
    P::Fast,
    P::Slow,
    P::Feather,
    P::Pearl,
    P::Style,
    P::Fuss,
    P::WaterBucket,
    P::LavaBucket,
    P::Teleport,
    P::Launcher,
    P::Rift,
    P::Rewind,
];

/// Saved id and shown name of a power.
pub fn names(p: P) -> (&'static str, &'static str) {
    match p {
        P::WaterBucket => ("waterbucket", "Water"),
        P::LavaBucket => ("lavabucket", "Lava"),
        P::Teleport => ("teleport", "Teleport"),
        P::Launcher => ("launcher", "Launcher"),
        P::Rift => ("rift", "Rift"),
        P::Rewind => ("rewind", "Rewind"),
        P::Tnt => ("tnt", "TNT"),
        P::Fast => ("fast", "Fast"),
        P::Slow => ("slow", "Slow"),
        P::Fuss => ("fuss", "Fuss"),
        P::Bolt => ("bolt", "Bolt"),
        P::Arrow => ("arrow", "Arrow"),
        P::Laser => ("laser", "Laser"),
        P::Ghast => ("ghast", "Ghast"),
        P::Box => ("box", "Box"),
        P::Magnet => ("magnet", "Magnet"),
        P::Villagers => ("villagers", "Villagers"),
        P::Mirror => ("mirror", "Mirror"),
        P::Dragon => ("dragon", "Dragon"),
        P::Thunder => ("thunder", "Thunder"),
        P::Lootbox => ("lootbox", "Lootbox"),
        P::RandomBox => ("randombox", "Random box"),
        P::GrowBuff => ("grow", "Grow"),
        P::LaserBurst => ("burst", "Burst"),
        P::Style => ("style", "Style"),
        P::Feather => ("feather", "Feather"),
        P::Anvil => ("anvil", "Anvil"),
        P::Golem => ("golem", "Golem"),
        P::Pearl => ("pearl", "Pearl"),
        P::Quake => ("quake", "Quake"),
        P::Shrink => ("shrink", "Shrink"),
        P::Wind => ("wind", "Wind"),
        P::BlackHole => ("blackhole", "Black hole"),
        P::IceAge => ("iceage", "Ice age"),
        P::Bees => ("bees", "Bees"),
    }
}

fn seconds_label(s: u32) -> String {
    match s {
        0 => "off".into(),
        s if s >= 60 && s % 60 == 0 => format!("{}m", s / 60),
        s => format!("{s}s"),
    }
}

/// Cooldown seconds per power id; a missing id means 0.
#[derive(Resource, Default, Clone, PartialEq, Debug)]
pub struct PowerCooldowns(HashMap<String, u32>);

impl PowerCooldowns {
    pub fn get(&self, p: P) -> u32 {
        self.0.get(names(p).0).copied().unwrap_or(0)
    }

    pub fn set(&mut self, p: P, seconds: u32) {
        if seconds == 0 {
            self.0.remove(names(p).0);
        } else {
            self.0.insert(names(p).0.to_string(), seconds);
        }
    }

    /// The next value in `STEPS` after the current one.
    fn step(&mut self, p: P) {
        let now = self.get(p);
        let next = STEPS.iter().copied().find(|&s| s > now).unwrap_or(0);
        self.set(p, next);
    }

    fn load() -> Self {
        if !persistence_enabled() {
            return Self::default();
        }
        read_settings_file(COOLDOWN_FILE)
            .and_then(|s| serde_json::from_str::<HashMap<String, u32>>(&s).ok())
            .map(|m| Self(m.into_iter().filter(|(_, s)| *s > 0).collect()))
            .unwrap_or_default()
    }

    fn save(&self) {
        if !persistence_enabled() {
            return;
        }
        let sorted: std::collections::BTreeMap<_, _> = self.0.iter().collect();
        if let Ok(json) = serde_json::to_string_pretty(&sorted) {
            write_settings_file(COOLDOWN_FILE, &json);
        }
    }
}

/// When each power may run again, in game seconds.
#[derive(Resource, Default)]
pub struct PowerReadyAt(HashMap<P, f32>);

impl PowerReadyAt {
    /// Ok when the power may run now (and starts its cooldown); otherwise the
    /// seconds left.
    pub fn try_use(&mut self, cooldowns: &PowerCooldowns, p: P, now: f32) -> Result<(), f32> {
        let seconds = cooldowns.get(p);
        if seconds == 0 {
            return Ok(());
        }
        if let Some(&ready) = self.0.get(&p) {
            // A shortened cooldown takes effect right away.
            let ready = ready.min(now + seconds as f32);
            if now < ready {
                return Err(ready - now);
            }
        }
        self.0.insert(p, now + seconds as f32);
        Ok(())
    }
}

#[derive(Component)]
pub(crate) struct CooldownButton(pub(crate) P);

#[derive(Component)]
struct AllOffButton;

pub struct PowerCooldownsPlugin;

impl Plugin for PowerCooldownsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PowerCooldowns::load())
            .init_resource::<PowerReadyAt>()
            .add_systems(
                Update,
                (fill_cooldown_page, press_buttons, label_buttons)
                    .chain()
                    .run_if(in_state(GameState::Settings)),
            );
    }
}

fn text(label: impl Into<String>, size: f32) -> impl Bundle {
    (
        bevy::picking::Pickable::IGNORE,
        Text::new(label),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
    )
}

fn button(marker: impl Bundle, width: Val) -> impl Bundle {
    (
        marker,
        Button,
        PickingInteraction::default(),
        Node {
            width,
            min_height: Val::Px(26.),
            padding: UiRect::axes(Val::Px(4.), Val::Px(4.)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.10, 0.16, 0.19)),
    )
}

fn fill_cooldown_page(
    mut commands: Commands,
    pages: Query<(Entity, &SettingsPageRoot), Added<SettingsPageRoot>>,
) {
    for (page, root) in &pages {
        if root.0 != SettingsPage::Cooldowns {
            continue;
        }
        commands.entity(page).with_children(|p| {
            p.spawn((
                text(
                    "Click a power to step its cooldown (off, 5s ... 10m). Off means it can be used again right away.",
                    13.,
                ),
                TextColor(Color::srgb(0.7, 0.75, 0.78)),
            ));
            p.spawn(Node {
                width: Val::Percent(100.),
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(4.),
                row_gap: Val::Px(4.),
                ..default()
            })
            .with_children(|grid| {
                for power in POWERS {
                    grid.spawn(button(CooldownButton(power), Val::Percent(24.)))
                        .with_child(text("", 12.));
                }
            });
            p.spawn(button(AllOffButton, Val::Percent(100.)))
                .with_child(text("All cooldowns off", 14.));
        });
    }
}

type Pressed<T> = (Changed<PickingInteraction>, With<T>);

fn press_buttons(
    mut cooldowns: ResMut<PowerCooldowns>,
    powers: Query<(Entity, &PickingInteraction, &CooldownButton), Pressed<CooldownButton>>,
    all_off: Query<(Entity, &PickingInteraction), Pressed<AllOffButton>>,
    mut presses: Local<crate::systems::settings_menu_system::PressTracker>,
) {
    let mut next = cooldowns.clone();
    for (entity, interaction, button) in &powers {
        if presses.pressed(entity, *interaction) {
            next.step(button.0);
        }
    }
    for (entity, interaction) in &all_off {
        if presses.pressed(entity, *interaction) {
            next = PowerCooldowns::default();
        }
    }
    if next != *cooldowns {
        next.save();
        *cooldowns = next;
    }
}

/// A cooldown page button: its state, colour, label child and power (none for "all off").
type CooldownLook<'a> = (
    &'a PickingInteraction,
    &'a mut BackgroundColor,
    &'a Children,
    Option<&'a CooldownButton>,
);

type AnyCooldownButton = Or<(With<CooldownButton>, With<AllOffButton>)>;

fn label_buttons(
    cooldowns: Res<PowerCooldowns>,
    mut buttons: Query<CooldownLook, AnyCooldownButton>,
    mut texts: Query<&mut Text>,
) {
    for (interaction, mut bg, children, power) in &mut buttons {
        let on = power.is_some_and(|p| cooldowns.get(p.0) > 0);
        bg.0 = match interaction {
            PickingInteraction::Pressed => Color::srgb(0.20, 0.42, 0.46),
            PickingInteraction::Hovered => Color::srgb(0.16, 0.28, 0.31),
            PickingInteraction::None if on => Color::srgb(0.32, 0.22, 0.10),
            PickingInteraction::None => Color::srgb(0.10, 0.16, 0.19),
        };
        let Some(power) = power else {
            continue;
        };
        let label = format!(
            "{} {}",
            names(power.0).1,
            seconds_label(cooldowns.get(power.0))
        );
        for child in children {
            if let Ok(mut t) = texts.get_mut(*child)
                && **t != label
            {
                **t = label.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_power_listed_once_with_a_unique_id() {
        let mut ids: Vec<&str> = POWERS.iter().map(|&p| names(p).0).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), POWERS.len());
    }

    #[test]
    fn default_is_no_cooldown_and_steps_wrap() {
        let mut c = PowerCooldowns::default();
        let mut ready = PowerReadyAt::default();
        for t in 0..5 {
            assert!(ready.try_use(&c, P::Bolt, t as f32 * 0.01).is_ok());
        }
        for &s in &STEPS[1..] {
            c.step(P::Bolt);
            assert_eq!(c.get(P::Bolt), s);
        }
        c.step(P::Bolt);
        assert_eq!(c.get(P::Bolt), 0);
    }

    #[test]
    fn cooldown_blocks_the_same_power_only_until_it_ends() {
        let mut c = PowerCooldowns::default();
        c.set(P::BlackHole, 30);
        let mut ready = PowerReadyAt::default();
        assert!(ready.try_use(&c, P::BlackHole, 10.).is_ok());
        assert_eq!(ready.try_use(&c, P::BlackHole, 20.), Err(20.));
        assert!(ready.try_use(&c, P::Bolt, 20.).is_ok());
        assert!(ready.try_use(&c, P::BlackHole, 40.).is_ok());
        // Turning the cooldown off frees the power at once.
        c.set(P::BlackHole, 0);
        assert!(ready.try_use(&c, P::BlackHole, 41.).is_ok());
    }
}
