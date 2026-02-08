//! GPU compute dispatch for scope analysis (histogram, waveform, vectorscope, CIE).

/// Dispatches scope compute shaders and manages their output buffers.
pub struct ScopeDispatch {
    _private: (),
}

impl ScopeDispatch {
    /// Dispatch all enabled scope compute shaders.
    pub fn dispatch(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let _ = (device, queue);
        todo!()
    }
}
