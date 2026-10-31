//! `--visual-smoke-test --pack-switch-test`: switches theme packs while the
//! game runs and checks that block textures and generated atlases change, then
//! switches back and checks they are restored. Needs at least two installed
//! packs. Saves `target/visual-pack-<step>.png`.
use crate::resources::asset_handles::{BlockAtlases, BlockTextures};
use crate::systems::asset_packs::{AssetPacks, SwitchPack};
use bevy::prelude::*;

/// Seconds to let the remaining reloads finish before a screenshot.
const SETTLE: f32 = 1.5;

#[derive(Resource)]
struct Probe {
    step: usize,
    since: f32,
    first: String,
    other: String,
    /// Pixels of the sampled images under the first pack.
    original: Vec<Vec<u8>>,
}

fn pixels(w: &World, handle: &Handle<Image>) -> Option<Vec<u8>> {
    w.resource::<Assets<Image>>()
        .get(handle)
        .and_then(|i| i.data.clone())
}

/// Sand and dirt textures, plus the oak log and pumpkin atlases built from textures.
fn samples(w: &World) -> Option<Vec<Vec<u8>>> {
    let t = w.get_resource::<BlockTextures>()?;
    let a = w.get_resource::<BlockAtlases>()?;
    [&t.sand, &t.dirt, &a.oak_log, &a.pumpkin]
        .into_iter()
        .map(|h| pixels(w, h))
        .collect()
}

fn capture(w: &mut World, name: &str) {
    w.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(format!(
            "target/visual-pack-{name}.png"
        )));
}

fn advance(w: &mut World, now: f32) {
    let mut p = w.resource_mut::<Probe>();
    p.step += 1;
    p.since = now;
}

pub fn run(w: &mut World) {
    let now = w.resource::<Time<Real>>().elapsed_secs();
    assert!(now < 120.0, "Pack switch test timed out");
    if !w.contains_resource::<Probe>() {
        let blocks = w
            .query_filtered::<(), With<crate::components::block::Block>>()
            .iter(w)
            .count();
        let Some(original) = samples(w) else {
            return;
        };
        if now < 8.0 || blocks < 100 {
            return;
        }
        let packs = w.resource::<AssetPacks>();
        let first = packs.active.clone();
        let other = packs
            .installed
            .iter()
            .map(|p| p.id.clone())
            .find(|id| *id != first)
            .expect("pack switch test needs two installed packs");
        eprintln!("[PACKS] start on {first}, switching to {other}");
        w.insert_resource(Probe {
            step: 0,
            since: now,
            first,
            other,
            original,
        });
        capture(w, "before");
        return;
    }
    let Some(now_pixels) = samples(w) else {
        return;
    };
    let p = w.resource::<Probe>();
    let (step, since) = (p.step, p.since);
    let all_changed = now_pixels.iter().zip(&p.original).all(|(a, b)| a != b);
    let all_restored = now_pixels == p.original;
    match step {
        0 if now - since > 1.0 => {
            let other = p.other.clone();
            w.write_message(SwitchPack(other));
            advance(w, now);
        }
        1 if all_changed => {
            assert_eq!(w.resource::<AssetPacks>().active, p.other);
            eprintln!(
                "[PACKS] switched to {} in {:.1}s: textures and atlases changed",
                p.other,
                now - since
            );
            advance(w, now);
        }
        1 => assert!(
            now - since < 20.0,
            "Textures did not all change after switching"
        ),
        2 if now - since > SETTLE => {
            capture(w, "switched");
            advance(w, now);
        }
        3 if now - since > 1.0 => {
            let first = p.first.clone();
            w.write_message(SwitchPack(first));
            advance(w, now);
        }
        4 if all_restored => {
            eprintln!(
                "[PACKS] back on {} in {:.1}s: textures and atlases restored",
                p.first,
                now - since
            );
            advance(w, now);
        }
        4 => assert!(
            now - since < 20.0,
            "Textures did not return after switching back"
        ),
        5 if now - since > SETTLE => {
            capture(w, "restored");
            advance(w, now);
        }
        6 if now - since > 1.0 => {
            eprintln!("[PACKS] live pack switching passed");
            w.write_message(AppExit::Success);
        }
        _ => {}
    }
}
