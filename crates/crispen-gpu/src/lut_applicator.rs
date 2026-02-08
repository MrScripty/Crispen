//! GPU compute pass for applying a baked 3D LUT to the source image.

/// Manages the `apply_lut.wgsl` compute pipeline and its resources.
pub struct LutApplicator {
    _private: (),
}

impl LutApplicator {
    /// Dispatch the LUT application compute shader on the source image.
    pub fn apply(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let _ = (device, queue);
        todo!()
    }
}
