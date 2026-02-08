//! Image loading and format conversion for the demo application.

use std::path::Path;

use crispen_core::image::GradingImage;

/// Load an image from disk and convert to the internal `GradingImage` format.
pub fn load_image(path: &Path) -> std::io::Result<GradingImage> {
    let _ = path;
    todo!()
}
