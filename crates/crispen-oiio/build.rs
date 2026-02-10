use std::env;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc/oiio_capi.h");
    println!("cargo:rerun-if-changed=csrc/oiio_capi.cpp");
    println!("cargo:rerun-if-env-changed=CRISPEN_OIIO_PREBUILT_DIR");
    println!("cargo:rerun-if-env-changed=CRISPEN_OIIO_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=CRISPEN_OIIO_SKIP_NATIVE_BUILD");
    println!("cargo:rerun-if-env-changed=CRISPEN_OIIO_CMAKE_PREFIX_PATH");

    if env_truthy("CRISPEN_OIIO_SKIP_NATIVE_BUILD") {
        println!(
            "cargo:warning=CRISPEN_OIIO_SKIP_NATIVE_BUILD=1: \
             skipping OpenImageIO native build (check-only mode)"
        );
        return;
    }

    if let Some(prebuilt_dir) = env_path("CRISPEN_OIIO_PREBUILT_DIR") {
        let include_dir = prebuilt_dir.join("include");
        let lib_dir = pick_lib_dir(&prebuilt_dir);
        if !include_dir.exists() || !lib_dir.exists() {
            panic!(
                "CRISPEN_OIIO_PREBUILT_DIR is missing include/lib paths: {}",
                prebuilt_dir.display()
            );
        }
        compile_wrapper(&include_dir);
        link_oiio(&lib_dir);
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let oiio_src = env_path("CRISPEN_OIIO_SOURCE_DIR")
        .unwrap_or_else(|| manifest_dir.join("../../extern/OpenImageIO"));
    if !oiio_src.exists() {
        panic!(
            "OpenImageIO source not found at {}. \
             Set CRISPEN_OIIO_SOURCE_DIR or CRISPEN_OIIO_PREBUILT_DIR.",
            oiio_src.display()
        );
    }

    let mut cmake_cfg = cmake::Config::new(&oiio_src);
    cmake_cfg
        .define("BUILD_SHARED_LIBS", "ON")
        .define("OIIO_BUILD_TOOLS", "OFF")
        .define("OIIO_BUILD_TESTS", "OFF")
        .define("USE_PYTHON", "OFF")
        .define("BUILD_DOCS", "OFF")
        .define("INSTALL_DOCS", "OFF")
        .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON");

    if let Some(prefix_path) = env_string("CRISPEN_OIIO_CMAKE_PREFIX_PATH") {
        cmake_cfg.define("CMAKE_PREFIX_PATH", prefix_path);
    }

    let oiio_dst = cmake_cfg.build();

    compile_wrapper(&oiio_dst.join("include"));
    link_oiio(&pick_lib_dir(&oiio_dst));
}

fn compile_wrapper(oiio_include: &Path) {
    cc::Build::new()
        .cpp(true)
        .file("csrc/oiio_capi.cpp")
        .include("csrc")
        .include(oiio_include)
        .flag_if_supported("-std=c++17")
        .compile("oiio_capi");
}

fn link_oiio(lib_dir: &Path) {
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=OpenImageIO");

    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    } else if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    }
}

fn env_path(var: &str) -> Option<PathBuf> {
    env::var(var)
        .ok()
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn env_string(var: &str) -> Option<String> {
    env::var(var).ok().filter(|v| !v.is_empty())
}

fn env_truthy(var: &str) -> bool {
    matches!(
        env::var(var).ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

fn pick_lib_dir(root: &Path) -> PathBuf {
    let lib = root.join("lib");
    if lib.exists() {
        return lib;
    }
    let lib64 = root.join("lib64");
    if lib64.exists() {
        return lib64;
    }
    lib
}
