//! RGB + luminance histogram computation.

use serde::{Deserialize, Serialize};

use crate::image::GradingImage;

/// Histogram data for R, G, B, and luminance channels (256 bins each).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramData {
    /// Bin counts for `[R, G, B, Luma]` channels. Each `Vec` has 256 entries.
    pub bins: [Vec<u32>; 4],
    /// Peak bin value across all channels (for normalization).
    pub peak: u32,
}

/// Compute histogram from a grading image.
pub fn compute(image: &GradingImage) -> HistogramData {
    let _ = image;
    todo!()
}
