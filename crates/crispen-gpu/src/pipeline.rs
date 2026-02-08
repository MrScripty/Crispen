//! Top-level GPU grading pipeline that orchestrates all compute passes.

use std::sync::Arc;

use crispen_core::image::GradingImage;
use crispen_core::transform::params::GradingParams;

use crate::buffers::{GpuImageHandle, GpuLutHandle, ScopeBuffers, ScopeConfig};
use crate::lut_applicator::LutApplicator;
use crate::lut_baker::LutBaker;
use crate::readback::{Readback, ScopeResults};
use crate::scope_dispatch::ScopeDispatch;

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
    lut_baker: LutBaker,
    lut_applicator: LutApplicator,
    scope_dispatch: ScopeDispatch,
    current_lut: Option<GpuLutHandle>,
    current_output: Option<GpuImageHandle>,
    scope_buffers: Option<ScopeBuffers>,
    readback: Option<Readback>,
    scope_config: ScopeConfig,
}

impl GpuGradingPipeline {
    /// Create the full GPU pipeline. Compiles all shaders.
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let lut_baker = LutBaker::new(&device, &queue);
        let lut_applicator = LutApplicator::new(&device);
        let scope_dispatch = ScopeDispatch::new(&device);

        Self {
            device,
            queue,
            lut_baker,
            lut_applicator,
            scope_dispatch,
            current_lut: None,
            current_output: None,
            scope_buffers: None,
            readback: None,
            scope_config: ScopeConfig::default(),
        }
    }

    /// Upload a source image to the GPU.
    pub fn upload_image(&self, image: &GradingImage) -> GpuImageHandle {
        GpuImageHandle::upload(&self.device, &self.queue, image)
    }

    /// Download a graded image from the GPU.
    pub fn download_image(&self, handle: &GpuImageHandle) -> GradingImage {
        Readback::download_image(&self.device, &self.queue, handle)
    }

    /// Bake grading parameters into a 3D LUT.
    pub fn bake_lut(&mut self, params: &GradingParams, lut_size: u32) {
        let lut = self
            .current_lut
            .get_or_insert_with(|| GpuLutHandle::new(&self.device, lut_size));

        // Recreate if size changed.
        if lut.size != lut_size {
            *lut = GpuLutHandle::new(&self.device, lut_size);
        }

        self.lut_baker
            .upload_curves(&self.device, &self.queue, params);
        self.lut_baker
            .bake(&self.device, &self.queue, params, lut);
    }

    /// Apply the current LUT to a source image. Returns the output handle.
    pub fn apply_lut(&mut self, source: &GpuImageHandle) -> &GpuImageHandle {
        let lut = self
            .current_lut
            .as_ref()
            .expect("must bake LUT before applying");

        let output = self.current_output.get_or_insert_with(|| {
            GpuImageHandle::create_output(&self.device, source.width, source.height)
        });

        // Recreate if dimensions changed.
        if output.width != source.width || output.height != source.height {
            *output = GpuImageHandle::create_output(&self.device, source.width, source.height);
        }

        self.lut_applicator
            .apply(&self.device, &self.queue, source, lut, output);

        self.current_output.as_ref().unwrap()
    }

    /// Compute scopes on the given image.
    pub fn compute_scopes(&mut self, image: &GpuImageHandle) -> ScopeResults {
        let cfg = self.scope_config;

        let scope_buffers = self.scope_buffers.get_or_insert_with(|| {
            ScopeBuffers::new(&self.device, &cfg, image.width)
        });

        self.scope_dispatch.dispatch(
            &self.device,
            &self.queue,
            image,
            scope_buffers,
            cfg.waveform_height,
            cfg.vectorscope_resolution,
            cfg.cie_resolution,
        );

        let readback = self.readback.get_or_insert_with(|| {
            Readback::new(&self.device, &cfg, image.width)
        });

        // Copy scope data to staging.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("crispen_scope_readback_encoder"),
            });
        readback.copy_to_staging(&mut encoder, scope_buffers);
        self.queue.submit(std::iter::once(encoder.finish()));

        readback.read_scopes(&self.device)
    }

    /// Run the full pipeline: bake + apply + scopes.
    pub fn execute(
        &mut self,
        source: &GpuImageHandle,
        params: &GradingParams,
        lut_size: u32,
    ) -> ScopeResults {
        self.bake_lut(params, lut_size);
        self.apply_lut(source);
        self.compute_scopes_on_output()
    }

    /// Compute scopes on the current output image.
    fn compute_scopes_on_output(&mut self) -> ScopeResults {
        let output = self.current_output.as_ref().expect("must apply LUT first");
        let width = output.width;
        let cfg = self.scope_config;

        let scope_buffers = self.scope_buffers.get_or_insert_with(|| {
            ScopeBuffers::new(&self.device, &cfg, width)
        });

        self.scope_dispatch.dispatch(
            &self.device,
            &self.queue,
            self.current_output.as_ref().unwrap(),
            scope_buffers,
            cfg.waveform_height,
            cfg.vectorscope_resolution,
            cfg.cie_resolution,
        );

        let readback = self.readback.get_or_insert_with(|| {
            Readback::new(&self.device, &cfg, width)
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("crispen_scope_readback_encoder"),
            });
        readback.copy_to_staging(&mut encoder, self.scope_buffers.as_ref().unwrap());
        self.queue.submit(std::iter::once(encoder.finish()));

        readback.read_scopes(&self.device)
    }

    /// Set the scope configuration (waveform height, vectorscope/CIE resolution).
    pub fn set_scope_config(&mut self, config: ScopeConfig) {
        self.scope_config = config;
        // Invalidate cached scope resources so they're recreated.
        self.scope_buffers = None;
        self.readback = None;
    }

    /// Get a reference to the current output image, if any.
    pub fn current_output(&self) -> Option<&GpuImageHandle> {
        self.current_output.as_ref()
    }
}
