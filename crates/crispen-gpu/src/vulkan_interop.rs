//! Vulkan external memory interop for sharing textures with Bevy's renderer.

/// Handles Vulkan external memory import/export for zero-copy texture sharing.
pub struct VulkanInterop {
    _private: (),
}

impl VulkanInterop {
    /// Import an external Vulkan texture as a wgpu texture.
    pub fn import_texture(&self, device: &wgpu::Device) -> wgpu::Texture {
        let _ = device;
        todo!()
    }
}
