# Scopes

## Purpose

CPU-based scope computation for color analysis. Each scope type produces a data structure that can be serialized and sent to the Svelte UI for rendering. These serve as both the primary scope engine and a reference for GPU scope validation.

## Contents

| File | Description |
|------|-------------|
| `mod.rs` | Module exports and re-exports of data types |
| `histogram.rs` | RGB + luminance histogram (256 bins per channel) |
| `waveform.rs` | Intensity vs. horizontal position density plot |
| `vectorscope.rs` | Cb/Cr chrominance 2D density map |
| `parade.rs` | RGB parade (separate waveforms per channel) |
| `cie.rs` | CIE 1931 xy chromaticity diagram projection |

## Design Decisions

- **CPU implementation**: Provides reference results for GPU scope validation and works as a fallback when GPU is unavailable.
- **Fixed bin counts**: 256 bins for histogram/waveform matches standard 8-bit display; vectorscope/CIE use configurable resolution.
- **Rec. 709 luminance**: All luminance calculations use Rec. 709 weights (0.2126, 0.7152, 0.0722).

## Dependencies

- **Internal**: `image` (`GradingImage` as input)
- **External**: `serde` (serialization of scope data structs)

## Usage Examples

```rust
use crispen_core::scopes::histogram;
use crispen_core::image::GradingImage;

let image: GradingImage = /* loaded image */;
let hist = histogram::compute(&image);
// hist.bins[0] = red channel, hist.bins[3] = luminance
```
