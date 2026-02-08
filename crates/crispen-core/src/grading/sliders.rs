//! Slider-based grading adjustments (contrast, shadows/highlights, saturation, hue).

/// Rec. 709 luminance weights.
const LUMA_REC709: [f32; 3] = [0.2126, 0.7152, 0.0722];

/// Equal-weight luminance.
const LUMA_EQUAL: [f32; 3] = [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];

/// Apply contrast with pivot point.
///
/// Contrast is applied as a power curve centered on the pivot value.
/// Values at the pivot are unchanged; values above are pushed further away,
/// values below are pulled closer.
///
/// ```text
/// out = pow(in / pivot, contrast) × pivot
/// ```
///
/// `contrast = 1.0` and any pivot produce no change.
pub fn apply_contrast(rgb: [f32; 3], contrast: f32, pivot: f32) -> [f32; 3] {
    if (contrast - 1.0).abs() < 1e-7 {
        return rgb;
    }

    let mut out = [0.0_f32; 3];
    for c in 0..3 {
        if rgb[c] <= 0.0 {
            out[c] = 0.0;
        } else {
            out[c] = (rgb[c] / pivot).powf(contrast) * pivot;
        }
    }
    out
}

/// Apply shadows and highlights recovery.
///
/// Uses a soft-knee isolation to separate shadows (below pivot) from
/// highlights (above pivot). The shadow parameter lifts dark values,
/// the highlight parameter compresses bright values.
///
/// ```text
/// shadow_weight  = 1 − smoothstep(0, 2×pivot, in)
/// highlight_weight = smoothstep(0, 2×pivot, in)
///
/// out = in + shadows × shadow_weight − highlights × highlight_weight
/// ```
///
/// Both at 0.0 produce no change.
pub fn apply_shadows_highlights(rgb: [f32; 3], shadows: f32, highlights: f32) -> [f32; 3] {
    if shadows.abs() < 1e-7 && highlights.abs() < 1e-7 {
        return rgb;
    }

    const PIVOT: f32 = 0.5;
    let range = 2.0 * PIVOT;

    let mut out = [0.0_f32; 3];
    for c in 0..3 {
        let t = (rgb[c] / range).clamp(0.0, 1.0);
        // Smoothstep: 3t² − 2t³
        let s = t * t * (3.0 - 2.0 * t);

        let shadow_weight = 1.0 - s;
        let highlight_weight = s;

        out[c] = rgb[c]
            + shadows * shadow_weight * 0.5
            - highlights * highlight_weight * 0.5;
    }
    out
}

/// Apply saturation and hue rotation.
///
/// Saturation scales chroma relative to luminance. Hue rotates the
/// chrominance angle. `luma_mix` blends between Rec. 709 luminance weights
/// and equal-weight luminance for the desaturation reference.
///
/// ```text
/// luma = lerp(dot(rgb, rec709_weights), dot(rgb, equal_weights), luma_mix)
/// chroma = rgb − luma
/// chroma_rotated = rotate_hue(chroma, hue_degrees)
/// out = luma + chroma_rotated × saturation
/// ```
///
/// `saturation = 1.0`, `hue = 0.0`, `luma_mix = 0.0` produce no change.
pub fn apply_saturation_hue(
    rgb: [f32; 3],
    saturation: f32,
    hue: f32,
    luma_mix: f32,
) -> [f32; 3] {
    if (saturation - 1.0).abs() < 1e-7 && hue.abs() < 1e-7 {
        return rgb;
    }

    // Compute blended luminance
    let luma_709 = rgb[0] * LUMA_REC709[0] + rgb[1] * LUMA_REC709[1] + rgb[2] * LUMA_REC709[2];
    let luma_eq = rgb[0] * LUMA_EQUAL[0] + rgb[1] * LUMA_EQUAL[1] + rgb[2] * LUMA_EQUAL[2];
    let luma = luma_709 * (1.0 - luma_mix) + luma_eq * luma_mix;

    // Extract chroma
    let mut chroma = [rgb[0] - luma, rgb[1] - luma, rgb[2] - luma];

    // Apply hue rotation in the chrominance plane
    if hue.abs() > 1e-7 {
        chroma = rotate_chroma(chroma, hue);
    }

    // Apply saturation
    [
        luma + chroma[0] * saturation,
        luma + chroma[1] * saturation,
        luma + chroma[2] * saturation,
    ]
}

/// Rotate the chrominance vector by `degrees` around the luminance axis.
///
/// Uses the Rodrigues rotation formula in the plane perpendicular to (1,1,1).
fn rotate_chroma(chroma: [f32; 3], degrees: f32) -> [f32; 3] {
    let rad = degrees.to_radians();
    let cos_a = rad.cos();
    let sin_a = rad.sin();

    // Rotation axis is the luminance direction (1,1,1)/sqrt(3)
    let inv_sqrt3 = 1.0 / 3.0_f32.sqrt();
    let k = [inv_sqrt3, inv_sqrt3, inv_sqrt3];

    // Rodrigues: v_rot = v*cos(a) + (k×v)*sin(a) + k*(k·v)*(1-cos(a))
    // Since chroma is perpendicular to k (by construction), k·v = 0
    let cross = [
        k[1] * chroma[2] - k[2] * chroma[1],
        k[2] * chroma[0] - k[0] * chroma[2],
        k[0] * chroma[1] - k[1] * chroma[0],
    ];

    [
        chroma[0] * cos_a + cross[0] * sin_a,
        chroma[1] * cos_a + cross[1] * sin_a,
        chroma[2] * cos_a + cross[2] * sin_a,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn test_contrast_at_pivot_is_identity() {
        let pivot = 0.435;
        let rgb = [pivot, pivot, pivot];
        let result = apply_contrast(rgb, 2.0, pivot);
        for i in 0..3 {
            assert!(
                (result[i] - pivot).abs() < EPSILON,
                "channel {i}: {:.8} vs {:.8}",
                result[i], pivot
            );
        }
    }

    #[test]
    fn test_contrast_one_is_identity() {
        let rgb = [0.3, 0.5, 0.7];
        let result = apply_contrast(rgb, 1.0, 0.435);
        assert_eq!(result, rgb);
    }

    #[test]
    fn test_contrast_increases_spread() {
        let rgb = [0.8, 0.8, 0.8];
        let pivot = 0.435;
        let result = apply_contrast(rgb, 2.0, pivot);
        // Values above pivot should move further above
        for i in 0..3 {
            assert!(result[i] > rgb[i], "contrast should push highlights higher");
        }
    }

    #[test]
    fn test_shadows_highlights_zero_is_identity() {
        let rgb = [0.3, 0.5, 0.7];
        let result = apply_shadows_highlights(rgb, 0.0, 0.0);
        assert_eq!(result, rgb);
    }

    #[test]
    fn test_saturation_zero_produces_grayscale() {
        let rgb = [0.8, 0.4, 0.2];
        let result = apply_saturation_hue(rgb, 0.0, 0.0, 0.0);
        // All channels should be equal (luminance)
        assert!((result[0] - result[1]).abs() < EPSILON);
        assert!((result[1] - result[2]).abs() < EPSILON);
    }

    #[test]
    fn test_saturation_one_hue_zero_is_identity() {
        let rgb = [0.5, 0.3, 0.7];
        let result = apply_saturation_hue(rgb, 1.0, 0.0, 0.0);
        for i in 0..3 {
            assert!((result[i] - rgb[i]).abs() < EPSILON);
        }
    }

    #[test]
    fn test_hue_rotation_360_is_identity() {
        let rgb = [0.5, 0.3, 0.7];
        let result = apply_saturation_hue(rgb, 1.0, 360.0, 0.0);
        for i in 0..3 {
            assert!(
                (result[i] - rgb[i]).abs() < EPSILON,
                "360° hue rotation should be identity: ch{i} {:.6} vs {:.6}",
                result[i], rgb[i]
            );
        }
    }
}
