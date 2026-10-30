#pragma once
#include <stdint.h>
struct MhFgStatus { int32_t result; uint32_t supported, maximum, presented, status, minimum, rendered; };
extern "C" {
void* mh_fg_init(const wchar_t* plugins, const wchar_t* logs);
int32_t mh_fg_probe(void* physicalDevice, MhFgStatus* out);
void mh_fg_shutdown();
void mh_fg_before_present();
void mh_fg_after_present(int32_t result);
}

struct MhFgTexture { uint64_t image, view; uint32_t width, height, format, usage, layout; };
struct MhFgInputs {
    MhFgTexture color, depth, motion;
    float projection[16], inverseProjection[16], clipToPrevious[16], previousToClip[16];
    float position[3], up[3], right[3], forward[3], jitter[2];
    float nearPlane, farPlane, fov, aspect;
    uint32_t reset, orthographic;
};
extern "C" {
int32_t mh_fg_begin(uint32_t frame, uint32_t limitUs);
void mh_fg_simulation_end(uint32_t frame);
void mh_fg_render_begin(uint32_t frame);
int32_t mh_fg_render_end(uint32_t frame, const MhFgInputs* inputs);
void mh_fg_stats(MhFgStatus* out);
}

// Fixed-width POD layout mirrored by Rust repr(C) types. Fail the native build
// instead of silently reading wrong texture handles or camera data after edits.
#include <stddef.h>
static_assert(sizeof(MhFgStatus) == 28);
static_assert(sizeof(MhFgTexture) == 40);
static_assert(sizeof(MhFgInputs) == 456);
static_assert(offsetof(MhFgInputs, projection) == 120);
static_assert(offsetof(MhFgInputs, reset) == 448);
extern "C" void mh_fg_suspend();
