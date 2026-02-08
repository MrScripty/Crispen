//! GPU-to-CPU readback utilities for scope data and debug output.

/// Reads GPU buffer data back to the CPU for scope display and diagnostics.
pub struct Readback {
    _private: (),
}

impl Readback {
    /// Map a GPU buffer and read its contents to CPU memory.
    pub fn read_buffer(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        let _ = (device, queue);
        todo!()
    }
}
