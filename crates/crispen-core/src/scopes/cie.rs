//! CIE chromaticity diagram scope computation.

use serde::{Deserialize, Serialize};

use crate::image::GradingImage;

/// CIE chromaticity diagram data — plots pixel colors on a CIE xy diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CieData {
    /// Resolution of the square CIE grid.
    pub resolution: u32,
    /// Density values for each grid cell. Length = resolution².
    pub density: Vec<u32>,
}

/// Compute CIE chromaticity diagram from a grading image.
pub fn compute(image: &GradingImage) -> CieData {
    let _ = image;
    todo!()
}
