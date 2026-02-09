//! GPU compute dispatch for scope analysis (histogram, waveform, vectorscope, CIE).

use std::num::NonZeroU64;

use crate::buffers::{GpuImageHandle, ScopeBuffers};

/// Dispatches scope compute shaders and manages their pipeline state.
pub struct ScopeDispatch {
    histogram_pipeline: wgpu::ComputePipeline,
    histogram_layout: wgpu::BindGroupLayout,
    waveform_pipeline: wgpu::ComputePipeline,
    waveform_layout: wgpu::BindGroupLayout,
    vectorscope_pipeline: wgpu::ComputePipeline,
    vectorscope_layout: wgpu::BindGroupLayout,
    cie_pipeline: wgpu::ComputePipeline,
    cie_layout: wgpu::BindGroupLayout,
    // Cached uniform buffers (updated via queue.write_buffer each frame).
    pixel_count_buf: wgpu::Buffer,
    wf_width_buf: wgpu::Buffer,
    wf_height_buf: wgpu::Buffer,
    wf_waveform_height_buf: wgpu::Buffer,
    vs_resolution_buf: wgpu::Buffer,
    cie_resolution_buf: wgpu::Buffer,
}

impl ScopeDispatch {
    /// Create all scope compute pipelines.
    pub fn new(device: &wgpu::Device) -> Self {
        let (histogram_pipeline, histogram_layout) = create_scope_pipeline(
            device,
            "histogram",
            include_str!("../shaders/histogram.wgsl"),
            &[
                storage_ro_entry(0),
                storage_rw_entry(1),
                uniform_entry(2, 4),
            ],
        );

        let (waveform_pipeline, waveform_layout) = create_scope_pipeline(
            device,
            "waveform",
            include_str!("../shaders/waveform.wgsl"),
            &[
                storage_ro_entry(0),
                storage_rw_entry(1),
                uniform_entry(2, 4),
                uniform_entry(3, 4),
                uniform_entry(4, 4),
            ],
        );

        let (vectorscope_pipeline, vectorscope_layout) = create_scope_pipeline(
            device,
            "vectorscope",
            include_str!("../shaders/vectorscope.wgsl"),
            &[
                storage_ro_entry(0),
                storage_rw_entry(1),
                uniform_entry(2, 4),
                uniform_entry(3, 4),
            ],
        );

        let (cie_pipeline, cie_layout) = create_scope_pipeline(
            device,
            "cie",
            include_str!("../shaders/cie.wgsl"),
            &[
                storage_ro_entry(0),
                storage_rw_entry(1),
                uniform_entry(2, 4),
                uniform_entry(3, 4),
            ],
        );

        // Pre-allocate cached uniform buffers (updated via queue.write_buffer).
        let make_uniform = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: 16, // u32 padded to 16 bytes
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        Self {
            histogram_pipeline,
            histogram_layout,
            waveform_pipeline,
            waveform_layout,
            vectorscope_pipeline,
            vectorscope_layout,
            cie_pipeline,
            cie_layout,
            pixel_count_buf: make_uniform("crispen_scope_pixel_count"),
            wf_width_buf: make_uniform("crispen_scope_wf_width"),
            wf_height_buf: make_uniform("crispen_scope_wf_height"),
            wf_waveform_height_buf: make_uniform("crispen_scope_wf_wh"),
            vs_resolution_buf: make_uniform("crispen_scope_vs_res"),
            cie_resolution_buf: make_uniform("crispen_scope_cie_res"),
        }
    }

    /// Dispatch all scope shaders onto the given encoder.
    ///
    /// The caller is responsible for submitting the encoder.
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &GpuImageHandle,
        scope_buffers: &ScopeBuffers,
        waveform_height: u32,
        vectorscope_resolution: u32,
        cie_resolution: u32,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let pixel_count = image.pixel_count();
        let workgroups = pixel_count.div_ceil(256);

        // Update cached uniform buffers via queue.write_buffer (no allocations).
        let pad = |v: u32| -> [u32; 4] { [v, 0, 0, 0] };
        queue.write_buffer(
            &self.pixel_count_buf,
            0,
            bytemuck::cast_slice(&pad(pixel_count)),
        );
        queue.write_buffer(
            &self.wf_width_buf,
            0,
            bytemuck::cast_slice(&pad(image.width)),
        );
        queue.write_buffer(
            &self.wf_height_buf,
            0,
            bytemuck::cast_slice(&pad(image.height)),
        );
        queue.write_buffer(
            &self.wf_waveform_height_buf,
            0,
            bytemuck::cast_slice(&pad(waveform_height)),
        );
        queue.write_buffer(
            &self.vs_resolution_buf,
            0,
            bytemuck::cast_slice(&pad(vectorscope_resolution)),
        );
        queue.write_buffer(
            &self.cie_resolution_buf,
            0,
            bytemuck::cast_slice(&pad(cie_resolution)),
        );

        // Clear all scope buffers.
        encoder.clear_buffer(&scope_buffers.histogram, 0, None);
        encoder.clear_buffer(&scope_buffers.waveform, 0, None);
        encoder.clear_buffer(&scope_buffers.vectorscope, 0, None);
        encoder.clear_buffer(&scope_buffers.cie, 0, None);

        // Histogram pass.
        let hist_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crispen_histogram_bg"),
            layout: &self.histogram_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: image.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: scope_buffers.histogram.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.pixel_count_buf.as_entire_binding(),
                },
            ],
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("crispen_histogram_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.histogram_pipeline);
            pass.set_bind_group(0, &hist_bg, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Waveform pass.
        let wf_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crispen_waveform_bg"),
            layout: &self.waveform_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: image.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: scope_buffers.waveform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.wf_width_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.wf_height_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.wf_waveform_height_buf.as_entire_binding(),
                },
            ],
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("crispen_waveform_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.waveform_pipeline);
            pass.set_bind_group(0, &wf_bg, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Vectorscope pass.
        let vs_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crispen_vectorscope_bg"),
            layout: &self.vectorscope_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: image.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: scope_buffers.vectorscope.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.pixel_count_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.vs_resolution_buf.as_entire_binding(),
                },
            ],
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("crispen_vectorscope_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.vectorscope_pipeline);
            pass.set_bind_group(0, &vs_bg, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // CIE pass.
        let cie_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crispen_cie_bg"),
            layout: &self.cie_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: image.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: scope_buffers.cie.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.pixel_count_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.cie_resolution_buf.as_entire_binding(),
                },
            ],
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("crispen_cie_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.cie_pipeline);
            pass.set_bind_group(0, &cie_bg, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn storage_ro_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(16),
        },
        count: None,
    }
}

fn storage_rw_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(4),
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

fn create_scope_pipeline(
    device: &wgpu::Device,
    name: &str,
    wgsl_source: &str,
    layout_entries: &[wgpu::BindGroupLayoutEntry],
) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("crispen_{name}_shader")),
        source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(&format!("crispen_{name}_layout")),
        entries: layout_entries,
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("crispen_{name}_pipeline_layout")),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    // Entry point names match the shader fn names.
    let entry_point = match name {
        "waveform" => "waveform_compute",
        "cie" => "cie_compute",
        _ => name,
    };

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(&format!("crispen_{name}_pipeline")),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some(entry_point),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    (pipeline, bind_group_layout)
}
