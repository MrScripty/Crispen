//! Central parameter struct that defines the entire color transform.
//!
//! `GradingParams` is the single source of truth for all grading adjustments.
//! Every tool writes here; the LUT bake shader reads the full struct.

use serde::{Deserialize, Serialize};

/// Identifies a color space for input/working/output transforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpaceId {
    /// ACES 2065-1 (AP0 primaries, linear).
    Aces2065_1,
    /// ACEScg (AP1 primaries, linear). Default working space.
    AcesCg,
    /// ACEScc (AP1 primaries, logarithmic).
    AcesCc,
    /// ACEScct (AP1 primaries, logarithmic with toe).
    AcesCct,
    /// sRGB (Rec. 709 primaries, sRGB transfer).
    Srgb,
    /// Linear sRGB (Rec. 709 primaries, linear).
    LinearSrgb,
    /// ITU-R BT.2020 (wide gamut).
    Rec2020,
    /// DCI-P3 (digital cinema).
    DciP3,
    /// ARRI LogC3 (ALEXA classic).
    ArriLogC3,
    /// ARRI LogC4 (ALEXA 35).
    ArriLogC4,
    /// Sony S-Log3.
    SLog3,
    /// RED Log3G10.
    RedLog3G10,
    /// Panasonic V-Log.
    VLog,
    /// User-defined color space by ID.
    Custom(u32),
}

impl ColorSpaceId {
    /// Human-readable label for UI menus and status text.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Aces2065_1 => "ACES2065-1",
            Self::AcesCg => "ACEScg",
            Self::AcesCc => "ACEScc",
            Self::AcesCct => "ACEScct",
            Self::Srgb => "sRGB",
            Self::LinearSrgb => "Linear sRGB",
            Self::Rec2020 => "Rec.2020",
            Self::DciP3 => "DCI-P3",
            Self::ArriLogC3 => "ARRI LogC3",
            Self::ArriLogC4 => "ARRI LogC4",
            Self::SLog3 => "Sony S-Log3",
            Self::RedLog3G10 => "RED Log3G10",
            Self::VLog => "Panasonic V-Log",
            Self::Custom(_) => "Custom",
        }
    }

    /// Built-in color spaces supported by Crispen's native color-management path.
    pub fn all() -> &'static [Self] {
        const ALL: [ColorSpaceId; 13] = [
            ColorSpaceId::Aces2065_1,
            ColorSpaceId::AcesCg,
            ColorSpaceId::AcesCc,
            ColorSpaceId::AcesCct,
            ColorSpaceId::Srgb,
            ColorSpaceId::LinearSrgb,
            ColorSpaceId::Rec2020,
            ColorSpaceId::DciP3,
            ColorSpaceId::ArriLogC3,
            ColorSpaceId::ArriLogC4,
            ColorSpaceId::SLog3,
            ColorSpaceId::RedLog3G10,
            ColorSpaceId::VLog,
        ];
        &ALL
    }
}

/// Identifies the display's electro-optical transfer function (OETF).
///
/// When the GPU pipeline outputs display-referred values (e.g. after an OCIO
/// ODT), the OETF encoding must be inverted so that Bevy's sRGB framebuffer
/// can re-apply the correct encoding for the monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayOetf {
    /// Output is already linear — no inverse needed.
    Linear,
    /// sRGB / Display P3 SDR (most common).
    Srgb,
    /// PQ (SMPTE ST.2084) for HDR displays.
    Pq,
    /// Hybrid Log-Gamma for broadcast HDR.
    Hlg,
}

impl DisplayOetf {
    /// GPU-compatible integer for the shader uniform.
    pub const fn to_u32(self) -> u32 {
        match self {
            Self::Linear => 0,
            Self::Srgb => 1,
            Self::Pq => 2,
            Self::Hlg => 3,
        }
    }

    /// Default for serde deserialization when the field is absent.
    fn default_srgb() -> Self {
        Self::Srgb
    }
}

/// Configuration for color space transforms in the grading pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorManagementConfig {
    /// Input color space of the source image.
    pub input_space: ColorSpaceId,
    /// Working color space for grading operations.
    pub working_space: ColorSpaceId,
    /// Output color space (gamut) for display. The shader outputs linear
    /// values in this gamut; Bevy's sRGB framebuffer applies the final OETF.
    pub output_space: ColorSpaceId,
    /// Display OETF to invert when using OCIO ODT output.
    #[serde(default = "DisplayOetf::default_srgb")]
    pub display_oetf: DisplayOetf,
}

impl Default for ColorManagementConfig {
    fn default() -> Self {
        Self {
            input_space: ColorSpaceId::Srgb,
            working_space: ColorSpaceId::AcesCg,
            output_space: ColorSpaceId::LinearSrgb,
            display_oetf: DisplayOetf::Srgb,
        }
    }
}

/// Every tool writes here. The LUT bake shader reads the full struct.
/// This is the immutable contract between UI, Bevy, and GPU.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradingParams {
    /// Color management configuration.
    pub color_management: ColorManagementConfig,

    // Primary Wheels [R, G, B, Master]
    /// Lift adjustment (shadows). Default: `[0, 0, 0, 0]`.
    pub lift: [f32; 4],
    /// Gamma adjustment (midtones). Default: `[1, 1, 1, 1]`.
    pub gamma: [f32; 4],
    /// Gain adjustment (highlights). Default: `[1, 1, 1, 1]`.
    pub gain: [f32; 4],
    /// Offset adjustment. Default: `[0, 0, 0, 0]`.
    pub offset: [f32; 4],

    // Sliders
    /// Color temperature shift. 0.0 = neutral.
    pub temperature: f32,
    /// Tint shift (green-magenta). 0.0 = neutral.
    pub tint: f32,
    /// Contrast multiplier. 1.0 = neutral.
    pub contrast: f32,
    /// Contrast pivot point. Default: 0.435.
    pub pivot: f32,
    /// Midtone detail enhancement. 0.0 = off (spatial, separate pass).
    pub midtone_detail: f32,
    /// Shadow recovery. 0.0 = neutral.
    pub shadows: f32,
    /// Highlight recovery. 0.0 = neutral.
    pub highlights: f32,
    /// Saturation multiplier. 1.0 = neutral.
    pub saturation: f32,
    /// Hue rotation in degrees. 0.0 = no rotation.
    pub hue: f32,
    /// Luma mix weight. 0.0 = full chroma weight.
    pub luma_mix: f32,

    // Curves (control points, baked to 1D LUTs before LUT bake)
    /// Hue-vs-hue curve control points.
    pub hue_vs_hue: Vec<[f32; 2]>,
    /// Hue-vs-saturation curve control points.
    pub hue_vs_sat: Vec<[f32; 2]>,
    /// Luminance-vs-saturation curve control points.
    pub lum_vs_sat: Vec<[f32; 2]>,
    /// Saturation-vs-saturation curve control points.
    pub sat_vs_sat: Vec<[f32; 2]>,
}

impl Default for GradingParams {
    /// Produces an identity (no-op) transform — image passes through unchanged.
    fn default() -> Self {
        Self {
            color_management: ColorManagementConfig::default(),
            lift: [0.0, 0.0, 0.0, 0.0],
            gamma: [1.0, 1.0, 1.0, 1.0],
            gain: [1.0, 1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0, 0.0],
            temperature: 0.0,
            tint: 0.0,
            contrast: 1.0,
            pivot: 0.435,
            midtone_detail: 0.0,
            shadows: 0.0,
            highlights: 0.0,
            saturation: 1.0,
            hue: 0.0,
            luma_mix: 0.0,
            hue_vs_hue: Vec::new(),
            hue_vs_sat: Vec::new(),
            lum_vs_sat: Vec::new(),
            sat_vs_sat: Vec::new(),
        }
    }
}
