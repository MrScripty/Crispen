//! Crispen Bevy Plugin — integrates the color grading pipeline into Bevy's ECS.
//!
//! Provides `CrispenPlugin` which registers all resources, events, and systems
//! needed to run the grading pipeline within a Bevy application.

pub mod events;
pub mod render_node;
pub mod resources;
pub mod scope_render;
pub mod systems;

use bevy::prelude::*;
use crispen_gpu::GpuGradingPipeline;
use crispen_gpu::vulkan_interop::VulkanInterop;

// Re-export for downstream crates.
pub use crispen_gpu::ViewerFormat;

use events::{ColorGradingCommand, ImageLoadedEvent, ParamsUpdatedEvent, ScopeDataReadyEvent};
use resources::{
    GpuPipelineState, GradingState, ImageState, PipelinePerfStats, ScopeConfig, ScopeState,
    ViewerData, VulkanInteropState,
};
#[cfg(feature = "ocio")]
use systems::bake_ocio_luts;
use systems::{
    consume_gpu_results, detect_param_changes, handle_grading_commands, submit_gpu_work,
};

/// Main Bevy plugin for the Crispen color grading pipeline.
///
/// Registers resources, events, and systems for:
/// - Managing `GradingParams` as a Bevy resource
/// - Triggering LUT re-bake on parameter changes via GPU
/// - Running scope computation when a graded image is available
/// - GPU pipeline creation at startup
pub struct CrispenPlugin;

impl Plugin for CrispenPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ColorGradingCommand>()
            .add_message::<ParamsUpdatedEvent>()
            .add_message::<ImageLoadedEvent>()
            .add_message::<ScopeDataReadyEvent>()
            .init_resource::<GradingState>()
            .init_resource::<ImageState>()
            .init_resource::<ViewerData>()
            .init_resource::<ScopeState>()
            .init_resource::<ScopeConfig>()
            .init_resource::<PipelinePerfStats>()
            .add_systems(Startup, init_gpu_pipeline)
            .add_systems(
                Update,
                (
                    handle_grading_commands,
                    consume_gpu_results.after(handle_grading_commands),
                    submit_gpu_work.after(consume_gpu_results),
                    detect_param_changes,
                ),
            );

        #[cfg(feature = "ocio")]
        app.add_systems(
            Update,
            bake_ocio_luts
                .after(handle_grading_commands)
                .before(submit_gpu_work),
        );
    }
}

/// Startup system: create the GPU grading pipeline and insert as a resource.
fn init_gpu_pipeline(mut commands: Commands) {
    match GpuGradingPipeline::create_blocking() {
        Ok(pipeline) => {
            let interop_caps = VulkanInterop::probe(
                pipeline.adapter_backend(),
                pipeline.device(),
                pipeline.enabled_features(),
            );
            let zero_copy_available = interop_caps.supports_d3d11_win32_import;

            tracing::info!(
                backend = ?interop_caps.backend,
                has_vulkan_hal_access = interop_caps.has_vulkan_hal_access,
                zero_copy_import_available = zero_copy_available,
                "Vulkan interop capabilities probed"
            );

            commands.insert_resource(VulkanInteropState {
                capabilities: interop_caps,
            });
            commands.insert_resource(GpuPipelineState {
                pipeline,
                source_handle: None,
            });
            tracing::info!("GPU grading pipeline initialized");
        }
        Err(e) => {
            tracing::error!("Failed to initialize GPU grading pipeline: {e}");
            tracing::warn!("Grading will not function without a GPU pipeline");
        }
    }
}
