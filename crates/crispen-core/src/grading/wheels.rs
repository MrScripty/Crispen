//! ASC CDL (Lift/Gamma/Gain/Offset) color wheel adjustments.
//!
//! Implements the DaVinci Resolve-style primary color wheels using an
//! extended ASC CDL (American Society of Cinematographers Color Decision List)
//! with separate lift control.
//!
//! # Formula
//! For each channel `c` in `{R, G, B}`:
//! ```text
//!   combined_gain   = gain[c] × gain[master]
//!   combined_lift   = lift[c] + lift[master]
//!   combined_offset = offset[c] + offset[master]
//!   combined_gamma  = gamma[c] × gamma[master]
//!
//!   x = in × combined_gain
//!       + combined_lift × (1 − combined_gain)
//!       + combined_offset
//!
//!   out = pow(max(x, 0), 1 / combined_gamma)
//! ```
//!
//! ```text
//!   Input ──→ ×Gain ──→ +Lift×(1−Gain) ──→ +Offset ──→ max(0) ──→ ^(1/Gamma) ──→ Output
//! ```

/// Apply ASC CDL transform with lift separation.
///
/// Each parameter is `[R, G, B, Master]` where master multiplies (gain, gamma)
/// or adds (lift, offset) to per-channel values.
///
/// Default identity values: lift=`[0,0,0,0]`, gamma=`[1,1,1,1]`,
/// gain=`[1,1,1,1]`, offset=`[0,0,0,0]`.
pub fn apply_cdl(
    rgb: [f32; 3],
    lift: &[f32; 4],
    gamma: &[f32; 4],
    gain: &[f32; 4],
    offset: &[f32; 4],
) -> [f32; 3] {
    let mut out = [0.0_f32; 3];
    for c in 0..3 {
        let combined_gain = gain[c] * gain[3];
        let combined_lift = lift[c] + lift[3];
        let combined_offset = offset[c] + offset[3];
        let combined_gamma = gamma[c] * gamma[3];

        let x = rgb[c] * combined_gain + combined_lift * (1.0 - combined_gain) + combined_offset;

        // Clamp to zero before power to avoid NaN from negative bases
        let clamped = x.max(0.0);

        // Gamma is applied as inverse power (1/gamma)
        // gamma > 1 darkens midtones, gamma < 1 brightens them
        if combined_gamma > 0.0 {
            out[c] = clamped.powf(1.0 / combined_gamma);
        } else {
            out[c] = clamped;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-6;

    fn default_lift() -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
    fn default_gamma() -> [f32; 4] {
        [1.0, 1.0, 1.0, 1.0]
    }
    fn default_gain() -> [f32; 4] {
        [1.0, 1.0, 1.0, 1.0]
    }
    fn default_offset() -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    #[test]
    fn test_cdl_identity_is_passthrough() {
        let rgb = [0.5, 0.3, 0.7];
        let result = apply_cdl(
            rgb,
            &default_lift(),
            &default_gamma(),
            &default_gain(),
            &default_offset(),
        );
        for i in 0..3 {
            assert!(
                (result[i] - rgb[i]).abs() < EPSILON,
                "channel {i}: {:.8} vs {:.8}",
                result[i],
                rgb[i]
            );
        }
    }

    #[test]
    fn test_cdl_gain_doubles_values() {
        let rgb = [0.25, 0.5, 0.125];
        let gain = [2.0, 2.0, 2.0, 1.0];
        let result = apply_cdl(
            rgb,
            &default_lift(),
            &default_gamma(),
            &gain,
            &default_offset(),
        );
        for i in 0..3 {
            assert!(
                (result[i] - rgb[i] * 2.0).abs() < EPSILON,
                "channel {i}: {:.8} vs {:.8}",
                result[i],
                rgb[i] * 2.0
            );
        }
    }

    #[test]
    fn test_cdl_master_gain_scales_all_channels() {
        let rgb = [0.25, 0.5, 0.125];
        let gain = [1.0, 1.0, 1.0, 2.0];
        let result = apply_cdl(
            rgb,
            &default_lift(),
            &default_gamma(),
            &gain,
            &default_offset(),
        );
        for i in 0..3 {
            assert!((result[i] - rgb[i] * 2.0).abs() < EPSILON);
        }
    }

    #[test]
    fn test_cdl_offset_adds_to_output() {
        let rgb = [0.5, 0.5, 0.5];
        let offset = [0.1, 0.1, 0.1, 0.0];
        let result = apply_cdl(
            rgb,
            &default_lift(),
            &default_gamma(),
            &default_gain(),
            &offset,
        );
        for i in 0..3 {
            assert!((result[i] - 0.6).abs() < EPSILON);
        }
    }

    #[test]
    fn test_cdl_lift_affects_shadows() {
        // With gain=1, lift shifts the entire signal:
        // out = x * 1 + lift * (1 - 1) + 0 = x (lift has no effect when gain=1)
        // Lift shows effect when gain != 1
        let rgb = [0.0, 0.0, 0.0];
        let lift = [0.1, 0.1, 0.1, 0.0];
        let gain = [0.5, 0.5, 0.5, 1.0];
        let result = apply_cdl(rgb, &lift, &default_gamma(), &gain, &default_offset());
        // At black with gain 0.5: x = 0*0.5 + 0.1*(1-0.5) = 0.05
        for i in 0..3 {
            assert!((result[i] - 0.05).abs() < EPSILON);
        }
    }

    #[test]
    fn test_cdl_negative_clamped_to_zero() {
        let rgb = [0.1, 0.1, 0.1];
        let offset = [-0.5, -0.5, -0.5, 0.0];
        let result = apply_cdl(
            rgb,
            &default_lift(),
            &default_gamma(),
            &default_gain(),
            &offset,
        );
        for i in 0..3 {
            assert!(result[i] >= 0.0, "output should never be negative");
        }
    }
}
