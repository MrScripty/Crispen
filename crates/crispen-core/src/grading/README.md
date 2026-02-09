# Grading

## Purpose

Individual color grading operations that compose the transform chain. Each module implements one category of adjustment (wheels, sliders, curves) following the ASC CDL model and DaVinci Resolve conventions.

## Contents

| File | Description |
|------|-------------|
| `mod.rs` | Module exports |
| `wheels.rs` | Lift/Gamma/Gain/Offset (ASC CDL) — primary color correction |
| `sliders.rs` | Contrast, pivot, shadows, highlights, saturation, hue rotation |
| `curves.rs` | Spline evaluation for Hue-vs-Hue, Hue-vs-Sat, Lum-vs-Sat, Sat-vs-Sat |
| `auto_balance.rs` | Automatic white balance via gray-world assumption |

## Design Decisions

- **ASC CDL model**: Lift/Gamma/Gain/Offset follows the industry-standard CDL formula for interoperability.
- **Per-channel + master**: Each wheel has R, G, B, and Master channels (`[f32; 4]`), matching DaVinci Resolve's interface.
- **Spline-based curves**: Control points are stored as `Vec<[f32; 2]>` and baked to 1D LUTs before GPU upload.

## Dependencies

- **Internal**: `color_management` (for white balance chromaticity), `image` (for auto-balance input)
- **External**: `glam` (vector math)

## Usage Examples

```rust
use crispen_core::grading::wheels::apply_cdl;

let lift = [0.0, 0.0, 0.0, 0.0];
let gamma = [1.0, 1.0, 1.0, 1.0];
let gain = [1.2, 1.0, 0.8, 1.0]; // warm tint
let offset = [0.0, 0.0, 0.0, 0.0];
let result = apply_cdl([0.5, 0.5, 0.5], &lift, &gamma, &gain, &offset);
```
