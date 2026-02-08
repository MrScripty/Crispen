//! RGB + luminance histogram computation.

use serde::{Deserialize, Serialize};

use crate::image::GradingImage;

/// Number of bins per channel.
const NUM_BINS: usize = 256;

/// Rec. 709 luminance weights.
const LUMA_R: f32 = 0.2126;
const LUMA_G: f32 = 0.7152;
const LUMA_B: f32 = 0.0722;

/// Histogram data for R, G, B, and luminance channels (256 bins each).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramData {
    /// Bin counts for `[R, G, B, Luma]` channels. Each `Vec` has 256 entries.
    pub bins: [Vec<u32>; 4],
    /// Peak bin value across all channels (for normalization).
    pub peak: u32,
}

/// Compute histogram from a grading image.
///
/// Maps each pixel's R, G, B, and luminance values to 256 bins spanning [0, 1].
/// Values outside [0, 1] are clamped.
pub fn compute(image: &GradingImage) -> HistogramData {
    let mut bins = [
        vec![0u32; NUM_BINS],
        vec![0u32; NUM_BINS],
        vec![0u32; NUM_BINS],
        vec![0u32; NUM_BINS],
    ];

    for px in &image.pixels {
        let r_bin = (px[0].clamp(0.0, 1.0) * 255.0) as usize;
        let g_bin = (px[1].clamp(0.0, 1.0) * 255.0) as usize;
        let b_bin = (px[2].clamp(0.0, 1.0) * 255.0) as usize;
        let luma = (px[0] * LUMA_R + px[1] * LUMA_G + px[2] * LUMA_B).clamp(0.0, 1.0);
        let l_bin = (luma * 255.0) as usize;

        bins[0][r_bin.min(NUM_BINS - 1)] += 1;
        bins[1][g_bin.min(NUM_BINS - 1)] += 1;
        bins[2][b_bin.min(NUM_BINS - 1)] += 1;
        bins[3][l_bin.min(NUM_BINS - 1)] += 1;
    }

    let peak = bins
        .iter()
        .flat_map(|b| b.iter())
        .copied()
        .max()
        .unwrap_or(0);

    HistogramData { bins, peak }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::BitDepth;

    #[test]
    fn test_histogram_bins_sum_to_pixel_count() {
        let pixels = vec![[0.5, 0.3, 0.7, 1.0]; 100];
        let image = GradingImage {
            width: 10,
            height: 10,
            pixels,
            source_bit_depth: BitDepth::F32,
        };
        let hist = compute(&image);
        for ch in 0..4 {
            let sum: u32 = hist.bins[ch].iter().sum();
            assert_eq!(sum, 100, "channel {ch} bins should sum to pixel count");
        }
    }

    #[test]
    fn test_histogram_uniform_image_single_bin() {
        let pixels = vec![[0.5, 0.5, 0.5, 1.0]; 50];
        let image = GradingImage {
            width: 10,
            height: 5,
            pixels,
            source_bit_depth: BitDepth::F32,
        };
        let hist = compute(&image);
        // All pixels at 0.5 → bin 127 or 128
        let bin = (0.5_f32 * 255.0) as usize;
        assert_eq!(hist.bins[0][bin], 50);
    }

    #[test]
    fn test_histogram_empty_image() {
        let image = GradingImage {
            width: 0,
            height: 0,
            pixels: vec![],
            source_bit_depth: BitDepth::F32,
        };
        let hist = compute(&image);
        assert_eq!(hist.peak, 0);
    }
}
