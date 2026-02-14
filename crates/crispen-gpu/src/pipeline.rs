//! Top-level GPU grading pipeline that orchestrates all compute passes.

use std::sync::Arc;

use crispen_core::image::GradingImage;
use crispen_core::transform::params::GradingParams;

use crate::async_readback::AsyncReadback;
use crate::buffers::{GpuImageHandle, GpuLutHandle, ScopeBuffers, ScopeConfig};
use crate::format_converter::{FormatConverter, ViewerFormat};
use crate::lut_applicator::LutApplicator;
use crate::lut_baker::LutBaker;
use crate::readback::{Readback, ScopeResults};
use crate::scope_dispatch::ScopeDispatch;

/// Results from a single frame submission.
pub struct FrameResult {
    /// Raw pixel bytes for the viewer (f16 or f32 depending on format).
    pub viewer_bytes: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// The pixel format of `viewer_bytes`.
    pub format: ViewerFormat,
    /// Scope computation results.
    pub scopes: Option<ScopeResults>,
}

/// Returns the minimum `wgpu::Features` required by the crispen-gpu pipeline.
///
/// Callers must request these features when creating the `wgpu::Device`.
pub fn required_features() -> wgpu::Features {
    // R32Float curve textures need bilinear filtering.
    wgpu::Features::FLOAT32_FILTERABLE
}

/// Orchestrates the full GPU grading pipeline: LUT bake → apply → scopes.
pub struct GpuGradingPipeline {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    adapter_backend: wgpu::Backend,
    enabled_features: wgpu::Features,
    lut_baker: LutBaker,
    lut_applicator: LutApplicator,
    format_converter: FormatConverter,
    scope_dispatch: ScopeDispatch,
    current_lut: Option<GpuLutHandle>,
    current_output: Option<GpuImageHandle>,
    scope_buffers: Option<ScopeBuffers>,
    readback: Option<Readback>,
    /// Cached staging buffer for blocking image readback (legacy path).
    image_readback_staging: Option<wgpu::Buffer>,
    /// Double-buffered async readback (primary path).
    async_readback: Option<AsyncReadback>,
    scope_config: ScopeConfig,
    viewer_format: ViewerFormat,
    /// Per-scope visibility flags (skips GPU compute when hidden).
    scope_histogram_visible: bool,
    scope_waveform_visible: bool,
    scope_vectorscope_visible: bool,
    scope_cie_visible: bool,
    /// Dimensions + format of the last async submission (for FrameResult).
    last_async_width: u32,
    last_async_height: u32,
    last_async_viewer_byte_size: u64,
}

impl GpuGradingPipeline {
    /// Create a GPU pipeline by requesting a new wgpu adapter and device.
    ///
    /// Blocks on async wgpu calls via `pollster`. Call this once at startup
    /// (e.g. from a Bevy startup system) and store the result as a resource.
    pub fn create_blocking() -> Result<Self, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .map_err(|e| format!("no suitable GPU adapter found: {e}"))?;
        let adapter_backend = adapter.get_info().backend;
        let required_features = required_features();

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("crispen_compute_device"),
            required_features,
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .map_err(|e| format!("failed to create GPU device: {e}"))?;
        let enabled_features = device.features();

        Ok(Self::new_with_metadata(
            Arc::new(device),
            Arc::new(queue),
            adapter_backend,
            enabled_features,
        ))
    }

    /// Create the full GPU pipeline. Compiles all shaders.
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let enabled_features = device.features();
        Self::new_with_metadata(device, queue, wgpu::Backend::Noop, enabled_features)
    }

    fn new_with_metadata(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        adapter_backend: wgpu::Backend,
        enabled_features: wgpu::Features,
    ) -> Self {
        let lut_baker = LutBaker::new(&device, &queue);
        let lut_applicator = LutApplicator::new(&device);
        let format_converter = FormatConverter::new(&device);
        let scope_dispatch = ScopeDispatch::new(&device);

        Self {
            device,
            queue,
            adapter_backend,
            enabled_features,
            lut_baker,
            lut_applicator,
            format_converter,
            scope_dispatch,
            current_lut: None,
            current_output: None,
            scope_buffers: None,
            readback: None,
            image_readback_staging: None,
            async_readback: None,
            scope_config: ScopeConfig::default(),
            viewer_format: ViewerFormat::Srgb8,
            scope_histogram_visible: true,
            scope_waveform_visible: true,
            scope_vectorscope_visible: true,
            scope_cie_visible: true,
            last_async_width: 0,
            last_async_height: 0,
            last_async_viewer_byte_size: 0,
        }
    }

    /// Access the backend used by the underlying adapter (when known).
    pub fn adapter_backend(&self) -> wgpu::Backend {
        self.adapter_backend
    }

    /// Access the enabled features on the underlying device.
    pub fn enabled_features(&self) -> wgpu::Features {
        self.enabled_features
    }

    /// Access the wgpu device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Access the wgpu queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Upload a source image to the GPU.
    pub fn upload_image(&self, image: &GradingImage) -> GpuImageHandle {
        GpuImageHandle::upload(&self.device, &self.queue, image)
    }

    /// Download a graded image from the GPU. Blocks until complete.
    pub fn download_image(&mut self, handle: &GpuImageHandle) -> GradingImage {
        Readback::download_image(
            &self.device,
            &self.queue,
            handle,
            &mut self.image_readback_staging,
        )
    }

    /// Download the most recently graded output image, if available.
    pub fn download_current_output(&mut self) -> Option<GradingImage> {
        let handle = self.current_output.as_ref()?;
        Some(Readback::download_image(
            &self.device,
            &self.queue,
            handle,
            &mut self.image_readback_staging,
        ))
    }

    /// Submit the full grading pipeline in a single GPU submission:
    /// bake LUT + apply LUT + format convert + scopes + staging copies.
    ///
    /// Returns raw viewer pixel bytes (f16 or f32) and scope results.
    /// Blocks on readback. Async readback replaces this in Phase 3.
    pub fn submit_frame(
        &mut self,
        source: &GpuImageHandle,
        params: &GradingParams,
        lut_size: u32,
    ) -> FrameResult {
        // Upload curve textures (immediate, no encoder needed).
        self.lut_baker
            .upload_curves(&self.device, &self.queue, params);

        // Ensure LUT handle exists.
        let lut = self
            .current_lut
            .get_or_insert_with(|| GpuLutHandle::new(&self.device, lut_size));
        if lut.size != lut_size {
            *lut = GpuLutHandle::new(&self.device, lut_size);
        }

        // Ensure output handle exists.
        let output = self.current_output.get_or_insert_with(|| {
            GpuImageHandle::create_output(&self.device, source.width, source.height)
        });
        if output.width != source.width || output.height != source.height {
            *output = GpuImageHandle::create_output(&self.device, source.width, source.height);
        }

        let cfg = self.scope_config;
        let _scope_buffers = self
            .scope_buffers
            .get_or_insert_with(|| ScopeBuffers::new(&self.device, &cfg, source.width));

        let _readback = self
            .readback
            .get_or_insert_with(|| Readback::new(&self.device, &cfg, source.width));

        // ── Single encoder for all GPU work ────────────────────────
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("crispen_frame_encoder"),
            });

        // 1. Bake LUT.
        let lut = self.current_lut.as_ref().unwrap();
        self.lut_baker
            .bake(&self.device, &self.queue, params, lut, &mut encoder);

        // 2. Apply LUT to source image.
        let output = self.current_output.as_ref().unwrap();
        self.lut_applicator
            .apply(&self.device, &self.queue, source, lut, output, &mut encoder);

        // 3. Format conversion + staging copy for viewer image.
        let pixel_count = output.pixel_count();
        let viewer_format = self.viewer_format;
        let viewer_byte_size = pixel_count as u64 * viewer_format.bytes_per_pixel();

        // Pre-allocate staging buffer before format conversion (avoids borrow conflicts).
        let needs_new_staging = match self.image_readback_staging.as_ref() {
            Some(buf) => buf.size() < viewer_byte_size,
            None => true,
        };
        if needs_new_staging {
            self.image_readback_staging =
                Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("crispen_image_staging"),
                    size: viewer_byte_size,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }));
        }

        match viewer_format {
            ViewerFormat::F16 => {
                let output = self.current_output.as_ref().unwrap();
                let f16_buf =
                    self.format_converter
                        .convert(&self.device, &self.queue, output, &mut encoder);
                let image_staging = self.image_readback_staging.as_ref().unwrap();
                encoder.copy_buffer_to_buffer(f16_buf, 0, image_staging, 0, viewer_byte_size);
            }
            ViewerFormat::F32 => {
                let output = self.current_output.as_ref().unwrap();
                let image_staging = self.image_readback_staging.as_ref().unwrap();
                encoder.copy_buffer_to_buffer(
                    &output.buffer,
                    0,
                    image_staging,
                    0,
                    viewer_byte_size,
                );
            }
            ViewerFormat::Srgb8 => {
                let output = self.current_output.as_ref().unwrap();
                let srgb_buf = self.format_converter.convert_to_srgb8(
                    &self.device,
                    &self.queue,
                    output,
                    &mut encoder,
                );
                let image_staging = self.image_readback_staging.as_ref().unwrap();
                encoder.copy_buffer_to_buffer(srgb_buf, 0, image_staging, 0, viewer_byte_size);
            }
        }

        // 4. Scope dispatches.
        let output = self.current_output.as_ref().unwrap();
        let scope_buffers = self.scope_buffers.as_ref().unwrap();
        self.scope_dispatch.dispatch(
            &self.device,
            &self.queue,
            output,
            scope_buffers,
            cfg.waveform_height,
            cfg.vectorscope_resolution,
            cfg.cie_resolution,
            &mut encoder,
            self.scope_histogram_visible,
            self.scope_waveform_visible,
            self.scope_vectorscope_visible,
            self.scope_cie_visible,
        );

        // 5. Copy scope data to staging buffers.
        let readback = self.readback.as_ref().unwrap();
        readback.copy_to_staging(&mut encoder, scope_buffers);

        // ── Single submit ──────────────────────────────────────────
        self.queue.submit(std::iter::once(encoder.finish()));

        // ── Blocking readback ──────────────────────────────────────
        let image_staging = self.image_readback_staging.as_ref().unwrap();
        image_staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, |_| {});

        let readback = self.readback.as_ref().unwrap();
        readback.map_staging_buffers();

        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        // Read viewer image bytes.
        let output = self.current_output.as_ref().unwrap();
        let viewer_bytes = {
            let data = image_staging.slice(..).get_mapped_range();
            let bytes = data[..viewer_byte_size as usize].to_vec();
            drop(data);
            image_staging.unmap();
            bytes
        };

        // Read scope data.
        let readback = self.readback.as_ref().unwrap();
        let scopes = readback.read_mapped_scopes();

        FrameResult {
            viewer_bytes,
            width: output.width,
            height: output.height,
            format: viewer_format,
            scopes: Some(scopes),
        }
    }

    /// Bake grading parameters into a 3D LUT (legacy single-step API).
    pub fn bake_lut(&mut self, params: &GradingParams, lut_size: u32) {
        let lut = self
            .current_lut
            .get_or_insert_with(|| GpuLutHandle::new(&self.device, lut_size));

        if lut.size != lut_size {
            *lut = GpuLutHandle::new(&self.device, lut_size);
        }

        self.lut_baker
            .upload_curves(&self.device, &self.queue, params);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("crispen_bake_lut_encoder"),
            });
        self.lut_baker
            .bake(&self.device, &self.queue, params, lut, &mut encoder);
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Apply the current LUT to a source image (legacy single-step API).
    pub fn apply_lut(&mut self, source: &GpuImageHandle) -> &GpuImageHandle {
        let lut = self
            .current_lut
            .as_ref()
            .expect("must bake LUT before applying");

        let output = self.current_output.get_or_insert_with(|| {
            GpuImageHandle::create_output(&self.device, source.width, source.height)
        });

        if output.width != source.width || output.height != source.height {
            *output = GpuImageHandle::create_output(&self.device, source.width, source.height);
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("crispen_apply_lut_encoder"),
            });
        self.lut_applicator
            .apply(&self.device, &self.queue, source, lut, output, &mut encoder);
        self.queue.submit(std::iter::once(encoder.finish()));

        self.current_output.as_ref().unwrap()
    }

    /// Compute scopes on the most recently graded output image, if available.
    pub fn compute_scopes_on_current_output(&mut self) -> Option<ScopeResults> {
        self.current_output.as_ref()?;
        Some(self.compute_scopes_on_output())
    }

    /// Compute scopes on the current output image.
    fn compute_scopes_on_output(&mut self) -> ScopeResults {
        let output = self.current_output.as_ref().expect("must apply LUT first");
        let width = output.width;
        let cfg = self.scope_config;

        let scope_buffers = self
            .scope_buffers
            .get_or_insert_with(|| ScopeBuffers::new(&self.device, &cfg, width));

        let readback = self
            .readback
            .get_or_insert_with(|| Readback::new(&self.device, &cfg, width));

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("crispen_scope_encoder"),
            });

        self.scope_dispatch.dispatch(
            &self.device,
            &self.queue,
            self.current_output.as_ref().unwrap(),
            scope_buffers,
            cfg.waveform_height,
            cfg.vectorscope_resolution,
            cfg.cie_resolution,
            &mut encoder,
            self.scope_histogram_visible,
            self.scope_waveform_visible,
            self.scope_vectorscope_visible,
            self.scope_cie_visible,
        );

        readback.copy_to_staging(&mut encoder, scope_buffers);
        self.queue.submit(std::iter::once(encoder.finish()));

        readback.read_scopes(&self.device)
    }

    // ── Async (non-blocking) API ──────────────────────────────────

    /// Submit GPU work (bake + apply + format convert + scopes) without
    /// blocking. Results are consumed later via [`try_consume_readback`].
    pub fn submit_gpu_work(
        &mut self,
        source: &GpuImageHandle,
        params: &GradingParams,
        lut_size: u32,
    ) {
        // Upload curve textures.
        self.lut_baker
            .upload_curves(&self.device, &self.queue, params);

        // Ensure LUT handle.
        let lut = self
            .current_lut
            .get_or_insert_with(|| GpuLutHandle::new(&self.device, lut_size));
        if lut.size != lut_size {
            *lut = GpuLutHandle::new(&self.device, lut_size);
        }

        // Ensure output handle.
        let output = self.current_output.get_or_insert_with(|| {
            GpuImageHandle::create_output(&self.device, source.width, source.height)
        });
        if output.width != source.width || output.height != source.height {
            *output = GpuImageHandle::create_output(&self.device, source.width, source.height);
        }

        let cfg = self.scope_config;
        let _scope_buffers = self
            .scope_buffers
            .get_or_insert_with(|| ScopeBuffers::new(&self.device, &cfg, source.width));

        let pixel_count = source.width as u64 * source.height as u64;
        let viewer_format = self.viewer_format;
        let viewer_byte_size = pixel_count * viewer_format.bytes_per_pixel();

        // Ensure async readback exists with correct sizing.
        if self.async_readback.is_none() {
            self.async_readback = Some(AsyncReadback::new(
                &self.device,
                &cfg,
                source.width,
                viewer_byte_size,
            ));
        }

        // ── Single encoder ───────────────────────────────────────
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("crispen_async_frame_encoder"),
            });

        // 1. Bake LUT.
        let lut = self.current_lut.as_ref().unwrap();
        self.lut_baker
            .bake(&self.device, &self.queue, params, lut, &mut encoder);

        // 2. Apply LUT.
        let output = self.current_output.as_ref().unwrap();
        self.lut_applicator
            .apply(&self.device, &self.queue, source, lut, output, &mut encoder);

        // 3. Format conversion + 4. Scope dispatches.
        let output = self.current_output.as_ref().unwrap();
        let scope_buffers = self.scope_buffers.as_ref().unwrap();

        // Format conversion — produces the viewer source buffer.
        let viewer_src: &wgpu::Buffer = match viewer_format {
            ViewerFormat::F16 => {
                self.format_converter
                    .convert(&self.device, &self.queue, output, &mut encoder)
            }
            ViewerFormat::Srgb8 => {
                self.format_converter.convert_to_srgb8(
                    &self.device,
                    &self.queue,
                    output,
                    &mut encoder,
                )
            }
            ViewerFormat::F32 => &output.buffer,
        };

        // 4. Scope dispatches (conditional on visibility).
        self.scope_dispatch.dispatch(
            &self.device,
            &self.queue,
            output,
            scope_buffers,
            cfg.waveform_height,
            cfg.vectorscope_resolution,
            cfg.cie_resolution,
            &mut encoder,
            self.scope_histogram_visible,
            self.scope_waveform_visible,
            self.scope_vectorscope_visible,
            self.scope_cie_visible,
        );

        // 5. Async readback staging copies.
        let async_rb = self.async_readback.as_mut().unwrap();
        async_rb.submit_readback(&mut encoder, viewer_src, viewer_byte_size, scope_buffers);

        // ── Single submit ────────────────────────────────────────
        self.queue.submit(std::iter::once(encoder.finish()));

        // Begin map_async (must happen after submit).
        let async_rb = self.async_readback.as_mut().unwrap();
        async_rb.begin_map_after_submit();

        // Track dimensions for when we consume the result.
        self.last_async_width = output.width;
        self.last_async_height = output.height;
        self.last_async_viewer_byte_size = viewer_byte_size;
    }

    /// Non-blocking: check if async readback data is ready and consume it.
    ///
    /// Returns `Some(FrameResult)` if data is available, `None` otherwise.
    /// Should be called every frame — it drives `device.poll()` internally.
    pub fn try_consume_readback(&mut self) -> Option<FrameResult> {
        let async_rb = self.async_readback.as_mut()?;
        let result = async_rb.try_consume(&self.device, self.last_async_viewer_byte_size)?;

        Some(FrameResult {
            viewer_bytes: result.viewer_bytes,
            width: self.last_async_width,
            height: self.last_async_height,
            format: self.viewer_format,
            scopes: Some(result.scopes),
        })
    }

    /// Set the scope configuration (waveform height, vectorscope/CIE resolution).
    pub fn set_scope_config(&mut self, config: ScopeConfig) {
        self.scope_config = config;
        // Invalidate cached scope resources so they're recreated.
        self.scope_buffers = None;
        self.readback = None;
        self.async_readback = None;
    }

    /// Upload optional OCIO IDT/ODT LUT textures used by `bake_lut.wgsl`.
    pub fn set_ocio_luts(
        &mut self,
        idt_lut: Option<&[[f32; 4]]>,
        odt_lut: Option<&[[f32; 4]]>,
        size: u32,
    ) {
        self.lut_baker
            .set_ocio_luts(&self.device, &self.queue, idt_lut, odt_lut, size);
    }

    /// Get a reference to the current output image, if any.
    pub fn current_output(&self) -> Option<&GpuImageHandle> {
        self.current_output.as_ref()
    }

    /// Set per-scope visibility flags. Hidden scopes skip GPU compute dispatch.
    pub fn set_scope_visibility(
        &mut self,
        histogram: bool,
        waveform: bool,
        vectorscope: bool,
        cie: bool,
    ) {
        self.scope_histogram_visible = histogram;
        self.scope_waveform_visible = waveform;
        self.scope_vectorscope_visible = vectorscope;
        self.scope_cie_visible = cie;
    }

    /// Set the viewer pixel format (F16, F32, or Srgb8).
    pub fn set_viewer_format(&mut self, format: ViewerFormat) {
        if self.viewer_format != format {
            self.viewer_format = format;
            // Invalidate staging buffers — size differs between formats.
            self.image_readback_staging = None;
            self.async_readback = None;
        }
    }

    /// Get the current viewer format.
    pub fn viewer_format(&self) -> ViewerFormat {
        self.viewer_format
    }

    /// Whether an async readback is currently in flight (not yet consumed).
    pub fn has_pending_readback(&self) -> bool {
        self.async_readback
            .as_ref()
            .is_some_and(|rb| rb.has_pending())
    }

    /// Upload a per-pixel scope mask. Pixels with mask=0 are excluded from scope analysis.
    pub fn set_scope_mask(&mut self, mask: &[u32]) {
        self.scope_dispatch
            .update_mask(&self.device, &self.queue, mask);
    }

    /// Clear the scope mask so all pixels are included in scope analysis.
    pub fn clear_scope_mask(&mut self) {
        self.scope_dispatch.clear_mask(&self.queue);
    }
}
