//! GPU compute pass for baking `GradingParams` into a 3D LUT.

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::num::NonZeroU64;

use crispen_core::transform::params::GradingParams;

use crate::GradingParamsGpu;
use crate::buffers::GpuLutHandle;

/// Default curve texture size (number of entries in each 1D LUT).
const CURVE_LUT_SIZE: u32 = 256;

/// Manages the `bake_lut.wgsl` compute pipeline and its resources.
pub struct LutBaker {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    params_buffer: wgpu::Buffer,
    lut_size_buffer: wgpu::Buffer,
    curve_textures: [wgpu::Texture; 4],
    curve_views: [wgpu::TextureView; 4],
    curve_sampler: wgpu::Sampler,
    ocio_idt_texture: wgpu::Texture,
    ocio_idt_view: wgpu::TextureView,
    ocio_odt_texture: wgpu::Texture,
    ocio_odt_view: wgpu::TextureView,
    ocio_sampler: wgpu::Sampler,
    use_ocio: bool,
    /// Hash of the last uploaded curve data (skip re-upload when unchanged).
    last_curve_hash: u64,
}

impl LutBaker {
    /// Create the LUT bake pipeline. Compiles `bake_lut.wgsl`.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("crispen_bake_lut_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/bake_lut.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("crispen_bake_lut_layout"),
            entries: &[
                // binding 0: writable 3D LUT storage texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D3,
                    },
                    count: None,
                },
                // binding 1: params uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            std::mem::size_of::<GradingParamsGpu>() as u64
                        ),
                    },
                    count: None,
                },
                // binding 2: lut_size uniform
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
                // bindings 3-6: curve 1D textures
                curve_texture_layout_entry(3),
                curve_texture_layout_entry(4),
                curve_texture_layout_entry(5),
                curve_texture_layout_entry(6),
                // binding 7: curve sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // binding 8: optional OCIO IDT 3D LUT texture
                ocio_lut_texture_layout_entry(8),
                // binding 9: optional OCIO ODT 3D LUT texture
                ocio_lut_texture_layout_entry(9),
                // binding 10: sampler for OCIO LUTs
                wgpu::BindGroupLayoutEntry {
                    binding: 10,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("crispen_bake_lut_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("crispen_bake_lut_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("bake_lut"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_bake_params_uniform"),
            size: std::mem::size_of::<GradingParamsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Pad lut_size to 16 bytes for uniform alignment.
        let lut_size_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_bake_lut_size_uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let curve_textures =
            std::array::from_fn(|i| create_identity_curve_texture(device, queue, i));
        let curve_views = std::array::from_fn(|i| {
            curve_textures[i].create_view(&wgpu::TextureViewDescriptor::default())
        });

        let curve_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("crispen_curve_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let ocio_idt_texture = create_identity_ocio_lut_texture(device, queue, "crispen_ocio_idt");
        let ocio_idt_view = ocio_idt_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let ocio_odt_texture = create_identity_ocio_lut_texture(device, queue, "crispen_ocio_odt");
        let ocio_odt_view = ocio_odt_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let ocio_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("crispen_ocio_lut_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            params_buffer,
            lut_size_buffer,
            curve_textures,
            curve_views,
            curve_sampler,
            ocio_idt_texture,
            ocio_idt_view,
            ocio_odt_texture,
            ocio_odt_view,
            ocio_sampler,
            use_ocio: false,
            last_curve_hash: 0,
        }
    }

    /// Upload OCIO IDT/ODT LUT data. Passing `None` disables OCIO sampling.
    pub fn set_ocio_luts(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        idt_lut: Option<&[[f32; 4]]>,
        odt_lut: Option<&[[f32; 4]]>,
        size: u32,
    ) {
        let expected_len = size as usize * size as usize * size as usize;
        let Some(idt) = idt_lut else {
            self.use_ocio = false;
            return;
        };
        let Some(odt) = odt_lut else {
            self.use_ocio = false;
            return;
        };
        if size < 2 || idt.len() != expected_len || odt.len() != expected_len {
            self.use_ocio = false;
            return;
        }

        let idt_tex = write_ocio_lut_texture(device, queue, idt, size, "crispen_ocio_idt");
        let odt_tex = write_ocio_lut_texture(device, queue, odt, size, "crispen_ocio_odt");
        self.ocio_idt_view = idt_tex.create_view(&wgpu::TextureViewDescriptor::default());
        self.ocio_odt_view = odt_tex.create_view(&wgpu::TextureViewDescriptor::default());
        self.ocio_idt_texture = idt_tex;
        self.ocio_odt_texture = odt_tex;
        self.use_ocio = true;
    }

    /// Upload curve data from `GradingParams` as 1D textures.
    ///
    /// Skips re-upload if curve data is unchanged since the last call.
    pub fn upload_curves(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        params: &GradingParams,
    ) {
        // Hash curve control points to skip redundant texture uploads.
        let mut hasher = DefaultHasher::new();
        for curve in [
            &params.hue_vs_hue,
            &params.hue_vs_sat,
            &params.lum_vs_sat,
            &params.sat_vs_sat,
        ] {
            hasher.write_usize(curve.len());
            for pt in curve {
                hasher.write(&pt[0].to_ne_bytes());
                hasher.write(&pt[1].to_ne_bytes());
            }
        }
        let hash = hasher.finish();
        if hash == self.last_curve_hash {
            return;
        }
        self.last_curve_hash = hash;

        let curves: [(&Vec<[f32; 2]>, usize); 4] = [
            (&params.hue_vs_hue, 0),
            (&params.hue_vs_sat, 1),
            (&params.lum_vs_sat, 2),
            (&params.sat_vs_sat, 3),
        ];

        for (points, idx) in curves {
            if points.is_empty() {
                continue;
            }
            let lut_data = bake_curve_cpu(points, CURVE_LUT_SIZE as usize, idx == 0);
            let texture = write_curve_texture(device, queue, &lut_data, idx);
            self.curve_views[idx] = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.curve_textures[idx] = texture;
        }
    }

    /// Dispatch the LUT bake compute shader onto the given encoder.
    ///
    /// The caller is responsible for submitting the encoder.
    pub fn bake(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        params: &GradingParams,
        lut: &GpuLutHandle,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let gpu_params = GradingParamsGpu::from_params(params, self.use_ocio);
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&gpu_params));

        let size_bytes = [lut.size, 0u32, 0u32, 0u32];
        queue.write_buffer(&self.lut_size_buffer, 0, bytemuck::cast_slice(&size_bytes));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crispen_bake_lut_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&lut.texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.lut_size_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.curve_views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.curve_views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&self.curve_views[2]),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&self.curve_views[3]),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&self.curve_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(&self.ocio_idt_view),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(&self.ocio_odt_view),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(&self.ocio_sampler),
                },
            ],
        });

        let wg_xy = lut.size.div_ceil(8);
        let wg_z = lut.size.div_ceil(4);

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("crispen_bake_lut_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(wg_xy, wg_xy, wg_z);
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn curve_texture_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D1,
            multisampled: false,
        },
        count: None,
    }
}

fn ocio_lut_texture_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D3,
            multisampled: false,
        },
        count: None,
    }
}

fn create_identity_curve_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    index: usize,
) -> wgpu::Texture {
    let identity_val = if index == 0 { 0.0f32 } else { 1.0f32 };
    let data: Vec<f32> = vec![identity_val; CURVE_LUT_SIZE as usize];
    write_curve_texture(device, queue, &data, index)
}

fn write_curve_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &[f32],
    index: usize,
) -> wgpu::Texture {
    let label = match index {
        0 => "crispen_curve_hue_vs_hue",
        1 => "crispen_curve_hue_vs_sat",
        2 => "crispen_curve_lum_vs_sat",
        _ => "crispen_curve_sat_vs_sat",
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: data.len() as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D1,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(data.len() as u32 * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: data.len() as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
    );

    texture
}

fn create_identity_ocio_lut_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
) -> wgpu::Texture {
    let px = [[0.0f32, 0.0f32, 0.0f32, 1.0f32]];
    write_ocio_lut_texture(device, queue, &px, 1, label)
}

fn write_ocio_lut_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &[[f32; 4]],
    size: u32,
    label: &str,
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: size,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size * 16),
            rows_per_image: Some(size),
        },
        wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: size,
        },
    );

    texture
}

/// Bake curve control points to a 1D LUT via linear interpolation.
fn bake_curve_cpu(points: &[[f32; 2]], size: usize, is_hue: bool) -> Vec<f32> {
    let identity = if is_hue { 0.0 } else { 1.0 };
    if points.is_empty() {
        return vec![identity; size];
    }

    (0..size)
        .map(|i| {
            let t = i as f32 / (size - 1) as f32;
            eval_curve_linear(points, t)
        })
        .collect()
}

fn eval_curve_linear(points: &[[f32; 2]], t: f32) -> f32 {
    if points.is_empty() {
        return t;
    }
    if t <= points[0][0] {
        return points[0][1];
    }
    let last = points.len() - 1;
    if t >= points[last][0] {
        return points[last][1];
    }
    for i in 0..last {
        if t >= points[i][0] && t <= points[i + 1][0] {
            let frac = (t - points[i][0]) / (points[i + 1][0] - points[i][0]);
            return points[i][1] + frac * (points[i + 1][1] - points[i][1]);
        }
    }
    points[last][1]
}
