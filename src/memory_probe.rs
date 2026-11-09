//! `--memory-probe`: lets the game play itself (as on an unattended stream) for
//! `FALLING_HEADS_MEMORY_MINUTES` minutes (default 5) and fails if memory keeps
//! growing after a one-minute warm-up. Checks process private bytes, the image
//! asset count and Bevy's glyph atlases (never freed; one 512x512 texture per
//! distinct font size). Prints `[MEMORY]` lines every ten seconds.
//!
//! Regression check for the 2026-11-08 out-of-memory: the combo HUD animated its
//! font size, which created a new glyph atlas every frame (about 2 GB a minute).
use crate::systems::combos::ComboStats;
use bevy::prelude::*;
use bevy::text::FontAtlasSet;

const WARMUP_SECS: f32 = 60.0;
const SAMPLE_SECS: f32 = 10.0;
/// Images may still appear after warm-up (biome art, pack textures), but not steadily.
const IMAGE_GROWTH_LIMIT: usize = 64;
/// Distinct (font, size) glyph atlas sets in the whole game.
const FONT_ATLAS_KEY_LIMIT: usize = 48;
const DEFAULT_PRIVATE_GROWTH_LIMIT_MIB: u64 = 1024;

#[derive(Clone, Copy)]
struct Sample {
    private_mib: Option<u64>,
    images: usize,
    font_keys: usize,
    font_atlases: usize,
}

#[derive(Resource)]
struct Probe {
    minutes: f32,
    private_limit_mib: u64,
    next_sample: f32,
    baseline: Option<Sample>,
    peak_private_mib: u64,
}

pub struct MemoryProbePlugin;

impl Plugin for MemoryProbePlugin {
    fn build(&self, app: &mut App) {
        fn env<T: std::str::FromStr>(name: &str) -> Option<T> {
            std::env::var(name).ok().and_then(|v| v.parse().ok())
        }
        let probe = Probe {
            minutes: env("FALLING_HEADS_MEMORY_MINUTES")
                .unwrap_or(5.0_f32)
                .max(1.5),
            private_limit_mib: env("FALLING_HEADS_MEMORY_LIMIT_MIB")
                .unwrap_or(DEFAULT_PRIVATE_GROWTH_LIMIT_MIB),
            next_sample: SAMPLE_SECS,
            baseline: None,
            peak_private_mib: 0,
        };
        eprintln!(
            "[MEMORY] probe: {:.1} min, warm-up {WARMUP_SECS:.0} s, limits: private +{} MiB, images +{IMAGE_GROWTH_LIMIT}, font atlas sets {FONT_ATLAS_KEY_LIMIT}",
            probe.minutes, probe.private_limit_mib
        );
        app.insert_resource(probe).add_systems(Last, run);
    }
}

fn sample(w: &World) -> Sample {
    let atlases = w.resource::<FontAtlasSet>();
    Sample {
        private_mib: private_bytes().map(|b| b / 1_048_576),
        images: w.resource::<Assets<Image>>().len(),
        font_keys: atlases.len(),
        font_atlases: atlases.values().map(Vec::len).sum(),
    }
}

fn run(w: &mut World) {
    let now = w.resource::<Time<Real>>().elapsed_secs();
    let (next, minutes) = {
        let p = w.resource::<Probe>();
        (p.next_sample, p.minutes)
    };
    if now < next {
        return;
    }
    let s = sample(w);
    let combos = w.resource::<ComboStats>();
    let (breaks, best) = (combos.credited, combos.hero_best);
    let mut p = w.resource_mut::<Probe>();
    p.next_sample = now + SAMPLE_SECS;
    p.peak_private_mib = p.peak_private_mib.max(s.private_mib.unwrap_or(0));
    eprintln!(
        "[MEMORY] t={now:.0}s private={} MiB images={} font_atlas_sets={} font_atlases={} breaks={breaks} hero_best_combo={best}",
        s.private_mib.map_or("?".into(), |m| m.to_string()),
        s.images,
        s.font_keys,
        s.font_atlases,
    );
    if p.baseline.is_none() && now >= WARMUP_SECS {
        p.baseline = Some(s);
    }
    let mut failures = Vec::new();
    if s.font_keys > FONT_ATLAS_KEY_LIMIT {
        failures.push(format!(
            "{} glyph atlas sets (limit {FONT_ATLAS_KEY_LIMIT}): some text animates its font size",
            s.font_keys
        ));
    }
    if let Some(b) = p.baseline {
        if s.images > b.images + IMAGE_GROWTH_LIMIT {
            failures.push(format!(
                "images grew from {} to {} after warm-up (limit +{IMAGE_GROWTH_LIMIT})",
                b.images, s.images
            ));
        }
        if let (Some(now_mib), Some(base_mib)) = (s.private_mib, b.private_mib)
            && now_mib > base_mib + p.private_limit_mib
        {
            failures.push(format!(
                "private bytes grew from {base_mib} to {now_mib} MiB after warm-up (limit +{} MiB)",
                p.private_limit_mib
            ));
        }
    }
    let done = now >= minutes * 60.0;
    if failures.is_empty() && !done {
        return;
    }
    if failures.is_empty() && breaks == 0 {
        failures.push("the hero broke no blocks, so auto-play was not exercised".into());
    }
    if failures.is_empty() {
        let b = p.baseline.unwrap_or(s);
        eprintln!(
            "[MEMORY] PASS after {now:.0}s: private {} -> {} MiB (peak {}), images {} -> {}, font atlas sets {}",
            b.private_mib.unwrap_or(0),
            s.private_mib.unwrap_or(0),
            p.peak_private_mib,
            b.images,
            s.images,
            s.font_keys
        );
        w.write_message(AppExit::Success);
    } else {
        for f in &failures {
            eprintln!("[MEMORY] FAIL: {f}");
        }
        w.write_message(AppExit::error());
    }
}

/// Committed private bytes of this process (what Windows counts against the
/// commit limit and what ran out on 2026-11-08).
#[cfg(windows)]
fn private_bytes() -> Option<u64> {
    #[repr(C)]
    struct ProcessMemoryCountersEx {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
        private_usage: usize,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn K32GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut ProcessMemoryCountersEx,
            size: u32,
        ) -> i32;
    }
    let size = std::mem::size_of::<ProcessMemoryCountersEx>() as u32;
    // SAFETY: plain-old-data out struct; all-zero is a valid value.
    let mut counters: ProcessMemoryCountersEx = unsafe { std::mem::zeroed() };
    counters.cb = size;
    // SAFETY: the pseudo-handle needs no closing; the pointer is valid for `size`
    // bytes, and `cb` tells the API which (EX) layout it may write.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) };
    (ok != 0).then_some(counters.private_usage as u64)
}

#[cfg(not(windows))]
fn private_bytes() -> Option<u64> {
    None
}
