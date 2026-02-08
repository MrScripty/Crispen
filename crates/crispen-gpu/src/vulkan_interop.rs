//! Vulkan external memory interop for zero-copy texture sharing with Bevy.
//!
//! ## Phase 3 Implementation Plan
//!
//! When Bevy and crispen-gpu share a Vulkan device, textures can be shared
//! via VK_KHR_external_memory extensions:
//!
//! 1. **Export from crispen-gpu**: Create textures with external memory handles.
//!    Obtain the `VkDeviceMemory` fd via `VK_KHR_external_memory_fd`.
//! 2. **Import in Bevy**: Use wgpu-hal's Vulkan backend to import the fd
//!    as a `wgpu::Texture`, avoiding the GPU→CPU→GPU roundtrip.
//!
//! This requires access to `wgpu-hal`'s `vulkan::Device` via `as_hal()`.
//! See `wgpu-hal/src/vulkan/` in the wgpu source tree for the raw API.

/// Placeholder for Vulkan external memory interop (Phase 3).
pub struct VulkanInterop {
    _private: (),
}

impl VulkanInterop {
    /// Import an external Vulkan texture as a wgpu texture.
    ///
    /// Not yet implemented — returns `None` and logs a warning.
    pub fn import_texture(&self, _device: &wgpu::Device) -> Option<wgpu::Texture> {
        tracing::warn!("Vulkan interop not yet implemented (Phase 3)");
        None
    }
}
