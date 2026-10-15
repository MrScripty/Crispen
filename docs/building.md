# Building Crispen

## Desktop launcher

Run these from the repository root:

```bash
./launcher.sh                 # Build the Svelte UI, demo, and CEF helper; launch
./launcher.sh --build         # Build without launching
./launcher.sh --release       # Build and launch in release mode
./launcher.sh --run           # Launch the existing debug binary
./launcher.sh --run --release # Launch the existing release binary
./launcher.sh --help
```

The default demo features are `ocio` and `cef`. A plain `cargo build --workspace` therefore does not exclude native color libraries or CEF.

The launcher finds the downloaded Linux CEF libraries in Cargo's build output and sets `LD_LIBRARY_PATH` and `CEF_HELPER_PATH`. A direct `cargo run` does not perform that setup or build the Svelte bundle/helper.

## Interface development

Start Vite in one terminal:

```bash
cd crates/crispen-demo/ui
npm install
npm run dev
```

From the repository root in another:

```bash
./launcher.sh --dev
```

Vite uses port 5174. The launcher sets `CRISPEN_DEV=1` so CEF loads that server. Without development mode, CEF searches for the built `crates/crispen-demo/dist/ui/index.html` or a packaged `ui/index.html` next to the executable. If it cannot find the bundle, it displays a placeholder.

## Native-UI fallback

```bash
./launcher.sh --no-cef
```

This builds the demo without its default features, then explicitly enables `ocio`. It uses the native Bevy interface and legacy WebSocket bridge; it still requires OCIO/OIIO. The launcher currently builds the Svelte bundle before this mode too.

For the framework-independent Rust core alone:

```bash
cargo test -p crispen-core
```

## Native color libraries

On Ubuntu/Debian:

```bash
sudo apt install pkg-config libopencolorio-dev libopenimageio-dev
```

The native wrappers first check explicit prebuilt directories, then `pkg-config`, then local source builds. Override the install prefixes when necessary:

```bash
export CRISPEN_OIIO_PREBUILT_DIR=/opt/oiio
export CRISPEN_OCIO_PREBUILT_DIR=/opt/ocio
```

The build scripts also accept `CRISPEN_OIIO_CMAKE_PREFIX_PATH` and `CRISPEN_OCIO_CMAKE_PREFIX_PATH` for source-build dependency discovery.

`CRISPEN_OIIO_SKIP_NATIVE_BUILD=1` and `CRISPEN_OCIO_SKIP_NATIVE_BUILD=1` skip native build/link setup for check-only work. They are not a way to produce a working desktop application.

Prefer the system-library or prebuilt route for a fresh checkout: the checked-in OpenColorIO submodule URL currently points to a maintainer-local filesystem location.

## Verification

```bash
make ci-lint  # Strict workspace Clippy
make ci-build # Full workspace build and tests
```

The GitHub workflow runs these underlying Rust checks on Ubuntu. It does not build or launch the desktop UI, so a passing Rust job alone does not verify the complete application.
