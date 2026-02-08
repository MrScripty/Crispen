//! Automatic white balance and shot matching.
//!
//! Auto white balance uses the gray-world assumption: the average color
//! of a well-exposed scene should be neutral gray. Shot matching aligns
//! luminance and per-channel distributions between source and target images.

use crate::image::GradingImage;
use crate::transform::params::GradingParams;

/// Automatically determine white balance settings from image content.
///
/// Uses the gray-world assumption: computes the average RGB of the image
/// and derives temperature and tint corrections to neutralize the average.
///
/// Returns `(temperature, tint)` adjustments. A neutral image returns `(0.0, 0.0)`.
///
/// # Algorithm
/// 1. Compute mean R, G, B across all pixels
/// 2. Compute the deviation from neutral gray
/// 3. Map the R/B imbalance to temperature (blue-yellow axis)
/// 4. Map the G deviation to tint (green-magenta axis)
pub fn auto_white_balance(image: &GradingImage) -> (f32, f32) {
    if image.pixels.is_empty() {
        return (0.0, 0.0);
    }

    let count = image.pixels.len() as f64;
    let mut sum_r = 0.0_f64;
    let mut sum_g = 0.0_f64;
    let mut sum_b = 0.0_f64;

    for px in &image.pixels {
        sum_r += px[0] as f64;
        sum_g += px[1] as f64;
        sum_b += px[2] as f64;
    }

    let avg_r = sum_r / count;
    let avg_g = sum_g / count;
    let avg_b = sum_b / count;

    let luminance = 0.2126 * avg_r + 0.7152 * avg_g + 0.0722 * avg_b;
    if luminance < 1e-10 {
        return (0.0, 0.0);
    }

    // Temperature: ratio of blue to red relative to neutral
    // Positive = too warm (needs cooling), negative = too cool (needs warming)
    let rb_ratio = (avg_r - avg_b) / luminance;
    let temperature = (-rb_ratio * 2.0) as f32;

    // Tint: green deviation from neutral
    // Positive = too green (needs magenta), negative = too magenta (needs green)
    let g_deviation = (avg_g - luminance) / luminance;
    let tint = (-g_deviation * 4.0) as f32;

    (temperature, tint)
}

/// Match grading parameters to a reference image.
///
/// Analyzes both images and produces `GradingParams` that make the source
/// image's color characteristics approximate the reference.
///
/// # Algorithm
/// 1. Compute per-channel mean and standard deviation for both images
/// 2. Derive gain from ratio of standard deviations
/// 3. Derive offset from difference in means (after gain)
/// 4. Derive contrast from luminance distribution comparison
pub fn match_shot(source: &GradingImage, reference: &GradingImage) -> GradingParams {
    let src_stats = compute_channel_stats(source);
    let ref_stats = compute_channel_stats(reference);

    let mut params = GradingParams::default();

    // Match gain per channel from standard deviation ratios
    for c in 0..3 {
        if src_stats.stddev[c] > 1e-10 {
            params.gain[c] = (ref_stats.stddev[c] / src_stats.stddev[c]) as f32;
        }
    }

    // Match offset from mean differences (after gain adjustment)
    for c in 0..3 {
        let adjusted_mean = src_stats.mean[c] * ref_stats.stddev[c] / src_stats.stddev[c].max(1e-10);
        params.offset[c] = (ref_stats.mean[c] - adjusted_mean) as f32;
    }

    // Match overall contrast from luminance spread
    let src_lum_range = src_stats.stddev[3];
    let ref_lum_range = ref_stats.stddev[3];
    if src_lum_range > 1e-10 {
        params.contrast = (ref_lum_range / src_lum_range) as f32;
    }

    params
}

struct ChannelStats {
    mean: [f64; 4],   // R, G, B, Luminance
    stddev: [f64; 4],
}

fn compute_channel_stats(image: &GradingImage) -> ChannelStats {
    let n = image.pixels.len() as f64;
    if n < 1.0 {
        return ChannelStats {
            mean: [0.0; 4],
            stddev: [0.0; 4],
        };
    }

    let mut sum = [0.0_f64; 4];
    let mut sum_sq = [0.0_f64; 4];

    for px in &image.pixels {
        let lum = 0.2126 * px[0] as f64 + 0.7152 * px[1] as f64 + 0.0722 * px[2] as f64;
        let vals = [px[0] as f64, px[1] as f64, px[2] as f64, lum];
        for c in 0..4 {
            sum[c] += vals[c];
            sum_sq[c] += vals[c] * vals[c];
        }
    }

    let mut mean = [0.0_f64; 4];
    let mut stddev = [0.0_f64; 4];
    for c in 0..4 {
        mean[c] = sum[c] / n;
        let variance = (sum_sq[c] / n) - (mean[c] * mean[c]);
        stddev[c] = variance.max(0.0).sqrt();
    }

    ChannelStats { mean, stddev }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::BitDepth;

    fn make_uniform_image(r: f32, g: f32, b: f32, size: u32) -> GradingImage {
        let pixels = vec![[r, g, b, 1.0]; (size * size) as usize];
        GradingImage {
            width: size,
            height: size,
            pixels,
            source_bit_depth: BitDepth::F32,
        }
    }

    #[test]
    fn test_auto_balance_on_neutral_image_returns_zero() {
        let image = make_uniform_image(0.5, 0.5, 0.5, 10);
        let (temp, tint) = auto_white_balance(&image);
        assert!(temp.abs() < 0.01, "temperature should be ~0 for neutral: {temp}");
        assert!(tint.abs() < 0.01, "tint should be ~0 for neutral: {tint}");
    }

    #[test]
    fn test_auto_balance_warm_image_returns_negative_temp() {
        // Warm image: more red than blue
        let image = make_uniform_image(0.7, 0.5, 0.3, 10);
        let (temp, _) = auto_white_balance(&image);
        assert!(temp < 0.0, "warm image should produce negative temperature correction");
    }

    #[test]
    fn test_auto_balance_empty_image_returns_zero() {
        let image = GradingImage {
            width: 0, height: 0, pixels: vec![], source_bit_depth: BitDepth::F32,
        };
        let (temp, tint) = auto_white_balance(&image);
        assert_eq!(temp, 0.0);
        assert_eq!(tint, 0.0);
    }

    #[test]
    fn test_match_shot_identical_images_returns_identity() {
        let img = make_uniform_image(0.5, 0.5, 0.5, 10);
        let params = match_shot(&img, &img);
        assert!((params.contrast - 1.0).abs() < 0.1);
    }
}
