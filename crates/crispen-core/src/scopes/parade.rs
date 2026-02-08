//! RGB parade scope computation.

use serde::{Deserialize, Serialize};

use crate::image::GradingImage;

/// Parade scope data — separate waveforms for R, G, B channels side by side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParadeData {
    /// Width of each channel's waveform display.
    pub width: u32,
    /// Height of each channel's waveform display.
    pub height: u32,
    /// Red channel density data.
    pub red: Vec<u32>,
    /// Green channel density data.
    pub green: Vec<u32>,
    /// Blue channel density data.
    pub blue: Vec<u32>,
}

/// Compute parade from a grading image.
pub fn compute(image: &GradingImage) -> ParadeData {
    let _ = image;
    todo!()
}
