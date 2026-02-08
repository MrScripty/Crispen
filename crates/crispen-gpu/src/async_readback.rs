//! Double-buffered asynchronous GPU readback for viewer image and scope data.
//!
//! Two staging buffer "slots" alternate: while the GPU writes to one slot,
//! the CPU reads from the other. The main thread never blocks on GPU work.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crispen_core::scopes::{CieData, HistogramData, VectorscopeData, WaveformData};

use crate::buffers::{ScopeBuffers, ScopeConfig};
use crate::readback::ScopeResults;

/// Double-buffered async readback for image + scope data.
pub struct AsyncReadback {
    slots: [ReadbackSlot; 2],
    /// Which slot has a pending `map_async` (None = no pending readback).
    pending_idx: Option<usize>,
    scope_config: ScopeConfig,
    image_width: u32,
}

/// One set of staging buffers for image + scopes.
struct ReadbackSlot {
    image_staging: wgpu::Buffer,
    histogram_staging: wgpu::Buffer,
    waveform_staging: wgpu::Buffer,
    vectorscope_staging: wgpu::Buffer,
    cie_staging: wgpu::Buffer,
    /// Counter incremented by each map_async callback. Ready when == 5.
    maps_done: Arc<AtomicU32>,
}

/// Results consumed from an async readback slot.
pub struct AsyncFrameResult {
    pub viewer_bytes: Vec<u8>,
    pub scopes: ScopeResults,
}

impl ReadbackSlot {
    fn new(device: &wgpu::Device, scope_config: &ScopeConfig, image_width: u32, image_staging_size: u64, slot_label: &str) -> Self {
        let image_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("crispen_image_staging_{slot_label}")),
            size: image_staging_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let histogram_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("crispen_histogram_staging_{slot_label}")),
            size: 1024 * 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let wf_size = (image_width as u64) * (scope_config.waveform_height as u64) * 3 * 4;
        let waveform_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("crispen_waveform_staging_{slot_label}")),
            size: wf_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let vs_size = (scope_config.vectorscope_resolution as u64).pow(2) * 4;
        let vectorscope_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("crispen_vectorscope_staging_{slot_label}")),
            size: vs_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let cie_size = (scope_config.cie_resolution as u64).pow(2) * 4;
        let cie_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("crispen_cie_staging_{slot_label}")),
            size: cie_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            image_staging,
            histogram_staging,
            waveform_staging,
            vectorscope_staging,
            cie_staging,
            maps_done: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Returns true if all 5 map_async callbacks have fired.
    fn is_ready(&self) -> bool {
        self.maps_done.load(Ordering::Acquire) >= 5
    }

    /// Record copy commands from GPU buffers to this slot's staging buffers.
    fn record_copies(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        viewer_src: &wgpu::Buffer,
        viewer_byte_size: u64,
        scope_buffers: &ScopeBuffers,
    ) {
        encoder.copy_buffer_to_buffer(viewer_src, 0, &self.image_staging, 0, viewer_byte_size);
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

    /// Initiate map_async on all 5 staging buffers. Resets the counter first.
    fn begin_map(&self) {
        self.maps_done.store(0, Ordering::Release);

        let bufs = [
            &self.image_staging,
            &self.histogram_staging,
            &self.waveform_staging,
            &self.vectorscope_staging,
            &self.cie_staging,
        ];

        for buf in bufs {
            let counter = Arc::clone(&self.maps_done);
            buf.slice(..).map_async(wgpu::MapMode::Read, move |_| {
                counter.fetch_add(1, Ordering::Release);
            });
        }
    }

    /// Read mapped data from all staging buffers and unmap them.
    fn consume(
        &self,
        viewer_byte_size: u64,
        scope_config: &ScopeConfig,
        image_width: u32,
    ) -> AsyncFrameResult {
        // Read viewer image bytes.
        let viewer_bytes = {
            let data = self.image_staging.slice(..).get_mapped_range();
            let bytes = data[..viewer_byte_size as usize].to_vec();
            drop(data);
            self.image_staging.unmap();
            bytes
        };

        // Read histogram.
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

        // Read waveform.
        let waveform = {
            let data = self.waveform_staging.slice(..).get_mapped_range();
            let flat: &[u32] = bytemuck::cast_slice(&data);
            let ch_stride = (image_width as usize) * (scope_config.waveform_height as usize);
            let channels: [Vec<u32>; 3] = std::array::from_fn(|ch| {
                let start = ch * ch_stride;
                flat[start..start + ch_stride].to_vec()
            });
            drop(data);
            self.waveform_staging.unmap();
            WaveformData {
                width: image_width,
                height: scope_config.waveform_height,
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
                resolution: scope_config.vectorscope_resolution,
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
                resolution: scope_config.cie_resolution,
                density,
            }
        };

        AsyncFrameResult {
            viewer_bytes,
            scopes: ScopeResults {
                histogram,
                waveform,
                vectorscope,
                cie,
            },
        }
    }
}

impl AsyncReadback {
    /// Create double-buffered staging resources.
    pub fn new(
        device: &wgpu::Device,
        scope_config: &ScopeConfig,
        image_width: u32,
        image_staging_size: u64,
    ) -> Self {
        let slots = [
            ReadbackSlot::new(device, scope_config, image_width, image_staging_size, "a"),
            ReadbackSlot::new(device, scope_config, image_width, image_staging_size, "b"),
        ];
        Self {
            slots,
            pending_idx: None,
            scope_config: *scope_config,
            image_width,
        }
    }

    /// Non-blocking poll: check if the pending readback is ready.
    /// Calls `device.poll(PollType::poll())` to process callbacks.
    /// Returns `Some(result)` if data is available, `None` otherwise.
    pub fn try_consume(
        &mut self,
        device: &wgpu::Device,
        viewer_byte_size: u64,
    ) -> Option<AsyncFrameResult> {
        // Drive the GPU event loop without blocking.
        let _ = device.poll(wgpu::PollType::Poll);

        let pending = self.pending_idx?;
        if !self.slots[pending].is_ready() {
            return None;
        }

        let result = self.slots[pending].consume(
            viewer_byte_size,
            &self.scope_config,
            self.image_width,
        );
        self.pending_idx = None;
        Some(result)
    }

    /// Record staging copies and begin map_async on the write slot.
    ///
    /// Must be called AFTER `queue.submit()` for the encoder that contains
    /// the compute dispatches.
    pub fn submit_readback(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        viewer_src: &wgpu::Buffer,
        viewer_byte_size: u64,
        scope_buffers: &ScopeBuffers,
    ) {
        // Choose the slot that's NOT pending (or 0 if none pending).
        let write_idx = match self.pending_idx {
            Some(idx) => 1 - idx,
            None => 0,
        };

        self.slots[write_idx].record_copies(encoder, viewer_src, viewer_byte_size, scope_buffers);

        // Note: begin_map must be called after queue.submit — the caller
        // handles this via the returned write_idx.
    }

    /// Begin map_async on the write slot. Call this AFTER `queue.submit()`.
    pub fn begin_map_after_submit(&mut self) {
        let write_idx = match self.pending_idx {
            Some(idx) => 1 - idx,
            None => 0,
        };
        self.slots[write_idx].begin_map();
        self.pending_idx = Some(write_idx);
    }

    /// Whether a readback is currently in flight.
    pub fn has_pending(&self) -> bool {
        self.pending_idx.is_some()
    }
}
