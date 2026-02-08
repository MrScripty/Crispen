//! Image loading and format conversion for the demo application.

use std::path::Path;

use crispen_core::image::{BitDepth, GradingImage};

/// Load an image from disk and convert to the internal `GradingImage` format.
///
/// Supports common formats via the `image` crate (PNG, JPEG, TIFF, EXR).
/// All images are converted to RGBA f32 linear internally.
pub fn load_image(path: &Path) -> Result<GradingImage, ImageLoadError> {
    let img = image::open(path).map_err(ImageLoadError::Decode)?;
    let rgba = img.to_rgba32f();
    let (width, height) = rgba.dimensions();

    let pixels: Vec<[f32; 4]> = rgba
        .pixels()
        .map(|p| [p.0[0], p.0[1], p.0[2], p.0[3]])
        .collect();

    let bit_depth = match img.color() {
        image::ColorType::Rgb8 | image::ColorType::Rgba8 => BitDepth::U8,
        image::ColorType::Rgb16 | image::ColorType::Rgba16 => BitDepth::U16,
        image::ColorType::Rgb32F | image::ColorType::Rgba32F => BitDepth::F32,
        _ => BitDepth::U8,
    };

    Ok(GradingImage {
        width,
        height,
        pixels,
        source_bit_depth: bit_depth,
    })
}

/// Errors that can occur during image loading.
#[derive(Debug, thiserror::Error)]
pub enum ImageLoadError {
    #[error("failed to decode image: {0}")]
    Decode(image::ImageError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
