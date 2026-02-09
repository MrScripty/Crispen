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

## Notes

- On Linux, the wrapper links against `stdc++`.
- OCIO support is enabled from downstream crates via cargo feature `ocio`.
