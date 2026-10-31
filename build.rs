fn main() {
    println!("cargo:rerun-if-changed=native/streamline.cpp");
    println!("cargo:rerun-if-changed=native/streamline.h");
    println!("cargo:rerun-if-env-changed=VULKAN_SDK");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    // .cargo/config.toml supplies a default only when the variable is not
    // already set, so another machine can point VULKAN_SDK at its own SDK.
    let Ok(vulkan_sdk) = std::env::var("VULKAN_SDK") else {
        panic!(
            "VULKAN_SDK is not set. Point it at a folder containing the Vulkan headers \
             (include/vulkan/vulkan.h), e.g. a LunarG Vulkan SDK install or a Vulkan-Headers \
             checkout. DLSS_SDK and LIBCLANG_PATH are needed too; see README.md (Building)."
        );
    };
    let include = std::path::Path::new(&vulkan_sdk).join("include");
    if !include.join("vulkan").join("vulkan.h").exists() {
        panic!("VULKAN_SDK={vulkan_sdk} has no include/vulkan/vulkan.h. See README.md (Building).");
    }
    cc::Build::new()
        .cpp(true)
        .file("native/streamline.cpp")
        .include("vendor/Streamline-2.14.16/include")
        .include(include)
        .flag("/std:c++17")
        .flag("/EHsc")
        .define("NOMINMAX", None)
        .compile("falling_heads_streamline");
}
