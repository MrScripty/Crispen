use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc/ocio_capi.h");
    println!("cargo:rerun-if-changed=csrc/ocio_capi.cpp");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let ocio_src = manifest_dir.join("../../extern/OpenColorIO");

    if !ocio_src.exists() {
        panic!(
            "OpenColorIO source not found at {}. Initialize submodules.",
            ocio_src.display()
        );
    }

    let ocio_dst = cmake::Config::new(&ocio_src)
        .define("BUILD_SHARED_LIBS", "ON")
        .define("OCIO_BUILD_APPS", "OFF")
        .define("OCIO_BUILD_TESTS", "OFF")
        .define("OCIO_BUILD_GPU_TESTS", "OFF")
        .define("OCIO_BUILD_PYTHON", "OFF")
        .define("OCIO_BUILD_JAVA", "OFF")
        .define("OCIO_BUILD_DOCS", "OFF")
        .define("OCIO_INSTALL_EXT_PACKAGES", "MISSING")
        .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
        .build();

    cc::Build::new()
        .cpp(true)
        .file("csrc/ocio_capi.cpp")
        .include("csrc")
        .include(ocio_dst.join("include"))
        .flag_if_supported("-std=c++17")
        .compile("ocio_capi");

    println!(
        "cargo:rustc-link-search=native={}",
        ocio_dst.join("lib").display()
    );
    println!("cargo:rustc-link-lib=OpenColorIO");

    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    } else if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    }
}
