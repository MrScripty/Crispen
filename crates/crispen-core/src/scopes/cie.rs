//! CIE chromaticity diagram scope computation.
//!
//! Projects each pixel's color onto the CIE 1931 xy chromaticity diagram
//! by converting linear RGB to XYZ and then normalizing to xy coordinates.

use serde::{Deserialize, Serialize};

use crate::image::GradingImage;

/// Default CIE diagram grid resolution.
const DEFAULT_RESOLUTION: u32 = 256;

/// sRGB (Rec. 709) to XYZ D65 matrix (row-major).
const SRGB_TO_XYZ: [[f32; 3]; 3] = [
    [0.4123908, 0.3575843, 0.1804808],
    [0.2126390, 0.7151687, 0.0721923],
    [0.0193308, 0.1191948, 0.9505322],
];

/// CIE chromaticity diagram data — plots pixel colors on a CIE xy diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CieData {
    /// Resolution of the square CIE grid.
    pub resolution: u32,
    /// Density values for each grid cell. Length = resolution².
    pub density: Vec<u32>,
}

/// Compute CIE chromaticity diagram from a grading image.
///
/// For each pixel:
/// 1. Convert linear RGB to CIE XYZ using the Rec. 709 NPM
/// 2. Compute chromaticity: x = X/(X+Y+Z), y = Y/(X+Y+Z)
/// 3. Map (x, y) to the grid (x range [0, 0.8], y range [0, 0.9])
pub fn compute(image: &GradingImage) -> CieData {
    let resolution = DEFAULT_RESOLUTION;
    let total = (resolution * resolution) as usize;
    let mut density = vec![0u32; total];

    let res_f = (resolution - 1) as f32;

    for px in &image.pixels {
        let r = px[0];
        let g = px[1];
        let b = px[2];

        // Convert to XYZ
        let x_val = SRGB_TO_XYZ[0][0] * r + SRGB_TO_XYZ[0][1] * g + SRGB_TO_XYZ[0][2] * b;
        let y_val = SRGB_TO_XYZ[1][0] * r + SRGB_TO_XYZ[1][1] * g + SRGB_TO_XYZ[1][2] * b;
        let z_val = SRGB_TO_XYZ[2][0] * r + SRGB_TO_XYZ[2][1] * g + SRGB_TO_XYZ[2][2] * b;

        let sum = x_val + y_val + z_val;
        if sum < 1e-10 {
            continue;
        }

        // CIE xy chromaticity
        let cx = x_val / sum;
        let cy = y_val / sum;

        // Map to grid (x: [0, 0.8], y: [0, 0.9])
        let gx = (cx / 0.8 * res_f).clamp(0.0, res_f) as u32;
        // Invert y so top of grid = high y values
        let gy = ((1.0 - cy / 0.9) * res_f).clamp(0.0, res_f) as u32;

        let idx = (gy * resolution + gx) as usize;
        if idx < total {
            density[idx] += 1;
        }
    }

    CieData { resolution, density }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::BitDepth;

    #[test]
    fn test_cie_empty_image() {
        let image = GradingImage {
            width: 0, height: 0, pixels: vec![], source_bit_depth: BitDepth::F32,
        };
        let cie = compute(&image);
        let total: u32 = cie.density.iter().sum();
        assert_eq!(total, 0);
    }

    #[test]
    fn test_cie_white_pixel_near_d65() {
        let pixels = vec![[1.0, 1.0, 1.0, 1.0]; 1];
        let image = GradingImage {
            width: 1, height: 1, pixels, source_bit_depth: BitDepth::F32,
        };
        let cie = compute(&image);
        let total: u32 = cie.density.iter().sum();
        assert_eq!(total, 1, "one white pixel should produce one density point");
    }

    #[test]
    fn test_cie_black_pixel_skipped() {
        let pixels = vec![[0.0, 0.0, 0.0, 1.0]; 10];
        let image = GradingImage {
            width: 10, height: 1, pixels, source_bit_depth: BitDepth::F32,
        };
        let cie = compute(&image);
        let total: u32 = cie.density.iter().sum();
        assert_eq!(total, 0, "black pixels have no chromaticity");
    }
}
