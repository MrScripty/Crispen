//! Shared parameter definitions (ranges, defaults, steps, labels).

use bevy::prelude::*;

/// Identifies which `GradingParams` field a control (dial or wheel) manages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub enum ParamId {
    Temperature,
    Tint,
    Contrast,
    Pivot,
    MidtoneDetail,
    Shadows,
    Highlights,
    Saturation,
    Hue,
    LumaMix,
}

/// Return the identity (no-op) default for a given param.
pub fn param_default(id: ParamId) -> f32 {
    match id {
        ParamId::Temperature
        | ParamId::Tint
        | ParamId::MidtoneDetail
        | ParamId::Shadows
        | ParamId::Highlights
        | ParamId::Hue
        | ParamId::LumaMix => 0.0,
        ParamId::Contrast | ParamId::Saturation => 1.0,
        ParamId::Pivot => 0.435,
    }
}

/// Return the (min, max) range for a given param.
pub fn param_range(id: ParamId) -> (f32, f32) {
    match id {
        ParamId::Temperature | ParamId::Tint => (-100.0, 100.0),
        ParamId::Contrast | ParamId::Saturation => (0.0, 4.0),
        ParamId::Pivot | ParamId::LumaMix => (0.0, 1.0),
        ParamId::MidtoneDetail => (-1.0, 1.0),
        ParamId::Shadows | ParamId::Highlights => (-1.0, 1.0),
        ParamId::Hue => (-180.0, 180.0),
    }
}

/// Return a reasonable step increment for a given param.
pub fn param_step(id: ParamId) -> f32 {
    match id {
        ParamId::Temperature | ParamId::Tint => 1.0,
        ParamId::Hue => 1.0,
        ParamId::Contrast | ParamId::Saturation => 0.01,
        ParamId::Pivot | ParamId::LumaMix => 0.005,
        ParamId::MidtoneDetail | ParamId::Shadows | ParamId::Highlights => 0.01,
    }
}

/// Human-readable short label for a given param.
pub fn param_label(id: ParamId) -> &'static str {
    match id {
        ParamId::Temperature => "TEMP",
        ParamId::Tint => "TINT",
        ParamId::Contrast => "CONTRAST",
        ParamId::Pivot => "PIVOT",
        ParamId::MidtoneDetail => "MID DETAIL",
        ParamId::Shadows => "SHADOWS",
        ParamId::Highlights => "HIGHLIGHTS",
        ParamId::Saturation => "SATURATION",
        ParamId::Hue => "HUE",
        ParamId::LumaMix => "LUMA MIX",
    }
}
