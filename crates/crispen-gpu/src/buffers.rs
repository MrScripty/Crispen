//! GPU buffer and texture management for the grading pipeline.

use crispen_core::image::GradingImage;
use wgpu::util::DeviceExt;

/// Handle to a GPU image stored as a storage buffer of `vec4<f32>`.
pub struct GpuImageHandle {
    pub buffer: wgpu::Buffer,
    pub width: u32,
    pub height: u32,
}

impl GpuImageHandle {
    /// Upload a [`GradingImage`] to the GPU as a storage buffer.
    pub fn upload(device: &wgpu::Device, queue: &wgpu::Queue, image: &GradingImage) -> Self {
        let data: &[u8] = bytemuck::cast_slice(&image.pixels);
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("crispen_image_upload"),
            contents: data,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });
        let _ = queue; // Data written via init descriptor
        Self {
            buffer,
            width: image.width,
            height: image.height,
        }
    }

    /// Create an uninitialized GPU image buffer for output.
    pub fn create_output(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let size = (width as u64) * (height as u64) * 16; // 4 x f32
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_image_output"),
            size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            width,
            height,
        }
    }

    /// Pixel count.
    pub fn pixel_count(&self) -> u32 {
        self.width * self.height
    }

    /// Buffer size in bytes.
    pub fn byte_size(&self) -> u64 {
        (self.width as u64) * (self.height as u64) * 16
    }
}

/// Handle to a 3D LUT on the GPU.
///
/// The bake shader writes directly into `texture` as a 3D storage texture.
/// The apply shader samples it via `texture_view` + `sampler`.
pub struct GpuLutHandle {
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: u32,
}

impl GpuLutHandle {
    /// Create a new LUT handle backed by a writable 3D texture.
    pub fn new(device: &wgpu::Device, size: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("crispen_lut_texture_3d"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: size,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("crispen_lut_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            texture_view,
            sampler,
            size,
        }
    }
}

/// Handles to all scope output buffers (atomic `u32` storage).
pub struct ScopeBuffers {
    /// 256 bins x 4 channels (R, G, B, Luma) = 1024 u32s.
    pub histogram: wgpu::Buffer,
    /// `width * waveform_height * 3` u32s (one per channel).
    pub waveform: wgpu::Buffer,
    /// `resolution^2` u32s.
    pub vectorscope: wgpu::Buffer,
    /// `resolution^2` u32s.
    pub cie: wgpu::Buffer,
}

/// Configuration for scope buffer dimensions.
#[derive(Debug, Clone, Copy)]
pub struct ScopeConfig {
    pub waveform_height: u32,
    pub vectorscope_resolution: u32,
    pub cie_resolution: u32,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self {
            waveform_height: 256,
            vectorscope_resolution: 512,
            cie_resolution: 512,
        }
    }
}

impl ScopeBuffers {
    /// Create all scope output buffers, zeroed.
    pub fn new(device: &wgpu::Device, config: &ScopeConfig, image_width: u32) -> Self {
        let histogram = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_histogram_buffer"),
            size: 1024 * 4, // 256 * 4 channels * sizeof(u32)
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let waveform_elements = (image_width as u64) * (config.waveform_height as u64) * 3;
        let waveform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_waveform_buffer"),
            size: waveform_elements * 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let vs_elements =
            (config.vectorscope_resolution as u64) * (config.vectorscope_resolution as u64);
        let vectorscope = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_vectorscope_buffer"),
            size: vs_elements * 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cie_elements = (config.cie_resolution as u64) * (config.cie_resolution as u64);
        let cie = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_cie_buffer"),
            size: cie_elements * 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            histogram,
            waveform,
            vectorscope,
            cie,
        }
    }
}
