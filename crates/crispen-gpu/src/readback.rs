//! GPU-to-CPU readback utilities for scope data and image download.

use crispen_core::image::{BitDepth, GradingImage};
use crispen_core::scopes::{CieData, HistogramData, VectorscopeData, WaveformData};

use crate::buffers::{GpuImageHandle, ScopeBuffers, ScopeConfig};

/// Results from GPU scope readback, converted to core types.
pub struct ScopeResults {
    pub histogram: HistogramData,
    pub waveform: WaveformData,
    pub vectorscope: VectorscopeData,
    pub cie: CieData,
}

/// Reads GPU scope buffers back to the CPU.
pub struct Readback {
    histogram_staging: wgpu::Buffer,
    waveform_staging: wgpu::Buffer,
    vectorscope_staging: wgpu::Buffer,
    cie_staging: wgpu::Buffer,
    scope_config: ScopeConfig,
    image_width: u32,
}

impl Readback {
    /// Create staging buffers sized to match the scope buffers.
    pub fn new(device: &wgpu::Device, scope_config: &ScopeConfig, image_width: u32) -> Self {
        let histogram_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_histogram_staging"),
            size: 1024 * 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let wf_size = (image_width as u64) * (scope_config.waveform_height as u64) * 3 * 4;
        let waveform_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_waveform_staging"),
            size: wf_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let vs_size = (scope_config.vectorscope_resolution as u64).pow(2) * 4;
        let vectorscope_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_vectorscope_staging"),
            size: vs_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let cie_size = (scope_config.cie_resolution as u64).pow(2) * 4;
        let cie_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("crispen_cie_staging"),
            size: cie_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            histogram_staging,
            waveform_staging,
            vectorscope_staging,
            cie_staging,
            scope_config: *scope_config,
            image_width,
        }
    }

    /// Record copy commands from GPU scope buffers to staging buffers.
    pub fn copy_to_staging(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        scope_buffers: &ScopeBuffers,
    ) {
        encoder.copy_buffer_to_buffer(
            &scope_buffers.histogram,
            0,
            &self.histogram_staging,
            0,
            self.histogram_staging.size(),
        );
        encoder.copy_buffer_to_buffer(
            &scope_buffers.waveform,
            0,
            &self.waveform_staging,
            0,
            self.waveform_staging.size(),
        );
        encoder.copy_buffer_to_buffer(
            &scope_buffers.vectorscope,
            0,
            &self.vectorscope_staging,
            0,
            self.vectorscope_staging.size(),
        );
        encoder.copy_buffer_to_buffer(
            &scope_buffers.cie,
            0,
            &self.cie_staging,
            0,
            self.cie_staging.size(),
        );
    }

    /// Initiate `map_async` on all scope staging buffers without polling.
    ///
    /// Call this before `device.poll()`, then use [`read_mapped_scopes`] after
    /// the poll completes to read the data.
    pub fn map_staging_buffers(&self) {
        self.histogram_staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, |_| {});
        self.waveform_staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, |_| {});
        self.vectorscope_staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, |_| {});
        self.cie_staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, |_| {});
    }

    /// Read scope data from already-mapped staging buffers and unmap them.
    ///
    /// Must be called after [`map_staging_buffers`] + `device.poll()`.
    pub fn read_mapped_scopes(&self) -> ScopeResults {
        // Read histogram: 1024 u32s → 4 channels of 256 bins.
        let histogram = {
            let data = self.histogram_staging.slice(..).get_mapped_range();
            let bins_flat: &[u32] = bytemuck::cast_slice(&data);
            let mut bins: [Vec<u32>; 4] = std::array::from_fn(|_| Vec::with_capacity(256));
            let mut peak = 0u32;
            for ch in 0..4 {
                let slice = &bins_flat[ch * 256..(ch + 1) * 256];
                for &val in slice {
                    peak = peak.max(val);
                }
                bins[ch] = slice.to_vec();
            }
            drop(data);
            self.histogram_staging.unmap();
            HistogramData { bins, peak }
        };

        // Read waveform: 3 channels x width x waveform_height.
        let waveform = {
            let data = self.waveform_staging.slice(..).get_mapped_range();
            let flat: &[u32] = bytemuck::cast_slice(&data);
            let ch_stride =
                (self.image_width as usize) * (self.scope_config.waveform_height as usize);
            let channels: [Vec<u32>; 3] = std::array::from_fn(|ch| {
                let start = ch * ch_stride;
                flat[start..start + ch_stride].to_vec()
            });
            drop(data);
            self.waveform_staging.unmap();
            WaveformData {
                width: self.image_width,
                height: self.scope_config.waveform_height,
                data: channels,
            }
        };

        // Read vectorscope.
        let vectorscope = {
            let data = self.vectorscope_staging.slice(..).get_mapped_range();
            let density: Vec<u32> = bytemuck::cast_slice::<u8, u32>(&data).to_vec();
            drop(data);
            self.vectorscope_staging.unmap();
            VectorscopeData {
                resolution: self.scope_config.vectorscope_resolution,
                density,
            }
        };

        // Read CIE.
        let cie = {
            let data = self.cie_staging.slice(..).get_mapped_range();
            let density: Vec<u32> = bytemuck::cast_slice::<u8, u32>(&data).to_vec();
            drop(data);
            self.cie_staging.unmap();
            CieData {
                resolution: self.scope_config.cie_resolution,
                density,
            }
        };

        ScopeResults {
            histogram,
            waveform,
            vectorscope,
            cie,
        }
    }

    /// Map staging buffers, block on device poll, and read scope data. Convenience wrapper.
    pub fn read_scopes(&self, device: &wgpu::Device) -> ScopeResults {
        self.map_staging_buffers();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        self.read_mapped_scopes()
    }

    /// Download a GPU image buffer back to a [`GradingImage`]. Blocks until complete.
    pub fn download_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        handle: &GpuImageHandle,
        staging_cache: &mut Option<wgpu::Buffer>,
    ) -> GradingImage {
        let size = handle.byte_size();
        let needs_new_staging = match staging_cache.as_ref() {
            Some(buf) => buf.size() < size,
            None => true,
        };
        if needs_new_staging {
            *staging_cache = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("crispen_image_staging"),
                size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }));
        }
        let staging = staging_cache
            .as_ref()
            .expect("staging cache should be initialized");

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("crispen_image_download_encoder"),
        });
        encoder.copy_buffer_to_buffer(&handle.buffer, 0, &staging, 0, size);
        queue.submit(std::iter::once(encoder.finish()));

        staging.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

        let data = staging.slice(..).get_mapped_range();
        let pixels_flat: &[[f32; 4]] = bytemuck::cast_slice(&data);
        let pixels = pixels_flat.to_vec();
        drop(data);
        staging.unmap();

        GradingImage {
            width: handle.width,
            height: handle.height,
            pixels,
            source_bit_depth: BitDepth::F32,
        }
    }
}
