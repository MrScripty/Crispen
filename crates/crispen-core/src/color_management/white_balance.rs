//! White balance adjustment via chromaticity shift.
//!
//! Temperature shifts along the Planckian locus (blue-yellow axis) in CIE xy,
//! then applies a Bradford chromatic adaptation. Tint shifts perpendicular to
//! the Planckian locus (green-magenta axis).
//!
//! # Reference
//! - Hernández-Andrés et al. (1999) — Planckian locus approximation
//! - Lindbloom, Bruce J. — Bradford chromatic adaptation

/// Apply white balance adjustment using temperature and tint.
///
/// - `temperature`: shift along the Planckian locus. 0.0 = neutral.
///   Positive values warm (toward yellow), negative values cool (toward blue).
///   Range is approximately −1.0 to +1.0, mapped to ~2000K–12000K shift.
/// - `tint`: shift perpendicular to Planckian locus. 0.0 = neutral.
///   Positive values shift toward magenta, negative toward green.
///
/// Both values at 0.0 produce no change (identity).
///
/// # Algorithm
/// 1. Map temperature to a correlated color temperature (CCT) offset
/// 2. Compute source and destination white points on the Planckian locus
/// 3. Apply tint as perpendicular offset in CIE xy
/// 4. Compute Bradford adaptation matrix from source to destination
/// 5. Apply matrix to input RGB
pub fn apply_white_balance(rgb: [f32; 3], temperature: f32, tint: f32) -> [f32; 3] {
    if temperature.abs() < 1e-7 && tint.abs() < 1e-7 {
        return rgb;
    }

    // Reference white point: D65 (neutral starting point)
    let ref_x: f64 = 0.3127;
    let ref_y: f64 = 0.3290;

    // Approximate Planckian locus tangent direction at D65
    // This simplified model shifts xy along the blue-yellow axis
    let temp_scale: f64 = 0.05;
    let tint_scale: f64 = 0.05;

    // Temperature shifts along the Planckian locus tangent (approximately)
    // At D65, the tangent direction is roughly (-0.35, -0.15) normalized
    let tangent_x: f64 = 0.3585;
    let tangent_y: f64 = 0.1501;

    // Tint shifts perpendicular to the Planckian locus
    let perp_x: f64 = -tangent_y; // Rotate 90 degrees
    let perp_y: f64 = tangent_x;

    // Compute destination white point
    let t = temperature as f64 * temp_scale;
    let p = tint as f64 * tint_scale;
    let dst_x = ref_x + tangent_x * t + perp_x * p;
    let dst_y = ref_y + tangent_y * t + perp_y * p;

    // Bradford cone response matrix
    const M: [[f64; 3]; 3] = [
        [0.8951, 0.2664, -0.1614],
        [-0.7502, 1.7135, 0.0367],
        [0.0389, -0.0685, 1.0296],
    ];
    const M_INV: [[f64; 3]; 3] = [
        [0.9869929055, -0.1470542564, 0.1599626517],
        [0.4323052697, 0.5183602715, 0.0492912282],
        [-0.0085286646, 0.0400428217, 0.9684866958],
    ];

    // Convert xy to XYZ (Y=1)
    let src_xyz = [ref_x / ref_y, 1.0, (1.0 - ref_x - ref_y) / ref_y];
    let dst_xyz = [dst_x / dst_y, 1.0, (1.0 - dst_x - dst_y) / dst_y];

    // Compute cone responses
    let src_cone = mat3_vec3(M, src_xyz);
    let dst_cone = mat3_vec3(M, dst_xyz);

    // Build adaptation matrix: M_INV * diag(dst/src) * M
    let scale = [
        dst_cone[0] / src_cone[0],
        dst_cone[1] / src_cone[1],
        dst_cone[2] / src_cone[2],
    ];

    // Compute M_INV * diag(scale) * M directly
    let adapt = compose_bradford(M_INV, scale, M);

    // Apply to RGB
    let r = rgb[0] as f64;
    let g = rgb[1] as f64;
    let b = rgb[2] as f64;
    [
        (adapt[0][0] * r + adapt[0][1] * g + adapt[0][2] * b) as f32,
        (adapt[1][0] * r + adapt[1][1] * g + adapt[1][2] * b) as f32,
        (adapt[2][0] * r + adapt[2][1] * g + adapt[2][2] * b) as f32,
    ]
}

fn mat3_vec3(m: [[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Compute M_INV * diag(s) * M in a single pass.
fn compose_bradford(
    m_inv: [[f64; 3]; 3],
    s: [f64; 3],
    m: [[f64; 3]; 3],
) -> [[f64; 3]; 3] {
    let sm = [
        [s[0] * m[0][0], s[0] * m[0][1], s[0] * m[0][2]],
        [s[1] * m[1][0], s[1] * m[1][1], s[1] * m[1][2]],
        [s[2] * m[2][0], s[2] * m[2][1], s[2] * m[2][2]],
    ];
    let mut out = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] =
                m_inv[i][0] * sm[0][j] + m_inv[i][1] * sm[1][j] + m_inv[i][2] * sm[2][j];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn test_white_balance_zero_is_identity() {
        let rgb = [0.5, 0.4, 0.3];
        let result = apply_white_balance(rgb, 0.0, 0.0);
        assert_eq!(result, rgb);
    }

    #[test]
    fn test_white_balance_warm_shifts_toward_yellow() {
        let rgb = [0.5, 0.5, 0.5];
        let result = apply_white_balance(rgb, 1.0, 0.0);
        assert!(result[2] < rgb[2], "blue should decrease when warming");
    }

    #[test]
    fn test_white_balance_cool_shifts_toward_blue() {
        let rgb = [0.5, 0.5, 0.5];
        let result = apply_white_balance(rgb, -1.0, 0.0);
        assert!(result[2] > rgb[2], "blue should increase when cooling");
    }

    #[test]
    fn test_white_balance_preserves_black() {
        let result = apply_white_balance([0.0, 0.0, 0.0], 0.5, 0.5);
        for i in 0..3 {
            assert!(result[i].abs() < EPSILON);
        }
    }
}
