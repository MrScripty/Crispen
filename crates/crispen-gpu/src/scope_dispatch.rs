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
}

impl ScopeDispatch {
    /// Create all scope compute pipelines.
    pub fn new(device: &wgpu::Device) -> Self {
        let (histogram_pipeline, histogram_layout) =
            create_scope_pipeline(device, "histogram", include_str!("../shaders/histogram.wgsl"), &[
                storage_ro_entry(0),
                storage_rw_entry(1),
                uniform_entry(2, 4),
            ]);

        let (waveform_pipeline, waveform_layout) =
            create_scope_pipeline(device, "waveform", include_str!("../shaders/waveform.wgsl"), &[
                storage_ro_entry(0),
                storage_rw_entry(1),
                uniform_entry(2, 4),
                uniform_entry(3, 4),
                uniform_entry(4, 4),
            ]);

        let (vectorscope_pipeline, vectorscope_layout) =
            create_scope_pipeline(device, "vectorscope", include_str!("../shaders/vectorscope.wgsl"), &[
                storage_ro_entry(0),
                storage_rw_entry(1),
                uniform_entry(2, 4),
                uniform_entry(3, 4),
            ]);

        let (cie_pipeline, cie_layout) =
            create_scope_pipeline(device, "cie", include_str!("../shaders/cie.wgsl"), &[
                storage_ro_entry(0),
                storage_rw_entry(1),
                uniform_entry(2, 4),
                uniform_entry(3, 4),
            ]);

        Self {
            histogram_pipeline,
            histogram_layout,
            waveform_pipeline,
            waveform_layout,
            vectorscope_pipeline,
            vectorscope_layout,
            cie_pipeline,
            cie_layout,
        }
    }

    /// Dispatch all scope shaders in a single command encoder submission.
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
    ) {
        let pixel_count = image.pixel_count();
        let workgroups = pixel_count.div_ceil(256);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("crispen_scope_encoder"),
        });

        // Clear all scope buffers.
        encoder.clear_buffer(&scope_buffers.histogram, 0, None);
        encoder.clear_buffer(&scope_buffers.waveform, 0, None);
        encoder.clear_buffer(&scope_buffers.vectorscope, 0, None);
        encoder.clear_buffer(&scope_buffers.cie, 0, None);

        // Histogram pass.
        let hist_uniform = create_u32_uniform(device, pixel_count, "crispen_hist_uniform");
        let hist_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crispen_histogram_bg"),
            layout: &self.histogram_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: image.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: scope_buffers.histogram.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: hist_uniform.as_entire_binding() },
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
        let wf_width_u = create_u32_uniform(device, image.width, "crispen_wf_width");
        let wf_height_u = create_u32_uniform(device, image.height, "crispen_wf_height");
        let wf_wh_u = create_u32_uniform(device, waveform_height, "crispen_wf_wh");
        let wf_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crispen_waveform_bg"),
            layout: &self.waveform_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: image.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: scope_buffers.waveform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: wf_width_u.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: wf_height_u.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: wf_wh_u.as_entire_binding() },
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
        let vs_count_u = create_u32_uniform(device, pixel_count, "crispen_vs_count");
        let vs_res_u = create_u32_uniform(device, vectorscope_resolution, "crispen_vs_res");
        let vs_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crispen_vectorscope_bg"),
            layout: &self.vectorscope_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: image.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: scope_buffers.vectorscope.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: vs_count_u.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: vs_res_u.as_entire_binding() },
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
        let cie_count_u = create_u32_uniform(device, pixel_count, "crispen_cie_count");
        let cie_res_u = create_u32_uniform(device, cie_resolution, "crispen_cie_res");
        let cie_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crispen_cie_bg"),
            layout: &self.cie_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: image.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: scope_buffers.cie.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: cie_count_u.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: cie_res_u.as_entire_binding() },
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

        queue.submit(std::iter::once(encoder.finish()));
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

    let bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

fn create_u32_uniform(device: &wgpu::Device, value: u32, label: &str) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    // Pad to 16 bytes for uniform alignment.
    let data = [value, 0u32, 0u32, 0u32];
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(&data),
        usage: wgpu::BufferUsages::UNIFORM,
    })
}
