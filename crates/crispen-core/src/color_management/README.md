# Color Management

## Purpose

Color space conversions and chromatic adaptation for the grading pipeline. All conversions route through CIE XYZ D65 as a hub, with Bradford chromatic adaptation for non-D65 white points (ACES).

## Contents

| File | Description |
|------|-------------|
| `mod.rs` | Module exports |
| `color_space.rs` | `ColorSpaceId` enum and 3x3 conversion matrices between 14 color spaces |
| `transfer.rs` | LOG transfer functions (LogC3/4, S-Log3, V-Log) — linearize/delinearize |
| `aces.rs` | ACES IDT/ODT matrices, ACEScg/cc/cct transforms |
| `white_balance.rs` | Temperature/tint chromaticity shift via Planckian locus approximation |

## Design Decisions

- **CIE XYZ D65 hub**: All conversions go through XYZ to avoid a quadratic explosion of direct conversion matrices.
- **Bradford adaptation**: Used for ACES color spaces with non-D65 white points, matching DaVinci Resolve's approach.
- **Enum-based IDs**: `ColorSpaceId` is an enum rather than strings for type safety and GPU-compatible `u32` mapping.

## Dependencies

- **Internal**: None (leaf module within crispen-core)
- **External**: `glam` (matrix math), `serde` (serialization)

## Usage Examples

```rust
use crispen_core::color_management::color_space::{ColorSpaceId, get_conversion_matrix};

let matrix = get_conversion_matrix(&ColorSpaceId::Srgb, &ColorSpaceId::AcesCg);
let acescg = matrix.transform([0.5, 0.3, 0.2]);
```
