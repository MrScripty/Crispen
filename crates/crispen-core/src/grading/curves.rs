//! Spline-based curve evaluation and 1D LUT baking.
//!
//! Implements Catmull-Rom spline interpolation for smooth curves through
//! user-defined control points. Used for hue-vs-hue, hue-vs-sat,
//! lum-vs-sat, and sat-vs-sat curve adjustments.
//!
//! # Algorithm
//! Catmull-Rom splines (1974) provide C1 continuity through control points.
//! For each segment between P1 and P2, with neighbors P0 and P3:
//! ```text
//! q(t) = 0.5 × ((2×P1) + (-P0 + P2)×t + (2×P0 - 5×P1 + 4×P2 - P3)×t² + (-P0 + 3×P1 - 3×P2 + P3)×t³)
//! ```
//!
//! # Complexity
//! - Evaluate: O(log N) binary search + O(1) interpolation
//! - Bake to 1D LUT: O(N × size)

use crate::transform::params::GradingParams;

/// Evaluates cubic Catmull-Rom spline curves from control points.
///
/// Control points are `[x, y]` pairs sorted by x-coordinate.
/// The curve passes through all control points with smooth interpolation.
///
/// # Performance
/// Borrows control points to avoid heap allocations in hot paths
/// (e.g. per-pixel evaluation during CPU LUT bake).
pub struct CurveEvaluator<'a> {
    /// Control points as `[x, y]` pairs, sorted by x.
    pub control_points: &'a [[f32; 2]],
}

impl CurveEvaluator<'_> {
    /// Evaluate the curve at position `t`.
    ///
    /// Uses Catmull-Rom interpolation between control points.
    /// Values outside the control point range are clamped to the
    /// first/last control point's y-value.
    ///
    /// Returns `t` (identity) if fewer than 2 control points.
    pub fn evaluate(&self, t: f32) -> f32 {
        let pts = &self.control_points;
        if pts.len() < 2 {
            return t;
        }

        // Clamp to range
        if t <= pts[0][0] {
            return pts[0][1];
        }
        if t >= pts[pts.len() - 1][0] {
            return pts[pts.len() - 1][1];
        }

        // Binary search for the segment containing t
        let mut lo = 0;
        let mut hi = pts.len() - 1;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if pts[mid][0] <= t {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        // Catmull-Rom: we need P0, P1 (lo), P2 (hi), P3
        let p1 = pts[lo];
        let p2 = pts[hi];

        // Virtual endpoints: mirror at boundaries
        let p0 = if lo > 0 {
            pts[lo - 1]
        } else {
            [2.0 * p1[0] - p2[0], 2.0 * p1[1] - p2[1]]
        };
        let p3 = if hi < pts.len() - 1 {
            pts[hi + 1]
        } else {
            [2.0 * p2[0] - p1[0], 2.0 * p2[1] - p1[1]]
        };

        // Parametric t within the segment
        let segment_t = if (p2[0] - p1[0]).abs() < 1e-10 {
            0.5
        } else {
            (t - p1[0]) / (p2[0] - p1[0])
        };

        catmull_rom(p0[1], p1[1], p2[1], p3[1], segment_t)
    }
}

/// Catmull-Rom cubic interpolation between P1 and P2.
///
/// ```text
/// q(t) = 0.5 × ((2×P1) + (-P0 + P2)×t + (2×P0 - 5×P1 + 4×P2 - P3)×t² + (-P0 + 3×P1 - 3×P2 + P3)×t³)
/// ```
fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// Bake a set of curve control points into a 1D LUT.
///
/// The LUT maps uniformly-spaced input values [0..1] to output values
/// using Catmull-Rom interpolation of the control points.
///
/// Returns a `Vec<f32>` of length `size` with evaluated curve values.
/// Empty control points produce an identity LUT.
pub fn bake_curve_to_1d_lut(control_points: &[[f32; 2]], size: usize) -> Vec<f32> {
    if control_points.is_empty() || size == 0 {
        return (0..size)
            .map(|i| i as f32 / (size - 1).max(1) as f32)
            .collect();
    }

    let evaluator = CurveEvaluator {
        control_points,
    };

    (0..size)
        .map(|i| {
            let t = i as f32 / (size - 1).max(1) as f32;
            evaluator.evaluate(t)
        })
        .collect()
}

/// Apply all curve adjustments (hue-vs-hue, hue-vs-sat, lum-vs-sat, sat-vs-sat).
///
/// Each curve type modifies a different aspect of the color:
/// - **hue_vs_hue**: rotates output hue based on input hue
/// - **hue_vs_sat**: adjusts saturation based on input hue
/// - **lum_vs_sat**: adjusts saturation based on input luminance
/// - **sat_vs_sat**: adjusts saturation based on input saturation
///
/// Empty control point vectors produce no adjustment (identity).
pub fn apply_curves(rgb: [f32; 3], params: &GradingParams) -> [f32; 3] {
    let no_curves = params.hue_vs_hue.is_empty()
        && params.hue_vs_sat.is_empty()
        && params.lum_vs_sat.is_empty()
        && params.sat_vs_sat.is_empty();

    if no_curves {
        return rgb;
    }

    // Convert to HSL-like representation for curve evaluation
    let (hue, sat, lum) = rgb_to_hsl(rgb);

    let mut out_hue = hue;
    let mut sat_mult = 1.0_f32;

    // Hue-vs-hue: rotate hue based on input hue
    if !params.hue_vs_hue.is_empty() {
        let eval = CurveEvaluator {
            control_points: &params.hue_vs_hue,
        };
        let hue_norm = hue / 360.0;
        let adjustment = eval.evaluate(hue_norm) - hue_norm;
        out_hue = (hue + adjustment * 360.0) % 360.0;
        if out_hue < 0.0 {
            out_hue += 360.0;
        }
    }

    // Hue-vs-sat: adjust saturation based on input hue
    if !params.hue_vs_sat.is_empty() {
        let eval = CurveEvaluator {
            control_points: &params.hue_vs_sat,
        };
        let hue_norm = hue / 360.0;
        sat_mult *= eval.evaluate(hue_norm) / hue_norm.max(1e-10);
    }

    // Lum-vs-sat: adjust saturation based on input luminance
    if !params.lum_vs_sat.is_empty() {
        let eval = CurveEvaluator {
            control_points: &params.lum_vs_sat,
        };
        sat_mult *= eval.evaluate(lum) / lum.max(1e-10);
    }

    // Sat-vs-sat: adjust saturation based on input saturation
    if !params.sat_vs_sat.is_empty() {
        let eval = CurveEvaluator {
            control_points: &params.sat_vs_sat,
        };
        sat_mult *= eval.evaluate(sat) / sat.max(1e-10);
    }

    let out_sat = (sat * sat_mult).clamp(0.0, 1.0);
    hsl_to_rgb(out_hue, out_sat, lum)
}

/// Convert RGB to HSL (hue in degrees, saturation and lightness in 0..1).
fn rgb_to_hsl(rgb: [f32; 3]) -> (f32, f32, f32) {
    let r = rgb[0];
    let g = rgb[1];
    let b = rgb[2];

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lum = (max + min) * 0.5;

    if (max - min).abs() < 1e-10 {
        return (0.0, 0.0, lum);
    }

    let delta = max - min;
    let sat = if lum > 0.5 {
        delta / (2.0 - max - min)
    } else {
        delta / (max + min)
    };

    let hue = if (max - r).abs() < 1e-10 {
        ((g - b) / delta) % 6.0
    } else if (max - g).abs() < 1e-10 {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };

    let hue = hue * 60.0;
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };

    (hue, sat, lum)
}

/// Convert HSL to RGB.
fn hsl_to_rgb(hue: f32, sat: f32, lum: f32) -> [f32; 3] {
    if sat.abs() < 1e-10 {
        return [lum, lum, lum];
    }

    let q = if lum < 0.5 {
        lum * (1.0 + sat)
    } else {
        lum + sat - lum * sat
    };
    let p = 2.0 * lum - q;
    let h = hue / 360.0;

    [
        hue_to_rgb(p, q, h + 1.0 / 3.0),
        hue_to_rgb(p, q, h),
        hue_to_rgb(p, q, h - 1.0 / 3.0),
    ]
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn test_catmull_rom_endpoints() {
        // At t=0, should return p1; at t=1, should return p2
        let v = catmull_rom(0.0, 0.25, 0.75, 1.0, 0.0);
        assert!((v - 0.25).abs() < EPSILON);
        let v = catmull_rom(0.0, 0.25, 0.75, 1.0, 1.0);
        assert!((v - 0.75).abs() < EPSILON);
    }

    #[test]
    fn test_curve_evaluator_identity_with_two_points() {
        let points = [[0.0, 0.0], [1.0, 1.0]];
        let eval = CurveEvaluator {
            control_points: &points,
        };
        assert!((eval.evaluate(0.0) - 0.0).abs() < EPSILON);
        assert!((eval.evaluate(0.5) - 0.5).abs() < 0.01);
        assert!((eval.evaluate(1.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_curve_evaluator_fewer_than_two_points_is_identity() {
        let eval = CurveEvaluator {
            control_points: &[],
        };
        assert!((eval.evaluate(0.5) - 0.5).abs() < EPSILON);

        let points = [[0.5, 0.5]];
        let eval = CurveEvaluator {
            control_points: &points,
        };
        assert!((eval.evaluate(0.3) - 0.3).abs() < EPSILON);
    }

    #[test]
    fn test_bake_curve_to_1d_lut_identity() {
        let lut = bake_curve_to_1d_lut(&[], 256);
        assert_eq!(lut.len(), 256);
        assert!((lut[0] - 0.0).abs() < EPSILON);
        assert!((lut[255] - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_apply_curves_empty_is_identity() {
        let params = GradingParams::default();
        let rgb = [0.5, 0.3, 0.7];
        let result = apply_curves(rgb, &params);
        assert_eq!(result, rgb);
    }

    #[test]
    fn test_hsl_roundtrip_preserves_values() {
        let original = [0.8, 0.4, 0.2];
        let (h, s, l) = rgb_to_hsl(original);
        let back = hsl_to_rgb(h, s, l);
        for i in 0..3 {
            assert!(
                (original[i] - back[i]).abs() < 0.001,
                "channel {i}: {:.6} vs {:.6}",
                original[i],
                back[i]
            );
        }
    }

    #[test]
    fn test_hsl_gray_has_zero_saturation() {
        let (_, s, _) = rgb_to_hsl([0.5, 0.5, 0.5]);
        assert!(s.abs() < EPSILON);
    }
}
