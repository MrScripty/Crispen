//! Waveform (luma) scope computation.

use serde::{Deserialize, Serialize};

use crate::image::GradingImage;

/// Waveform scope data — plots pixel intensity vs. horizontal position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformData {
    /// Width of the waveform display in columns.
    pub width: u32,
    /// Height of the waveform display in rows.
    pub height: u32,
    /// Density data for R, G, B channels.
    pub data: [Vec<u32>; 3],
}

/// Compute waveform from a grading image.
pub fn compute(image: &GradingImage) -> WaveformData {
    let _ = image;
    todo!()
}
