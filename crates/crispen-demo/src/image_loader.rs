//! Image loading and format conversion for the demo application.

use std::path::Path;

use crispen_core::image::{BitDepth, GradingImage};
use crispen_core::transform::params::ColorSpaceId;
use image::imageops::FilterType;

/// Result of loading an image, including optional detected color space.
pub struct LoadedImage {
    pub image: GradingImage,
    /// Color space detected from file metadata (OIIO's `oiio:ColorSpace`).
    /// `None` when loaded via the `image` crate fallback or when no metadata
    /// was present.
    pub detected_color_space: Option<String>,
}

/// Load an image from disk and convert to the internal `GradingImage` format.
///
/// Supports common formats via the `image` crate (PNG, JPEG, TIFF, EXR).
/// All images are converted to RGBA f32 linear internally.
#[allow(dead_code)]
pub fn load_image(path: &Path) -> Result<LoadedImage, ImageLoadError> {
    load_image_for_display(path, None)
}

/// Load an image via OpenImageIO when the `ocio` feature is enabled.
///
/// OIIO supports 100+ formats and auto-detects the color space from file
/// metadata. The detected color space string matches OCIO config names.
#[cfg(feature = "ocio")]
pub fn load_image_oiio(
    path: &Path,
    max_display_size: Option<(u32, u32)>,
) -> Result<LoadedImage, ImageLoadError> {
    let input = crispen_oiio::OiioImageInput::open(path).map_err(ImageLoadError::Oiio)?;

    let color_space = input.color_space();
    let bit_depth = input.bit_depth();
    let pixels = input.read_rgba_f32().map_err(ImageLoadError::Oiio)?;
    let width = input.width();
    let height = input.height();

    let image = GradingImage {
        width,
        height,
        pixels,
        source_bit_depth: bit_depth,
    };

    let image = maybe_resize_grading_image(image, max_display_size);

    Ok(LoadedImage {
        image,
        detected_color_space: color_space,
    })
}

/// Load an image and optionally downscale to a display target.
///
/// `max_display_size` is interpreted as `(max_width, max_height)` in pixels.
/// If the source image exceeds either dimension it is downscaled preserving
/// aspect ratio before conversion to RGBA f32.
pub fn load_image_for_display(
    path: &Path,
    max_display_size: Option<(u32, u32)>,
) -> Result<LoadedImage, ImageLoadError> {
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

    Ok(LoadedImage {
        image: GradingImage {
            width,
            height,
            pixels,
            source_bit_depth: original_bit_depth,
        },
        detected_color_space: None,
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

/// Downscale a `GradingImage` if it exceeds the given display bounds.
///
/// Uses simple bilinear interpolation on the f32 pixel data.
#[cfg_attr(not(feature = "ocio"), allow(dead_code))]
fn maybe_resize_grading_image(
    image: GradingImage,
    max_display_size: Option<(u32, u32)>,
) -> GradingImage {
    let Some((max_w, max_h)) = max_display_size else {
        return image;
    };
    if max_w == 0 || max_h == 0 || (image.width <= max_w && image.height <= max_h) {
        return image;
    }

    let scale_w = max_w as f32 / image.width as f32;
    let scale_h = max_h as f32 / image.height as f32;
    let scale = scale_w.min(scale_h);
    let dst_w = (image.width as f32 * scale).round().max(1.0) as u32;
    let dst_h = (image.height as f32 * scale).round().max(1.0) as u32;

    let src_w = image.width as usize;
    let mut dst_pixels = Vec::with_capacity(dst_w as usize * dst_h as usize);

    for y in 0..dst_h {
        for x in 0..dst_w {
            let src_x = (x as f32 + 0.5) / scale - 0.5;
            let src_y = (y as f32 + 0.5) / scale - 0.5;

            let x0 = (src_x.floor() as i32).clamp(0, image.width as i32 - 1) as usize;
            let y0 = (src_y.floor() as i32).clamp(0, image.height as i32 - 1) as usize;
            let x1 = (x0 + 1).min(image.width as usize - 1);
            let y1 = (y0 + 1).min(image.height as usize - 1);

            let fx = src_x - src_x.floor();
            let fy = src_y - src_y.floor();

            let p00 = image.pixels[y0 * src_w + x0];
            let p10 = image.pixels[y0 * src_w + x1];
            let p01 = image.pixels[y1 * src_w + x0];
            let p11 = image.pixels[y1 * src_w + x1];

            let mut pixel = [0.0f32; 4];
            for c in 0..4 {
                let top = p00[c] * (1.0 - fx) + p10[c] * fx;
                let bot = p01[c] * (1.0 - fx) + p11[c] * fx;
                pixel[c] = top * (1.0 - fy) + bot * fy;
            }
            dst_pixels.push(pixel);
        }
    }

    GradingImage {
        width: dst_w,
        height: dst_h,
        pixels: dst_pixels,
        source_bit_depth: image.source_bit_depth,
    }
}

/// Map an OIIO-detected color space string to a [`ColorSpaceId`].
///
/// Falls back to inferring from bit depth when the string is unrecognised.
pub fn detected_color_space_to_id(
    name: Option<&str>,
    bit_depth: BitDepth,
) -> ColorSpaceId {
    if let Some(name) = name {
        let lower = name.to_ascii_lowercase();
        // Match common OIIO / OCIO color space names.
        if lower == "srgb" || lower.contains("srgb") && !lower.contains("linear") {
            return ColorSpaceId::Srgb;
        }
        if lower.contains("linear") && (lower.contains("srgb") || lower.contains("709")) {
            return ColorSpaceId::LinearSrgb;
        }
        if lower.contains("acescg") {
            return ColorSpaceId::AcesCg;
        }
        if lower.contains("aces2065") || lower == "aces" {
            return ColorSpaceId::Aces2065_1;
        }
        if lower.contains("acescc") && !lower.contains("acescct") {
            return ColorSpaceId::AcesCc;
        }
        if lower.contains("acescct") {
            return ColorSpaceId::AcesCct;
        }
        if lower.contains("logc3") || lower.contains("logc ei") {
            return ColorSpaceId::ArriLogC3;
        }
        if lower.contains("logc4") {
            return ColorSpaceId::ArriLogC4;
        }
        if lower.contains("slog3") || lower.contains("s-log3") {
            return ColorSpaceId::SLog3;
        }
        if lower.contains("log3g10") || lower.contains("redlog") {
            return ColorSpaceId::RedLog3G10;
        }
        if lower.contains("vlog") || lower.contains("v-log") {
            return ColorSpaceId::VLog;
        }
        if lower.contains("rec2020") || lower.contains("rec.2020") || lower.contains("bt.2020") {
            return ColorSpaceId::Rec2020;
        }
        if lower.contains("p3") {
            return ColorSpaceId::DciP3;
        }
    }

    // Fallback: infer from bit depth.
    infer_input_space_from_bit_depth(bit_depth)
}

/// Infer a reasonable input color space from the source bit depth.
///
/// Standard 8/16-bit images (JPEG, PNG, etc.) are almost always sRGB-encoded.
/// Float images (EXR, HDR) are typically linear scene-referred.
pub fn infer_input_space_from_bit_depth(bit_depth: BitDepth) -> ColorSpaceId {
    match bit_depth {
        BitDepth::U8 | BitDepth::U10 | BitDepth::U12 | BitDepth::U16 => ColorSpaceId::Srgb,
        BitDepth::F16 | BitDepth::F32 => ColorSpaceId::LinearSrgb,
    }
}

/// Errors that can occur during image loading.
#[derive(Debug, thiserror::Error)]
pub enum ImageLoadError {
    #[error("failed to decode image: {0}")]
    Decode(image::ImageError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(feature = "ocio")]
    #[error("OIIO error: {0}")]
    Oiio(crispen_oiio::OiioError),
}
