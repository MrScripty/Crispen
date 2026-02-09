//! Image loading and format conversion for the demo application.

use std::path::Path;

use crispen_core::image::{BitDepth, GradingImage};
use image::imageops::FilterType;

/// Load an image from disk and convert to the internal `GradingImage` format.
///
/// Supports common formats via the `image` crate (PNG, JPEG, TIFF, EXR).
/// All images are converted to RGBA f32 linear internally.
#[allow(dead_code)]
pub fn load_image(path: &Path) -> Result<GradingImage, ImageLoadError> {
    load_image_for_display(path, None)
}

/// Load an image and optionally downscale to a display target.
///
/// `max_display_size` is interpreted as `(max_width, max_height)` in pixels.
/// If the source image exceeds either dimension it is downscaled preserving
/// aspect ratio before conversion to RGBA f32.
pub fn load_image_for_display(
    path: &Path,
    max_display_size: Option<(u32, u32)>,
) -> Result<GradingImage, ImageLoadError> {
    let img = image::open(path).map_err(ImageLoadError::Decode)?;
    let original_bit_depth = match img.color() {
        image::ColorType::Rgb8 | image::ColorType::Rgba8 => BitDepth::U8,
        image::ColorType::Rgb16 | image::ColorType::Rgba16 => BitDepth::U16,
        image::ColorType::Rgb32F | image::ColorType::Rgba32F => BitDepth::F32,
        _ => BitDepth::U8,
    };

    let img = maybe_resize_to_fit(img, max_display_size);
    let rgba = img.to_rgba32f();
    let (width, height) = rgba.dimensions();

    let pixels: Vec<[f32; 4]> = rgba
        .pixels()
        .map(|p| [p.0[0], p.0[1], p.0[2], p.0[3]])
        .collect();

    Ok(GradingImage {
        width,
        height,
        pixels,
        source_bit_depth: original_bit_depth,
    })
}

fn maybe_resize_to_fit(
    image: image::DynamicImage,
    max_display_size: Option<(u32, u32)>,
) -> image::DynamicImage {
    let Some((max_w, max_h)) = max_display_size else {
        return image;
    };
    if max_w == 0 || max_h == 0 {
        return image;
    }

    let src_w = image.width();
    let src_h = image.height();
    if src_w <= max_w && src_h <= max_h {
        return image;
    }

    let scale_w = max_w as f32 / src_w as f32;
    let scale_h = max_h as f32 / src_h as f32;
    let scale = scale_w.min(scale_h);
    let dst_w = (src_w as f32 * scale).round().max(1.0) as u32;
    let dst_h = (src_h as f32 * scale).round().max(1.0) as u32;

    image.resize_exact(dst_w, dst_h, FilterType::Triangle)
}

/// Errors that can occur during image loading.
#[derive(Debug, thiserror::Error)]
pub enum ImageLoadError {
    #[error("failed to decode image: {0}")]
    Decode(image::ImageError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
