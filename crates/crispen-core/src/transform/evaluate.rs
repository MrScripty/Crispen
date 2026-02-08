//! Core transform evaluation — applies the full grading chain to a single pixel.

use crate::transform::params::GradingParams;

/// The core function. GPU `bake_lut.wgsl` mirrors this exactly.
///
/// Applies the complete grading transform chain to a single RGB pixel:
/// 1. Input color space transform
/// 2. White balance
/// 3. CDL (lift/gamma/gain/offset)
/// 4. Contrast with pivot
/// 5. Shadows/highlights recovery
/// 6. Saturation and hue rotation
/// 7. Curve adjustments
/// 8. Output color space transform
pub fn evaluate_transform(rgb: [f32; 3], params: &GradingParams) -> [f32; 3] {
    let _ = (rgb, params);
    todo!()
}
