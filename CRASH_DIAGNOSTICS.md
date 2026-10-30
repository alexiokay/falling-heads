# Local crash diagnostics

Use the **MineHeads - Play** desktop shortcut. Every launch creates a separate
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
records a MineHeads crash on 24 October at 04:12:22, build
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
it on an existing player session. `tools/Run-MineHeadsSession.ps1` provides the same
monitor used by the launcher for these tests.

Water visual regression: `tools/Test-MineHeadsOverhaul.ps1 -SkipBuild -Scene water`.
It compares fifteen dry/partially submerged/submerged faces at three sizes, two
camera positions, and raster, DLSS and ray-traced renderer modes.


## Native startup observation — 27 October 2026

Two new water-test launches and the previous published binary failed before loading the scene while a user game was open. Windows Application event 1000 identified `nvoglv64.dll` 32.0.16.1742, exception `0xc0000005`, offset `0xB857CF` for all three. The previous build also faulted and remained in crash handling until the owned test timeout. Evidence: `target/water-driver-startup-events.json`, `target/water-startup-driver-crash.log`, `target/water-old-baseline-err.log`. After the user closed the game, the new build completed all five water captures (raster, DLSS, RT) and shut down cleanly. This correlates the startup failures with simultaneous instances, but does not establish the root cause inside the driver/native initialization. It is distinct from the earlier historical 0xC0000409 report. No driver, registry, or pagefile settings were changed.
