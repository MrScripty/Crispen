//! Image representation for the color grading pipeline.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported bit depths for source images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BitDepth {
    /// 8-bit unsigned integer.
    U8,
    /// 10-bit unsigned integer.
    U10,
    /// 12-bit unsigned integer.
    U12,
    /// 16-bit unsigned integer.
    U16,
    /// 16-bit floating point.
    F16,
    /// 32-bit floating point.
    F32,
}

impl fmt::Display for BitDepth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::U8 => write!(f, "8-bit"),
            Self::U10 => write!(f, "10-bit"),
            Self::U12 => write!(f, "12-bit"),
            Self::U16 => write!(f, "16-bit"),
            Self::F16 => write!(f, "16-bit float"),
            Self::F32 => write!(f, "32-bit float"),
        }
    }
}

impl From<u8> for BitDepth {
    fn from(bits: u8) -> Self {
        match bits {
            8 => Self::U8,
            10 => Self::U10,
            12 => Self::U12,
            16 => Self::U16,
            _ => Self::U8,
        }
    }
}

/// Internal image representation. Always stored as RGBA f32 linear.
#[derive(Debug, Clone)]
pub struct GradingImage {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Pixel data in RGBA f32 linear format.
    pub pixels: Vec<[f32; 4]>,
    /// Original bit depth of the source image.
    pub source_bit_depth: BitDepth,
}
