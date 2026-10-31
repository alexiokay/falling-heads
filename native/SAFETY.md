# Native renderer boundary

The production renderer is isolated in `src/systems/native_gpu.rs`, the frame-generation bridge in `src/systems/frame_generation.rs`, `native/streamline.{h,cpp}`, and the patched crates recorded in Cargo.toml/Cargo.lock.

## Contracts checked in this pass

- Rust `repr(C)` types and C++ fixed-width POD structures agree: status 28 bytes, texture 40, inputs 456, projection offset 120, reset offset 448. Both compilers assert these values. SDK structures never cross this ABI.
- Initialization passes live, NUL-terminated UTF-16 paths. The bridge verifies/loads the interposer before Vulkan initialization; the module deliberately remains process-lived because ash retains function pointers.
- `Copies` owns color/depth/motion snapshots and views. Raw guards are released before acquiring another guard to avoid wgpu's non-recursive snatch lock. GPU submissions precede tagging on the same render queue; tags are valid through present.
- The C call consumes the stack `NativeInputs` synchronously. GPU texture ownership survives it. Resize preflight suspends FG before surface recreation and skips two frames; camera cuts reset temporal history.
- The main-thread simulation token index is extracted into the render world. Token-ring access is mutex-protected; counters/support state are atomic. RenderGraph/present ordering owns the current rendering token and enabled state.
- Shutdown uses an atomic exchange and is idempotent. It runs before device destruction, with the interposer still loaded. The smoke test requires a successful Streamline shutdown.

These checks document the application's ownership and ordering assumptions; they do not prove correctness of NVIDIA drivers or every vendored unsafe implementation. Long-running resize/device-loss/concurrent-instance faults remain a separate diagnostic investigation. Do not remove owners, unload the interposer early, move callbacks to unrelated schedules, or add broad `unsafe` allowances to silence errors.

Run `tools/Test-FallingHeadsFG.ps1` for the hidden pause/resume/resize/quality/enable-disable verification. Run water and crowded raster/RT scenes separately; compilation or another game instance makes performance measurements unreliable.
