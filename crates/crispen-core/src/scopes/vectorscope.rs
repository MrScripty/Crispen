//! Vectorscope (chromaticity) scope computation.

use serde::{Deserialize, Serialize};

use crate::image::GradingImage;

/// Vectorscope data — plots color saturation and hue on a circular display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorscopeData {
    /// Resolution of the square vectorscope grid.
    pub resolution: u32,
    /// Density values for each grid cell. Length = resolution².
    pub density: Vec<u32>,
}

/// Compute vectorscope from a grading image.
pub fn compute(image: &GradingImage) -> VectorscopeData {
    let _ = image;
    todo!()
}
