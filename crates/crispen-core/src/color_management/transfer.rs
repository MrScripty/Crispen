//! Transfer function (OETF/EOTF) implementations for log curves and gamma.
//!
//! Each implementation uses the published specification constants.
//! Transfer functions convert between non-linear (encoded) and linear light values.

use crate::transform::params::ColorSpaceId;

/// A transfer function that converts between linear and non-linear encodings.
pub trait TransferFunction: Send + Sync {
    /// Convert from non-linear (encoded) to linear light.
    fn to_linear(&self, encoded: f32) -> f32;

    /// Convert from linear light to non-linear (encoded).
    fn to_encoded(&self, linear: f32) -> f32;
}

/// Get the transfer function for a color space, if it has a non-linear encoding.
///
/// Returns `None` for linear color spaces (LinearSrgb, AcesCg, Aces2065_1,
/// Rec2020, DciP3).
pub fn get_transfer(space: ColorSpaceId) -> Option<Box<dyn TransferFunction>> {
    match space {
        ColorSpaceId::Srgb => Some(Box::new(SrgbTransfer)),
        ColorSpaceId::AcesCc => Some(Box::new(AcesCcTransfer)),
        ColorSpaceId::AcesCct => Some(Box::new(AcesCctTransfer)),
        ColorSpaceId::ArriLogC3 => Some(Box::new(ArriLogC3Transfer)),
        ColorSpaceId::ArriLogC4 => Some(Box::new(ArriLogC4Transfer)),
        ColorSpaceId::SLog3 => Some(Box::new(SLog3Transfer)),
        ColorSpaceId::RedLog3G10 => Some(Box::new(RedLog3G10Transfer)),
        ColorSpaceId::VLog => Some(Box::new(VLogTransfer)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// sRGB (IEC 61966-2-1)
// ---------------------------------------------------------------------------

/// sRGB transfer function per IEC 61966-2-1.
///
/// ```text
/// to_linear:   V <= 0.04045 → V / 12.92
///              V >  0.04045 → ((V + 0.055) / 1.055) ^ 2.4
///
/// from_linear: L <= 0.0031308 → L × 12.92
///              L >  0.0031308 → 1.055 × L^(1/2.4) − 0.055
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SrgbTransfer;

impl TransferFunction for SrgbTransfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        if encoded <= 0.04045 {
            encoded / 12.92
        } else {
            ((encoded + 0.055) / 1.055).powf(2.4)
        }
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        if linear <= 0.0031308 {
            linear * 12.92
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        }
    }
}

// ---------------------------------------------------------------------------
// ARRI LogC3 (ALEXA classic, EI 800)
// ---------------------------------------------------------------------------

/// ARRI LogC3 transfer function for ALEXA classic cameras at EI 800.
///
/// # Reference
/// ARRI LogC Curve — Usage in VFX (2017)
///
/// ```text
/// to_linear: t <= E_CUT → (t - D) / C
///            t >  E_CUT → (10^((t - D) / C) - B) / A
///
/// from_linear: x <= CUT → C × x + D
///              x >  CUT → C × log10(A × x + B) + D
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ArriLogC3Transfer;

impl ArriLogC3Transfer {
    // EI 800 constants from ARRI specification
    const A: f32 = 5.555556;
    const B: f32 = 0.052272;
    const C: f32 = 0.247190;
    const D: f32 = 0.385537;
    const CUT: f32 = 0.010591;
    // Linear segment uses separate E/F constants
    const E: f32 = 5.367655;
    const F: f32 = 0.092809;
    const E_CUT: f32 = 0.149_651; // E * CUT + F
}

impl TransferFunction for ArriLogC3Transfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        if encoded <= Self::E_CUT {
            (encoded - Self::F) / Self::E
        } else {
            (10.0_f32.powf((encoded - Self::D) / Self::C) - Self::B) / Self::A
        }
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        if linear <= Self::CUT {
            Self::E * linear + Self::F
        } else {
            Self::C * (Self::A * linear + Self::B).log10() + Self::D
        }
    }
}

// ---------------------------------------------------------------------------
// ARRI LogC4 (ALEXA 35)
// ---------------------------------------------------------------------------

/// ARRI LogC4 transfer function for ALEXA 35 cameras.
///
/// # Reference
/// ARRI LogC4 Specification (2022)
///
/// ```text
/// to_linear: t <= E_CUT → (t - D) / C
///            t >  E_CUT → (2^((t - D) / C) - B) / A
///
/// from_linear: x <= CUT → C × x + D
///              x >  CUT → C × log2(A × x + B) + D
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ArriLogC4Transfer;

impl ArriLogC4Transfer {
    const A: f32 = 2231.826_3;
    const B: f32 = 64.0;
    const C: f32 = 0.074_107_56;
    const D: f32 = 0.092_864_12;
    const CUT: f32 = -0.023_440_45;
    const E_CUT: f32 = 0.090_600_96;
}

impl TransferFunction for ArriLogC4Transfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        if encoded <= Self::E_CUT {
            (encoded - Self::D) / Self::C
        } else {
            (2.0_f32.powf((encoded - Self::D) / Self::C) - Self::B) / Self::A
        }
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        if linear <= Self::CUT {
            Self::C * linear + Self::D
        } else {
            Self::C * (Self::A * linear + Self::B).log2() + Self::D
        }
    }
}

// ---------------------------------------------------------------------------
// Sony S-Log3
// ---------------------------------------------------------------------------

/// Sony S-Log3 transfer function.
///
/// # Reference
/// Sony Technical Summary for S-Gamut3.Cine/S-Log3 (2014)
///
/// ```text
/// to_linear: t >= THRESHOLD_E → (10^((t - 0.4105571850) / 0.2556207230) + 0.0526315790) / 4.7368421060
///            t <  THRESHOLD_E → (t - 0.0929 - 0.0155818840) / (0.1677922920 × 4.7368421060)
///
/// from_linear: x >= THRESHOLD → 0.4105571850 + 0.2556207230 × log10(4.7368421060 × x − 0.0526315790)
///              x <  THRESHOLD → 0.1677922920 × (4.7368421060 × x + 0.0155818840) + 0.0929
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SLog3Transfer;

impl SLog3Transfer {
    const THRESHOLD: f32 = 0.011_25;
    // Encoded value at threshold: (0.01125 * (171.2102946929 - 95) / 0.01125 + 95) / 1023
    const THRESHOLD_E: f32 = 0.167_360; // 171.2102946929 / 1023
}

impl TransferFunction for SLog3Transfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        if encoded >= Self::THRESHOLD_E {
            0.19 * 10.0_f32.powf((encoded * 1023.0 - 420.0) / 261.5) - 0.01
        } else {
            (encoded * 1023.0 - 95.0) * 0.011_25 / (171.210_3 - 95.0)
        }
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        if linear >= Self::THRESHOLD {
            (420.0 + 261.5 * ((linear + 0.01) / 0.19).log10()) / 1023.0
        } else {
            (linear * (171.210_3 - 95.0) / 0.011_25 + 95.0) / 1023.0
        }
    }
}

// ---------------------------------------------------------------------------
// RED Log3G10
// ---------------------------------------------------------------------------

/// RED Log3G10 transfer function.
///
/// # Reference
/// RED White Paper: REDWideGamutRGB and Log3G10 (2017)
///
/// ```text
/// to_linear: t <= 0 → (t - 0.01) / 155.975327
///            t >  0 → (10^(t / 0.224282) − 1) / 155.975327
///
/// from_linear: x <= CUT → x × 155.975327 + 0.01
///              x >  CUT → 0.224282 × log10(x × 155.975327 + 1) + 0.0
/// ```
#[derive(Debug, Clone, Copy)]
pub struct RedLog3G10Transfer;

impl RedLog3G10Transfer {
    const A: f32 = 155.975_33;
    const B: f32 = 0.01;
    const C: f32 = 0.224_282;
}

impl TransferFunction for RedLog3G10Transfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        if encoded < 0.0 {
            (encoded - Self::B) / Self::A
        } else {
            (10.0_f32.powf(encoded / Self::C) - 1.0) / Self::A
        }
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        let x = linear * Self::A;
        if x < 0.0 {
            x + Self::B
        } else {
            Self::C * (x + 1.0).log10()
        }
    }
}

// ---------------------------------------------------------------------------
// Panasonic V-Log
// ---------------------------------------------------------------------------

/// Panasonic V-Log transfer function.
///
/// # Reference
/// Panasonic V-Log/V-Gamut Technical Documentation (2014)
///
/// ```text
/// to_linear: t < D → (t - 0.125) / 5.6
///            t >= D → 10^((t - D) / C) − B
///
/// from_linear: x < CUT → 5.6 × x + 0.125
///              x >= CUT → C × log10(x + B) + D
/// ```
#[derive(Debug, Clone, Copy)]
pub struct VLogTransfer;

impl VLogTransfer {
    const B: f32 = 0.00873;
    const C: f32 = 0.241514;
    const D: f32 = 0.598206;
    const CUT: f32 = 0.01;
    // Encoded value at CUT: 5.6 * CUT + 0.125
    const CUT_ENCODED: f32 = 0.181;
}

impl TransferFunction for VLogTransfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        if encoded < Self::CUT_ENCODED {
            (encoded - 0.125) / 5.6
        } else {
            10.0_f32.powf((encoded - Self::D) / Self::C) - Self::B
        }
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        if linear < Self::CUT {
            5.6 * linear + 0.125
        } else {
            Self::C * (linear + Self::B).log10() + Self::D
        }
    }
}

// ---------------------------------------------------------------------------
// ACEScc (logarithmic, S-2014-003)
// ---------------------------------------------------------------------------

/// ACEScc transfer function — pure logarithmic encoding in AP1.
///
/// # Reference
/// S-2014-003: ACEScc — A Logarithmic Encoding of ACES Data
///
/// ```text
/// to_linear: t <= −0.3014 → (2^(t × 17.52 − 9.72) − 1e-15) × 2
///            otherwise   → 2^(t × 17.52 − 9.72)
///
/// from_linear: x <= 0       → (log2(1e-15) + 9.72) / 17.52
///              x <  2^-15   → (log2(1e-15 + x × 0.5) + 9.72) / 17.52
///              otherwise    → (log2(x) + 9.72) / 17.52
/// ```
#[derive(Debug, Clone, Copy)]
pub struct AcesCcTransfer;

impl TransferFunction for AcesCcTransfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        if encoded <= -0.3014 {
            (2.0_f32.powf(encoded * 17.52 - 9.72) - 1e-15) * 2.0
        } else {
            2.0_f32.powf(encoded * 17.52 - 9.72)
        }
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        let min_val: f32 = 2.0_f32.powi(-15);
        if linear <= 0.0 {
            (1e-15_f32.log2() + 9.72) / 17.52
        } else if linear < min_val {
            ((1e-15 + linear * 0.5).log2() + 9.72) / 17.52
        } else {
            (linear.log2() + 9.72) / 17.52
        }
    }
}

// ---------------------------------------------------------------------------
// ACEScct (logarithmic with toe, S-2016-001)
// ---------------------------------------------------------------------------

/// ACEScct transfer function — logarithmic encoding with a toe for shadow detail.
///
/// # Reference
/// S-2016-001: ACEScct — A Quasi-Logarithmic Encoding of ACES Data
///
/// ```text
/// CUT = 0.0078125 (2^-7)
/// CUT_ENCODED ≈ 0.155251141552511
///
/// to_linear: t <= CUT_ENCODED → (t − 0.0729055341958355) / 10.5402377416545
///            otherwise        → 2^(t × 17.52 − 9.72)
///
/// from_linear: x <= CUT → 10.5402377416545 × x + 0.0729055341958355
///              otherwise → (log2(x) + 9.72) / 17.52
/// ```
#[derive(Debug, Clone, Copy)]
pub struct AcesCctTransfer;

impl AcesCctTransfer {
    const CUT: f32 = 0.0078125;
    const CUT_ENCODED: f32 = 0.155_251_14;
    const SLOPE: f32 = 10.540_238;
    const OFFSET: f32 = 0.072_905_534;
}

impl TransferFunction for AcesCctTransfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        if encoded <= Self::CUT_ENCODED {
            (encoded - Self::OFFSET) / Self::SLOPE
        } else {
            2.0_f32.powf(encoded * 17.52 - 9.72)
        }
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        if linear <= Self::CUT {
            Self::SLOPE * linear + Self::OFFSET
        } else {
            (linear.log2() + 9.72) / 17.52
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    fn assert_roundtrip(tf: &dyn TransferFunction, values: &[f32]) {
        for &v in values {
            let encoded = tf.to_encoded(v);
            let back = tf.to_linear(encoded);
            assert!(
                (v - back).abs() < EPSILON,
                "roundtrip failed for {v}: encoded={encoded}, back={back}, diff={}",
                (v - back).abs()
            );
        }
    }

    #[test]
    fn test_srgb_linearize_roundtrip_preserves_values() {
        let tf = SrgbTransfer;
        assert_roundtrip(&tf, &[0.0, 0.001, 0.01, 0.1, 0.5, 0.9, 1.0]);
    }

    #[test]
    fn test_srgb_linearize_known_values() {
        let tf = SrgbTransfer;
        assert!((tf.to_linear(0.0) - 0.0).abs() < EPSILON);
        assert!((tf.to_linear(1.0) - 1.0).abs() < EPSILON);
        // Mid-gray sRGB ≈ 0.5 encodes ~0.214 linear
        assert!((tf.to_linear(0.5) - 0.214041).abs() < 0.001);
    }

    #[test]
    fn test_logc3_linearize_roundtrip_preserves_values() {
        let tf = ArriLogC3Transfer;
        assert_roundtrip(&tf, &[0.0, 0.005, 0.01, 0.1, 0.5, 1.0, 5.0]);
    }

    #[test]
    fn test_logc4_linearize_roundtrip_preserves_values() {
        let tf = ArriLogC4Transfer;
        assert_roundtrip(&tf, &[0.0, 0.001, 0.01, 0.1, 0.5, 1.0]);
    }

    #[test]
    fn test_slog3_linearize_roundtrip_preserves_values() {
        let tf = SLog3Transfer;
        assert_roundtrip(&tf, &[0.01, 0.1, 0.5, 1.0]);
    }

    #[test]
    fn test_redlog3g10_linearize_roundtrip_preserves_values() {
        let tf = RedLog3G10Transfer;
        assert_roundtrip(&tf, &[0.0, 0.01, 0.1, 0.5, 1.0]);
    }

    #[test]
    fn test_vlog_linearize_roundtrip_preserves_values() {
        let tf = VLogTransfer;
        assert_roundtrip(&tf, &[0.01, 0.1, 0.5, 1.0]);
    }

    #[test]
    fn test_acescc_linearize_roundtrip_preserves_values() {
        let tf = AcesCcTransfer;
        assert_roundtrip(&tf, &[0.001, 0.01, 0.1, 0.5, 1.0]);
    }

    #[test]
    fn test_acescct_linearize_roundtrip_preserves_values() {
        let tf = AcesCctTransfer;
        assert_roundtrip(&tf, &[0.001, 0.01, 0.1, 0.5, 1.0]);
    }

    #[test]
    fn test_get_transfer_returns_none_for_linear_spaces() {
        assert!(get_transfer(ColorSpaceId::LinearSrgb).is_none());
        assert!(get_transfer(ColorSpaceId::AcesCg).is_none());
        assert!(get_transfer(ColorSpaceId::Aces2065_1).is_none());
        assert!(get_transfer(ColorSpaceId::Rec2020).is_none());
        assert!(get_transfer(ColorSpaceId::DciP3).is_none());
    }

    #[test]
    fn test_get_transfer_returns_some_for_encoded_spaces() {
        assert!(get_transfer(ColorSpaceId::Srgb).is_some());
        assert!(get_transfer(ColorSpaceId::ArriLogC3).is_some());
        assert!(get_transfer(ColorSpaceId::ArriLogC4).is_some());
        assert!(get_transfer(ColorSpaceId::SLog3).is_some());
        assert!(get_transfer(ColorSpaceId::RedLog3G10).is_some());
        assert!(get_transfer(ColorSpaceId::VLog).is_some());
        assert!(get_transfer(ColorSpaceId::AcesCc).is_some());
        assert!(get_transfer(ColorSpaceId::AcesCct).is_some());
    }
}
