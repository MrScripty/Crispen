//! Bevy systems for the color grading pipeline.
//!
//! These systems are the ONLY place grading state changes. The frontend
//! sends commands via `ColorGradingCommand`, Bevy processes them, and
//! pushes new state back via outbound messages.

use bevy::prelude::*;
use std::time::Instant;

use crispen_core::grading::auto_balance;
use crispen_core::transform::params::GradingParams;
use crispen_gpu::ScopeResults;

use crate::events::{
    ColorGradingCommand, ImageLoadedEvent, ParamsUpdatedEvent, ScopeDataReadyEvent,
};
#[cfg(feature = "ocio")]
use crate::resources::OcioColorManagement;
use crate::resources::{
    GpuPipelineState, GradingState, ImageState, PipelinePerfStats, ScopeConfig, ScopeMaskData,
    ScopeState, ViewerData,
};

/// Process inbound grading commands from the UI.
///
/// Reads `ColorGradingCommand` messages and mutates `GradingState`,
/// `ImageState`, and `ScopeConfig` accordingly. Fires outbound
/// notification messages when state changes.
pub fn handle_grading_commands(
    mut commands: MessageReader<ColorGradingCommand>,
    mut state: ResMut<GradingState>,
    images: Res<ImageState>,
    mut scope_config: ResMut<ScopeConfig>,
    mut params_updated: MessageWriter<ParamsUpdatedEvent>,
    mut _image_loaded: MessageWriter<ImageLoadedEvent>,
) {
    let mut pending_params_update: Option<GradingParams> = None;

    for cmd in commands.read() {
        match cmd {
            ColorGradingCommand::SetParams { params } => {
                if state.params != *params {
                    state.params = params.clone();
                    state.dirty = true;
                    pending_params_update = Some(state.params.clone());
                }
            }
            ColorGradingCommand::AutoBalance => {
                if let Some(ref source) = images.source {
                    let (temp, tint) = auto_balance::auto_white_balance(source);
                    if state.params.temperature != temp || state.params.tint != tint {
                        state.params.temperature = temp;
                        state.params.tint = tint;
                        state.dirty = true;
                        pending_params_update = Some(state.params.clone());
                    }
                } else {
                    tracing::warn!("AutoBalance: no source image loaded");
                }
            }
            ColorGradingCommand::ResetGrade => {
                let defaults = GradingParams::default();
                if state.params != defaults {
                    state.params = defaults;
                    state.dirty = true;
                    pending_params_update = Some(state.params.clone());
                }
            }
            ColorGradingCommand::LoadImage { path } => {
                // Actual loading handled by the demo app's image_loader.
                // The demo converts UiToBevy::LoadImage into this command,
                // loads the file, and injects into ImageState directly.
                tracing::info!("LoadImage command received: {}", path);
            }
            ColorGradingCommand::LoadLut { path, slot } => {
                tracing::info!("LoadLut: {} -> slot {}", path, slot);
            }
            ColorGradingCommand::ExportLut { path, size } => {
                if state.lut.is_some() {
                    tracing::info!("ExportLut: {} (size {})", path, size);
                } else {
                    tracing::warn!("ExportLut: no LUT baked yet");
                }
            }
            ColorGradingCommand::ToggleScope {
                scope_type,
                visible,
            } => match scope_type.as_str() {
                "histogram" => {
                    if scope_config.histogram_visible != *visible {
                        scope_config.histogram_visible = *visible;
                    }
                }
                "waveform" => {
                    if scope_config.waveform_visible != *visible {
                        scope_config.waveform_visible = *visible;
                    }
                }
                "vectorscope" => {
                    if scope_config.vectorscope_visible != *visible {
                        scope_config.vectorscope_visible = *visible;
                    }
                }
                "cie" => {
                    if scope_config.cie_visible != *visible {
                        scope_config.cie_visible = *visible;
                    }
                }
                other => tracing::warn!("Unknown scope type: {}", other),
            },
        }
    }

    if let Some(params) = pending_params_update {
        params_updated.write(ParamsUpdatedEvent { params });
    }
}

/// Diagnostic system that logs when `GradingState` is changed.
pub fn detect_param_changes(state: Res<GradingState>) {
    if state.is_changed() && !state.is_added() {
        tracing::debug!("GradingState changed, dirty={}", state.dirty);
    }
}

/// Re-bake OCIO IDT/ODT LUTs when OCIO display/input selection changes.
#[cfg(feature = "ocio")]
pub fn bake_ocio_luts(
    ocio: Option<ResMut<OcioColorManagement>>,
    mut grading: ResMut<GradingState>,
) {
    let Some(mut ocio) = ocio else { return };
    if !ocio.dirty {
        return;
    }

    let mut idt_lut = None;
    let mut odt_lut = None;

    match ocio
        .config
        .processor(&ocio.input_space, &ocio.working_space)
        .and_then(|p| p.cpu_f32())
    {
        Ok(cpu) => {
            idt_lut = Some(cpu.bake_3d_lut(65));
        }
        Err(err) => {
            tracing::warn!(
                "OCIO IDT bake failed for '{}' -> '{}': {err}",
                ocio.input_space,
                ocio.working_space
            );
        }
    }

    match ocio
        .config
        .display_view_processor(&ocio.working_space, &ocio.display, &ocio.view)
        .and_then(|p| p.cpu_f32())
    {
        Ok(cpu) => {
            odt_lut = Some(cpu.bake_3d_lut(65));
        }
        Err(err) => {
            tracing::warn!(
                "OCIO ODT bake failed for '{}' -> {}/{}: {err}",
                ocio.working_space,
                ocio.display,
                ocio.view
            );
        }
    }

    ocio.idt_lut = idt_lut;
    ocio.odt_lut = odt_lut;
    ocio.dirty = false;
    grading.dirty = true;
}

/// Submit GPU work (bake + apply + scopes) when params are dirty. Non-blocking.
///
/// The actual results are consumed by [`consume_gpu_results`] on a subsequent frame.
pub fn submit_gpu_work(
    mut state: ResMut<GradingState>,
    mut perf: ResMut<PipelinePerfStats>,
    gpu: Option<ResMut<GpuPipelineState>>,
    #[cfg(feature = "ocio")] ocio: Option<Res<OcioColorManagement>>,
) {
    if !state.dirty {
        return;
    }

    let Some(mut gpu) = gpu else {
        tracing::warn!("submit_gpu_work: dirty but no GPU pipeline — discarding");
        state.dirty = false;
        return;
    };

    let gpu = &mut *gpu;

    let Some(ref source_handle) = gpu.source_handle else {
        tracing::debug!("submit_gpu_work: dirty but no source image — waiting");
        state.dirty = false;
        return;
    };

    #[cfg(feature = "ocio")]
    {
        if let Some(ocio) = ocio.as_ref() {
            gpu.pipeline
                .set_ocio_luts(ocio.idt_lut.as_deref(), ocio.odt_lut.as_deref(), 65);
            // Sync the OCIO display OETF into the grading params so the shader
            // knows which inverse OETF to apply after the ODT.
            state.params.color_management.display_oetf = ocio.display_oetf;
        } else {
            gpu.pipeline.set_ocio_luts(None, None, 65);
        }
    }

    // Don't submit if the previous readback hasn't been consumed yet.
    // Keep dirty=true so we retry next frame after consume frees the slot.
    if gpu.pipeline.has_pending_readback() {
        return;
    }

    let submit_start = Instant::now();

    // Non-blocking GPU submission: bake → apply → format convert → scopes → async readback.
    gpu.pipeline
        .submit_gpu_work(source_handle, &state.params, 65);

    let submit_time = submit_start.elapsed();
    perf.updates += 1;
    perf.total_time = submit_time;

    if submit_time >= perf.slow_update_threshold || perf.last_log_at.elapsed().as_secs_f32() >= 1.0
    {
        tracing::info!(
            "gpu submit: {:.2}ms (update #{})",
            submit_time.as_secs_f64() * 1000.0,
            perf.updates
        );
        perf.last_log_at = Instant::now();
    }

    state.dirty = false;
}

/// Non-blocking: poll for async GPU readback results and update viewer + scopes.
///
/// Runs every frame. If no results are ready yet, returns immediately.
pub fn consume_gpu_results(
    mut viewer_data: ResMut<ViewerData>,
    mut scope_state: ResMut<ScopeState>,
    mut scope_ready: MessageWriter<ScopeDataReadyEvent>,
    gpu: Option<ResMut<GpuPipelineState>>,
) {
    let Some(mut gpu) = gpu else {
        return;
    };

    let Some(result) = gpu.pipeline.try_consume_readback() else {
        return;
    };

    viewer_data.pixel_bytes = result.viewer_bytes;
    viewer_data.width = result.width;
    viewer_data.height = result.height;
    viewer_data.format = result.format;

    if let Some(results) = result.scopes {
        apply_scope_results(&mut scope_state, results);
        scope_ready.write(ScopeDataReadyEvent);
    }
}

/// Upload the scope mask to the GPU pipeline when it changes.
pub fn upload_scope_mask(
    mut mask_data: ResMut<ScopeMaskData>,
    gpu: Option<ResMut<GpuPipelineState>>,
) {
    if !mask_data.dirty {
        return;
    }
    let Some(mut gpu) = gpu else {
        mask_data.dirty = false;
        return;
    };

    if mask_data.active && !mask_data.mask.is_empty() {
        gpu.pipeline.set_scope_mask(&mask_data.mask);
    } else {
        gpu.pipeline.clear_scope_mask();
    }
    mask_data.dirty = false;
}

fn apply_scope_results(scope_state: &mut ScopeState, results: ScopeResults) {
    let ScopeResults {
        histogram,
        waveform,
        vectorscope,
        cie,
    } = results;

    scope_state.histogram = Some(histogram);
    scope_state.waveform = Some(waveform);
    scope_state.vectorscope = Some(vectorscope);
    scope_state.cie = Some(cie);
}
