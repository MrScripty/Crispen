//! Spline-based curve evaluation and 1D LUT baking.

use crate::transform::params::GradingParams;

/// Evaluates cubic spline curves from control points.
pub struct CurveEvaluator {
    /// Control points as `[x, y]` pairs, sorted by x.
    pub control_points: Vec<[f32; 2]>,
}

impl CurveEvaluator {
    /// Evaluate the curve at position `t` (0.0–1.0).
    pub fn evaluate(&self, t: f32) -> f32 {
        let _ = t;
        todo!()
    }
}

/// Bake a set of curve control points into a 1D LUT.
pub fn bake_curve_to_1d_lut(control_points: &[[f32; 2]], size: usize) -> Vec<f32> {
    let _ = (control_points, size);
    todo!()
}

/// Apply all curve adjustments (hue-vs-hue, hue-vs-sat, lum-vs-sat, sat-vs-sat).
pub fn apply_curves(rgb: [f32; 3], params: &GradingParams) -> [f32; 3] {
    let _ = (rgb, params);
    todo!()
}
