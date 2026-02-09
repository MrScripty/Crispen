# crispen-ocio

Feature-gated OpenColorIO integration for Crispen.

## Build Inputs

`crispen-ocio` supports two build modes:

1. Source build (default)
- Builds OpenColorIO from `extern/OpenColorIO` (or `CRISPEN_OCIO_SOURCE_DIR`).
- Dependency install behavior is controlled by `CRISPEN_OCIO_INSTALL_EXT_PACKAGES`.

2. Prebuilt link mode
- Set `CRISPEN_OCIO_PREBUILT_DIR` to an OCIO install prefix containing `include/` and `lib/` (or `lib64/`).
- This skips building OCIO and only builds the C wrapper.

## Environment Variables

- `CRISPEN_OCIO_PREBUILT_DIR`
  - Example: `/opt/opencolorio`
  - If set, source build is skipped.

- `CRISPEN_OCIO_SOURCE_DIR`
  - Override source path for OCIO CMake build.
  - Default: `../../extern/OpenColorIO` relative to this crate.

- `CRISPEN_OCIO_INSTALL_EXT_PACKAGES`
  - Passed through to OCIO CMake option `OCIO_INSTALL_EXT_PACKAGES`.
  - Typical values: `MISSING`, `ALL`, `NONE`.
  - Default: `MISSING`.

- `CRISPEN_OCIO_CMAKE_PREFIX_PATH`
  - Optional CMake prefix path list to help `find_package` resolve dependencies.

- `CRISPEN_OCIO_MINIZIP_NG_ROOT`
- `CRISPEN_OCIO_MINIZIP_NG_DIR`
- `CRISPEN_OCIO_MINIZIP_NG_LIBRARY`
- `CRISPEN_OCIO_MINIZIP_NG_INCLUDE_DIR`
  - Optional hints passed to OCIO CMake as `minizip-ng_*` variables.
  - Useful on distros that do not ship a `minizip-ng` development package.

- `CRISPEN_OCIO_ZLIB_ROOT`
- `CRISPEN_OCIO_ZLIB_LIBRARY`
- `CRISPEN_OCIO_ZLIB_INCLUDE_DIR`
  - Optional zlib hints passed through as `ZLIB_*` CMake variables.
  - Useful when pairing a custom `minizip-ng` build with a non-default zlib.

## Notes

- On Linux, the wrapper links against `stdc++`.
- OCIO support is enabled from downstream crates via cargo feature `ocio`.

## Linux Distros Without `minizip-ng` Packages

Some distros provide only legacy `minizip` packages (`libminizip-dev`) and not
`minizip-ng`. In that case, either:

1. Use `CRISPEN_OCIO_PREBUILT_DIR` with a prebuilt OCIO install, or
2. Build/install `minizip-ng` yourself and point this crate at it using:
   - `CRISPEN_OCIO_MINIZIP_NG_ROOT` (or `*_DIR`/`*_LIBRARY`/`*_INCLUDE_DIR`)
   - `CRISPEN_OCIO_ZLIB_ROOT`/`CRISPEN_OCIO_ZLIB_*` if needed
