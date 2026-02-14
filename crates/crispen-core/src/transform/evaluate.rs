//! Core transform evaluation — applies the full grading chain to a single pixel.
//!
//! This is the CPU reference implementation. The GPU `bake_lut.wgsl` shader
//! mirrors this function exactly to ensure visual consistency.

use crate::color_management::aces::{apply_input_transform, apply_output_transform};
use crate::color_management::white_balance::apply_white_balance;
use crate::grading::curves::apply_curves;
use crate::grading::sliders::{apply_contrast, apply_saturation_hue, apply_shadows_highlights};
use crate::grading::wheels::apply_cdl;
use crate::transform::params::GradingParams;

/// Apply the complete grading transform chain to a single RGB pixel.
///
/// This is the canonical transform order. Every grading adjustment feeds
/// into a single composite function that is baked into a 3D LUT:
///
/// ```text
/// Input RGB
///   │
///   ├─ 1. Input color space transform (linearize + gamut convert)
///   ├─ 2. White balance (Bradford chromatic adaptation)
///   ├─ 3. CDL (lift/gamma/gain/offset color wheels)
///   ├─ 4. Contrast with pivot
///   ├─ 5. Shadows/highlights recovery
///   ├─ 6. Saturation and hue rotation
///   ├─ 7. Curve adjustments (hue-vs-hue, hue-vs-sat, etc.)
///   ├─ 8. Output color space transform (gamut convert + encode)
///   │
///   └─→ Output RGB
/// ```
///
/// The GPU shader must match this order exactly.
pub fn evaluate_transform(rgb: [f32; 3], params: &GradingParams) -> [f32; 3] {
    let mut c = rgb;
    c = apply_input_transform(c, &params.color_management);
    c = apply_white_balance(c, params.temperature, params.tint);
    c = apply_cdl(
        c,
        &params.combined_lift(),
        &params.combined_gamma(),
        &params.combined_gain(),
        &params.combined_offset(),
    );
    c = apply_contrast(c, params.contrast, params.pivot);
    c = apply_shadows_highlights(c, params.shadows, params.highlights);
    c = apply_saturation_hue(c, params.saturation, params.hue, params.luma_mix);
    c = apply_curves(c, params);
    c = apply_output_transform(c, &params.color_management);
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn test_evaluate_transform_identity_params_produces_finite() {
        let params = GradingParams::default();
        let rgb = [0.5, 0.3, 0.7];
        let result = evaluate_transform(rgb, &params);
        for (i, channel) in result.iter().enumerate().take(3) {
            assert!(channel.is_finite(), "channel {i} should be finite");
        }
    }

    #[test]
    fn test_evaluate_transform_same_space_identity() {
        use crate::transform::params::{ColorManagementConfig, ColorSpaceId};
        let params = GradingParams {
            color_management: ColorManagementConfig {
                input_space: ColorSpaceId::AcesCg,
                working_space: ColorSpaceId::AcesCg,
                output_space: ColorSpaceId::AcesCg,
            },
            ..GradingParams::default()
        };
        let rgb = [0.5, 0.3, 0.7];
        let result = evaluate_transform(rgb, &params);
        for (i, channel) in result.iter().enumerate().take(3) {
            assert!(
                (*channel - rgb[i]).abs() < EPSILON,
                "channel {i}: {:.8} vs {:.8}",
                channel,
                rgb[i]
            );
        }
    }

    #[test]
    fn test_evaluate_transform_preserves_black() {
        use crate::transform::params::{ColorManagementConfig, ColorSpaceId};
        let params = GradingParams {
            color_management: ColorManagementConfig {
                input_space: ColorSpaceId::AcesCg,
                working_space: ColorSpaceId::AcesCg,
                output_space: ColorSpaceId::AcesCg,
            },
            ..GradingParams::default()
        };
        let result = evaluate_transform([0.0, 0.0, 0.0], &params);
        for channel in result.iter().take(3) {
            assert!(channel.abs() < EPSILON);
        }
    }
}
