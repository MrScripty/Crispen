//! Top-level GPU grading pipeline that orchestrates all compute passes.

use crispen_core::transform::params::GradingParams;

/// Orchestrates the full GPU grading pipeline: LUT bake → apply → scopes.
pub struct GpuGradingPipeline {
    /// The wgpu device handle.
    device: wgpu::Device,
    /// The wgpu queue handle.
    queue: wgpu::Queue,
}

impl GpuGradingPipeline {
    /// Create a new GPU grading pipeline from an existing wgpu device and queue.
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self { device, queue }
    }

    /// Execute the full grading pipeline for the given parameters.
    pub fn execute(&self, params: &GradingParams) {
        let _ = params;
        todo!()
    }
}
