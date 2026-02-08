//! Bevy systems for the color grading pipeline.
//!
//! These systems are the ONLY place grading state changes. The frontend
//! sends commands via `ColorGradingCommand`, Bevy processes them, and
//! pushes new state back via outbound messages.

use bevy::prelude::*;

use crispen_core::grading::auto_balance;
use crispen_core::scopes::{cie, histogram, vectorscope, waveform};
use crispen_core::transform::params::GradingParams;

use crate::events::{
    ColorGradingCommand, ImageLoadedEvent, ParamsUpdatedEvent, ScopeDataReadyEvent,
};
use crate::resources::{GpuPipelineState, GradingState, ImageState, ScopeConfig, ScopeState};

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
    for cmd in commands.read() {
        match cmd {
            ColorGradingCommand::SetParams { params } => {
                state.params = params.clone();
                state.dirty = true;
                params_updated.write(ParamsUpdatedEvent {
                    params: params.clone(),
                });
            }
            ColorGradingCommand::AutoBalance => {
                if let Some(ref source) = images.source {
                    let (temp, tint) = auto_balance::auto_white_balance(source);
                    state.params.temperature = temp;
                    state.params.tint = tint;
                    state.dirty = true;
                    params_updated.write(ParamsUpdatedEvent {
                        params: state.params.clone(),
                    });
                } else {
                    tracing::warn!("AutoBalance: no source image loaded");
                }
            }
            ColorGradingCommand::ResetGrade => {
                state.params = GradingParams::default();
                state.dirty = true;
                params_updated.write(ParamsUpdatedEvent {
                    params: state.params.clone(),
                });
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
                "histogram" => scope_config.histogram_visible = *visible,
                "waveform" => scope_config.waveform_visible = *visible,
                "vectorscope" => scope_config.vectorscope_visible = *visible,
                "cie" => scope_config.cie_visible = *visible,
                other => tracing::warn!("Unknown scope type: {}", other),
            },
        }
    }
}

/// Diagnostic system that logs when `GradingState` is changed.
pub fn detect_param_changes(state: Res<GradingState>) {
    if state.is_changed() && !state.is_added() {
        tracing::debug!("GradingState changed, dirty={}", state.dirty);
    }
}

/// Re-bake the 3D LUT on the GPU and apply to the source image when params are dirty.
///
/// Dispatches GPU compute for LUT baking and image grading, then reads
/// back the graded image for CPU scope computation.
pub fn rebake_lut_if_dirty(
    mut state: ResMut<GradingState>,
    mut images: ResMut<ImageState>,
    gpu: Option<ResMut<GpuPipelineState>>,
) {
    if !state.dirty {
        return;
    }

    let Some(mut gpu) = gpu else {
        // No GPU pipeline available — clear dirty flag to avoid spinning.
        state.dirty = false;
        return;
    };

    // Dereference ResMut to enable split borrows on struct fields.
    let gpu = &mut *gpu;

    let Some(ref source_handle) = gpu.source_handle else {
        // No source image uploaded yet — nothing to grade.
        state.dirty = false;
        return;
    };

    // GPU bake LUT + apply to source image.
    gpu.pipeline.bake_lut(&state.params, 65);
    gpu.pipeline.apply_lut(source_handle);

    // Readback graded image for CPU scope computation.
    let output = gpu.pipeline.current_output().expect("output exists after apply_lut");
    let graded = gpu.pipeline.download_image(output);
    images.graded = Some(graded);

    state.dirty = false;
}

/// Compute scope data from the graded image.
///
/// Phase 1 guard: only runs when a graded image exists (which it won't
/// until the full LUT apply pipeline works). Calls into crispen-core
/// scope functions that are `todo!()` stubs.
pub fn update_scopes(
    images: Res<ImageState>,
    scope_config: Res<ScopeConfig>,
    mut scope_state: ResMut<ScopeState>,
    mut scope_ready: MessageWriter<ScopeDataReadyEvent>,
) {
    let Some(ref graded) = images.graded else {
        return;
    };

    if scope_config.histogram_visible {
        scope_state.histogram = Some(histogram::compute(graded));
    }
    if scope_config.waveform_visible {
        scope_state.waveform = Some(waveform::compute(graded));
    }
    if scope_config.vectorscope_visible {
        scope_state.vectorscope = Some(vectorscope::compute(graded));
    }
    if scope_config.cie_visible {
        scope_state.cie = Some(cie::compute(graded));
    }

    scope_ready.write(ScopeDataReadyEvent);
}
