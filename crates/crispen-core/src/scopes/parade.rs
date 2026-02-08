//! RGB parade scope computation.
//!
//! Generates separate waveforms for R, G, B channels displayed side by side.
//! Each channel's waveform shows intensity distribution vs. horizontal position.

use serde::{Deserialize, Serialize};

use crate::image::GradingImage;

/// Default parade display height in rows.
const DEFAULT_HEIGHT: u32 = 256;

/// Parade scope data — separate waveforms for R, G, B channels side by side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParadeData {
    /// Width of each channel's waveform display.
    pub width: u32,
    /// Height of each channel's waveform display.
    pub height: u32,
    /// Red channel density data (width × height, row-major).
    pub red: Vec<u32>,
    /// Green channel density data (width × height, row-major).
    pub green: Vec<u32>,
    /// Blue channel density data (width × height, row-major).
    pub blue: Vec<u32>,
}

/// Compute parade from a grading image.
///
/// Each channel gets its own waveform plot where x = source column
/// and y = that channel's intensity level.
pub fn compute(image: &GradingImage) -> ParadeData {
    let width = image.width;
    let height = DEFAULT_HEIGHT;
    let total = (width * height) as usize;

    let mut red = vec![0u32; total];
    let mut green = vec![0u32; total];
    let mut blue = vec![0u32; total];

    if width == 0 || image.height == 0 {
        return ParadeData { width, height, red, green, blue };
    }

    let height_f = (height - 1) as f32;

    for y in 0..image.height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let px = image.pixels[idx];

            let channels = [px[0], px[1], px[2]];
            let buffers = [&mut red, &mut green, &mut blue];

            for (ch, buf) in channels.iter().zip(buffers) {
                let val = ch.clamp(0.0, 1.0);
                let row = (height - 1) - (val * height_f) as u32;
                let wf_idx = (row * width + x) as usize;
                buf[wf_idx] += 1;
            }
        }
    }

    ParadeData { width, height, red, green, blue }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::BitDepth;

    #[test]
    fn test_parade_empty_image() {
        let image = GradingImage {
            width: 0, height: 0, pixels: vec![], source_bit_depth: BitDepth::F32,
        };
        let pd = compute(&image);
        assert_eq!(pd.width, 0);
    }

    #[test]
    fn test_parade_pixel_counts_match() {
        let pixels = vec![[0.3, 0.6, 0.9, 1.0]; 20];
        let image = GradingImage {
            width: 4, height: 5, pixels, source_bit_depth: BitDepth::F32,
        };
        let pd = compute(&image);
        let r_total: u32 = pd.red.iter().sum();
        let g_total: u32 = pd.green.iter().sum();
        let b_total: u32 = pd.blue.iter().sum();
        assert_eq!(r_total, 20);
        assert_eq!(g_total, 20);
        assert_eq!(b_total, 20);
    }
}
