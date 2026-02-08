//! Slider-based grading adjustments (contrast, shadows/highlights, saturation).

/// Apply contrast with pivot point.
///
/// Contrast is applied as a power curve centered on the pivot value.
/// `contrast = 1.0` and any pivot produce no change.
pub fn apply_contrast(rgb: [f32; 3], contrast: f32, pivot: f32) -> [f32; 3] {
    let _ = (rgb, contrast, pivot);
    todo!()
}

/// Apply shadows and highlights recovery.
///
/// Shadows lifts dark values, highlights compresses bright values.
/// Both at 0.0 produce no change.
pub fn apply_shadows_highlights(rgb: [f32; 3], shadows: f32, highlights: f32) -> [f32; 3] {
    let _ = (rgb, shadows, highlights);
    todo!()
}

/// Apply saturation and hue rotation.
///
/// `saturation = 1.0`, `hue = 0.0`, and `luma_mix = 0.0` produce no change.
pub fn apply_saturation_hue(
    rgb: [f32; 3],
    saturation: f32,
    hue: f32,
    luma_mix: f32,
) -> [f32; 3] {
    let _ = (rgb, saturation, hue, luma_mix);
    todo!()
}
