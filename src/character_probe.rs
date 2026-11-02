//! `--visual-smoke-test --character-test`: picks a character and the Splitter
//! style on Settings > Character with the real buttons, checks the hero took
//! them, then makes the hero land hard and checks it splits and merges back.
//! Saves `target/visual-character-<step>.png`.
use crate::components::hero::Hero;
use crate::components::play_style::{PlayStyle, Split, SplitCooldown, SplitTwin};
use crate::components::settings_menu::{SettingsNavigation, SettingsPage};
use crate::states::GameState;
use crate::systems::character_select::{CHARACTERS, ChosenCharacter};
use avian3d::prelude::LinearVelocity;
use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;

#[derive(Resource)]
struct Probe {
    step: usize,
    since: f32,
}

fn capture(w: &mut World, name: &str) {
    w.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(format!(
            "target/visual-character-{name}.png"
        )));
}

/// Clicks the first matching button like a pointer: hover on the first call,
/// press on the next. Returns true once the press is done; panics if missing.
fn press<T: Component>(w: &mut World, matches: impl Fn(&T) -> bool) -> bool {
    let target = w
        .query::<(Entity, &T, &PickingInteraction)>()
        .iter(w)
        .find(|(_, b, _)| matches(b))
        .map(|(e, _, i)| (e, *i));
    let Some((e, interaction)) = target else {
        panic!("button not found");
    };
    if interaction == PickingInteraction::Hovered {
        w.entity_mut(e).insert(PickingInteraction::Pressed);
        true
    } else {
        w.entity_mut(e).insert(PickingInteraction::Hovered);
        false
    }
}

fn hero(w: &mut World) -> Option<Entity> {
    w.query_filtered::<Entity, With<Hero>>().iter(w).next()
}

pub fn run(w: &mut World) {
    let now = w.resource::<Time<Real>>().elapsed_secs();
    assert!(now < 110.0, "Character test timed out");
    if !w.contains_resource::<Probe>() {
        if now < 8.0 || hero(w).is_none() {
            return;
        }
        w.resource_mut::<NextState<GameState>>()
            .set(GameState::Settings);
        w.insert_resource(Probe {
            step: 0,
            since: now,
        });
        return;
    }
    let Probe { step, since } = *w.resource::<Probe>();
    let next = |w: &mut World| {
        let mut p = w.resource_mut::<Probe>();
        p.step += 1;
        p.since = now;
    };
    let ember = CHARACTERS.iter().position(|c| c.id == "ember").unwrap();
    match step {
        0 if now - since > 1.0 => {
            w.resource_mut::<SettingsNavigation>().0 = SettingsPage::Character;
            next(w);
        }
        1 if now - since > 1.5 => {
            capture(w, "page");
            next(w);
        }
        2 if now - since > 1.0 => {
            if press::<crate::systems::character_select::CharacterButtonId>(w, |b| b.0 == ember) {
                next(w);
            }
        }
        3 if now - since > 0.5 => {
            if press::<crate::systems::character_select::StyleButtonId>(w, |b| {
                b.0 == PlayStyle::Splitter
            }) {
                next(w);
            }
        }
        4 if now - since > 1.0 => {
            let chosen = w.resource::<ChosenCharacter>().clone();
            assert_eq!(chosen.character, ember, "character not chosen");
            assert_eq!(chosen.style, PlayStyle::Splitter, "style not chosen");
            let hero = hero(w).unwrap();
            assert_eq!(w.get::<PlayStyle>(hero), Some(&PlayStyle::Splitter));
            // The hero's face texture is now Ember's.
            let material = w
                .get::<MeshMaterial3d<StandardMaterial>>(hero)
                .unwrap()
                .0
                .clone();
            let texture = w
                .resource::<Assets<StandardMaterial>>()
                .get(&material)
                .and_then(|m| m.base_color_texture.clone())
                .unwrap();
            let path = w
                .resource::<AssetServer>()
                .get_path(&texture)
                .unwrap()
                .to_string();
            assert!(path.contains("skinmc-avatar (3)"), "hero face is {path}");
            eprintln!("[CHARACTER] Ember + Splitter applied to the hero ({path})");
            capture(w, "chosen");
            w.resource_mut::<NextState<GameState>>()
                .set(GameState::Playing);
            next(w);
        }
        // Slam the hero down until it splits.
        5 => {
            let hero = hero(w).unwrap();
            if w.get::<Split>(hero).is_some() {
                let twins = w.query::<&SplitTwin>().iter(w).count();
                assert_eq!(twins, 1, "expected one twin");
                eprintln!("[CHARACTER] hero split after {:.1}s", now - since);
                next(w);
                return;
            }
            assert!(now - since < 30.0, "hero never split");
            if let Some(mut v) = w.get_mut::<LinearVelocity>(hero)
                && v.y > -200.0
            {
                v.y = -900.0;
            }
        }
        6 if now - since > 0.6 => {
            capture(w, "split");
            next(w);
        }
        // Merges back after the split time, then may not split right away.
        7 => {
            let hero = hero(w).unwrap();
            if w.get::<Split>(hero).is_none() {
                assert!(
                    w.get::<SplitCooldown>(hero).is_some(),
                    "no cooldown after merge"
                );
                assert_eq!(w.query::<&SplitTwin>().iter(w).count(), 0, "twin left over");
                eprintln!("[CHARACTER] merged back after {:.1}s", now - since);
                capture(w, "merged");
                next(w);
                return;
            }
            assert!(now - since < 8.0, "twin never merged back");
        }
        // Settings > Cooldowns: the page lists every power; a click steps BOLT
        // from off to 5 s.
        8 if now - since > 1.0 => {
            w.resource_mut::<NextState<GameState>>()
                .set(GameState::Settings);
            next(w);
        }
        9 if now - since > 1.5 => {
            use crate::systems::power_cooldowns::CooldownButton;
            // Entering Settings resets the tab, so pick it once the menu is up.
            if w.resource::<SettingsNavigation>().0 != SettingsPage::Cooldowns {
                w.resource_mut::<SettingsNavigation>().0 = SettingsPage::Cooldowns;
                return;
            }
            let buttons = w.query::<&CooldownButton>().iter(w).count();
            assert_eq!(buttons, crate::systems::power_cooldowns::POWERS.len());
            capture(w, "cooldowns");
            if press::<CooldownButton>(w, |b| {
                b.0 == crate::resources::command_queue::PowerCommandType::Bolt
            }) {
                next(w);
            }
        }
        10 if now - since > 1.0 => {
            use crate::resources::command_queue::PowerCommandType as P;
            let mut c = w.resource_mut::<crate::systems::power_cooldowns::PowerCooldowns>();
            assert_eq!(c.get(P::Bolt), 5, "BOLT cooldown not stepped");
            c.set(P::Bolt, 0);
            capture(w, "cooldowns-bolt");
            eprintln!("[CHARACTER] cooldown page stepped BOLT to 5s");
            next(w);
        }
        11 if now - since > 0.5 => {
            eprintln!("[CHARACTER] passed");
            w.write_message(AppExit::Success);
        }
        _ => {}
    }
}
