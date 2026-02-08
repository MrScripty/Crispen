# Crispen — Agent Prompts

Detailed prompts for each parallel agent. Each agent should be given its prompt in full. All agents share the frozen contracts defined in Phase 0.

## Reference Paths

### This Project

- **Crispen repo:** `/media/jeremy/OrangeCream/Linux Software/Crispen/`
- **Plan:** `/media/jeremy/OrangeCream/Linux Software/Crispen/PLAN.md`

### Coding Standards (read these first)

- **All standards:** `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/`
- **Architecture patterns:** `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/ARCHITECTURE-PATTERNS.md` — Layered architecture, Backend-Owned Data, Immutable Contracts, IPC Message Contract, Phased Mutation
- **Coding standards:** `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/CODING-STANDARDS.md` — 500-line file limit, Rust error handling, module organization, naming conventions
- **Concurrency:** `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/CONCURRENCY-STANDARDS.md` — parking_lot::Mutex, message passing over shared state
- **Interop:** `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/INTEROP-STANDARDS.md` — Validate at boundaries, copy foreign buffers, document thread requirements
- **Documentation:** `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/DOCUMENTATION-STANDARDS.md` — Directory READMEs, doc comment format, algorithm documentation
- **Testing:** `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/TESTING-STANDARDS.md` — Test naming, Arrange-Act-Assert, property-based testing, verification layers
- **Commits:** `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/COMMIT-STANDARDS.md` — Conventional commits, agent footer

### Sister Projects (reference implementations)

- **Pentimento** (Bevy 0.18 + Svelte, same architecture pattern):
  `/media/jeremy/OrangeCream/Linux Software/Pentimento/`
  - Workspace Cargo.toml: `Pentimento/Cargo.toml` — dependency versions to match exactly
  - IPC messages pattern: `Pentimento/crates/ipc/src/messages.rs` — `BevyToUi`/`UiToBevy` with `#[serde(tag = "type", content = "data")]`
  - Plugin pattern: `Pentimento/crates/scene/src/lib.rs` — `ScenePlugin` with feature-gated subsystems
  - Painting pipeline: `Pentimento/crates/painting/src/` — Surface, tiles, pipeline pattern
  - Painting system: `Pentimento/crates/scene/src/painting_system.rs` — Canvas texture GPU upload
  - Svelte UI: `Pentimento/ui/src/` — App.svelte, lib/bridge.ts, lib/components/
  - App entry: `Pentimento/crates/app/src/main.rs` — Bevy app setup with webview
  - Render compositing: `Pentimento/crates/app/src/render/` — Webview compositing backends

- **Pantograph** (Rust + Svelte + node graphs):
  `/media/jeremy/OrangeCream/Linux Software/Pantograph/`
  - Multi-crate Rust workspace with Svelte frontend
  - Reference for: node-based UI, Svelte component patterns

- **Bevy engine source** (reference for render graph, compute shaders, ECS):
  `/media/jeremy/OrangeCream/Linux Software/bevy/`
  - Compute shader examples: `bevy/examples/shader/compute_shader_game_of_life.rs`
  - GPU readback: `bevy/examples/shader/gpu_readback.rs`
  - Custom post-processing: `bevy/examples/shader/custom_post_processing.rs`
  - Render graph: `bevy/crates/bevy_render/src/render_graph/`
  - Color grading built-in: `bevy/crates/bevy_core_pipeline/src/tonemapping/`

- **wgpu source** (reference for compute pipelines, buffer management):
  `/media/jeremy/OrangeCream/Linux Software/wgpu/`
  - Compute example: `wgpu/examples/src/hello_compute/`
  - HAL Vulkan backend: `wgpu/wgpu-hal/src/vulkan/`
  - Buffer creation: `wgpu/wgpu-core/src/device/resource.rs`

---

## Phase 0 Prompt: Scaffolding Agent

```
You are setting up the Crispen workspace — a Rust color grading library.

## Your Task

Create the full Cargo workspace and define all shared type contracts. These
contracts will be FROZEN after this phase — other agents will implement against
them in parallel, so they must be complete and correct.

## Read First

1. /media/jeremy/OrangeCream/Linux Software/Crispen/PLAN.md — the full plan
2. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/CODING-STANDARDS.md
3. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/ARCHITECTURE-PATTERNS.md
4. /media/jeremy/OrangeCream/Linux Software/Pentimento/Cargo.toml — match these dependency versions exactly
5. /media/jeremy/OrangeCream/Linux Software/Pentimento/crates/ipc/src/messages.rs — IPC pattern to follow

## Steps

1. Create workspace root Cargo.toml with all [workspace.dependencies] matching
   Pentimento versions. Edition 2024, resolver "2", members = ["crates/*"].

2. Create Cargo.toml for each crate:
   - crispen-core: glam, serde, serde_json, thiserror, tracing, bytemuck, palette, image, parking_lot
   - crispen-gpu: crispen-core (path), wgpu, bytemuck, tracing, parking_lot
   - crispen-bevy: crispen-core (path), crispen-gpu (path), bevy (with features: bevy_core_pipeline, bevy_render, bevy_asset, bevy_log, bevy_ui), tracing
   - crispen-ofx: crispen-core (path), openfx-sys, tracing
   - crispen-demo: crispen-bevy (path), bevy (with features: bevy_core_pipeline, bevy_render, bevy_asset, bevy_log, bevy_ui, bevy_winit), wry, tokio, tokio-tungstenite, futures-util, serde_json, tracing, image

3. Create all source files with module structure:

   crispen-core/src/lib.rs — pub mod declarations + re-exports
   crispen-core/src/image.rs — GradingImage, BitDepth enum with Display/From impls
   crispen-core/src/transform/mod.rs — pub mod params, evaluate, lut
   crispen-core/src/transform/params.rs — GradingParams with Default impl, ColorManagementConfig, ColorSpaceId
   crispen-core/src/transform/evaluate.rs — pub fn evaluate_transform() signature with todo!() body
   crispen-core/src/transform/lut.rs — Lut3D struct with method signatures (bake, apply, load_cube, save_cube) as todo!()
   crispen-core/src/color_management/mod.rs — pub mod color_space, transfer, aces, white_balance
   crispen-core/src/color_management/color_space.rs — ColorSpaceId (already in params, re-export), transform matrix type stubs
   crispen-core/src/color_management/transfer.rs — TransferFunction trait, stub impls
   crispen-core/src/color_management/aces.rs — apply_input_transform/apply_output_transform signatures
   crispen-core/src/color_management/white_balance.rs — apply_white_balance signature
   crispen-core/src/grading/mod.rs — pub mod wheels, sliders, curves, auto_balance
   crispen-core/src/grading/wheels.rs — apply_cdl signature
   crispen-core/src/grading/sliders.rs — apply_contrast, apply_shadows_highlights, apply_saturation_hue signatures
   crispen-core/src/grading/curves.rs — CurveEvaluator, bake_curve_to_1d_lut, apply_curves signatures
   crispen-core/src/grading/auto_balance.rs — auto_white_balance, match_shot signatures
   crispen-core/src/scopes/mod.rs — pub mod + all scope data structs
   crispen-core/src/scopes/{histogram,waveform,vectorscope,parade,cie}.rs — Data structs + compute() stubs

   crispen-gpu/src/lib.rs — pub mod declarations
   crispen-gpu/src/pipeline.rs — GpuGradingPipeline struct stub
   crispen-gpu/src/{lut_baker,lut_applicator,scope_dispatch,buffers,vulkan_interop,readback}.rs — struct/fn stubs

   crispen-bevy/src/lib.rs — CrispenPlugin stub
   crispen-bevy/src/{resources,events,systems,render_node,scope_render}.rs — type stubs

   crispen-ofx/src/lib.rs — pub mod host
   crispen-ofx/src/host.rs — OfxHost stub

   crispen-demo/src/main.rs — fn main() with minimal Bevy app
   crispen-demo/src/ipc.rs — BevyToUi / UiToBevy enums (full, matching PLAN.md contracts)
   crispen-demo/src/{config,image_loader}.rs — stubs
   crispen-demo/src/render/mod.rs — stub

4. Create .editorconfig (4-space indent for Rust, 2-space for TS/Svelte/JSON/CSS)

5. Verify: cargo build --workspace must succeed with zero errors.

## Commit

feat(workspace): scaffold workspace with frozen type contracts

Agent: scaffolding-agent

## Rules

- All function bodies that are not yet implemented use todo!() macro
- All pub types must have doc comments explaining their purpose
- GradingParams Default impl must produce identity (no-op) transform
- Follow Rust module organization from CODING-STANDARDS.md
- Do NOT implement any business logic — only type definitions and signatures
```

---

## Phase 1, Agent A Prompt: crispen-core

```
You are implementing the crispen-core crate — the platform-agnostic color
grading domain library with zero GPU dependencies.

## Your Task

Implement all color science, grading math, LUT baking, and CPU scope
computation. This is the mathematical heart of the system.

## Read First

1. /media/jeremy/OrangeCream/Linux Software/Crispen/PLAN.md — full architecture
2. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/CODING-STANDARDS.md — 500 line limit, Rust guidelines
3. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/TESTING-STANDARDS.md — test naming, property-based testing
4. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/DOCUMENTATION-STANDARDS.md — algorithm docs
5. /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-core/src/ — your workspace (stubs from Phase 0)

## Reference Code

- Bevy built-in color grading for reference math:
  /media/jeremy/OrangeCream/Linux Software/bevy/crates/bevy_core_pipeline/src/tonemapping/
- Pentimento painting surface (f32x4 pixel format you must match):
  /media/jeremy/OrangeCream/Linux Software/Pentimento/crates/painting/src/surface.rs

## Scope — Files You Own

Only modify files under /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-core/

## Implementation Steps

### A.1: color_management/color_space.rs
- Define ColorMatrix type (3x3 f64 array wrapper)
- Implement transform matrices for ALL ColorSpaceId pairs:
  - sRGB/Rec.709 ↔ ACEScg (AP1) ↔ ACES2065-1 (AP0)
  - Rec.2020 ↔ ACEScg
  - DCI-P3 ↔ ACEScg
- fn get_transform_matrix(from: ColorSpaceId, to: ColorSpaceId) -> ColorMatrix
- fn apply_matrix(rgb: [f32; 3], matrix: &ColorMatrix) -> [f32; 3]
- Use published CIE chromaticity coordinates and Bradford chromatic adaptation

### A.2: color_management/transfer.rs
- trait TransferFunction { fn to_linear(encoded: f32) -> f32; fn from_linear(linear: f32) -> f32; }
- Implement for: ARRI LogC3, ARRI LogC4, Sony S-Log3, RED Log3G10, Panasonic V-Log
- Implement ACEScc and ACEScct log encodings
- Implement sRGB gamma (IEC 61966-2-1)
- fn get_transfer(space: ColorSpaceId) -> Option<Box<dyn TransferFunction>>
- Each curve must use the published specification constants

### A.3: color_management/aces.rs
- fn apply_input_transform(rgb: [f32; 3], config: &ColorManagementConfig) -> [f32; 3]
  Linearize via transfer function, then matrix to working space
- fn apply_output_transform(rgb: [f32; 3], config: &ColorManagementConfig) -> [f32; 3]
  Matrix from working space, then apply output transfer function
- Handle identity case (input == working) as early return

### A.4: color_management/white_balance.rs
- fn apply_white_balance(rgb: [f32; 3], temperature: f32, tint: f32) -> [f32; 3]
- Temperature: shift along Planckian locus in CIE xy, then Bradford adapt
- Tint: shift perpendicular to Planckian locus (green-magenta axis)
- 0.0 temperature and 0.0 tint = identity

### A.5: grading/wheels.rs
- fn apply_cdl(rgb: [f32; 3], lift: &[f32; 4], gamma: &[f32; 4], gain: &[f32; 4], offset: &[f32; 4]) -> [f32; 3]
- ASC CDL extended with lift separation:
  Per channel c: out = pow(max(in * gain[c]*gain[3] + lift[c]+lift[3] * (1 - gain[c]*gain[3]) + offset[c]+offset[3], 0), 1/(gamma[c]*gamma[3]))
- Document the formula with /// doc comments and ASCII diagram

### A.6: grading/sliders.rs
- fn apply_contrast(rgb: [f32; 3], contrast: f32, pivot: f32) -> [f32; 3]
  Per channel: out = pow(in/pivot, contrast) * pivot (log-space S-curve)
- fn apply_shadows_highlights(rgb: [f32; 3], shadows: f32, highlights: f32) -> [f32; 3]
  Soft knee isolation: shadows affect below pivot, highlights above
- fn apply_saturation_hue(rgb: [f32; 3], saturation: f32, hue: f32, luma_mix: f32) -> [f32; 3]
  Convert to HSL domain for hue rotation, saturation multiply
  luma_mix blends between rec709 luma and equal-weight luma for contrast operations

### A.7: grading/curves.rs
- struct CurveEvaluator with Catmull-Rom spline interpolation
- fn bake_curve_to_1d_lut(points: &[[f32; 2]], size: u32) -> Vec<f32>
- fn apply_curves(rgb: [f32; 3], params: &GradingParams) -> [f32; 3]
  - hue_vs_hue: rotate hue based on input hue
  - hue_vs_sat: adjust saturation based on input hue
  - lum_vs_sat: adjust saturation based on input luminance
  - sat_vs_sat: adjust saturation based on input saturation
- Empty control point vec = identity (no adjustment)

### A.8: grading/auto_balance.rs
- fn auto_white_balance(image: &GradingImage) -> (f32, f32) → returns (temperature, tint)
  Gray-world assumption: compute average RGB, derive correction
- fn match_shot(source: &GradingImage, target: &GradingImage) -> GradingParams
  Histogram matching: align luminance and per-channel distributions

### A.9: transform/evaluate.rs
- Implement evaluate_transform() by calling each function in order (see PLAN.md)
- This is the reference implementation that GPU bake_lut.wgsl must match exactly

### A.10: transform/lut.rs
- Lut3D::bake(params: &GradingParams, size: u32) -> Self
  Iterate over size³ grid, call evaluate_transform() on each point
- Lut3D::apply(&self, rgb: [f32; 3]) -> [f32; 3]
  Trilinear interpolation in the 3D grid
- Lut3D::load_cube(path: &Path) -> Result<Self>
  Parse Iridas .cube file format (TITLE, DOMAIN_MIN, DOMAIN_MAX, LUT_3D_SIZE, data lines)
- Lut3D::save_cube(&self, path: &Path) -> Result<()>

### A.11: scopes/*.rs
- HistogramData::compute(image: &GradingImage) -> Self
  256 bins per channel (R, G, B, Luma), count pixels
- WaveformData::compute(image: &GradingImage, height: u32) -> Self
- VectorscopeData::compute(image: &GradingImage, resolution: u32) -> Self
- CieData::compute(image: &GradingImage, resolution: u32) -> Self

### A.12: Unit Tests
Place in each module file under #[cfg(test)] mod tests { }.
Test naming: test_<function>_<scenario>_<expected_result>

Required tests:
- test_cdl_identity_is_passthrough — default params produce input == output
- test_cdl_gain_doubles_values — gain [2,2,2,1] doubles RGB
- test_srgb_to_acescg_roundtrip — convert forward then back, max error < 1e-5
- test_logc3_linearize_roundtrip — encode then decode, max error < 1e-5
- test_lut_bake_matches_evaluate — bake 17³ LUT, sample at grid points, exact match
- test_lut_trilinear_interpolation — sample between grid points, verify smooth
- test_cube_file_roundtrip — save then load, data matches within 1e-6
- test_contrast_at_pivot_is_identity — contrast adjustment preserves pivot point
- test_saturation_zero_produces_grayscale
- test_histogram_bins_sum_to_pixel_count
- test_auto_balance_on_neutral_image_returns_zero

## Commits (make multiple small commits)

feat(core): implement ACES color management pipeline
feat(core): implement ASC CDL lift/gamma/gain/offset
feat(core): implement slider operations
feat(core): implement curve evaluation and 1D LUT baking
feat(core): implement 3D LUT baking and .cube file I/O
feat(core): implement CPU scope computation
feat(core): implement auto white balance
test(core): add unit tests for color transform chain

Agent: core-agent

## Rules

- No GPU dependencies. This crate must compile without a GPU.
- All math in f32 for GPU compatibility (f64 only for matrix constants)
- Every pub fn gets a doc comment with formula and any relevant reference
- Follow CODING-STANDARDS.md Rust guidelines (Result vs panic, borrow vs clone)
- Keep files under 500 lines — split if needed
```

---

## Phase 1, Agent B Prompt: crispen-gpu

```
You are implementing the crispen-gpu crate — the GPU compute backend using
wgpu. This crate has NO Bevy dependency; it works with raw wgpu.

## Your Task

Write all WGSL compute shaders and the Rust pipeline that orchestrates them.
The bake_lut.wgsl shader must produce identical output to the CPU
evaluate_transform() function in crispen-core.

## Read First

1. /media/jeremy/OrangeCream/Linux Software/Crispen/PLAN.md — GPU pipeline design section
2. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/CODING-STANDARDS.md
3. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/INTEROP-STANDARDS.md — boundary validation
4. /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-core/src/transform/params.rs — GradingParams struct (THE contract)
5. /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-core/src/transform/evaluate.rs — CPU reference to match

## Reference Code

- Bevy compute shader examples:
  /media/jeremy/OrangeCream/Linux Software/bevy/examples/shader/compute_shader_game_of_life.rs
  /media/jeremy/OrangeCream/Linux Software/bevy/examples/shader/gpu_readback.rs
- wgpu compute example:
  /media/jeremy/OrangeCream/Linux Software/wgpu/examples/src/hello_compute/
- wgpu HAL Vulkan backend (for vulkan_interop.rs reference):
  /media/jeremy/OrangeCream/Linux Software/wgpu/wgpu-hal/src/vulkan/

## Scope — Files You Own

Only modify files under /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-gpu/

## Implementation Steps

### B.1: shaders/bake_lut.wgsl

This is the most critical shader. It must mirror evaluate_transform() exactly.

struct GradingParamsGpu {
    // Pack all GradingParams fields as uniform-compatible layout
    lift: vec4<f32>,
    gamma: vec4<f32>,
    gain: vec4<f32>,
    offset: vec4<f32>,
    temperature: f32,
    tint: f32,
    contrast: f32,
    pivot: f32,
    shadows: f32,
    highlights: f32,
    saturation: f32,
    hue: f32,
    luma_mix: f32,
    input_space: u32,    // ColorSpaceId as integer
    working_space: u32,
    output_space: u32,
}

@group(0) @binding(0) var<storage, read_write> lut_data: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> params: GradingParamsGpu;
@group(0) @binding(2) var<uniform> lut_size: u32;
@group(0) @binding(3) var curve_hue_vs_hue: texture_1d<f32>;
@group(0) @binding(4) var curve_hue_vs_sat: texture_1d<f32>;
@group(0) @binding(5) var curve_lum_vs_sat: texture_1d<f32>;
@group(0) @binding(6) var curve_sat_vs_sat: texture_1d<f32>;
@group(0) @binding(7) var curve_sampler: sampler;

@compute @workgroup_size(8, 8, 8)

Implement ALL operations matching the CPU evaluate_transform():
1. Input transform (matrix + linearization based on input_space)
2. White balance (Planckian locus shift)
3. CDL (lift/gamma/gain/offset)
4. Contrast with pivot
5. Shadows/highlights
6. Saturation + hue rotation + luma mix
7. Curves (sample 1D textures)
8. Output transform (matrix + output transfer based on output_space)

Embed the color space matrices as constants in the shader.
Embed LOG curve constants in the shader.

### B.2: shaders/apply_lut.wgsl

@group(0) @binding(0) var<storage, read> source: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(2) var lut_texture: texture_3d<f32>;
@group(0) @binding(3) var lut_sampler: sampler;  // Trilinear
@group(0) @binding(4) var<uniform> dimensions: vec2<u32>;

@compute @workgroup_size(16, 16, 1)
- Normalize pixel RGB to [0,1] (clamp)
- textureSampleLevel(lut_texture, lut_sampler, normalized_rgb, 0.0)
- Preserve alpha from source

### B.3: shaders/midtone_detail.wgsl

Separable Gaussian blur + unsharp mask for local contrast.
- Two passes: horizontal blur, vertical blur (use shared workgroup memory)
- Unsharp: detail = original - blurred; output = original + midtone_detail * detail
- Only applied to luminance channel
@workgroup_size(256, 1, 1) with shared memory tile

### B.4: shaders/histogram.wgsl

- Workgroup-local histogram bins (256 per channel, 4 channels) in var<workgroup>
- Each thread processes multiple pixels
- Workgroup barrier, then atomicAdd to global storage buffer
- @workgroup_size(256, 1, 1)

### B.5: shaders/waveform.wgsl

- Atomic scatter: for each pixel, compute bin = floor(value * (height-1))
- atomicAdd(&buffer[(channel * width + x) * height + bin], 1u)
- Output: width × height × 3 channels
- @workgroup_size(256, 1, 1)

### B.6: shaders/vectorscope.wgsl

- Convert RGB to YCbCr (BT.709)
- Map Cb,Cr to grid position in [0, resolution)
- atomicAdd(&density[y * resolution + x], 1u)
- @workgroup_size(256, 1, 1)

### B.7: shaders/cie.wgsl

- Convert RGB to CIE XYZ via matrix
- Compute xy chromaticity: x = X/(X+Y+Z), y = Y/(X+Y+Z)
- Map to grid position, atomicAdd to density
- @workgroup_size(256, 1, 1)

### B.8: pipeline.rs — GpuGradingPipeline

pub struct GpuGradingPipeline {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    bake_pipeline: wgpu::ComputePipeline,
    apply_pipeline: wgpu::ComputePipeline,
    midtone_pipeline: wgpu::ComputePipeline,
    histogram_pipeline: wgpu::ComputePipeline,
    waveform_pipeline: wgpu::ComputePipeline,
    vectorscope_pipeline: wgpu::ComputePipeline,
    cie_pipeline: wgpu::ComputePipeline,
    // bind group layouts, staging buffers, etc.
}

impl GpuGradingPipeline {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self;
    pub fn bake_lut(&self, params: &GradingParams) -> GpuLutHandle;
    pub fn apply_lut(&self, source: &GpuImageHandle, lut: &GpuLutHandle) -> GpuImageHandle;
    pub fn compute_scopes(&self, image: &GpuImageHandle) -> ScopeHandles;
    pub fn readback_scopes(&self, handles: &ScopeHandles) -> ScopeResults;
    pub fn upload_image(&self, image: &GradingImage) -> GpuImageHandle;
    pub fn download_image(&self, handle: &GpuImageHandle) -> GradingImage;
}

### B.9-B.13: Supporting modules

- lut_baker.rs: Create bind group for bake_lut.wgsl, dispatch ceil(size/8)³, copy storage→texture_3d
- lut_applicator.rs: Create bind group for apply_lut.wgsl, dispatch ceil(w/16) × ceil(h/16)
- scope_dispatch.rs: Create bind groups for all scope shaders, clear buffers before dispatch
- buffers.rs: GpuImageHandle (wgpu::Buffer + metadata), upload via queue.write_buffer, staging for readback
- readback.rs: Map staging buffer, copy to CPU scope data structs, async poll
- vulkan_interop.rs: Stub with doc comments explaining wgpu-hal approach. Mark as todo!() — Phase 3.

### B.14: Integration Tests

Tests require a real wgpu device. Use wgpu::Instance::new() with backends::all().

- test_gpu_lut_bake_matches_cpu_reference
  Bake LUT on GPU, readback, compare against Lut3D::bake() from crispen-core.
  Max per-component error < 1e-4 (f32 precision).
- test_apply_lut_identity
  Bake identity LUT (default params), apply to test image, output == input within 1e-4.
- test_histogram_bins_sum_to_pixel_count
  Upload known image, compute histogram, verify sum of all bins == width * height.
- test_bake_lut_workgroup_coverage
  Test with non-power-of-2 LUT sizes to verify edge workgroups handle bounds correctly.

## Commits

feat(gpu): implement LUT bake and apply WGSL shaders
feat(gpu): implement scope compute shaders (histogram, waveform, vectorscope, CIE)
feat(gpu): implement GPU pipeline orchestration
feat(gpu): implement buffer management and readback
test(gpu): add GPU vs CPU reference tests

Agent: gpu-agent

## Rules

- WGSL only for shaders (no GLSL, no SPIR-V, no rust-gpu)
- Shader math must match CPU evaluate_transform() — use same constants
- All wgpu resource creation uses descriptive labels (label: Some("crispen_bake_lut_pipeline"))
- Use include_str!() or Bevy's asset loading for shader source
- Workgroup sizes as specified in PLAN.md
- Handle image dimensions that aren't multiples of workgroup size (bounds check in shader)
- Follow INTEROP-STANDARDS.md for any unsafe GPU operations
```

---

## Phase 1, Agent C Prompt: crispen-bevy + crispen-demo

```
You are implementing the crispen-bevy plugin and crispen-demo standalone
application with Svelte 5 frontend.

## Your Task

Create the Bevy 0.18 plugin that wires crispen-core and crispen-gpu into ECS,
and build the standalone demo app with a Svelte UI communicating via WebSocket
IPC. Follow Pentimento's architecture exactly.

## Read First

1. /media/jeremy/OrangeCream/Linux Software/Crispen/PLAN.md — full plan
2. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/CODING-STANDARDS.md
3. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/ARCHITECTURE-PATTERNS.md — Backend-Owned Data, IPC Message Contract, View Model
4. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/CONCURRENCY-STANDARDS.md — message passing, parking_lot
5. /media/jeremy/OrangeCream/Linux Software/Coding-Standards/TOOLING-STANDARDS.md — ESLint, Prettier, .editorconfig for Svelte

## Reference Code — Study These Carefully

- Pentimento app entry (your template):
  /media/jeremy/OrangeCream/Linux Software/Pentimento/crates/app/src/main.rs
- Pentimento plugin registration:
  /media/jeremy/OrangeCream/Linux Software/Pentimento/crates/scene/src/lib.rs
- Pentimento IPC messages (your pattern):
  /media/jeremy/OrangeCream/Linux Software/Pentimento/crates/ipc/src/messages.rs
- Pentimento webview render compositing:
  /media/jeremy/OrangeCream/Linux Software/Pentimento/crates/app/src/render/
- Pentimento Svelte UI:
  /media/jeremy/OrangeCream/Linux Software/Pentimento/ui/src/App.svelte
  /media/jeremy/OrangeCream/Linux Software/Pentimento/ui/src/lib/bridge.ts
  /media/jeremy/OrangeCream/Linux Software/Pentimento/ui/src/lib/components/
  /media/jeremy/OrangeCream/Linux Software/Pentimento/ui/package.json
- Pantograph Svelte UI (additional reference):
  /media/jeremy/OrangeCream/Linux Software/Pantograph/
- Bevy render graph examples:
  /media/jeremy/OrangeCream/Linux Software/bevy/examples/shader/compute_shader_game_of_life.rs
  /media/jeremy/OrangeCream/Linux Software/bevy/examples/shader/gpu_readback.rs

## Scope — Files You Own

- /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-bevy/
- /media/jeremy/OrangeCream/Linux Software/Crispen/crates/crispen-demo/

## Implementation Steps — crispen-bevy

### C.1: lib.rs — CrispenPlugin

pub struct CrispenPlugin;
impl Plugin for CrispenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GradingState>()
            .init_resource::<ScopeResults>()
            .init_resource::<ScopeUpdateTimer>()
            .add_event::<ColorGradingCommand>()
            .add_systems(Update, (
                handle_grading_commands,
                rebake_lut_if_dirty,
            ).chain())
            .add_systems(Update, update_scopes
                .run_if(resource_changed::<GradingState>));
        // Register render world systems
    }
}

### C.2: resources.rs

#[derive(Resource)]
pub struct GradingState {
    pub params: GradingParams,
    pub dirty: bool,              // True when params changed, triggers LUT rebake
    pub lut: Option<Lut3D>,       // CPU fallback LUT
    pub source_image: Option<GradingImage>,
    pub graded_image: Option<GradingImage>,
}

#[derive(Resource, Default)]
pub struct ScopeResults {
    pub histogram: Option<HistogramData>,
    pub waveform: Option<WaveformData>,
    pub vectorscope: Option<VectorscopeData>,
    pub cie: Option<CieData>,
}

#[derive(Resource)]
pub struct ScopeUpdateTimer {
    pub timer: Timer,  // 15 fps max
}

### C.3: events.rs

#[derive(Event)]
pub enum ColorGradingCommand {
    SetParams(GradingParams),
    AutoBalance,
    ResetGrade,
    LoadImage(String),
    LoadLut { path: String, slot: String },
    ExportLut { path: String, size: u32 },
}

### C.4: systems.rs

- fn handle_grading_commands: Read events, update GradingState, mark dirty
- fn rebake_lut_if_dirty: If dirty, call Lut3D::bake() (CPU path for now),
  apply to source_image, store graded_image, clear dirty flag
- fn update_scopes: Compute HistogramData from graded_image, update ScopeResults

Backend-Owned Data: These systems are the ONLY place state changes.
The frontend sends commands, Bevy processes them, and pushes new state back.

### C.5: render_node.rs

Stub for now — will dispatch GPU pipeline in Phase 2.
Document that this will replace the CPU path in rebake_lut_if_dirty.

### C.6: scope_render.rs

fn scope_to_image(histogram: &HistogramData, width: u32, height: u32) -> Image
Render histogram data into a Bevy Image asset for display.
Simple CPU rasterization: for each bin, draw a vertical bar.

## Implementation Steps — crispen-demo

### C.7: main.rs

Match Pentimento's main.rs pattern:
- Create Bevy App with windowed mode
- Add DefaultPlugins (with render features)
- Add CrispenPlugin
- Add WebSocket IPC plugin/system
- Spawn camera + fullscreen quad for image display
- Load a default test image on startup

### C.8: config.rs

struct CrispenConfig {
    pub window_width: f32,
    pub window_height: f32,
    pub webview_port: u16,
    pub default_image: Option<String>,
}

### C.9: ipc.rs

BevyToUi / UiToBevy enums matching PLAN.md contracts exactly.
WebSocket server system:
- Start tokio WebSocket listener on config port
- Receive UiToBevy messages → convert to ColorGradingCommand events
- Send BevyToUi messages when state changes (ParamsUpdated, ScopeData)

Follow Pentimento's bridge.ts pattern for the protocol.

### C.10: image_loader.rs

fn load_image_system: Watch for LoadImage commands, load via image crate,
convert to GradingImage, store in GradingState.source_image.

### C.11: render/mod.rs

Webview compositing matching Pentimento's approach.
Use wry to embed Svelte UI as overlay on Bevy window.
Reference: Pentimento/crates/app/src/render/

### C.12: Svelte UI

Create Svelte 5 project in crates/crispen-demo/ui/:

package.json: svelte 5, vite, typescript, eslint, prettier
vite.config.ts: standard Svelte Vite config, build output to ../assets/ui/
tsconfig.json: strict mode per TOOLING-STANDARDS.md

src/main.ts: Mount App
src/App.svelte: Layout with panels for viewer, controls, scopes
src/lib/bridge.ts: WebSocket connection, send/receive typed messages
src/lib/types.ts: TypeScript interfaces matching BevyToUi/UiToBevy exactly

Components (initial stubs with basic functionality):
- ColorWheels.svelte: 4 wheels (lift/gamma/gain/offset), each a circular control
  Sends SetParams on drag. Read-only display from ParamsUpdated.
- PrimaryBars.svelte: Bar-style alternative to wheels (vertical sliders per channel)
- Sliders.svelte: All slider params (temperature through luma_mix)
  Each slider sends SetParams on change.
- CurveEditor.svelte: Canvas-based curve drawing. Stub with placeholder.
- ScopeDisplay.svelte: Renders histogram as canvas bars from ScopeData messages.
- ColorSpaceSelector.svelte: Dropdown for input/working/output color spaces.

Follow Backend-Owned Data: UI never stores grading state locally.
All state comes from BevyToUi messages. UI sends actions via UiToBevy.

## Commits

feat(bevy): implement CrispenPlugin with ECS resources and systems
feat(demo): scaffold Bevy app with image display and webview
feat(demo): implement WebSocket IPC bridge
feat(demo): scaffold Svelte 5 UI with components

Agent: bevy-demo-agent

## Rules

- Follow Backend-Owned Data pattern strictly — UI displays what backend sends
- Follow Pentimento's patterns for webview compositing, IPC, and Svelte structure
- Use Bevy events (not shared mutable state) for cross-system communication
- Svelte components use TypeScript strict mode
- ESLint + Prettier configured per TOOLING-STANDARDS.md
- Keep Rust files under 500 lines
- All IPC messages validated at deserialization boundary
```

---

## Phase 2 Prompt: Integration Agent

```
You are wiring the three parallel implementations together into a working
end-to-end pipeline.

## Your Task

Connect crispen-core, crispen-gpu, and crispen-bevy/crispen-demo so that:
1. Loading an image → displays it in the viewer
2. Adjusting a wheel/slider in Svelte → updates GradingParams → rebakes LUT → displays graded image
3. Histogram scope updates in real-time with grading changes

## Read First

1. /media/jeremy/OrangeCream/Linux Software/Crispen/PLAN.md
2. All source code in /media/jeremy/OrangeCream/Linux Software/Crispen/crates/

## Steps

1. Wire crispen-bevy's rebake_lut_if_dirty to use GpuGradingPipeline instead of CPU Lut3D::bake()
   - Get wgpu Device/Queue from Bevy's RenderDevice
   - Create GpuGradingPipeline once as a Resource
   - Dispatch bake + apply on parameter change
   - Readback graded image for scope computation (or compute scopes on GPU too)

2. Wire crispen-demo's IPC to crispen-bevy events
   - UiToBevy::SetParams → ColorGradingCommand::SetParams event
   - GradingState changes → BevyToUi::ParamsUpdated + BevyToUi::ScopeData

3. Wire Svelte UI controls to send complete GradingParams on every change
   - ColorWheels drag → update local copy → send SetParams
   - Sliders change → same
   - Receive ParamsUpdated → update all UI controls to reflect backend state

4. Wire scope display
   - ScopeData messages → render histogram in ScopeDisplay.svelte

5. Add directory READMEs per DOCUMENTATION-STANDARDS.md for all directories with 3+ files

6. End-to-end test: cargo run -p crispen-demo, open browser, load image,
   adjust controls, verify graded output and histogram.

## Commits

feat(demo): wire end-to-end grading pipeline
docs: add directory READMEs

Agent: integration-agent
```
