//! Bevy resources for the color grading pipeline.

use bevy::prelude::*;
use crispen_core::image::GradingImage;
use crispen_core::scopes::{CieData, HistogramData, VectorscopeData, WaveformData};
use crispen_core::transform::lut::Lut3D;
use crispen_core::transform::params::GradingParams;
use crispen_gpu::pipeline::GpuGradingPipeline;
use crispen_gpu::GpuImageHandle;

/// Bevy resource holding the current grading parameters.
///
/// This is the single source of truth for grading state within the ECS.
/// Systems watch for changes via `Res<GradingState>` and trigger LUT re-bake.
#[derive(Resource)]
pub struct GradingState {
    /// The current grading parameters.
    pub params: GradingParams,
    /// Whether the params have changed since last LUT bake.
    pub dirty: bool,
    /// The baked 3D LUT (None until first bake).
    pub lut: Option<Lut3D>,
}

impl Default for GradingState {
    fn default() -> Self {
        Self {
            params: GradingParams::default(),
            dirty: true,
            lut: None,
        }
    }
}

/// Bevy resource holding the source and graded images.
#[derive(Resource, Default)]
pub struct ImageState {
    /// The original source image (None until loaded).
    pub source: Option<GradingImage>,
    /// The graded output image (None until first LUT apply).
    pub graded: Option<GradingImage>,
}

/// Bevy resource holding the latest scope computation results.
#[derive(Resource, Default)]
pub struct ScopeState {
    pub histogram: Option<HistogramData>,
    pub waveform: Option<WaveformData>,
    pub vectorscope: Option<VectorscopeData>,
    pub cie: Option<CieData>,
}

/// Configuration for which scopes are active.
#[derive(Resource)]
pub struct ScopeConfig {
    pub histogram_visible: bool,
    pub waveform_visible: bool,
    pub vectorscope_visible: bool,
    pub cie_visible: bool,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self {
            histogram_visible: true,
            waveform_visible: true,
            vectorscope_visible: false,
            cie_visible: false,
        }
    }
}

/// Bevy resource holding the GPU grading pipeline and uploaded source image.
///
/// Created once at startup via `GpuGradingPipeline::create_blocking()`.
/// Systems use this to bake LUTs, apply grading, and read back results.
#[derive(Resource)]
pub struct GpuPipelineState {
    /// The GPU compute pipeline for LUT baking, application, and scopes.
    pub pipeline: GpuGradingPipeline,
    /// Handle to the source image uploaded to the GPU (None until first load).
    pub source_handle: Option<GpuImageHandle>,
}
