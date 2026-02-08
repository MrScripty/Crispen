//! GPU buffer and texture management for the grading pipeline.

/// Manages all GPU buffers and textures used by the grading pipeline.
pub struct GpuBuffers {
    _private: (),
}

impl GpuBuffers {
    /// Create GPU buffers sized for an image of the given dimensions.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let _ = (device, width, height);
        todo!()
    }
}
