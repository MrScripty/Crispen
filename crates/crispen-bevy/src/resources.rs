//! Bevy resources for the color grading pipeline.

use bevy::prelude::*;
use crispen_core::image::GradingImage;
use crispen_core::scopes::{CieData, HistogramData, VectorscopeData, WaveformData};
use crispen_core::transform::lut::Lut3D;
use crispen_core::transform::params::GradingParams;
use crispen_gpu::GpuImageHandle;
use crispen_gpu::ViewerFormat;
use crispen_gpu::pipeline::GpuGradingPipeline;
#[cfg(feature = "ocio")]
use crispen_ocio::OcioConfig;
use std::time::{Duration, Instant};

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

/// Optional OCIO-based color management. When present, it overrides the native
/// input/output color transforms in the LUT bake shader.
#[cfg(feature = "ocio")]
#[derive(Resource)]
pub struct OcioColorManagement {
    pub config: OcioConfig,
    pub input_space: String,
    pub working_space: String,
    pub display: String,
    pub view: String,
    pub idt_lut: Option<Vec<[f32; 4]>>,
    pub odt_lut: Option<Vec<[f32; 4]>>,
    pub dirty: bool,
}

/// Bevy resource holding the source image.
#[derive(Resource, Default)]
pub struct ImageState {
    /// The original source image (None until loaded).
    pub source: Option<GradingImage>,
}

/// Raw pixel bytes for the viewer, produced by the GPU pipeline.
///
/// Contains either f16 or f32 linear-light data ready to be written
/// directly into a Bevy `Image` asset with the matching `TextureFormat`.
#[derive(Resource)]
pub struct ViewerData {
    /// Raw pixel bytes (f16 or f32 depending on `format`).
    pub pixel_bytes: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// The pixel format of `pixel_bytes`.
    pub format: ViewerFormat,
}

impl Default for ViewerData {
    fn default() -> Self {
        Self {
            pixel_bytes: Vec::new(),
            width: 0,
            height: 0,
            format: ViewerFormat::F16,
        }
    }
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
            histogram_visible: false,
            waveform_visible: false,
            vectorscope_visible: false,
            cie_visible: false,
        }
    }
}

impl ScopeConfig {
    /// Whether any scope computation should run.
    pub fn any_visible(&self) -> bool {
        self.histogram_visible
            || self.waveform_visible
            || self.vectorscope_visible
            || self.cie_visible
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

/// Runtime timings for the grading pipeline.
#[derive(Resource)]
pub struct PipelinePerfStats {
    pub updates: u64,
    pub bake_time: Duration,
    pub apply_time: Duration,
    pub readback_time: Duration,
    pub total_time: Duration,
    pub slow_update_threshold: Duration,
    pub last_log_at: Instant,
}

impl Default for PipelinePerfStats {
    fn default() -> Self {
        Self {
            updates: 0,
            bake_time: Duration::ZERO,
            apply_time: Duration::ZERO,
            readback_time: Duration::ZERO,
            total_time: Duration::ZERO,
            slow_update_threshold: Duration::from_millis(10),
            last_log_at: Instant::now(),
        }
    }
}
