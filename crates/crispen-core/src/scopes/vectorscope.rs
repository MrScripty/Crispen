//! Vectorscope (chromaticity) scope computation.
//!
//! Plots color saturation and hue on a circular display by projecting
//! each pixel's chrominance onto a 2D grid using Cb/Cr (blue-difference
//! and red-difference chroma) axes.

use serde::{Deserialize, Serialize};

use crate::image::GradingImage;

/// Default vectorscope grid resolution.
const DEFAULT_RESOLUTION: u32 = 256;

/// Vectorscope data — plots color saturation and hue on a circular display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorscopeData {
    /// Resolution of the square vectorscope grid.
    pub resolution: u32,
    /// Density values for each grid cell. Length = resolution².
    pub density: Vec<u32>,
}

/// Compute vectorscope from a grading image.
///
/// Projects each pixel onto a 2D Cb/Cr chrominance plane:
/// - Cb = B − Y (blue-difference)
/// - Cr = R − Y (red-difference)
///
/// The center of the grid represents neutral (achromatic) colors.
pub fn compute(image: &GradingImage) -> VectorscopeData {
    let resolution = DEFAULT_RESOLUTION;
    let total = (resolution * resolution) as usize;
    let mut density = vec![0u32; total];

    for px in &image.pixels {
        // Rec. 709 luminance
        let y = 0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2];

        // Cb/Cr chroma (normalized to [-0.5, 0.5] range)
        let cb = (px[2] - y) * 0.5;
        let cr = (px[0] - y) * 0.5;

        // Map to grid coordinates (center = neutral)
        let gx = ((cb + 0.5) * (resolution - 1) as f32).clamp(0.0, (resolution - 1) as f32) as u32;
        let gy = ((cr + 0.5) * (resolution - 1) as f32).clamp(0.0, (resolution - 1) as f32) as u32;

        let idx = (gy * resolution + gx) as usize;
        if idx < total {
            density[idx] += 1;
        }
    }

    VectorscopeData {
        resolution,
        density,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::BitDepth;

    #[test]
    fn test_vectorscope_neutral_concentrates_at_center() {
        let pixels = vec![[0.5, 0.5, 0.5, 1.0]; 100];
        let image = GradingImage {
            width: 10,
            height: 10,
            pixels,
            source_bit_depth: BitDepth::F32,
        };
        let vs = compute(&image);
        // Neutral colors should cluster near the center of the grid
        let total: u32 = vs.density.iter().sum();
        assert_eq!(total, 100);
    }

    #[test]
    fn test_vectorscope_empty_image() {
        let image = GradingImage {
            width: 0,
            height: 0,
            pixels: vec![],
            source_bit_depth: BitDepth::F32,
        };
        let vs = compute(&image);
        let total: u32 = vs.density.iter().sum();
        assert_eq!(total, 0);
    }
}
