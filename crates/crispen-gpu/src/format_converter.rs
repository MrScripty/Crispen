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
}

impl ViewerFormat {
    /// Bytes per pixel for this format.
    pub fn bytes_per_pixel(self) -> u64 {
        match self {
            ViewerFormat::F16 => 8,
            ViewerFormat::F32 => 16,
        }
    }
}

/// Manages the f32→f16 conversion pipeline and its output buffer.
pub struct FormatConverter {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    pixel_count_buffer: wgpu::Buffer,
    /// Cached f16 output buffer (reallocated on dimension change).
    f16_output: Option<F16Output>,
}

struct F16Output {
    buffer: wgpu::Buffer,
    pixel_count: u32,
}

impl FormatConverter {
    /// Create the format conversion pipeline. Compiles `format_convert.wgsl`.
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("crispen_format_convert_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/format_convert.wgsl").into(),
            ),
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("crispen_format_convert_layout"),
                entries: &[
                    // binding 0: f32 input (read)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(16),
                        },
                        count: None,
                    },
                    // binding 1: f16 output (read_write)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(8),
                        },
                        count: None,
                    },
                    // binding 2: pixel_count uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(4),
                        },
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("crispen_format_convert_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("crispen_format_convert_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("convert_f32_to_f16"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let pixel_count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_format_convert_pixel_count"),
            size: 16, // u32 padded to 16 bytes for uniform alignment
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group_layout,
            pixel_count_buffer,
            f16_output: None,
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
            self.f16_output = Some(F16Output {
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
            layout: &self.bind_group_layout,
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
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        &f16_out.buffer
    }

    /// Get a reference to the f16 output buffer, if allocated.
    pub fn f16_buffer(&self) -> Option<&wgpu::Buffer> {
        self.f16_output.as_ref().map(|o| &o.buffer)
    }
}
