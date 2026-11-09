# Local crash diagnostics

Use the **Falling Heads - Play** desktop shortcut. Every launch creates a separate
`logs/session-...` directory in the game folder. Nothing is uploaded.

- `launcher.json`: exact build/hash, process ID, start/end time and Windows exit code.
- `stderr.log` / `stdout.log`: captured engine/native-library messages, including
  allocation failures and GPU errors that may bypass Rust's panic hook.
- `runtime.log` / `runtime.previous.log`: engine tracing, rotating at approximately
  8 MiB per file; these also work when launching the executable directly.
- `panic.txt`: Rust panic message, thread and forced backtrace, if a panic occurred.
- `last-state.json`: most recent game state, seed, hero position, actor/block counts,
  renderer settings and weather, sampled every ten seconds.
- `memory.jsonl`: game private/working-set bytes and system memory headroom,
  sampled by the desktop launcher every ten seconds.
- `app-exit.json`: written when the game returns normally from its event loop.
- `windows-events.txt`: matching recent Windows Application errors when available.

For diagnosis, use the latest session folder; a missing `panic.txt` does **not**
mean there was no crash. Native aborts, memory exhaustion, driver failures and
external termination can bypass a Rust panic. A hard kill or power loss can also
prevent the launcher from recording the final exit status. Reports are diagnostic
evidence, not automatic crash repair. No memory dumps or Windows registry changes
are enabled. Direct executable launches do not have the launcher's stderr/exit
monitor, so the shortcut provides the most complete record.

Matching PDB symbols are retained beside published builds so later recompilation
does not invalidate their backtraces. Logs are retained until manually removed;
stdout/stderr and session history are not automatically pruned.

## Existing evidence inspected on 26 October 2026

The old shortcut did not redirect console output. Windows Application event 1000
records a Falling Heads crash on 24 October at 04:12:22, build
`20261024-035006-4bb3b7c7b7`, exception `0xc0000409`, offset `0x5b0a821`.
That exit code alone does not identify the cause. No matching game dump was found
in the user's CrashDumps folder.

During this investigation rustc independently failed a memory allocation and
exited with the same code. The configured 4 GiB page file was fully used. This is
evidence of machine memory pressure, **not proof** of the earlier game's cause.
New reports include memory headroom to help distinguish this from engine/GPU bugs.

## Developer verification

`--diagnostics-test-clean` and `--diagnostics-test-panic` run before graphics/audio
initialization. The latter deliberately panics in an isolated process. Never use
it on an existing player session. `tools/Run-FallingHeadsSession.ps1` provides the same
monitor used by the launcher for these tests.

Water visual regression: `tools/Test-FallingHeadsOverhaul.ps1 -SkipBuild -Scene water`.
It compares fifteen dry/partially submerged/submerged faces at three sizes, two
camera positions, and raster, DLSS and ray-traced renderer modes.


## Native startup observation — 27 October 2026

Two new water-test launches and the previous published binary failed before loading the scene while a user game was open. Windows Application event 1000 identified `nvoglv64.dll` 32.0.16.1742, exception `0xc0000005`, offset `0xB857CF` for all three. The previous build also faulted and remained in crash handling until the owned test timeout. Evidence: `target/water-driver-startup-events.json`, `target/water-startup-driver-crash.log`, `target/water-old-baseline-err.log`. After the user closed the game, the new build completed all five water captures (raster, DLSS, RT) and shut down cleanly. This correlates the startup failures with simultaneous instances, but does not establish the root cause inside the driver/native initialization. It is distinct from the earlier historical 0xC0000409 report. No driver, registry, or pagefile settings were changed.

## Out-of-memory during unattended play — 8 November 2026

Build `20261105-001609-c0254780bb` grew about 2 GB of private bytes per minute while
playing itself (OBS window capture, free pack) and failed an allocation at 23.7 GB after
11.5 minutes (`memory allocation of 1048576 bytes failed`, exit `0xc0000409`), which
froze other applications. Entities, meshes and materials stayed flat; image assets rose
by roughly nine 1 MiB (512x512) textures per second. Cause: the combo HUD
(`src/systems/combos.rs`, `combo_hud`) animated `TextFont::font_size` every frame
during the break pulse. Bevy rasterizes a separate glyph atlas for every exact font size
and never frees it. The leak reproduced with frame generation off, DLSS off and without
the Streamline DLLs, so it was independent of the NVIDIA runtimes.

Fix (build `20261109-084543-ac3fd2b548`): the HUD uses a few whole-pixel sizes and
animates the pulse with `UiTransform` scale. Same conditions afterwards: flat at about
1.9 GB over three minutes (previously 6.6 GB).

Regression check: `tools/Test-FallingHeadsMemory.ps1 [-Executable ...] [-Minutes 5] [-Pack free]`
runs `--memory-probe`. After a one-minute warm-up it fails on more than 48 glyph atlas
sets, on more than 64 extra images or on more than 1 GiB extra private bytes. The old
code fails within ten seconds. The performance log also prints a `Memory:` line
(images, CPU image bytes, glyph atlas sets) every ten seconds. Never animate a text's
font size; scale the node or `Transform` instead. The `heap-stats` cargo feature adds the
live Rust heap size to that line for diagnosis.
