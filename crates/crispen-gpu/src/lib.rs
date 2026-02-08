//! Crispen GPU — wgpu-based compute pipeline for LUT baking, application, and scopes.
//!
//! This crate owns all GPU resources. No Bevy dependency — it exposes a
//! plain wgpu API that `crispen-bevy` wraps into ECS resources and systems.

use crispen_core::transform::params::{ColorSpaceId, GradingParams};

pub mod buffers;
pub mod lut_applicator;
pub mod lut_baker;
pub mod pipeline;
pub mod readback;
pub mod scope_dispatch;
pub mod vulkan_interop;

pub use buffers::{GpuImageHandle, GpuLutHandle, ScopeBuffers, ScopeConfig};
pub use pipeline::{required_features, GpuGradingPipeline};
pub use readback::ScopeResults;

/// GPU-compatible grading parameters packed for a wgpu uniform buffer.
///
/// WGSL uniform buffers require 16-byte alignment for `vec4<f32>`.
/// Layout: 4 vec4s (64 bytes) then scalars in groups of 4 (16 bytes each)
/// then color space IDs. Total: 112 bytes.
///
/// The `Vec` curve fields from [`GradingParams`] are excluded — they are
/// baked to 1D textures on the CPU and bound separately.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GradingParamsGpu {
    pub lift: [f32; 4],
    pub gamma: [f32; 4],
    pub gain: [f32; 4],
    /// Named `offset_val` because `offset` is a WGSL built-in.
    pub offset_val: [f32; 4],

    // Scalar group 1 (16 bytes)
    pub temperature: f32,
    pub tint: f32,
    pub contrast: f32,
    pub pivot: f32,

    // Scalar group 2 (16 bytes)
    pub shadows: f32,
    pub highlights: f32,
    pub saturation: f32,
    pub hue: f32,

    // Scalar group 3 (16 bytes) — luma_mix + color space IDs as u32
    pub luma_mix: f32,
    pub input_space: u32,
    pub working_space: u32,
    pub output_space: u32,
}

impl GradingParamsGpu {
    /// Convert from the core [`GradingParams`] to the GPU-compatible layout.
    pub fn from_params(params: &GradingParams) -> Self {
        Self {
            lift: params.lift,
            gamma: params.gamma,
            gain: params.gain,
            offset_val: params.offset,
            temperature: params.temperature,
            tint: params.tint,
            contrast: params.contrast,
            pivot: params.pivot,
            shadows: params.shadows,
            highlights: params.highlights,
            saturation: params.saturation,
            hue: params.hue,
            luma_mix: params.luma_mix,
            input_space: color_space_to_u32(&params.color_management.input_space),
            working_space: color_space_to_u32(&params.color_management.working_space),
            output_space: color_space_to_u32(&params.color_management.output_space),
        }
    }
}

/// Map a [`ColorSpaceId`] to a `u32` for GPU uniform consumption.
pub fn color_space_to_u32(id: &ColorSpaceId) -> u32 {
    match id {
        ColorSpaceId::Aces2065_1 => 0,
        ColorSpaceId::AcesCg => 1,
        ColorSpaceId::AcesCc => 2,
        ColorSpaceId::AcesCct => 3,
        ColorSpaceId::Srgb => 4,
        ColorSpaceId::LinearSrgb => 5,
        ColorSpaceId::Rec2020 => 6,
        ColorSpaceId::DciP3 => 7,
        ColorSpaceId::ArriLogC3 => 8,
        ColorSpaceId::ArriLogC4 => 9,
        ColorSpaceId::SLog3 => 10,
        ColorSpaceId::RedLog3G10 => 11,
        ColorSpaceId::VLog => 12,
        ColorSpaceId::Custom(n) => 100 + n,
    }
}
