# GPU Pipeline

## Purpose

wgpu-based compute pipeline for GPU-accelerated LUT baking, image grading, and scope computation. No Bevy dependency — exposes a plain wgpu API that `crispen-bevy` wraps into ECS resources.

## Contents

| File | Description |
|------|-------------|
| `lib.rs` | `GradingParamsGpu` (GPU uniform layout), `color_space_to_u32()`, module exports |
| `pipeline.rs` | `GpuGradingPipeline` — top-level orchestrator for bake → apply → scopes |
| `lut_baker.rs` | `LutBaker` — dispatches `bake_lut.wgsl`, manages params uniform and curve textures |
| `lut_applicator.rs` | `LutApplicator` — dispatches `apply_lut.wgsl` with trilinear 3D LUT sampling |
| `scope_dispatch.rs` | `ScopeDispatch` — dispatches histogram, waveform, vectorscope, CIE compute shaders |
| `buffers.rs` | `GpuImageHandle`, `GpuLutHandle`, `ScopeBuffers`, `ScopeConfig` — GPU buffer management |
| `readback.rs` | `Readback`, `ScopeResults` — staging buffer mapping for GPU-to-CPU data transfer |
| `vulkan_interop.rs` | Placeholder for Vulkan external memory interop (Phase 3) |

## Design Decisions

- **Standalone wgpu**: No Bevy coupling; takes `Arc<Device>` + `Arc<Queue>` so it can use Bevy's device or its own.
- **Storage → Texture copy for LUT**: Bake shader writes to a storage buffer, then copies to a 3D texture for hardware trilinear filtering in the apply shader.
- **FLOAT32_FILTERABLE**: Required for R32Float curve textures with bilinear sampling.
- **Blocking readback**: `read_scopes()` and `download_image()` block via `device.poll(wait_indefinitely())` — acceptable for a demo; production would use async.

## Dependencies

- **Internal**: `crispen-core` (domain types: `GradingParams`, `GradingImage`, scope data structs)
- **External**: `wgpu` (GPU compute), `bytemuck` (Pod casting), `pollster` (async blocking), `tracing`, `parking_lot`

## Usage Examples

```rust
use crispen_gpu::GpuGradingPipeline;

let mut pipeline = GpuGradingPipeline::create_blocking()?;
let source = pipeline.upload_image(&grading_image);
let frame = pipeline.submit_frame(&source, &params, 65);
let scopes = frame.scopes.expect("scope readback should be present");
```
