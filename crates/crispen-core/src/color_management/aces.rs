//! ACES color management transforms (IDT/ODT).
//!
//! Handles input device transforms (IDT) to convert camera-native color
//! into the ACES working space, and output device transforms (ODT) to
//! convert graded images for display.

use crate::transform::params::ColorManagementConfig;

/// Apply the input device transform based on the color management configuration.
///
/// Converts from the source color space to the working color space.
pub fn apply_input_transform(rgb: [f32; 3], config: &ColorManagementConfig) -> [f32; 3] {
    let _ = (rgb, config);
    todo!()
}

/// Apply the output device transform based on the color management configuration.
///
/// Converts from the working color space to the output/display color space.
pub fn apply_output_transform(rgb: [f32; 3], config: &ColorManagementConfig) -> [f32; 3] {
    let _ = (rgb, config);
    todo!()
}
