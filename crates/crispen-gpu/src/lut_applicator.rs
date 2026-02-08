//! GPU compute pass for applying a baked 3D LUT to the source image.

use std::num::NonZeroU64;

use crate::buffers::{GpuImageHandle, GpuLutHandle};

/// Manages the `apply_lut.wgsl` compute pipeline and its resources.
pub struct LutApplicator {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    dimensions_buffer: wgpu::Buffer,
}

impl LutApplicator {
    /// Create the LUT application pipeline. Compiles `apply_lut.wgsl`.
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("crispen_apply_lut_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/apply_lut.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("crispen_apply_lut_layout"),
            entries: &[
                // binding 0: source storage (read)
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
                // binding 1: output storage (read_write)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(16),
                    },
                    count: None,
                },
                // binding 2: LUT 3D texture
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 3: LUT sampler (trilinear)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // binding 4: dimensions uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(8),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("crispen_apply_lut_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("crispen_apply_lut_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("apply_lut"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let dimensions_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_apply_dimensions_uniform"),
            size: 16, // vec2<u32> padded to 16 bytes
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group_layout,
            dimensions_buffer,
        }
    }

    /// Dispatch the LUT application compute shader onto the given encoder.
    ///
    /// The caller is responsible for submitting the encoder.
    pub fn apply(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &GpuImageHandle,
        lut: &GpuLutHandle,
        output: &GpuImageHandle,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let dims = [source.width, source.height, 0u32, 0u32];
        queue.write_buffer(&self.dimensions_buffer, 0, bytemuck::cast_slice(&dims));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crispen_apply_lut_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: source.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&lut.texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&lut.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.dimensions_buffer.as_entire_binding(),
                },
            ],
        });

        let wg_x = source.width.div_ceil(16);
        let wg_y = source.height.div_ceil(16);

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("crispen_apply_lut_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
    }
}
