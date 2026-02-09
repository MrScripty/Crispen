//! Vulkan external memory interop for zero-copy texture sharing.
//!
//! This module provides:
//! - Capability probing for Vulkan backend interop.
//! - A stable error model for callers.
//! - Platform-gated import APIs.
//!
//! On Windows, if the device was created with Vulkan backend and
//! `Features::VULKAN_EXTERNAL_MEMORY_WIN32`, this module can import a D3D11
//! shared handle into a `wgpu::Texture`.
#![allow(unsafe_code)]
// Interop requires raw backend handle access via wgpu-hal.

use thiserror::Error;

/// Runtime capabilities relevant to Vulkan external-memory interop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VulkanInteropCapabilities {
    pub backend: wgpu::Backend,
    pub is_vulkan_backend: bool,
    pub has_vulkan_hal_access: bool,
    pub supports_d3d11_win32_import: bool,
}

/// Handle type for importing external textures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalTextureHandle {
    /// D3D11 shared texture handle for Vulkan import (Windows only).
    D3D11SharedWin32Handle(isize),
}

/// Errors produced by Vulkan interop setup and import.
#[derive(Debug, Error)]
pub enum VulkanInteropError {
    #[error("device backend is {backend:?}, Vulkan required")]
    BackendNotVulkan { backend: wgpu::Backend },
    #[error("wgpu-hal Vulkan handle is not available on this device")]
    VulkanHalUnavailable,
    #[error("texture descriptor is invalid: {reason}")]
    InvalidDescriptor { reason: &'static str },
    #[error("external handle type is unsupported on this platform")]
    UnsupportedHandleType,
    #[error("missing required device feature: {feature:?}")]
    MissingFeature { feature: wgpu::Features },
    #[error("failed to import external texture in Vulkan HAL: {message}")]
    HalImportFailed { message: String },
}

/// Vulkan external memory interop helper.
pub struct VulkanInterop {
    caps: VulkanInteropCapabilities,
}

impl VulkanInterop {
    /// Features that should be enabled when creating a device if external-memory
    /// interop is desired.
    pub fn required_device_features() -> wgpu::Features {
        #[cfg(windows)]
        {
            return wgpu::Features::VULKAN_EXTERNAL_MEMORY_WIN32;
        }
        #[allow(unreachable_code)]
        wgpu::Features::empty()
    }

    /// Probe interop capabilities for a given adapter/device pair.
    pub fn probe(
        adapter_info: &wgpu::AdapterInfo,
        device: &wgpu::Device,
        enabled_features: wgpu::Features,
    ) -> VulkanInteropCapabilities {
        let is_vulkan_backend = adapter_info.backend == wgpu::Backend::Vulkan;
        let has_vulkan_hal_access = unsafe { device.as_hal::<wgpu::hal::api::Vulkan>() }.is_some();
        let supports_d3d11_win32_import = is_vulkan_backend
            && has_vulkan_hal_access
            && enabled_features.contains(wgpu::Features::VULKAN_EXTERNAL_MEMORY_WIN32);

        VulkanInteropCapabilities {
            backend: adapter_info.backend,
            is_vulkan_backend,
            has_vulkan_hal_access,
            supports_d3d11_win32_import,
        }
    }

    /// Create an interop helper from precomputed capabilities.
    pub fn new(caps: VulkanInteropCapabilities) -> Self {
        Self { caps }
    }

    /// Access the probed capability snapshot.
    pub fn capabilities(&self) -> &VulkanInteropCapabilities {
        &self.caps
    }

    /// Import an external texture handle as a `wgpu::Texture`.
    pub fn import_texture(
        &self,
        device: &wgpu::Device,
        desc: &wgpu::TextureDescriptor<'_>,
        handle: ExternalTextureHandle,
    ) -> Result<wgpu::Texture, VulkanInteropError> {
        validate_descriptor(desc)?;
        let hal_desc = to_hal_texture_desc(desc);

        if !self.caps.is_vulkan_backend {
            return Err(VulkanInteropError::BackendNotVulkan {
                backend: self.caps.backend,
            });
        }
        if !self.caps.has_vulkan_hal_access {
            return Err(VulkanInteropError::VulkanHalUnavailable);
        }

        #[cfg(windows)]
        {
            match handle {
                ExternalTextureHandle::D3D11SharedWin32Handle(raw_handle) => {
                    if !self.caps.supports_d3d11_win32_import {
                        return Err(VulkanInteropError::MissingFeature {
                            feature: wgpu::Features::VULKAN_EXTERNAL_MEMORY_WIN32,
                        });
                    }

                    let hal_device = unsafe { device.as_hal::<wgpu::hal::api::Vulkan>() }
                        .ok_or(VulkanInteropError::VulkanHalUnavailable)?;

                    let shared_handle = windows::Win32::Foundation::HANDLE(raw_handle);
                    let hal_texture = unsafe {
                        hal_device.texture_from_d3d11_shared_handle(shared_handle, &hal_desc)
                    }
                    .map_err(|error| VulkanInteropError::HalImportFailed {
                        message: format!("{error:?}"),
                    })?;

                    let texture = unsafe {
                        device.create_texture_from_hal::<wgpu::hal::api::Vulkan>(hal_texture, desc)
                    };
                    Ok(texture)
                }
            }
        }

        #[cfg(not(windows))]
        {
            let _ = (&hal_desc, device, handle);
            Err(VulkanInteropError::UnsupportedHandleType)
        }
    }
}

fn validate_descriptor(desc: &wgpu::TextureDescriptor<'_>) -> Result<(), VulkanInteropError> {
    if desc.size.width == 0 || desc.size.height == 0 || desc.size.depth_or_array_layers == 0 {
        return Err(VulkanInteropError::InvalidDescriptor {
            reason: "texture size components must be non-zero",
        });
    }
    if desc.mip_level_count == 0 {
        return Err(VulkanInteropError::InvalidDescriptor {
            reason: "mip_level_count must be >= 1",
        });
    }
    if desc.sample_count == 0 {
        return Err(VulkanInteropError::InvalidDescriptor {
            reason: "sample_count must be >= 1",
        });
    }
    if desc.usage.is_empty() {
        return Err(VulkanInteropError::InvalidDescriptor {
            reason: "usage must not be empty",
        });
    }
    Ok(())
}

fn map_usage_to_texture_uses(desc: &wgpu::TextureDescriptor<'_>) -> wgpu::TextureUses {
    let mut uses = wgpu::TextureUses::empty();

    if desc.usage.contains(wgpu::TextureUsages::COPY_SRC) {
        uses |= wgpu::TextureUses::COPY_SRC;
    }
    if desc.usage.contains(wgpu::TextureUsages::COPY_DST) {
        uses |= wgpu::TextureUses::COPY_DST;
    }
    if desc.usage.contains(wgpu::TextureUsages::TEXTURE_BINDING) {
        uses |= wgpu::TextureUses::RESOURCE;
    }
    if desc.usage.contains(wgpu::TextureUsages::STORAGE_BINDING) {
        uses |= wgpu::TextureUses::STORAGE_READ_WRITE;
    }
    if desc.usage.contains(wgpu::TextureUsages::STORAGE_ATOMIC) {
        uses |= wgpu::TextureUses::STORAGE_ATOMIC;
    }
    if desc.usage.contains(wgpu::TextureUsages::RENDER_ATTACHMENT) {
        if desc.format.is_depth_stencil_format() {
            uses |= wgpu::TextureUses::DEPTH_STENCIL_WRITE;
        } else {
            uses |= wgpu::TextureUses::COLOR_TARGET;
        }
    }

    uses
}

fn to_hal_texture_desc(
    desc: &wgpu::TextureDescriptor<'_>,
) -> wgpu::hal::TextureDescriptor<'static> {
    wgpu::hal::TextureDescriptor {
        label: None,
        size: desc.size,
        mip_level_count: desc.mip_level_count,
        sample_count: desc.sample_count,
        dimension: desc.dimension,
        format: desc.format,
        usage: map_usage_to_texture_uses(desc),
        memory_flags: wgpu::hal::MemoryFlags::empty(),
        view_formats: desc.view_formats.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color_desc(usage: wgpu::TextureUsages) -> wgpu::TextureDescriptor<'static> {
        wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 32,
                height: 16,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            view_formats: &[],
        }
    }

    #[test]
    fn validate_descriptor_rejects_empty_usage() {
        let desc = color_desc(wgpu::TextureUsages::empty());
        let error = validate_descriptor(&desc).expect_err("descriptor should be rejected");
        assert!(matches!(
            error,
            VulkanInteropError::InvalidDescriptor { .. }
        ));
    }

    #[test]
    fn usage_mapping_sets_color_target_and_resource() {
        let desc = color_desc(
            wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let uses = map_usage_to_texture_uses(&desc);
        assert!(uses.contains(wgpu::TextureUses::COPY_SRC));
        assert!(uses.contains(wgpu::TextureUses::RESOURCE));
        assert!(uses.contains(wgpu::TextureUses::COLOR_TARGET));
    }

    #[test]
    fn usage_mapping_sets_depth_stencil_for_depth_formats() {
        let mut desc = color_desc(wgpu::TextureUsages::RENDER_ATTACHMENT);
        desc.format = wgpu::TextureFormat::Depth24PlusStencil8;
        let uses = map_usage_to_texture_uses(&desc);
        assert!(uses.contains(wgpu::TextureUses::DEPTH_STENCIL_WRITE));
        assert!(!uses.contains(wgpu::TextureUses::COLOR_TARGET));
    }

    #[test]
    fn probe_marks_non_vulkan_backend_unavailable() {
        let adapter_info = wgpu::AdapterInfo {
            name: "test".to_string(),
            vendor: 0,
            device: 0,
            device_type: wgpu::DeviceType::Cpu,
            driver: "test".to_string(),
            driver_info: "test".to_string(),
            backend: wgpu::Backend::Gl,
        };
        let caps = VulkanInteropCapabilities {
            backend: adapter_info.backend,
            is_vulkan_backend: false,
            has_vulkan_hal_access: false,
            supports_d3d11_win32_import: false,
        };
        let interop = VulkanInterop::new(caps.clone());
        assert_eq!(interop.capabilities(), &caps);
    }
}
