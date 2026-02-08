//! GPU compute pass for baking `GradingParams` into a 3D LUT.

use crispen_core::transform::params::GradingParams;

/// Manages the `bake_lut.wgsl` compute pipeline and its resources.
pub struct LutBaker {
    _private: (),
}

impl LutBaker {
    /// Dispatch the LUT bake compute shader with the given parameters.
    pub fn bake(&self, device: &wgpu::Device, queue: &wgpu::Queue, params: &GradingParams) {
        let _ = (device, queue, params);
        todo!()
    }
}
