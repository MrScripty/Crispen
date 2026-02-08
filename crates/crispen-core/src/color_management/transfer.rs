//! Transfer function (OETF/EOTF) trait and implementations for LOG curves.

/// A transfer function that converts between linear and non-linear encodings.
pub trait TransferFunction {
    /// Convert from non-linear (encoded) to linear light.
    fn to_linear(&self, encoded: f32) -> f32;

    /// Convert from linear light to non-linear (encoded).
    fn to_encoded(&self, linear: f32) -> f32;
}

/// sRGB transfer function (IEC 61966-2-1).
#[derive(Debug, Clone, Copy)]
pub struct SrgbTransfer;

impl TransferFunction for SrgbTransfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        let _ = encoded;
        todo!()
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        let _ = linear;
        todo!()
    }
}

/// ARRI LogC3 transfer function (ALEXA classic cameras).
#[derive(Debug, Clone, Copy)]
pub struct ArriLogC3Transfer;

impl TransferFunction for ArriLogC3Transfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        let _ = encoded;
        todo!()
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        let _ = linear;
        todo!()
    }
}

/// ARRI LogC4 transfer function (ALEXA 35 cameras).
#[derive(Debug, Clone, Copy)]
pub struct ArriLogC4Transfer;

impl TransferFunction for ArriLogC4Transfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        let _ = encoded;
        todo!()
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        let _ = linear;
        todo!()
    }
}

/// Sony S-Log3 transfer function.
#[derive(Debug, Clone, Copy)]
pub struct SLog3Transfer;

impl TransferFunction for SLog3Transfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        let _ = encoded;
        todo!()
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        let _ = linear;
        todo!()
    }
}

/// RED Log3G10 transfer function.
#[derive(Debug, Clone, Copy)]
pub struct RedLog3G10Transfer;

impl TransferFunction for RedLog3G10Transfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        let _ = encoded;
        todo!()
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        let _ = linear;
        todo!()
    }
}

/// Panasonic V-Log transfer function.
#[derive(Debug, Clone, Copy)]
pub struct VLogTransfer;

impl TransferFunction for VLogTransfer {
    fn to_linear(&self, encoded: f32) -> f32 {
        let _ = encoded;
        todo!()
    }

    fn to_encoded(&self, linear: f32) -> f32 {
        let _ = linear;
        todo!()
    }
}
