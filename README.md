# Crispen

Lightweight color grading suite resembling Davinci Resolve(TM) color page. Supports OpenColorIO and OpenFX. Can be used as a stand-alone app for fast color adjustments, or as a LIB for embedding into other software.

Serves as the color module for Studio Whip, providing optomized support for real-time grading of hybrid 3D/2D scenes.

## Prerequisites

### System Dependencies (Ubuntu/Debian)

```bash
sudo apt install libopencolorio-dev libopenimageio-dev
```

These provide the OpenColorIO (OCIO) and OpenImageIO (OIIO) libraries used for
color management and image I/O. The build scripts discover them automatically
via `pkg-config`.

### Rust

Install via [rustup](https://rustup.rs/).

## Building

```bash
# Core workspace (no OCIO/OIIO features)
cargo build --workspace

# Full build with color management and image I/O
cargo build --workspace --features crispen-demo/ocio

# Run the demo app
cargo run -p crispen-demo --features ocio
```

## Linting

```bash
make ci-lint
```

## Advanced: Overriding Library Paths

If you need a specific OIIO/OCIO version instead of system packages, the build
scripts support explicit overrides (checked before pkg-config):

```bash
# Point to a custom install prefix
export CRISPEN_OIIO_PREBUILT_DIR=/opt/oiio-2.5
export CRISPEN_OCIO_PREBUILT_DIR=/opt/ocio-2.3

# Or skip native builds entirely (check-only / lint mode)
export CRISPEN_OIIO_SKIP_NATIVE_BUILD=1
export CRISPEN_OCIO_SKIP_NATIVE_BUILD=1
```
