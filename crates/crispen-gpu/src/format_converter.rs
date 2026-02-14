//! GPU compute pass for converting f32 pixels to f16 (for viewer readback).

use std::num::NonZeroU64;

use crate::buffers::GpuImageHandle;

/// Viewer pixel format — configurable for profiling quality vs bandwidth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerFormat {
    /// Rgba16Float — 8 bytes/pixel. GPU compute converts f32→f16.
    F16,
    /// Rgba32Float — 16 bytes/pixel. No conversion; raw f32 readback.
    F32,
    /// Rgba8UnormSrgb — 4 bytes/pixel. GPU compute applies sRGB OETF.
    /// Eliminates CPU-side powf(1/2.4) and halves readback bandwidth vs F16.
    Srgb8,
}

impl ViewerFormat {
    /// Bytes per pixel for this format.
    pub fn bytes_per_pixel(self) -> u64 {
        match self {
            ViewerFormat::F16 => 8,
            ViewerFormat::F32 => 16,
            ViewerFormat::Srgb8 => 4,
        }
    }
}

/// Manages the f32→f16 and f32→sRGB8 conversion pipelines and their output buffers.
pub struct FormatConverter {
    f16_pipeline: wgpu::ComputePipeline,
    f16_layout: wgpu::BindGroupLayout,
    srgb_pipeline: wgpu::ComputePipeline,
    srgb_layout: wgpu::BindGroupLayout,
    pixel_count_buffer: wgpu::Buffer,
    /// Cached f16 output buffer (reallocated on dimension change).
    f16_output: Option<ConvertOutput>,
    /// Cached sRGB8 output buffer (reallocated on dimension change).
    srgb_output: Option<ConvertOutput>,
}

struct ConvertOutput {
    buffer: wgpu::Buffer,
    pixel_count: u32,
}

impl FormatConverter {
    /// Create the format conversion pipelines. Compiles `format_convert.wgsl`
    /// and `linear_to_srgb.wgsl`.
    pub fn new(device: &wgpu::Device) -> Self {
        // Shared pixel_count uniform buffer.
        let pixel_count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_format_convert_pixel_count"),
            size: 16, // u32 padded to 16 bytes for uniform alignment
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── F16 pipeline ─────────────────────────────────────────────
        let f16_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("crispen_format_convert_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/format_convert.wgsl").into()),
        });

        let f16_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("crispen_format_convert_layout"),
            entries: &[
                storage_ro_entry(0, 16),
                storage_rw_entry(1, 8),
                uniform_entry(2, 4),
            ],
        });

        let f16_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("crispen_format_convert_pipeline_layout"),
            bind_group_layouts: &[&f16_layout],
            push_constant_ranges: &[],
        });

        let f16_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("crispen_format_convert_pipeline"),
            layout: Some(&f16_pipeline_layout),
            module: &f16_shader,
            entry_point: Some("convert_f32_to_f16"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // ── sRGB pipeline ────────────────────────────────────────────
        let srgb_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("crispen_linear_to_srgb_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/linear_to_srgb.wgsl").into(),
            ),
        });

        let srgb_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("crispen_linear_to_srgb_layout"),
            entries: &[
                storage_ro_entry(0, 16),
                storage_rw_entry(1, 4),
                uniform_entry(2, 4),
            ],
        });

        let srgb_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("crispen_linear_to_srgb_pipeline_layout"),
            bind_group_layouts: &[&srgb_layout],
            push_constant_ranges: &[],
        });

        let srgb_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("crispen_linear_to_srgb_pipeline"),
            layout: Some(&srgb_pipeline_layout),
            module: &srgb_shader,
            entry_point: Some("convert_linear_to_srgb8"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self {
            f16_pipeline,
            f16_layout,
            srgb_pipeline,
            srgb_layout,
            pixel_count_buffer,
            f16_output: None,
            srgb_output: None,
        }
    }

    /// Dispatch the f32→f16 conversion on the given encoder.
    ///
    /// Returns a reference to the f16 output buffer for staging copy.
    pub fn convert(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &GpuImageHandle,
        encoder: &mut wgpu::CommandEncoder,
    ) -> &wgpu::Buffer {
        let pixel_count = source.pixel_count();

        // Ensure f16 output buffer exists and is correctly sized.
        let needs_realloc = match &self.f16_output {
            Some(out) => out.pixel_count != pixel_count,
            None => true,
        };
        if needs_realloc {
            let size = pixel_count as u64 * 8; // 8 bytes per f16 pixel
            self.f16_output = Some(ConvertOutput {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("crispen_f16_output"),
                    size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                pixel_count,
            });
        }

        let f16_out = self.f16_output.as_ref().unwrap();

        // Upload pixel count.
        queue.write_buffer(
            &self.pixel_count_buffer,
            0,
            bytemuck::cast_slice(&[pixel_count, 0u32, 0u32, 0u32]),
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crispen_format_convert_bg"),
            layout: &self.f16_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: source.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: f16_out.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.pixel_count_buffer.as_entire_binding(),
                },
            ],
        });

        let workgroups = pixel_count.div_ceil(256);

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("crispen_format_convert_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.f16_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        &f16_out.buffer
    }

    /// Dispatch the f32→sRGB8 conversion on the given encoder.
    ///
    /// Returns a reference to the sRGB8 output buffer for staging copy.
    /// Output is packed RGBA8 (4 bytes/pixel) with sRGB transfer applied.
    pub fn convert_to_srgb8(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &GpuImageHandle,
        encoder: &mut wgpu::CommandEncoder,
    ) -> &wgpu::Buffer {
        let pixel_count = source.pixel_count();

        // Ensure sRGB output buffer exists and is correctly sized.
        let needs_realloc = match &self.srgb_output {
            Some(out) => out.pixel_count != pixel_count,
            None => true,
        };
        if needs_realloc {
            let size = pixel_count as u64 * 4; // 4 bytes per sRGB pixel
            self.srgb_output = Some(ConvertOutput {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("crispen_srgb8_output"),
                    size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                pixel_count,
            });
        }

        let srgb_out = self.srgb_output.as_ref().unwrap();

        // Upload pixel count (shared buffer).
        queue.write_buffer(
            &self.pixel_count_buffer,
            0,
            bytemuck::cast_slice(&[pixel_count, 0u32, 0u32, 0u32]),
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crispen_linear_to_srgb_bg"),
            layout: &self.srgb_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: source.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: srgb_out.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.pixel_count_buffer.as_entire_binding(),
                },
            ],
        });

        let workgroups = pixel_count.div_ceil(256);

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("crispen_linear_to_srgb_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.srgb_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        &srgb_out.buffer
    }

    /// Get a reference to the f16 output buffer, if allocated.
    pub fn f16_buffer(&self) -> Option<&wgpu::Buffer> {
        self.f16_output.as_ref().map(|o| &o.buffer)
    }

    /// Get a reference to the sRGB8 output buffer, if allocated.
    pub fn srgb_buffer(&self) -> Option<&wgpu::Buffer> {
        self.srgb_output.as_ref().map(|o| &o.buffer)
    }
}

// ── Layout helpers ──────────────────────────────────────────────────

fn storage_ro_entry(binding: u32, min_size: u64) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(min_size),
        },
        count: None,
    }
}

fn storage_rw_entry(binding: u32, min_size: u64) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(min_size),
        },
        count: None,
    }
}

fn uniform_entry(binding: u32, min_size: u64) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(min_size),
        },
        count: None,
    }
}
