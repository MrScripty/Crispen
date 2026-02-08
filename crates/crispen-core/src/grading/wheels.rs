//! ASC CDL (Lift/Gamma/Gain/Offset) color wheel adjustments.

/// Apply ASC CDL (American Society of Cinematographers Color Decision List) transform.
///
/// Each parameter is `[R, G, B, Master]`. The CDL formula is:
/// `output = (input * gain + offset) ^ (1/gamma) + lift`
pub fn apply_cdl(
    rgb: [f32; 3],
    lift: &[f32; 4],
    gamma: &[f32; 4],
    gain: &[f32; 4],
    offset: &[f32; 4],
) -> [f32; 3] {
    let _ = (rgb, lift, gamma, gain, offset);
    todo!()
}
