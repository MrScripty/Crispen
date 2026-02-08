# Crispen — Architecture & Implementation Plan

## Context

Crispen is a Rust-based image color grading library modeled after DaVinci Resolve's Color page. It is a standalone demo and reusable module for the Studio Whip project, designed for later integration with Pentimento. The repo is currently empty (LICENSE + .gitignore only).

**Core design principle**: Every adjustment tool (wheels, sliders, curves, color space transforms) contributes values to a **single composite color transform**. This transform is baked into a **3D LUT** which is applied to the image in one texture lookup per pixel. When any parameter changes, the LUT is re-baked on GPU (~65³ = 274K samples, <0.1ms).

This plan follows the project [Coding Standards](/media/jeremy/OrangeCream/Linux Software/Coding-Standards/) and is structured for **parallel Claude agent execution** using the Immutable Contracts pattern: define shared types first, freeze them, then implement crates independently.

## Standards Compliance

| Standard | How Applied |
|----------|-------------|
| Layered Architecture | `core` (domain) → `gpu` (infrastructure) → `bevy` (application) → `demo` (presentation) |
| Backend-Owned Data | Bevy owns `GradingParams`; Svelte displays and sends actions, no optimistic updates |
| Immutable Contracts | Phase 0 defines all shared types/IPC; frozen before parallel implementation |
| IPC/Message Contract | `#[serde(tag = "type", content = "data")]` enums matching Pentimento pattern |
| 500 Line File Limit | Modules split by responsibility; each file single-purpose |
| Directory READMEs | Every directory with 3+ files gets README.md |
| Conventional Commits | `feat(core):`, `feat(gpu):`, etc. with `Agent:` footer per agent |
| Message Passing | Bevy events for cross-system communication, not shared mutable state |
| Validate at Boundaries | FFI/Vulkan interop, IPC deserialization, image loading |
| parking_lot::Mutex | Default mutex choice per concurrency standards |

## Workspace Structure

```text
crispen/
├── Cargo.toml
├── PLAN.md
├── AGENT-PROMPTS.md
├── .editorconfig
├── crates/
│   ├── crispen-core/                   # Domain — no GPU deps
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── image.rs                # GradingImage, BitDepth
│   │       ├── color_management/
│   │       │   ├── mod.rs
│   │       │   ├── color_space.rs      # ColorSpaceId, 3x3 matrices
│   │       │   ├── transfer.rs         # LOG curves (LogC3/4, S-Log3, V-Log)
│   │       │   ├── aces.rs             # ACES IDT/ODT, ACEScg/cc/cct
│   │       │   └── white_balance.rs    # Temperature/tint chromaticity shift
│   │       ├── transform/
│   │       │   ├── mod.rs
│   │       │   ├── params.rs           # GradingParams (single source of truth)
│   │       │   ├── evaluate.rs         # evaluate_transform() — full chain on one pixel
│   │       │   └── lut.rs              # Lut3D: bake, apply, load/save .cube
│   │       ├── grading/
│   │       │   ├── mod.rs
│   │       │   ├── wheels.rs           # Lift/Gamma/Gain/Offset (ASC CDL)
│   │       │   ├── sliders.rs          # Contrast, Pivot, Shadows, Highlights, Sat, Hue
│   │       │   ├── curves.rs           # Spline eval, 1D LUT baking
│   │       │   └── auto_balance.rs     # Auto white balance + shot matching
│   │       └── scopes/
│   │           ├── mod.rs
│   │           ├── histogram.rs
│   │           ├── waveform.rs
│   │           ├── vectorscope.rs
│   │           ├── parade.rs
│   │           └── cie.rs
│   │
│   ├── crispen-gpu/                    # Infrastructure — wgpu only, no Bevy
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── pipeline.rs
│   │   │   ├── lut_baker.rs
│   │   │   ├── lut_applicator.rs
│   │   │   ├── scope_dispatch.rs
│   │   │   ├── buffers.rs
│   │   │   ├── vulkan_interop.rs
│   │   │   └── readback.rs
│   │   └── shaders/
│   │       ├── bake_lut.wgsl
│   │       ├── apply_lut.wgsl
│   │       ├── midtone_detail.wgsl
│   │       ├── histogram.wgsl
│   │       ├── waveform.wgsl
│   │       ├── vectorscope.wgsl
│   │       └── cie.wgsl
│   │
│   ├── crispen-bevy/                   # Application — Bevy 0.18 plugin
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── resources.rs
│   │       ├── events.rs
│   │       ├── systems.rs
│   │       ├── render_node.rs
│   │       └── scope_render.rs
│   │
│   ├── crispen-ofx/                    # Infrastructure — OpenFX host
│   │   └── src/
│   │       ├── lib.rs
│   │       └── host.rs
│   │
│   └── crispen-demo/                   # Presentation — standalone app
│       ├── src/
│       │   ├── main.rs
│       │   ├── config.rs
│       │   ├── ipc.rs
│       │   ├── image_loader.rs
│       │   └── render/
│       │       └── mod.rs
│       ├── ui/                         # Svelte 5 frontend
│       │   ├── package.json
│       │   ├── vite.config.ts
│       │   ├── eslint.config.js
│       │   ├── tsconfig.json
│       │   └── src/
│       │       ├── main.ts
│       │       ├── App.svelte
│       │       └── lib/
│       │           ├── bridge.ts
│       │           ├── types.ts
│       │           └── components/
│       │               ├── ColorWheels.svelte
│       │               ├── PrimaryBars.svelte
│       │               ├── Sliders.svelte
│       │               ├── CurveEditor.svelte
│       │               ├── ScopeDisplay.svelte
│       │               └── ColorSpaceSelector.svelte
│       └── assets/
│           └── test_images/
```

## Workspace Dependencies

All versions must match Pentimento exactly to avoid duplicate types at integration time.

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"

[workspace.dependencies]
bevy = { version = "0.18", default-features = false }
wgpu = "27"
glam = "0.30"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
tracing = "0.1"
bytemuck = { version = "1.21", features = ["derive"] }
image = "0.25"
palette = "0.7"
openfx-sys = "0.3"
wry = "0.53"
tokio = { version = "1.44", features = ["sync", "rt-multi-thread"] }
tokio-tungstenite = "0.26"
futures-util = "0.3"
parking_lot = "0.12"

[workspace.lints.rust]
unsafe_code = "warn"
```

### Crate dependency layers (inward only)

```text
Presentation:   crispen-demo → crispen-bevy, bevy, wry, tokio
Application:    crispen-bevy → crispen-core, crispen-gpu, bevy
Infrastructure: crispen-gpu  → crispen-core, wgpu
                crispen-ofx  → crispen-core, openfx-sys
Domain:         crispen-core → glam, serde, bytemuck, palette, image
```

## Contract Definitions (Frozen Before Parallel Work)

### GradingParams — The Single Transform

```rust
/// Every tool writes here. The LUT bake shader reads the full struct.
/// This is the immutable contract between UI, Bevy, and GPU.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradingParams {
    pub color_management: ColorManagementConfig,

    // Primary Wheels [R, G, B, Master]
    pub lift: [f32; 4],          // Default [0,0,0,0]
    pub gamma: [f32; 4],         // Default [1,1,1,1]
    pub gain: [f32; 4],          // Default [1,1,1,1]
    pub offset: [f32; 4],        // Default [0,0,0,0]

    // Sliders
    pub temperature: f32,        // 0.0 = neutral
    pub tint: f32,               // 0.0 = neutral
    pub contrast: f32,           // 1.0 = neutral
    pub pivot: f32,              // Default 0.435
    pub midtone_detail: f32,     // 0.0 = off (spatial, separate pass)
    pub shadows: f32,            // 0.0 = neutral
    pub highlights: f32,         // 0.0 = neutral
    pub saturation: f32,         // 1.0 = neutral
    pub hue: f32,                // 0.0 = no rotation (degrees)
    pub luma_mix: f32,           // 0.0 = chroma weight

    // Curves (control points, baked to 1D LUTs before LUT bake)
    pub hue_vs_hue: Vec<[f32; 2]>,
    pub hue_vs_sat: Vec<[f32; 2]>,
    pub lum_vs_sat: Vec<[f32; 2]>,
    pub sat_vs_sat: Vec<[f32; 2]>,
}
```

### Supporting Types

```rust
pub struct ColorManagementConfig {
    pub input_space: ColorSpaceId,
    pub working_space: ColorSpaceId,  // Default: ACEScg
    pub output_space: ColorSpaceId,
}

pub enum ColorSpaceId {
    Aces2065_1, AcesCg, AcesCc, AcesCct,
    Srgb, LinearSrgb, Rec2020, DciP3,
    ArriLogC3, ArriLogC4, SLog3, RedLog3G10, VLog,
    Custom(u32),
}

pub struct GradingImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[f32; 4]>,       // Always RGBA f32 linear internally
    pub source_bit_depth: BitDepth,
}

pub enum BitDepth { U8, U10, U12, U16, F16, F32 }

pub struct Lut3D {
    pub size: u32,                    // 33 or 65
    pub data: Vec<[f32; 4]>,          // size³ entries
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
}
```

### IPC Messages

```rust
#[serde(tag = "type", content = "data")]
pub enum BevyToUi {
    Initialize { params: GradingParams },
    ParamsUpdated { params: GradingParams },
    ScopeData { histogram: HistogramData, waveform: WaveformData,
                vectorscope: VectorscopeData, cie: CieData },
    ImageLoaded { width: u32, height: u32, bit_depth: String },
    Error { message: String },
}

#[serde(tag = "type", content = "data")]
pub enum UiToBevy {
    SetParams { params: GradingParams },
    AutoBalance,
    ResetGrade,
    LoadImage { path: String },
    LoadLut { path: String, slot: String },
    ExportLut { path: String, size: u32 },
    ToggleScope { scope_type: String, visible: bool },
}
```

### Scope Data

```rust
pub struct HistogramData  { pub bins: [[u32; 256]; 4], pub peak: u32 }
pub struct WaveformData   { pub width: u32, pub height: u32, pub data: [Vec<u32>; 3] }
pub struct VectorscopeData { pub resolution: u32, pub density: Vec<u32> }
pub struct CieData        { pub resolution: u32, pub density: Vec<u32> }
```

### evaluate_transform()

```rust
/// The core function. GPU bake_lut.wgsl mirrors this exactly.
pub fn evaluate_transform(rgb: [f32; 3], params: &GradingParams) -> [f32; 3] {
    let mut c = rgb;
    c = apply_input_transform(c, &params.color_management);
    c = apply_white_balance(c, params.temperature, params.tint);
    c = apply_cdl(c, &params.lift, &params.gamma, &params.gain, &params.offset);
    c = apply_contrast(c, params.contrast, params.pivot);
    c = apply_shadows_highlights(c, params.shadows, params.highlights);
    c = apply_saturation_hue(c, params.saturation, params.hue, params.luma_mix);
    c = apply_curves(c, params);
    c = apply_output_transform(c, &params.color_management);
    c
}
```

## GPU Pipeline

```text
GradingParams change detected
    │
    ▼
[bake_lut.wgsl] @workgroup_size(8,8,8) on 65³ grid
    │  Reads: uniform GradingParamsGpu + 1D curve textures
    │  Writes: storage buffer → copy to texture_3d (hardware trilinear)
    │
    ▼
[apply_lut.wgsl] @workgroup_size(16,16,1) on full image
    │  Reads: source image + lut_texture
    │  Writes: graded output
    │
    ├──► [midtone_detail.wgsl] (only if midtone_detail != 0.0)
    │
    ├──► [histogram.wgsl]    @workgroup_size(256,1,1)  → readback
    ├──► [waveform.wgsl]     @workgroup_size(256,1,1)  → readback
    ├──► [vectorscope.wgsl]  @workgroup_size(256,1,1)  → readback
    └──► [cie.wgsl]          @workgroup_size(256,1,1)  → readback
```

## Parallel Agent Phases

### Phase 0: Contracts & Scaffolding (single agent, sequential)

One agent sets up the workspace and defines all shared types so they are frozen before parallel work begins.

**Creates:** All `Cargo.toml` files, all `lib.rs` / `mod.rs` stubs, `GradingParams`, `ColorSpaceId`, `Lut3D`, `GradingImage`, `BitDepth`, scope data structs, IPC enums, `.editorconfig`.

**Exit criteria:** `cargo build --workspace` succeeds with no errors.

### Phase 1: Parallel Implementation (3 agents)

Contracts are frozen. Three agents work on separate crates simultaneously.

- **Agent A** — `crispen-core`: Color science, grading math, LUT baking, CPU scopes, unit tests
- **Agent B** — `crispen-gpu`: All WGSL shaders, GPU pipeline orchestration, GPU-vs-CPU reference tests
- **Agent C** — `crispen-bevy` + `crispen-demo`: Bevy plugin, ECS systems, Svelte UI scaffold, IPC bridge

### Phase 2: Integration (single agent, sequential)

Wire parallel work together. End-to-end: load image → adjust params → graded output + live scopes.

### Phase 3: Advanced Features (parallel, later)

Curves UI, remaining scopes, OpenFX host, Vulkan external memory interop.

## Pentimento Integration (future)

```toml
# In Pentimento crates/scene/Cargo.toml:
[features]
color_grading = ["dep:crispen-bevy"]
```

```rust
// In ScenePlugin::build():
#[cfg(feature = "color_grading")]
app.add_plugins(crispen_bevy::CrispenPlugin);
```

## Verification

| Layer | Command | Verifies |
|-------|---------|----------|
| Static | `cargo clippy --workspace` | No warnings |
| Build | `cargo build --workspace` | All crates compile |
| Unit | `cargo test -p crispen-core` | Math correctness |
| GPU | `cargo test -p crispen-gpu` | GPU matches CPU reference |
| Svelte | `cd crates/crispen-demo/ui && npm run build` | Frontend compiles |
| E2E | `cargo run -p crispen-demo` | Full pipeline works |

## Commit Scopes

| Scope | Covers |
|-------|--------|
| `workspace` | Root Cargo.toml, config, workspace-level |
| `core` | crispen-core |
| `gpu` | crispen-gpu + shaders |
| `bevy` | crispen-bevy |
| `ofx` | crispen-ofx |
| `demo` | crispen-demo Rust + Svelte |

All commits use `Agent: <agent-name>` footer per commit standards.
