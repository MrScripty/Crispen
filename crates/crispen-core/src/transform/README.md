# Transform

## Purpose

The core color transform chain and 3D LUT infrastructure. `GradingParams` is the single source of truth for all grading adjustments. `evaluate_transform()` applies the full chain to one pixel. `Lut3D` bakes the transform into a lookup table for fast GPU application.

## Contents

| File | Description |
|------|-------------|
| `mod.rs` | Module exports |
| `params.rs` | `GradingParams` struct — frozen contract between UI, Bevy, and GPU |
| `evaluate.rs` | `evaluate_transform()` — applies full grading chain to a single RGB pixel |
| `lut.rs` | `Lut3D` — CPU 3D LUT baking, trilinear interpolation, `.cube` file I/O |

## Design Decisions

- **Single composite transform**: All tools (wheels, sliders, curves) contribute to one `GradingParams`. The LUT bake shader mirrors `evaluate_transform()` exactly.
- **Frozen contract**: `GradingParams` is immutable once defined — UI, Bevy, and GPU all share this struct. Changes require coordinated updates across all layers.
- **65³ LUT**: Default grid size balances quality vs. bake time (~274K evaluations).

## Dependencies

- **Internal**: `color_management` (color space transforms), `grading` (CDL, curves, etc.)
- **External**: `serde` (serialization), `glam` (vector math)

## Usage Examples

```rust
use crispen_core::transform::params::GradingParams;
use crispen_core::transform::lut::Lut3D;

let params = GradingParams { saturation: 1.5, ..Default::default() };
let mut lut = Lut3D::new(33);
lut.bake(&params);
let graded = lut.apply([0.5, 0.3, 0.2]);
```
