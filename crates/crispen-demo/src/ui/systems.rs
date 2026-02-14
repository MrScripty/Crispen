//! ECS systems that synchronize UI state with GradingParams.
//!
//! Bidirectional sync:
//! - Dials/wheels → `GradingState.params` (user interaction)
//! - `GradingState.params` → dials/wheels (external changes like reset)

use std::path::Path;

use bevy::prelude::*;
use bevy::ui_render::prelude::MaterialNode;
use bevy::ui_widgets::ValueChange;
use bevy::window::PrimaryWindow;

use crispen_bevy::events::ImageLoadedEvent;
#[cfg(feature = "ocio")]
use crispen_bevy::resources::OcioColorManagement;
use crispen_bevy::resources::{GpuPipelineState, GradingState, ImageState};

use super::color_wheel::{ColorWheelMaterial, WheelType};
use super::components::ParamId;
use super::dial::{DialMaterial, DialRange, DialValue, ParamDial};
use super::master_slider::{
    MasterSliderInner, MasterSliderMaterial, MasterSliderRange, MasterSliderValue,
    MasterSliderWheel,
};
use super::theme;
use crate::image_loader;

const PARAM_SYNC_EPSILON: f32 = 1e-4;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Read the current value of a param from GradingState.
fn read_param(state: &GradingState, id: ParamId) -> f32 {
    match id {
        ParamId::Temperature => state.params.temperature,
        ParamId::Tint => state.params.tint,
        ParamId::Contrast => state.params.contrast,
        ParamId::Pivot => state.params.pivot,
        ParamId::MidtoneDetail => state.params.midtone_detail,
        ParamId::Shadows => state.params.shadows,
        ParamId::Highlights => state.params.highlights,
        ParamId::Saturation => state.params.saturation,
        ParamId::Hue => state.params.hue,
        ParamId::LumaMix => state.params.luma_mix,
    }
}

/// Write a param value into GradingState.
fn write_param(state: &mut GradingState, id: ParamId, v: f32) {
    match id {
        ParamId::Temperature => state.params.temperature = v,
        ParamId::Tint => state.params.tint = v,
        ParamId::Contrast => state.params.contrast = v,
        ParamId::Pivot => state.params.pivot = v,
        ParamId::MidtoneDetail => state.params.midtone_detail = v,
        ParamId::Shadows => state.params.shadows = v,
        ParamId::Highlights => state.params.highlights = v,
        ParamId::Saturation => state.params.saturation = v,
        ParamId::Hue => state.params.hue = v,
        ParamId::LumaMix => state.params.luma_mix = v,
    }
}

// ── Dials → GradingState ────────────────────────────────────────────────────

/// Sync dial value changes to GradingState parameters.
///
/// Only mutates `GradingState` when the dial value actually differs from
/// the current param, preventing feedback loops with `sync_params_to_dials`.
pub fn sync_dials_to_params(
    dials: Query<(&DialValue, &ParamDial), Changed<DialValue>>,
    mut state: ResMut<GradingState>,
) {
    for (value, dial) in dials.iter() {
        let current = read_param(&state, dial.0);
        if (current - value.0).abs() > PARAM_SYNC_EPSILON {
            tracing::info!(
                "sync_dials_to_params: {:?} {} -> {}",
                dial.0,
                current,
                value.0,
            );
            write_param(&mut state, dial.0, value.0);
            state.dirty = true;
        }
    }
}

// ── Wheels → GradingState ───────────────────────────────────────────────────

/// Observer: sync wheel value changes to GradingState lift/gamma/gain/offset.
///
/// Converts the 0..1 Vec2 position to \[R, G, B, Master\] channel adjustments:
/// - X axis: B channel (right = more blue, left = less blue) → aligns with Cb (vectorscope X)
/// - Y axis: R-G balance (down = more red, up = more green) → aligns with Cr (vectorscope Y)
pub fn on_wheel_value_change(
    event: On<ValueChange<Vec2>>,
    wheels: Query<&WheelType>,
    mut state: ResMut<GradingState>,
) {
    let Ok(wheel_type) = wheels.get(event.source) else {
        tracing::warn!(
            "on_wheel_value_change: source entity {:?} has no WheelType",
            event.source
        );
        return;
    };

    // Map 0..1 UI space to -1..1 offset space.
    let dx = (event.value.x - 0.5) * 2.0;
    let dy = (event.value.y - 0.5) * 2.0;

    tracing::info!(
        "on_wheel_value_change: {:?} dx={:.3} dy={:.3}",
        wheel_type,
        dx,
        dy,
    );

    let channels = match wheel_type {
        WheelType::Lift | WheelType::Offset => {
            // Additive channels (neutral = 0).
            // X → B (Cb direction), Y → R/G balance (Cr direction).
            let master = match wheel_type {
                WheelType::Lift => state.params.lift_wheel[3],
                _ => state.params.offset_wheel[3],
            };
            [dy, -dy, dx, master]
        }
        WheelType::Gamma | WheelType::Gain => {
            // Multiplicative channels (neutral = 1).
            // X → B (Cb direction), Y → R/G balance (Cr direction).
            let master = match wheel_type {
                WheelType::Gamma => state.params.gamma_wheel[3],
                _ => state.params.gain_wheel[3],
            };
            [1.0 + dy, 1.0 - dy, 1.0 + dx, master]
        }
    };

    match wheel_type {
        WheelType::Lift => state.params.lift_wheel = channels,
        WheelType::Gamma => state.params.gamma_wheel = channels,
        WheelType::Gain => state.params.gain_wheel = channels,
        WheelType::Offset => state.params.offset_wheel = channels,
    }
    state.dirty = true;
}

// ── GradingState → Dials ────────────────────────────────────────────────────

/// Sync GradingState back to dial values when params change externally
/// (e.g. ResetGrade, AutoBalance).
pub fn sync_params_to_dials(
    state: Res<GradingState>,
    mut dials: Query<(Entity, &ParamDial, &mut DialValue, &DialRange)>,
    q_material: Query<&MaterialNode<DialMaterial>>,
    mut materials: ResMut<Assets<DialMaterial>>,
) {
    if !state.is_changed() {
        return;
    }
    for (entity, dial, mut current, range) in dials.iter_mut() {
        let target = read_param(&state, dial.0);
        if (current.0 - target).abs() > PARAM_SYNC_EPSILON {
            current.0 = target;

            // Update material uniform so the knob visual matches.
            if let Ok(mat_node) = q_material.get(entity)
                && let Some(mat) = materials.get_mut(mat_node.id())
            {
                let span = range.max - range.min;
                mat.value_norm = if span.abs() < f32::EPSILON {
                    0.5
                } else {
                    ((target - range.min) / span).clamp(0.0, 1.0)
                };
            }
        }
    }
}

// ── Image Loading ───────────────────────────────────────────────────────────

/// Open a native file dialog on Ctrl+O and load the selected image into the
/// grading pipeline.
pub fn handle_load_image_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut image_state: ResMut<ImageState>,
    mut grading_state: ResMut<GradingState>,
    gpu: Option<ResMut<GpuPipelineState>>,
    #[cfg(feature = "ocio")] mut ocio: Option<ResMut<OcioColorManagement>>,
    mut image_loaded: MessageWriter<ImageLoadedEvent>,
) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if !(ctrl && keys.just_pressed(KeyCode::KeyO)) {
        return;
    }

    let dialog = rfd::FileDialog::new().set_title("Load Image");
    #[cfg(feature = "ocio")]
    let dialog = dialog.add_filter(
        "Images",
        &[
            "jpg", "jpeg", "png", "tif", "tiff", "exr", "dpx", "cin", "hdr", "bmp", "tga", "webp",
            "psd", "gif",
        ],
    );
    #[cfg(not(feature = "ocio"))]
    let dialog = dialog.add_filter("Images", &["jpg", "jpeg", "png", "tif", "tiff", "exr"]);
    let Some(path) = dialog.pick_file() else {
        return;
    };

    let preview_size = viewer_target_size(&window_q);
    load_image_from_path(
        &path,
        preview_size,
        &mut image_state,
        &mut grading_state,
        gpu,
        #[cfg(feature = "ocio")]
        ocio.as_deref_mut(),
        &mut image_loaded,
    );
}

/// Load an image file into the grading pipeline.
fn load_image_from_path(
    path: &Path,
    preview_size: Option<(u32, u32)>,
    image_state: &mut ResMut<ImageState>,
    grading_state: &mut ResMut<GradingState>,
    gpu: Option<ResMut<GpuPipelineState>>,
    #[cfg(feature = "ocio")] ocio_state: Option<&mut OcioColorManagement>,
    image_loaded: &mut MessageWriter<ImageLoadedEvent>,
) {
    // Use OIIO when available (auto-detects color space from metadata),
    // fall back to the `image` crate otherwise.
    #[cfg(feature = "ocio")]
    let loaded = match image_loader::load_image_oiio(path, preview_size) {
        Ok(loaded) => loaded,
        Err(e) => {
            tracing::error!("Failed to load image {}: {e}", path.display());
            return;
        }
    };
    #[cfg(not(feature = "ocio"))]
    let loaded = match image_loader::load_image_for_display(path, preview_size) {
        Ok(loaded) => loaded,
        Err(e) => {
            tracing::error!("Failed to load image {}: {e}", path.display());
            return;
        }
    };

    let image = loaded.image;

    // Auto-detect input color space from OIIO metadata or bit depth.
    let detected_input_space = image_loader::detected_color_space_to_id(
        loaded.detected_color_space.as_deref(),
        image.source_bit_depth,
    );

    tracing::info!(
        "Loaded image: {}x{} {:?} from {} (color space: {:?}, input: {})",
        image.width,
        image.height,
        image.source_bit_depth,
        path.display(),
        loaded.detected_color_space,
        detected_input_space.label(),
    );

    let width = image.width;
    let height = image.height;
    let bit_depth = format!("{:?}", image.source_bit_depth);

    // Update input space to match the actual source encoding.
    grading_state.params.color_management.input_space = detected_input_space;

    // Upload to GPU if the pipeline is available.
    if let Some(mut gpu) = gpu {
        let handle = gpu.pipeline.upload_image(&image);
        gpu.source_handle = Some(handle);
    }

    image_state.source = Some(image);
    image_state.source_path = Some(path.display().to_string());
    grading_state.dirty = true;

    #[cfg(feature = "ocio")]
    if let Some(ocio) = ocio_state {
        // Prefer the color space OIIO detected from file metadata; fall back
        // to the manual mapping from ColorSpaceId.
        if let Some(ref cs) = loaded.detected_color_space {
            ocio.input_space = cs.clone();
        } else {
            ocio.input_space =
                crate::ocio_support::map_detected_to_ocio_name(detected_input_space, &ocio.config);
        }
        ocio.dirty = true;
    }

    image_loaded.write(ImageLoadedEvent {
        path: path.display().to_string(),
        width,
        height,
        bit_depth,
    });
}

fn viewer_target_size(window_q: &Query<&Window, With<PrimaryWindow>>) -> Option<(u32, u32)> {
    let window = window_q.iter().next()?;
    let scale = window.resolution.scale_factor();
    let width = window.width() * scale;
    let height = window.height() * scale;

    // Match the UI layout: full-width viewer above a fixed-height primaries panel,
    // with a small margin for panel padding/frame borders.
    let target_width = (width - 24.0).max(128.0).round() as u32;
    let target_height = (height - theme::PRIMARIES_PANEL_HEIGHT * scale - 32.0)
        .max(128.0)
        .round() as u32;

    Some((target_width, target_height))
}

// ── Master Sliders → GradingState ───────────────────────────────────────────

/// Sync master slider value changes to GradingState lift/gamma/gain/offset[3].
pub fn sync_master_sliders_to_params(
    sliders: Query<
        (&MasterSliderValue, &MasterSliderWheel),
        (Changed<MasterSliderValue>, With<MasterSliderInner>),
    >,
    mut state: ResMut<GradingState>,
) {
    for (value, wheel) in sliders.iter() {
        let current_master = match wheel.0 {
            WheelType::Lift => state.params.lift_wheel[3],
            WheelType::Gamma => state.params.gamma_wheel[3],
            WheelType::Gain => state.params.gain_wheel[3],
            WheelType::Offset => state.params.offset_wheel[3],
        };
        if (current_master - value.0).abs() > PARAM_SYNC_EPSILON {
            tracing::info!(
                "sync_master_sliders_to_params: {:?} master {} -> {}",
                wheel.0,
                current_master,
                value.0,
            );
            match wheel.0 {
                WheelType::Lift => state.params.lift_wheel[3] = value.0,
                WheelType::Gamma => state.params.gamma_wheel[3] = value.0,
                WheelType::Gain => state.params.gain_wheel[3] = value.0,
                WheelType::Offset => state.params.offset_wheel[3] = value.0,
            }
            state.dirty = true;
        }
    }
}

// ── GradingState → Master Sliders ──────────────────────────────────────────

/// Sync GradingState back to master slider values when params change externally.
pub fn sync_params_to_master_sliders(
    state: Res<GradingState>,
    mut sliders: Query<
        (
            Entity,
            &MasterSliderWheel,
            &mut MasterSliderValue,
            &MasterSliderRange,
        ),
        With<MasterSliderInner>,
    >,
    q_material: Query<&MaterialNode<MasterSliderMaterial>>,
    mut materials: ResMut<Assets<MasterSliderMaterial>>,
) {
    if !state.is_changed() {
        return;
    }
    for (entity, wheel, mut value, range) in sliders.iter_mut() {
        let target = match wheel.0 {
            WheelType::Lift => state.params.lift_wheel[3],
            WheelType::Gamma => state.params.gamma_wheel[3],
            WheelType::Gain => state.params.gain_wheel[3],
            WheelType::Offset => state.params.offset_wheel[3],
        };
        if (value.0 - target).abs() > PARAM_SYNC_EPSILON {
            value.0 = target;

            if let Ok(mat_node) = q_material.get(entity)
                && let Some(mat) = materials.get_mut(mat_node.id())
            {
                let span = range.max - range.min;
                mat.value_norm = if span.abs() < f32::EPSILON {
                    0.5
                } else {
                    ((target - range.min) / span).clamp(0.0, 1.0)
                };
            }
        }
    }
}

// ── GradingState → Wheels ───────────────────────────────────────────────────

/// Sync GradingState back to wheel cursor positions and thumb nodes when
/// params change externally.
pub fn sync_params_to_wheels(
    state: Res<GradingState>,
    wheels: Query<(&WheelType, &Children)>,
    q_children: Query<&Children>,
    q_material: Query<&MaterialNode<ColorWheelMaterial>>,
    mut q_node: Query<&mut Node>,
    mut materials: ResMut<Assets<ColorWheelMaterial>>,
) {
    if !state.is_changed() {
        return;
    }

    for (wheel_type, children) in wheels.iter() {
        let channels = match wheel_type {
            WheelType::Lift => &state.params.lift_wheel,
            WheelType::Gamma => &state.params.gamma_wheel,
            WheelType::Gain => &state.params.gain_wheel,
            WheelType::Offset => &state.params.offset_wheel,
        };

        // Reverse the channel → Vec2 mapping.
        let (dx, dy) = match wheel_type {
            WheelType::Lift | WheelType::Offset => {
                // Additive: R = dy, G = -dy, B = dx → dx = B, dy = (R-G)/2
                (channels[2], (channels[0] - channels[1]) / 2.0)
            }
            WheelType::Gamma | WheelType::Gain => {
                // Multiplicative: R = 1+dy, G = 1-dy, B = 1+dx → dx = B-1, dy = (R-G)/2
                (channels[2] - 1.0, (channels[0] - channels[1]) / 2.0)
            }
        };

        // Convert offset (-1..1) to normalized UI position (0..1).
        let norm_x = (dx / 2.0 + 0.5).clamp(0.0, 1.0);
        let norm_y = (dy / 2.0 + 0.5).clamp(0.0, 1.0);

        // First child of the wheel entity is the inner node (with MaterialNode).
        let Some(&inner_ent) = children.first() else {
            continue;
        };

        // Update material cursor uniforms (shader expects -1..1).
        if let Ok(mat_node) = q_material.get(inner_ent)
            && let Some(mat) = materials.get_mut(mat_node.id())
        {
            mat.cursor_x = dx;
            mat.cursor_y = dy;
            mat.master = match wheel_type {
                WheelType::Lift | WheelType::Offset => (channels[3] * 0.5 + 0.5).clamp(0.0, 1.0),
                WheelType::Gamma | WheelType::Gain => (channels[3] * 0.5).clamp(0.0, 1.0),
            };
        }

        // Update thumb position (first child of inner node).
        let Ok(inner_children) = q_children.get(inner_ent) else {
            continue;
        };
        let Some(&thumb_ent) = inner_children.first() else {
            continue;
        };
        if let Ok(mut thumb_node) = q_node.get_mut(thumb_ent) {
            thumb_node.left = Val::Percent(norm_x * 100.0);
            thumb_node.top = Val::Percent(norm_y * 100.0);
        }
    }
}
