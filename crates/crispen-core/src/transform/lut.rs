//! 3D LUT baking, application, and `.cube` file I/O.

use std::path::Path;

use crate::transform::params::GradingParams;

/// A 3D lookup table for fast color transform application.
///
/// The LUT maps input RGB values to graded output RGB values using
/// trilinear interpolation. Typical sizes are 33³ or 65³ entries.
#[derive(Debug, Clone)]
pub struct Lut3D {
    /// Grid size per axis (typically 33 or 65).
    pub size: u32,
    /// LUT entries as RGBA values. Length = size³.
    pub data: Vec<[f32; 4]>,
    /// Minimum domain values per channel.
    pub domain_min: [f32; 3],
    /// Maximum domain values per channel.
    pub domain_max: [f32; 3],
}

impl Lut3D {
    /// Bake the full grading transform into this 3D LUT.
    pub fn bake(&mut self, params: &GradingParams) {
        let _ = params;
        todo!()
    }

    /// Apply this LUT to an RGB pixel using trilinear interpolation.
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let _ = rgb;
        todo!()
    }

    /// Load a 3D LUT from a `.cube` file.
    pub fn load_cube(path: &Path) -> std::io::Result<Self> {
        let _ = path;
        todo!()
    }

    /// Save this 3D LUT to a `.cube` file.
    pub fn save_cube(&self, path: &Path) -> std::io::Result<()> {
        let _ = path;
        todo!()
    }
}
