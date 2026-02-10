# Crispen

Lightweight color grading suite resembling Davinci Resolve(TM) color page. Supports OpenColorIO and OpenFX. Can be used as a stand-alone app for fast color adjustments, or as a LIB for embedding into other software. 

Serves as the color module for Studio Whip, providing optomized support for real-time grading of hybrid 3D/2D scenes.

## CI: Prebuilt OCIO Setup

The GitHub Actions workflow includes a `build-ocio-prebuilt` job that builds
the demo with `--features ocio` using a prebuilt OCIO install path.

Required repository variable:

- `CRISPEN_OCIO_PREBUILT_DIR`
  - Set this in GitHub: `Settings -> Secrets and variables -> Actions -> Variables`.
  - Value must be an absolute path on the CI runner to an OCIO install prefix.
  - The path must contain:
    - `include/`
    - `lib/` or `lib64/`

If this variable is unset, the `build-ocio-prebuilt` job is skipped.

Local equivalent:

```bash
make ci-build-ocio PREBUILT_OCIO_DIR=/path/to/opencolorio-install
```
