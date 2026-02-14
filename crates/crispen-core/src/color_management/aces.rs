//! ACES color management transforms (IDT/ODT).
//!
//! Handles input device transforms (IDT) to convert camera-native color
//! into the ACES working space, and output device transforms (ODT) to
//! convert graded images for display.
//!
//! The pipeline is:
//! ```text
//! Input (encoded) → linearize → matrix to working → [grading] → matrix to output → encode
//! ```

use crate::color_management::color_space::get_conversion_matrix;
use crate::color_management::transfer::get_transfer;
use crate::transform::params::ColorManagementConfig;

/// Apply the input device transform based on the color management configuration.
///
/// Converts from the source color space to the working color space:
/// 1. Linearize via transfer function (if source is non-linear)
/// 2. Matrix convert from source gamut to working gamut
///
/// Returns the input unchanged if source == working and both are linear.
pub fn apply_input_transform(rgb: [f32; 3], config: &ColorManagementConfig) -> [f32; 3] {
    if config.input_space == config.working_space {
        return rgb;
    }

    // Step 1: Linearize the input if it has a non-linear transfer function
    let linear = if let Some(tf) = get_transfer(config.input_space) {
        [
            tf.to_linear(rgb[0]),
            tf.to_linear(rgb[1]),
            tf.to_linear(rgb[2]),
        ]
    } else {
        rgb
    };

    // Step 2: Matrix convert from source gamut to working gamut
    let matrix = get_conversion_matrix(config.input_space, config.working_space);
    matrix.apply(linear)
}

/// Apply the output device transform based on the color management configuration.
///
/// Converts from the working color space to the output/display color space:
/// 1. Matrix convert from working gamut to output gamut
/// 2. Apply output transfer function (if output is non-linear)
///
/// Returns the input unchanged if working == output and both are linear.
pub fn apply_output_transform(rgb: [f32; 3], config: &ColorManagementConfig) -> [f32; 3] {
    if config.working_space == config.output_space {
        return rgb;
    }

    // Step 1: Matrix convert from working gamut to output gamut
    let matrix = get_conversion_matrix(config.working_space, config.output_space);
    let converted = matrix.apply(rgb);

    // Step 2: Apply output transfer function if non-linear
    if let Some(tf) = get_transfer(config.output_space) {
        [
            tf.to_encoded(converted[0]),
            tf.to_encoded(converted[1]),
            tf.to_encoded(converted[2]),
        ]
    } else {
        converted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::params::{ColorSpaceId, DisplayOetf};

    const EPSILON: f32 = 1e-4;

    #[test]
    fn test_input_transform_identity_when_same_space() {
        let config = ColorManagementConfig {
            input_space: ColorSpaceId::AcesCg,
            working_space: ColorSpaceId::AcesCg,
            output_space: ColorSpaceId::Srgb,
            display_oetf: DisplayOetf::Srgb,
        };
        let rgb = [0.5, 0.3, 0.7];
        assert_eq!(apply_input_transform(rgb, &config), rgb);
    }

    #[test]
    fn test_output_transform_identity_when_same_space() {
        let config = ColorManagementConfig {
            input_space: ColorSpaceId::Srgb,
            working_space: ColorSpaceId::AcesCg,
            output_space: ColorSpaceId::AcesCg,
            display_oetf: DisplayOetf::Srgb,
        };
        let rgb = [0.5, 0.3, 0.7];
        assert_eq!(apply_output_transform(rgb, &config), rgb);
    }

    #[test]
    fn test_input_output_roundtrip_linear_spaces() {
        let config = ColorManagementConfig {
            input_space: ColorSpaceId::LinearSrgb,
            working_space: ColorSpaceId::AcesCg,
            output_space: ColorSpaceId::LinearSrgb,
            display_oetf: DisplayOetf::Srgb,
        };
        let rgb = [0.5, 0.3, 0.7];
        let working = apply_input_transform(rgb, &config);
        let back = apply_output_transform(working, &config);
        for i in 0..3 {
            assert!(
                (rgb[i] - back[i]).abs() < EPSILON,
                "channel {i}: {:.6} vs {:.6}",
                rgb[i],
                back[i]
            );
        }
    }
}
