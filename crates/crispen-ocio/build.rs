use std::env;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=csrc/ocio_capi.h");
    println!("cargo:rerun-if-changed=csrc/ocio_capi.cpp");
    println!("cargo:rerun-if-env-changed=CRISPEN_OCIO_PREBUILT_DIR");
    println!("cargo:rerun-if-env-changed=CRISPEN_OCIO_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=CRISPEN_OCIO_INSTALL_EXT_PACKAGES");
    println!("cargo:rerun-if-env-changed=CRISPEN_OCIO_CMAKE_PREFIX_PATH");
    println!("cargo:rerun-if-env-changed=CRISPEN_OCIO_MINIZIP_NG_ROOT");
    println!("cargo:rerun-if-env-changed=CRISPEN_OCIO_MINIZIP_NG_DIR");
    println!("cargo:rerun-if-env-changed=CRISPEN_OCIO_MINIZIP_NG_LIBRARY");
    println!("cargo:rerun-if-env-changed=CRISPEN_OCIO_MINIZIP_NG_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=CRISPEN_OCIO_ZLIB_ROOT");
    println!("cargo:rerun-if-env-changed=CRISPEN_OCIO_ZLIB_LIBRARY");
    println!("cargo:rerun-if-env-changed=CRISPEN_OCIO_ZLIB_INCLUDE_DIR");

    if let Some(prebuilt_dir) = env_path("CRISPEN_OCIO_PREBUILT_DIR") {
        let include_dir = prebuilt_dir.join("include");
        let lib_dir = pick_lib_dir(&prebuilt_dir);
        if !include_dir.exists() || !lib_dir.exists() {
            panic!(
                "CRISPEN_OCIO_PREBUILT_DIR is missing include/lib paths: {}",
                prebuilt_dir.display()
            );
        }
        compile_wrapper(&include_dir);
        link_ocio(&lib_dir);
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let ocio_src = env_path("CRISPEN_OCIO_SOURCE_DIR")
        .unwrap_or_else(|| manifest_dir.join("../../extern/OpenColorIO"));
    if !ocio_src.exists() {
        panic!(
            "OpenColorIO source not found at {}. Initialize submodules.",
            ocio_src.display()
        );
    }

    let install_ext = env::var("CRISPEN_OCIO_INSTALL_EXT_PACKAGES")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "MISSING".to_string());

    let mut cmake_cfg = cmake::Config::new(&ocio_src);
    cmake_cfg
        .define("BUILD_SHARED_LIBS", "ON")
        .define("OCIO_BUILD_APPS", "OFF")
        .define("OCIO_BUILD_TESTS", "OFF")
        .define("OCIO_BUILD_GPU_TESTS", "OFF")
        .define("OCIO_BUILD_PYTHON", "OFF")
        .define("OCIO_BUILD_JAVA", "OFF")
        .define("OCIO_BUILD_DOCS", "OFF")
        .define("OCIO_INSTALL_EXT_PACKAGES", &install_ext)
        .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON");
    apply_dependency_hints(&mut cmake_cfg);

    let ocio_dst = cmake_cfg.build();

    compile_wrapper(&ocio_dst.join("include"));
    link_ocio(&pick_lib_dir(&ocio_dst));
}

fn compile_wrapper(ocio_include: &Path) {
    cc::Build::new()
        .cpp(true)
        .file("csrc/ocio_capi.cpp")
        .include("csrc")
        .include(ocio_include)
        .flag_if_supported("-std=c++17")
        .compile("ocio_capi");
}

fn link_ocio(lib_dir: &Path) {
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=OpenColorIO");

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

fn apply_dependency_hints(cfg: &mut cmake::Config) {
    if let Some(prefix_path) = env_string("CRISPEN_OCIO_CMAKE_PREFIX_PATH") {
        cfg.define("CMAKE_PREFIX_PATH", prefix_path);
    }

    if let Some(path) = env_path("CRISPEN_OCIO_MINIZIP_NG_ROOT") {
        cfg.define("minizip-ng_ROOT", path);
    }
    if let Some(path) = env_path("CRISPEN_OCIO_MINIZIP_NG_DIR") {
        cfg.define("minizip-ng_DIR", path);
    }
    if let Some(path) = env_path("CRISPEN_OCIO_MINIZIP_NG_LIBRARY") {
        cfg.define("minizip-ng_LIBRARY", path);
    }
    if let Some(path) = env_path("CRISPEN_OCIO_MINIZIP_NG_INCLUDE_DIR") {
        cfg.define("minizip-ng_INCLUDE_DIR", path);
    }

    if let Some(path) = env_path("CRISPEN_OCIO_ZLIB_ROOT") {
        cfg.define("ZLIB_ROOT", path);
    }
    if let Some(path) = env_path("CRISPEN_OCIO_ZLIB_LIBRARY") {
        cfg.define("ZLIB_LIBRARY", path);
    }
    if let Some(path) = env_path("CRISPEN_OCIO_ZLIB_INCLUDE_DIR") {
        cfg.define("ZLIB_INCLUDE_DIR", path);
    }
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
