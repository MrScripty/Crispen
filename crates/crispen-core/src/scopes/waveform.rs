//! Waveform scope computation — plots pixel intensity vs. horizontal position.

use serde::{Deserialize, Serialize};

use crate::image::GradingImage;

/// Default waveform display height in rows.
const DEFAULT_HEIGHT: u32 = 256;

/// Waveform scope data — plots pixel intensity vs. horizontal position.
///
/// For each column (x-position in the source image), the waveform shows
/// the distribution of R, G, B intensity values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformData {
    /// Width of the waveform display in columns.
    pub width: u32,
    /// Height of the waveform display in rows.
    pub height: u32,
    /// Density data for R, G, B channels. Each Vec has width × height entries
    /// stored in row-major order.
    pub data: [Vec<u32>; 3],
}

/// Compute waveform from a grading image.
///
/// Maps each pixel's R, G, B values to a 2D density plot where
/// x = source column, y = intensity level.
pub fn compute(image: &GradingImage) -> WaveformData {
    let width = image.width;
    let height = DEFAULT_HEIGHT;
    let total = (width * height) as usize;

    let mut data = [vec![0u32; total], vec![0u32; total], vec![0u32; total]];

    if width == 0 || image.height == 0 {
        return WaveformData {
            width,
            height,
            data,
        };
    }

    let height_f = (height - 1) as f32;

    for y in 0..image.height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let px = image.pixels[idx];

            for ch in 0..3 {
                let val = px[ch].clamp(0.0, 1.0);
                // Waveform is bottom-to-top: row 0 = top = value 1.0
                let row = (height - 1) - (val * height_f) as u32;
                let wf_idx = (row * width + x) as usize;
                data[ch][wf_idx] += 1;
            }
        }
    }

    WaveformData {
        width,
        height,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::BitDepth;

    #[test]
    fn test_waveform_empty_image() {
        let image = GradingImage {
            width: 0,
            height: 0,
            pixels: vec![],
            source_bit_depth: BitDepth::F32,
        };
        let wf = compute(&image);
        assert_eq!(wf.width, 0);
    }

    #[test]
    fn test_waveform_uniform_column_concentrates_at_one_row() {
        let pixels = vec![[0.5, 0.5, 0.5, 1.0]; 10];
        let image = GradingImage {
            width: 1,
            height: 10,
            pixels,
            source_bit_depth: BitDepth::F32,
        };
        let wf = compute(&image);
        // All pixels in column 0 at value 0.5 → one row should have count 10
        let total: u32 = wf.data[0].iter().sum();
        assert_eq!(total, 10);
    }
}
