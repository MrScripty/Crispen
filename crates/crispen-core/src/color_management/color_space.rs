//! Color space definitions and 3×3 matrix transforms.
//!
//! All gamut conversions route through CIE XYZ D65 as a hub color space.
//! Matrices are computed from published CIE chromaticity coordinates using
//! Bradford chromatic adaptation where white points differ from D65.
//!
//! # Reference
//! - IEC 61966-2-1:1999 (sRGB primaries)
//! - SMPTE ST 2065-1:2012 (ACES AP0 primaries)
//! - S-2014-004: ACEScg color space (AP1 primaries)
//! - ITU-R BT.2020 (Rec.2020 primaries)
//! - SMPTE RP 431-2:2011 (DCI-P3 primaries)

pub use crate::transform::params::ColorSpaceId;

/// Type alias for 3×3 f64 matrix used in color space math.
type Mat3 = [[f64; 3]; 3];

/// A 3×3 color matrix for linear color space conversions.
///
/// Stored in f64 for precision during matrix composition.
/// Applied to `[f32; 3]` pixel data with f64 intermediate computation.
///
/// ```text
/// ┌          ┐   ┌   ┐   ┌    ┐
/// │ m00 m01 m02 │   │ R │   │ R' │
/// │ m10 m11 m12 │ × │ G │ = │ G' │
/// │ m20 m21 m22 │   │ B │   │ B' │
/// └          ┘   └   ┘   └    ┘
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ColorMatrix(pub Mat3);

impl ColorMatrix {
    /// Identity matrix — no-op transform.
    pub const IDENTITY: Self = Self([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);

    /// Apply this matrix to an RGB triplet.
    ///
    /// `out[i] = Σ_j M[i][j] × rgb[j]`
    ///
    /// Computation uses f64 internally; result is f32.
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let r = rgb[0] as f64;
        let g = rgb[1] as f64;
        let b = rgb[2] as f64;
        [
            (self.0[0][0] * r + self.0[0][1] * g + self.0[0][2] * b) as f32,
            (self.0[1][0] * r + self.0[1][1] * g + self.0[1][2] * b) as f32,
            (self.0[2][0] * r + self.0[2][1] * g + self.0[2][2] * b) as f32,
        ]
    }
}

// ---------------------------------------------------------------------------
// Matrix arithmetic (f64)
// ---------------------------------------------------------------------------

fn mat3_mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut out = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    out
}

/// Invert a 3×3 matrix using cofactor expansion.
fn mat3_inv(m: &Mat3) -> Mat3 {
    let c00 = m[1][1] * m[2][2] - m[1][2] * m[2][1];
    let c01 = m[1][2] * m[2][0] - m[1][0] * m[2][2];
    let c02 = m[1][0] * m[2][1] - m[1][1] * m[2][0];
    let c10 = m[0][2] * m[2][1] - m[0][1] * m[2][2];
    let c11 = m[0][0] * m[2][2] - m[0][2] * m[2][0];
    let c12 = m[0][1] * m[2][0] - m[0][0] * m[2][1];
    let c20 = m[0][1] * m[1][2] - m[0][2] * m[1][1];
    let c21 = m[0][2] * m[1][0] - m[0][0] * m[1][2];
    let c22 = m[0][0] * m[1][1] - m[0][1] * m[1][0];

    let det = m[0][0] * c00 + m[0][1] * c01 + m[0][2] * c02;
    let inv_det = 1.0 / det;

    [
        [c00 * inv_det, c10 * inv_det, c20 * inv_det],
        [c01 * inv_det, c11 * inv_det, c21 * inv_det],
        [c02 * inv_det, c12 * inv_det, c22 * inv_det],
    ]
}

fn mat3_vec3_mul(m: &Mat3, v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

// ---------------------------------------------------------------------------
// CIE chromaticity data
// ---------------------------------------------------------------------------

/// CIE 1931 xy chromaticity coordinates for a set of RGB primaries and white point.
#[derive(Debug, Clone, Copy)]
pub struct CieChromaticity {
    /// Red primary (x, y).
    pub r: [f64; 2],
    /// Green primary (x, y).
    pub g: [f64; 2],
    /// Blue primary (x, y).
    pub b: [f64; 2],
    /// White point (x, y).
    pub w: [f64; 2],
}

const D65_WHITE: [f64; 2] = [0.3127, 0.3290];
const ACES_WHITE: [f64; 2] = [0.32168, 0.33767];

const REC709: CieChromaticity = CieChromaticity {
    r: [0.6400, 0.3300],
    g: [0.3000, 0.6000],
    b: [0.1500, 0.0600],
    w: D65_WHITE,
};

const AP0: CieChromaticity = CieChromaticity {
    r: [0.73470, 0.26530],
    g: [0.00000, 1.00000],
    b: [0.00010, -0.07700],
    w: ACES_WHITE,
};

const AP1: CieChromaticity = CieChromaticity {
    r: [0.71300, 0.29300],
    g: [0.16500, 0.83000],
    b: [0.12800, 0.04400],
    w: ACES_WHITE,
};

const REC2020: CieChromaticity = CieChromaticity {
    r: [0.70800, 0.29200],
    g: [0.17000, 0.79700],
    b: [0.13100, 0.04600],
    w: D65_WHITE,
};

const DISPLAY_P3: CieChromaticity = CieChromaticity {
    r: [0.68000, 0.32000],
    g: [0.26500, 0.69000],
    b: [0.15000, 0.06000],
    w: D65_WHITE,
};

const ARRI_WG3: CieChromaticity = CieChromaticity {
    r: [0.68400, 0.31300],
    g: [0.22100, 0.84800],
    b: [0.08610, -0.10200],
    w: D65_WHITE,
};

const ARRI_WG4: CieChromaticity = CieChromaticity {
    r: [0.73470, 0.26530],
    g: [0.14240, 0.85760],
    b: [0.09910, -0.03080],
    w: D65_WHITE,
};

const S_GAMUT3_CINE: CieChromaticity = CieChromaticity {
    r: [0.76600, 0.27500],
    g: [0.22500, 0.80000],
    b: [0.08900, -0.08700],
    w: D65_WHITE,
};

const RED_WIDE_GAMUT: CieChromaticity = CieChromaticity {
    r: [0.78010, 0.30490],
    g: [0.12120, 1.49310],
    b: [0.09530, -0.08490],
    w: D65_WHITE,
};

const V_GAMUT: CieChromaticity = CieChromaticity {
    r: [0.73000, 0.28000],
    g: [0.16500, 0.84000],
    b: [0.10000, -0.03000],
    w: D65_WHITE,
};

// ---------------------------------------------------------------------------
// Normalized Primary Matrix (NPM) computation
// ---------------------------------------------------------------------------

/// Compute the Normalized Primary Matrix from CIE chromaticity coordinates.
///
/// The NPM converts linear RGB to CIE XYZ using the gamut's native white point.
///
/// # Algorithm
/// 1. Convert (x, y) primaries to XYZ with Y = 1
/// 2. Solve `P · S = W_xyz` for scaling vector S
/// 3. NPM = P · diag(S)
///
/// # Reference
/// IEC 61966-2-1:1999, Annex F
fn compute_npm(c: &CieChromaticity) -> Mat3 {
    let xr = c.r[0] / c.r[1];
    let zr = (1.0 - c.r[0] - c.r[1]) / c.r[1];
    let xg = c.g[0] / c.g[1];
    let zg = (1.0 - c.g[0] - c.g[1]) / c.g[1];
    let xb = c.b[0] / c.b[1];
    let zb = (1.0 - c.b[0] - c.b[1]) / c.b[1];

    let xw = c.w[0] / c.w[1];
    let zw = (1.0 - c.w[0] - c.w[1]) / c.w[1];

    let p = [[xr, xg, xb], [1.0, 1.0, 1.0], [zr, zg, zb]];
    let p_inv = mat3_inv(&p);
    let s = mat3_vec3_mul(&p_inv, [xw, 1.0, zw]);

    [
        [s[0] * xr, s[1] * xg, s[2] * xb],
        [s[0], s[1], s[2]],
        [s[0] * zr, s[1] * zg, s[2] * zb],
    ]
}

// ---------------------------------------------------------------------------
// Bradford chromatic adaptation
// ---------------------------------------------------------------------------

/// Bradford cone response matrix.
const BRADFORD: Mat3 = [
    [0.8951, 0.2664, -0.1614],
    [-0.7502, 1.7135, 0.0367],
    [0.0389, -0.0685, 1.0296],
];

/// Inverse of the Bradford cone response matrix.
const BRADFORD_INV: Mat3 = [
    [0.9869929055, -0.1470542564, 0.1599626517],
    [0.4323052697, 0.5183602715, 0.0492912282],
    [-0.0085286646, 0.0400428217, 0.9684866958],
];

/// Compute Bradford chromatic adaptation matrix from one white point to another.
///
/// # Reference
/// Lindbloom, Bruce J. "Chromatic Adaptation"
fn bradford_adaptation(src_xy: [f64; 2], dst_xy: [f64; 2]) -> Mat3 {
    let src_xyz = [
        src_xy[0] / src_xy[1],
        1.0,
        (1.0 - src_xy[0] - src_xy[1]) / src_xy[1],
    ];
    let dst_xyz = [
        dst_xy[0] / dst_xy[1],
        1.0,
        (1.0 - dst_xy[0] - dst_xy[1]) / dst_xy[1],
    ];

    let src_cone = mat3_vec3_mul(&BRADFORD, src_xyz);
    let dst_cone = mat3_vec3_mul(&BRADFORD, dst_xyz);

    let scale: Mat3 = [
        [dst_cone[0] / src_cone[0], 0.0, 0.0],
        [0.0, dst_cone[1] / src_cone[1], 0.0],
        [0.0, 0.0, dst_cone[2] / src_cone[2]],
    ];

    // M_A_INV * scale * M_A
    let tmp = mat3_mul(&scale, &BRADFORD);
    mat3_mul(&BRADFORD_INV, &tmp)
}

// ---------------------------------------------------------------------------
// Gamut identification and XYZ D65 conversion
// ---------------------------------------------------------------------------

/// Internal gamut identifier — groups `ColorSpaceId`s that share the same primaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
enum Gamut {
    Rec709,
    Ap0,
    Ap1,
    Rec2020,
    DisplayP3,
    ArriWg3,
    ArriWg4,
    SGamut3Cine,
    RedWideGamut,
    VGamut,
}

fn gamut_of(space: ColorSpaceId) -> Gamut {
    match space {
        ColorSpaceId::Srgb | ColorSpaceId::LinearSrgb => Gamut::Rec709,
        ColorSpaceId::Aces2065_1 => Gamut::Ap0,
        ColorSpaceId::AcesCg | ColorSpaceId::AcesCc | ColorSpaceId::AcesCct => Gamut::Ap1,
        ColorSpaceId::Rec2020 => Gamut::Rec2020,
        ColorSpaceId::DciP3 => Gamut::DisplayP3,
        ColorSpaceId::ArriLogC3 => Gamut::ArriWg3,
        ColorSpaceId::ArriLogC4 => Gamut::ArriWg4,
        ColorSpaceId::SLog3 => Gamut::SGamut3Cine,
        ColorSpaceId::RedLog3G10 => Gamut::RedWideGamut,
        ColorSpaceId::VLog => Gamut::VGamut,
        ColorSpaceId::Custom(_) => Gamut::Rec709,
    }
}

fn chromaticity_of(gamut: Gamut) -> &'static CieChromaticity {
    match gamut {
        Gamut::Rec709 => &REC709,
        Gamut::Ap0 => &AP0,
        Gamut::Ap1 => &AP1,
        Gamut::Rec2020 => &REC2020,
        Gamut::DisplayP3 => &DISPLAY_P3,
        Gamut::ArriWg3 => &ARRI_WG3,
        Gamut::ArriWg4 => &ARRI_WG4,
        Gamut::SGamut3Cine => &S_GAMUT3_CINE,
        Gamut::RedWideGamut => &RED_WIDE_GAMUT,
        Gamut::VGamut => &V_GAMUT,
    }
}

/// Compute the matrix converting from a gamut's linear RGB to CIE XYZ D65.
///
/// For gamuts with D65 white points, this is simply the NPM.
/// For gamuts with non-D65 white points (ACES), Bradford adaptation is applied.
fn to_xyz_d65(gamut: Gamut) -> Mat3 {
    let c = chromaticity_of(gamut);
    let npm = compute_npm(c);

    if (c.w[0] - D65_WHITE[0]).abs() < 1e-10 && (c.w[1] - D65_WHITE[1]).abs() < 1e-10 {
        npm
    } else {
        let adapt = bradford_adaptation(c.w, D65_WHITE);
        mat3_mul(&adapt, &npm)
    }
}

/// Compute the matrix converting from CIE XYZ D65 to a gamut's linear RGB.
fn from_xyz_d65(gamut: Gamut) -> Mat3 {
    mat3_inv(&to_xyz_d65(gamut))
}

/// Get the 3×3 transform matrix to convert between color spaces.
///
/// Routes through CIE XYZ D65 as a hub. If both spaces share the same
/// gamut (primaries + white point), returns the identity matrix — only the
/// transfer function differs (handled by [`crate::color_management::transfer`]).
///
/// # Example
/// ```ignore
/// let m = get_conversion_matrix(ColorSpaceId::LinearSrgb, ColorSpaceId::AcesCg);
/// let aces_rgb = m.apply(srgb_linear);
/// ```
pub fn get_conversion_matrix(from: ColorSpaceId, to: ColorSpaceId) -> ColorMatrix {
    let from_gamut = gamut_of(from);
    let to_gamut = gamut_of(to);

    if from_gamut == to_gamut {
        return ColorMatrix::IDENTITY;
    }

    let m_to_xyz = to_xyz_d65(from_gamut);
    let m_from_xyz = from_xyz_d65(to_gamut);
    ColorMatrix(mat3_mul(&m_from_xyz, &m_to_xyz))
}

/// Get the CIE 1931 xy chromaticity coordinates for a color space.
///
/// Returns the R, G, B primary coordinates and white point used by the
/// gamut associated with this color space. Multiple `ColorSpaceId`s that
/// share the same gamut (e.g. `Srgb` and `LinearSrgb`) return identical
/// coordinates.
pub fn chromaticity(id: ColorSpaceId) -> &'static CieChromaticity {
    chromaticity_of(gamut_of(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    fn assert_rgb_close(a: [f32; 3], b: [f32; 3], eps: f32) {
        for i in 0..3 {
            assert!(
                (a[i] - b[i]).abs() < eps,
                "channel {i}: {:.8} vs {:.8} (diff {:.8})",
                a[i],
                b[i],
                (a[i] - b[i]).abs()
            );
        }
    }

    #[test]
    fn test_identity_matrix_is_passthrough() {
        let rgb = [0.5, 0.3, 0.7];
        let result = ColorMatrix::IDENTITY.apply(rgb);
        assert_eq!(result, rgb);
    }

    #[test]
    fn test_srgb_to_acescg_roundtrip_preserves_values() {
        let original = [0.5, 0.25, 0.75];
        let to_aces = get_conversion_matrix(ColorSpaceId::LinearSrgb, ColorSpaceId::AcesCg);
        let from_aces = get_conversion_matrix(ColorSpaceId::AcesCg, ColorSpaceId::LinearSrgb);
        let aces = to_aces.apply(original);
        let back = from_aces.apply(aces);
        assert_rgb_close(original, back, EPSILON);
    }

    #[test]
    fn test_same_gamut_returns_identity() {
        let m = get_conversion_matrix(ColorSpaceId::Srgb, ColorSpaceId::LinearSrgb);
        let rgb = [0.3, 0.6, 0.9];
        assert_eq!(m.apply(rgb), rgb);
    }

    #[test]
    fn test_rec2020_to_acescg_roundtrip_preserves_values() {
        let original = [0.4, 0.5, 0.6];
        let fwd = get_conversion_matrix(ColorSpaceId::Rec2020, ColorSpaceId::AcesCg);
        let rev = get_conversion_matrix(ColorSpaceId::AcesCg, ColorSpaceId::Rec2020);
        let result = rev.apply(fwd.apply(original));
        assert_rgb_close(original, result, EPSILON);
    }

    #[test]
    fn test_dcip3_to_acescg_roundtrip_preserves_values() {
        let original = [0.7, 0.2, 0.1];
        let fwd = get_conversion_matrix(ColorSpaceId::DciP3, ColorSpaceId::AcesCg);
        let rev = get_conversion_matrix(ColorSpaceId::AcesCg, ColorSpaceId::DciP3);
        let result = rev.apply(fwd.apply(original));
        assert_rgb_close(original, result, EPSILON);
    }

    #[test]
    fn test_ap0_to_ap1_roundtrip_preserves_values() {
        let original = [0.3, 0.4, 0.5];
        let fwd = get_conversion_matrix(ColorSpaceId::Aces2065_1, ColorSpaceId::AcesCg);
        let rev = get_conversion_matrix(ColorSpaceId::AcesCg, ColorSpaceId::Aces2065_1);
        let result = rev.apply(fwd.apply(original));
        assert_rgb_close(original, result, EPSILON);
    }

    #[test]
    fn test_srgb_npm_matches_published_values() {
        // Verify our computed sRGB NPM against published IEC 61966-2-1 values
        let npm = compute_npm(&REC709);
        let published = [
            [0.4123907993, 0.3575843394, 0.1804807884],
            [0.2126390059, 0.7151686788, 0.0721923154],
            [0.0193308187, 0.1191947798, 0.9505321522],
        ];
        for (i, row) in npm.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                assert!(
                    (*value - published[i][j]).abs() < 1e-6,
                    "NPM[{i}][{j}]: {:.10} vs {:.10}",
                    value,
                    published[i][j]
                );
            }
        }
    }

    #[test]
    fn test_mat3_inverse_of_identity_is_identity() {
        let id = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let inv = mat3_inv(&id);
        for (i, row) in inv.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((*value - expected).abs() < 1e-12);
            }
        }
    }
}
