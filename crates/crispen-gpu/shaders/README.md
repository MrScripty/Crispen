# Shaders

## Purpose

WGSL compute shaders for the GPU grading pipeline. Each shader handles one stage of the pipeline: LUT baking, LUT application, spatial processing, or scope computation.

## Contents

| File | Description |
|------|-------------|
| `bake_lut.wgsl` | Bakes `GradingParams` into a 65³ 3D LUT — mirrors `evaluate_transform()` exactly |
| `apply_lut.wgsl` | Applies 3D LUT to source image via hardware trilinear sampling |
| `midtone_detail.wgsl` | Spatial midtone detail enhancement (separate pass, only when `midtone_detail != 0`) |
| `histogram.wgsl` | Computes 256-bin RGBL histogram using atomic increments |
| `waveform.wgsl` | Computes intensity-vs-position waveform density using atomic increments |
| `vectorscope.wgsl` | Computes Cb/Cr chrominance density map using atomic increments |
| `cie.wgsl` | Computes CIE 1931 xy chromaticity density map |

## Design Decisions

- **Workgroup sizes**: LUT bake uses `(8,8,8)` for 3D grid; apply uses `(16,16,1)` for 2D image; scopes use `(256,1,1)` for parallel reduction.
- **Atomic u32 for scopes**: All scope shaders use `atomicAdd` on `u32` storage buffers — avoids race conditions without explicit synchronization.
- **1D curve textures**: Curves (Hue-vs-Hue etc.) are baked to R32Float 1D textures on CPU and bound separately from the params uniform.
- **Exact CPU parity**: `bake_lut.wgsl` implements the same transform chain as `evaluate_transform()` for bit-exact matching in GPU reference tests.

## Dependencies

- **Internal**: Bound resources managed by `crispen-gpu/src/` Rust code (LutBaker, LutApplicator, ScopeDispatch)
- **External**: wgpu WGSL shader compiler

## Usage Examples

Shaders are compiled and dispatched by the Rust pipeline code:

```rust
// bake_lut.wgsl is dispatched by LutBaker::bake()
// apply_lut.wgsl is dispatched by LutApplicator::apply()
// Scope shaders are dispatched by ScopeDispatch::dispatch()
```
