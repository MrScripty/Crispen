//! Color space definitions and 3x3 matrix transforms.

pub use crate::transform::params::ColorSpaceId;

/// A 3x3 color matrix for linear color space conversions.
#[derive(Debug, Clone, Copy)]
pub struct ColorMatrix(pub [[f32; 3]; 3]);

impl ColorMatrix {
    /// Returns the identity matrix (no-op transform).
    pub fn identity() -> Self {
        Self([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    }

    /// Apply this matrix to an RGB triplet.
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let _ = rgb;
        todo!()
    }
}

/// Get the 3x3 transform matrix to convert between color spaces.
pub fn get_conversion_matrix(from: ColorSpaceId, to: ColorSpaceId) -> ColorMatrix {
    let _ = (from, to);
    todo!()
}
