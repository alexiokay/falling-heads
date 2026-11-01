//! `--visual-smoke-test --combo-test`: checks combo streaks. Waits until the
//! terrain solver credits a real hit by the hero, then breaks blocks next to
//! the hero in quick succession (as the hero's hits) and checks the counter,
//! the x10 milestone, bonus XP and the end summary. Saves
//! `target/visual-combo-<step>.png`.
use crate::components::block::{Block, BlockType};
use crate::components::hero::Hero;
use crate::events::block_events::{BlockDestroyed, HeadHitBlock};
use crate::systems::combos::{COMBO_WINDOW, Combo, ComboHud, ComboStats};
use bevy::prelude::*;

const BREAKS: u32 = 14;

#[derive(Resource)]
struct Probe {
    step: usize,
    since: f32,
    broken: u32,
    next_break: f32,
}

fn capture(w: &mut World, name: &str) {
    w.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(format!(
            "target/visual-combo-{name}.png"
        )));
}

fn hud_text(w: &mut World) -> String {
    w.query_filtered::<&Text, With<ComboHud>>()
        .iter(w)
        .next()
        .map(|t| t.0.clone())
        .unwrap_or_default()
}

/// Breaks the nearest block to the hero as if the hero had hit it.
fn break_one(w: &mut World) -> bool {
    let Some((hero, at)) = w
        .query_filtered::<(Entity, &Transform), With<Hero>>()
        .iter(w)
        .next()
        .map(|(e, t)| (e, t.translation))
    else {
        return false;
    };
    let nearest = w
        .query_filtered::<(Entity, &Transform, &BlockType), With<Block>>()
        .iter(w)
        .filter(|(_, t, _)| t.translation.xy().distance(at.xy()) < 400.0)
        .min_by(|a, b| {
            a.1.translation
                .xy()
                .distance(at.xy())
                .total_cmp(&b.1.translation.xy().distance(at.xy()))
        })
        .map(|(e, t, k)| (e, t.translation, *k));
    let Some((block, position, block_type)) = nearest else {
        return false;
    };
    w.write_message(HeadHitBlock { head: hero, block });
    w.write_message(BlockDestroyed {
        tile: None,
        entity: block,
        block_type,
        position,
    });
    w.entity_mut(block).despawn();
    true
}

pub fn run(w: &mut World) {
    let now = w.resource::<Time<Real>>().elapsed_secs();
    assert!(now < 90.0, "Combo test timed out");
    if !w.contains_resource::<Probe>() {
        let blocks = w.query_filtered::<(), With<Block>>().iter(w).count();
        if now < 8.0 || blocks < 100 {
            return;
        }
        w.insert_resource(Probe {
            step: 0,
            since: now,
            broken: 0,
            next_break: 0.0,
        });
        return;
    }
    let (step, since) = {
        let p = w.resource::<Probe>();
        (p.step, p.since)
    };
    let next = |w: &mut World| {
        let mut p = w.resource_mut::<Probe>();
        p.step += 1;
        p.since = now;
    };
    match step {
        // Natural play first: the solver reports real hits by heads, and the
        // hero's own combos are measured for 12 s.
        0 => {
            if now - since > 12.0 && w.resource::<ComboStats>().head_hits > 0 {
                let live = w
                    .query_filtered::<&Combo, With<Hero>>()
                    .iter(w)
                    .next()
                    .map_or(0, |c| c.count);
                let stats = w.resource::<ComboStats>();
                eprintln!(
                    "[COMBO] natural 12 s: finished combos {:?}, live x{live}, best x{}",
                    stats.hero_ended, stats.hero_best
                );
                eprintln!(
                    "[COMBO] solver reported {} head hits",
                    w.resource::<ComboStats>().head_hits
                );
                next(w);
            } else {
                assert!(now - since < 50.0, "no head ever hit a block");
            }
        }
        // Break blocks a quarter second apart: well inside the combo window.
        1 => {
            let p = w.resource::<Probe>();
            if p.broken >= BREAKS {
                next(w);
                return;
            }
            if now >= p.next_break && break_one(w) {
                let mut p = w.resource_mut::<Probe>();
                p.broken += 1;
                p.next_break = now + 0.25;
            }
        }
        2 if now - since > 0.2 => {
            let combo = w
                .query_filtered::<&Combo, With<Hero>>()
                .iter(w)
                .next()
                .cloned()
                .expect("hero has a combo");
            assert!(
                combo.count >= BREAKS - 1,
                "combo only reached x{}",
                combo.count
            );
            assert!(
                combo.xp > combo.count,
                "no bonus XP ({} for x{})",
                combo.xp,
                combo.count
            );
            let text = hud_text(w);
            assert!(
                text.contains("COMBO") && text.contains("ON FIRE"),
                "HUD shows {text:?}"
            );
            eprintln!(
                "[COMBO] x{} with {} XP; HUD {:?}",
                combo.count, combo.xp, text
            );
            capture(w, "on-fire");
            next(w);
        }
        // Stop breaking: the long combo ends (the hero's own breaks may keep
        // it going a little longer) and its summary shows.
        3 if now - since > COMBO_WINDOW => {
            let ended = w
                .resource::<ComboStats>()
                .hero_ended
                .iter()
                .any(|n| *n >= BREAKS - 1);
            if !ended {
                assert!(now - since < 20.0, "the long combo never ended");
                return;
            }
            let best = w
                .query_filtered::<&Combo, With<Hero>>()
                .iter(w)
                .next()
                .unwrap()
                .best;
            assert!(best >= BREAKS - 1, "best combo {best}");
            let text = hud_text(w);
            assert!(
                text.contains("COMBO"),
                "after the combo the HUD shows {text:?}"
            );
            eprintln!("[COMBO] ended: best x{best}; HUD {text:?}");
            capture(w, "summary");
            next(w);
        }
        4 if now - since > 0.6 => {
            eprintln!("[COMBO] passed");
            w.write_message(AppExit::Success);
        }
        _ => {}
    }
}
