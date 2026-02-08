//! White balance adjustment via chromaticity shift.

/// Apply white balance adjustment using temperature and tint.
///
/// Temperature shifts along the blue-yellow axis, tint shifts along
/// the green-magenta axis. Both values at 0.0 produce no change.
pub fn apply_white_balance(rgb: [f32; 3], temperature: f32, tint: f32) -> [f32; 3] {
    let _ = (rgb, temperature, tint);
    todo!()
}
