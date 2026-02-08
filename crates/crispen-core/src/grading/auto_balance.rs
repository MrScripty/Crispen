//! Automatic white balance and shot matching.

use crate::image::GradingImage;
use crate::transform::params::GradingParams;

/// Automatically determine white balance settings from image content.
///
/// Analyzes the image to find a neutral point and returns
/// `(temperature, tint)` adjustments.
pub fn auto_white_balance(image: &GradingImage) -> (f32, f32) {
    let _ = image;
    todo!()
}

/// Match grading parameters to a reference image.
///
/// Analyzes both images and produces `GradingParams` that make the
/// source image's color characteristics match the reference.
pub fn match_shot(source: &GradingImage, reference: &GradingImage) -> GradingParams {
    let _ = (source, reference);
    todo!()
}
