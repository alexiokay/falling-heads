// MineHeads' narrow C ABI around the version-pinned NVIDIA Streamline C++ API.
// No SDK structures cross the Rust/C++ boundary. The verified module is process-lived.
#include "streamline.h"
#include <windows.h>
#include <vulkan/vulkan.h>
#include <sl.h>
#include <sl_dlss_g.h>
#include <sl_reflex.h>
#include <sl_pcl.h>
#include <sl_security.h>
#include <atomic>
#include <cstdio>
#include <string>
#include <mutex>
#include <array>
#include <cstring>

namespace {
HMODULE module{};
std::atomic<bool> initialized{false};
PFun_slShutdown* shutdownFn{};
PFun_slIsFeatureSupported* supportedFn{};
PFun_slGetFeatureFunction* featureFn{};
PFun_slDLSSGGetState* stateFn{};
PFun_slDLSSGSetOptions* optionsFn{};
PFun_slReflexSetOptions* reflexOptionsFn{};
PFun_slReflexGetState* reflexStateFn{};
PFun_slReflexSleep* sleepFn{};
PFun_slPCLSetMarker* markerFn{};
PFun_slGetNewFrameToken* tokenFn{};
PFun_slSetConstants* constantsFn{};
PFun_slSetTagForFrame* tagsFn{};
std::atomic<uint32_t> rendered{0}, presented{0}, generatedMax{0}, latestStatus{0}, minimumSize{0};
std::atomic<int32_t> latestResult{0};
std::atomic<bool> supported{false};
std::array<sl::FrameToken*, 16> tokens{};
std::array<uint32_t, 16> indices{};
std::mutex tokenMutex;
// RenderGraph systems may run on workers; present runs on the render thread.
// These callbacks are ordered by the render schedule and share its one current token.
sl::FrameToken* renderingToken{};
bool enabled{};
sl::FrameToken* findToken(uint32_t frame) {
    std::lock_guard<std::mutex> lock(tokenMutex);
    return indices[frame % tokens.size()] == frame ? tokens[frame % tokens.size()] : nullptr;
}
void mark(sl::PCLMarker marker, sl::FrameToken* token) {
    if (initialized && markerFn && token) markerFn(marker,*token);
}
int32_t setEnabled(bool value) {
    if (value == enabled) return 0;
    sl::DLSSGOptions options{};
    options.mode = value ? sl::DLSSGMode::eOn : sl::DLSSGMode::eOff;
    options.numFramesToGenerate = 1;
    options.flags = sl::DLSSGFlags::eRetainResourcesWhenOff;
    options.queueParallelismMode = sl::DLSSGQueueParallelismMode::eBlockPresentingClientQueue;
    const auto result = optionsFn(sl::ViewportHandle(0),options);
    if (result != sl::Result::eOk) latestResult = int32_t(result);
    if (result == sl::Result::eOk) enabled = value;
    fprintf(stderr,"[FG] enabled=%d result=%d\n",value,int(result));
    return int32_t(result);
}
void message(sl::LogType type, const char* text) { if (type == sl::LogType::eInfo) return; fprintf(stderr, "[Streamline %u] %s\n", unsigned(type), text); }
template<class T> bool api(T*& out, const char* name) {
    out = reinterpret_cast<T*>(GetProcAddress(module, name));
    return out != nullptr;
}
template<class T> bool feature(T*& out, sl::Feature f, const char* name) {
    void* p{};
    const auto result = featureFn(f, name, p);
    out = reinterpret_cast<T*>(p);
    if (result != sl::Result::eOk) fprintf(stderr,"[FG] %s failed: %d\n", name, int(result));
    return result == sl::Result::eOk && out;
}
}
extern "C" void* mh_fg_init(const wchar_t* plugins, const wchar_t* logs) {
    try {
        const auto path = std::wstring(plugins) + L"\\sl.interposer.dll";
        if (!sl::security::verifyEmbeddedSignature(path.c_str())) {
            fprintf(stderr,"[FG] NVIDIA interposer signature verification failed\n"); return nullptr;
        }
        module = LoadLibraryExW(path.c_str(), nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);
        if (!module) { fprintf(stderr,"[FG] LoadLibrary failed: %lu\n",GetLastError()); return nullptr; }
        PFun_slInit* init{};
        if (!api(init,"slInit") || !api(shutdownFn,"slShutdown") || !api(supportedFn,"slIsFeatureSupported") ||
            !api(featureFn,"slGetFeatureFunction") || !api(tokenFn,"slGetNewFrameToken") ||
            !api(constantsFn,"slSetConstants") || !api(tagsFn,"slSetTagForFrame")) return nullptr;
        sl::Preferences pref{};
        const sl::Feature features[] = { sl::kFeatureDLSS_G, sl::kFeatureReflex, sl::kFeaturePCL };
        pref.featuresToLoad = features; pref.numFeaturesToLoad = 3;
        pref.pathsToPlugins = &plugins; pref.numPathsToPlugins = 1;
        pref.pathToLogsAndData = logs;
        pref.engine = sl::EngineType::eCustom; pref.engineVersion = "Bevy 0.20.0 / MineHeads 0.2.5";
        pref.projectId = "2a6ed339-f7f5-4a33-940a-5e7e9adc6137";
        pref.renderAPI = sl::RenderAPI::eVulkan;
        // Use the verified, packaged versions for reproducible validation, not OTA plugins.
        pref.flags = sl::PreferenceFlags::eDisableCLStateTracking | sl::PreferenceFlags::eUseFrameBasedResourceTagging;
        pref.logLevel = sl::LogLevel::eDefault; pref.logMessageCallback = message;
        const auto result = init(pref, sl::kSDKVersion);
        fprintf(stderr,"[FG] slInit result=%d\n",int(result));
        if (result != sl::Result::eOk) return nullptr;
        initialized = true;
        auto gipa = GetProcAddress(module,"vkGetInstanceProcAddr");
        if (!gipa) { mh_fg_shutdown(); return nullptr; }
        return reinterpret_cast<void*>(gipa);
    } catch (...) { fprintf(stderr,"[FG] Native initialization exception\n"); return nullptr; }
}
extern "C" int32_t mh_fg_probe(void* physicalDevice, MhFgStatus* out) {
    if (!initialized || !out) return -1;
    sl::AdapterInfo adapter{}; adapter.vkPhysicalDevice = physicalDevice;
    auto result = supportedFn(sl::kFeatureDLSS_G, adapter);
    *out = {}; out->result = int32_t(result);
    if (result != sl::Result::eOk) return out->result;
    if (!feature(stateFn,sl::kFeatureDLSS_G,"slDLSSGGetState") ||
        !feature(optionsFn,sl::kFeatureDLSS_G,"slDLSSGSetOptions") ||
        !feature(reflexOptionsFn,sl::kFeatureReflex,"slReflexSetOptions") ||
        !feature(reflexStateFn,sl::kFeatureReflex,"slReflexGetState") ||
        !feature(sleepFn,sl::kFeatureReflex,"slReflexSleep") ||
        !feature(markerFn,sl::kFeaturePCL,"slPCLSetMarker")) return out->result = -2;
    sl::DLSSGState state{};
    result = stateFn(sl::ViewportHandle(0), state, nullptr);
    out->result = int32_t(result);
    out->maximum = state.numFramesToGenerateMax;
    out->minimum = state.minWidthOrHeight;
    out->presented = state.numFramesActuallyPresented;
    out->status = uint32_t(state.status);
    sl::ReflexState reflex{};
    const auto rr = reflexStateFn(reflex);
    out->supported = result == sl::Result::eOk && rr == sl::Result::eOk && reflex.lowLatencyAvailable;
    fprintf(stderr,"[FG] capability result=%d supported=%u maxGenerated=%u minSize=%u reflex=%d/%d\n",
        int(result),out->supported,out->maximum,out->minimum,int(rr),reflex.lowLatencyAvailable);
    supported = out->supported != 0;
    generatedMax = out->maximum; minimumSize = out->minimum;
    return out->result;
}
extern "C" void mh_fg_shutdown() {
    if (initialized.exchange(false)) {
        const auto result = shutdownFn();
        fprintf(stderr,"[FG] slShutdown result=%d\n",int(result));
    }
    // Keep loaded: ash still owns pointers to the Vulkan forwarding functions.
}

extern "C" int32_t mh_fg_begin(uint32_t frame, uint32_t limitUs) {
    if (!initialized || !supported) return -1;
    sl::FrameToken* token{};
    auto result = tokenFn(token,&frame);
    if (result != sl::Result::eOk) { latestResult=int32_t(result); return int32_t(result); }
    {
        std::lock_guard<std::mutex> lock(tokenMutex);
        tokens[frame % tokens.size()] = token; indices[frame % tokens.size()] = frame;
    }
    sl::ReflexOptions options{}; options.mode = sl::ReflexMode::eLowLatency; options.frameLimitUs = limitUs;
    result = reflexOptionsFn(options);
    if (result != sl::Result::eOk) { latestResult=int32_t(result); return int32_t(result); }
    result = sleepFn(*token);
    if (result != sl::Result::eOk) { latestResult=int32_t(result); return int32_t(result); }
    mark(sl::PCLMarker::eSimulationStart,token);
    return 0;
}
extern "C" void mh_fg_simulation_end(uint32_t frame) { mark(sl::PCLMarker::eSimulationEnd,findToken(frame)); }
extern "C" void mh_fg_render_begin(uint32_t frame) {
    renderingToken = findToken(frame); mark(sl::PCLMarker::eRenderSubmitStart,renderingToken);
}
extern "C" int32_t mh_fg_render_end(uint32_t frame, const MhFgInputs* inputs) {
    if (!initialized || !supported) return -1;
    renderingToken = findToken(frame);
    int32_t result{};
    if (!inputs || !renderingToken) { result = setEnabled(false); }
    else {
        auto resource = [](const MhFgTexture& t) {
            sl::Resource r(sl::ResourceType::eTex2d, reinterpret_cast<void*>(t.image), nullptr,
                reinterpret_cast<void*>(t.view), t.layout);
            r.width=t.width; r.height=t.height; r.nativeFormat=t.format; r.usage=t.usage;
            r.mipLevels=1; r.arrayLayers=1; r.flags=0;
            return r;
        };
        auto color=resource(inputs->color), depth=resource(inputs->depth), motion=resource(inputs->motion);
        sl::Extent colorExtent{0,0,color.width,color.height}, depthExtent{0,0,depth.width,depth.height};
        sl::ResourceTag tags[] = {
            {&color,sl::kBufferTypeHUDLessColor,sl::eValidUntilPresent,&colorExtent},
            {nullptr,sl::kBufferTypeBackbuffer,sl::eValidUntilPresent,&colorExtent},
            {&depth,sl::kBufferTypeDepth,sl::eValidUntilPresent,&depthExtent},
            {&motion,sl::kBufferTypeMotionVectors,sl::eValidUntilPresent,&depthExtent},
        };
        sl::Constants c{};
        for (int r=0;r<4;++r) c.clipToLensClip.row[r]=sl::float4(r==0?1.f:0.f,r==1?1.f:0.f,r==2?1.f:0.f,r==3?1.f:0.f);
        c.cameraPinholeOffset={0.f,0.f};
        memcpy(&c.cameraViewToClip,inputs->projection,64); memcpy(&c.clipToCameraView,inputs->inverseProjection,64);
        memcpy(&c.clipToPrevClip,inputs->clipToPrevious,64); memcpy(&c.prevClipToClip,inputs->previousToClip,64);
        memcpy(&c.cameraPos,inputs->position,12); memcpy(&c.cameraUp,inputs->up,12);
        memcpy(&c.cameraRight,inputs->right,12); memcpy(&c.cameraFwd,inputs->forward,12);
        c.jitterOffset={inputs->jitter[0],inputs->jitter[1]}; c.mvecScale={-1.f,-1.f};
        c.cameraNear=inputs->nearPlane; c.cameraFar=inputs->farPlane; c.cameraFOV=inputs->fov; c.cameraAspectRatio=inputs->aspect;
        c.depthInverted=sl::eTrue; c.cameraMotionIncluded=sl::eTrue; c.motionVectors3D=sl::eFalse;
        c.reset=inputs->reset ? sl::eTrue : sl::eFalse;
        c.orthographicProjection=inputs->orthographic ? sl::eTrue : sl::eFalse;
        c.motionVectorsDilated=sl::eFalse; c.motionVectorsJittered=sl::eFalse;
        result=int32_t(constantsFn(c,*renderingToken,sl::ViewportHandle(0)));
        if (!result) result=int32_t(tagsFn(*renderingToken,sl::ViewportHandle(0),tags,4,nullptr));
        if (!result) result=setEnabled(true);
        if (result) { latestResult=result; setEnabled(false); fprintf(stderr,"[FG] input error=%d\n",result); }
    }
    mark(sl::PCLMarker::eRenderSubmitEnd,renderingToken);
    return result;
}
extern "C" void mh_fg_before_present() { mark(sl::PCLMarker::ePresentStart,renderingToken); }
extern "C" void mh_fg_after_present(int32_t result) {
    mark(sl::PCLMarker::ePresentEnd,renderingToken);
    if (!initialized || !supported) return;
    sl::DLSSGState state{};
    const auto r=stateFn(sl::ViewportHandle(0),state,nullptr);
    // Preserve an earlier input/pacing error; a successful query cannot erase it.
    if (r!=sl::Result::eOk) latestResult=int32_t(r);
    if (result < 0) latestResult=result;
    latestStatus=uint32_t(state.status);
    if (!result && r==sl::Result::eOk) { presented+=state.numFramesActuallyPresented; ++rendered; }
    static uint32_t count{};
    if (++count % 60 == 0) fprintf(stderr,"[FG PRESENT] enabled=%d actual=%u status=%u result=%d total=%u\n",
        enabled,state.numFramesActuallyPresented,uint32_t(state.status),int(r),presented.load());
}
extern "C" void mh_fg_stats(MhFgStatus* out) {
    if (out) *out={latestResult.load(),uint32_t(supported.load()),generatedMax.load(),presented.load(),latestStatus.load(),minimumSize.load(),rendered.load()};
}


extern "C" void mh_fg_suspend() { if (initialized && supported) setEnabled(false); }
