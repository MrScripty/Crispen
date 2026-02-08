//! ECS systems that synchronize UI state with GradingParams.
//!
//! Bidirectional sync:
//! - Sliders/wheels → `GradingState.params` (user interaction)
//! - `GradingState.params` → sliders/wheels (external changes like reset)

use bevy::prelude::*;
use bevy::ui_render::prelude::MaterialNode;
use bevy::ui_widgets::{SliderValue, ValueChange};

use crispen_bevy::resources::GradingState;

use super::color_wheel::{ColorWheelMaterial, WheelType};
use super::components::{ParamId, ParamSlider};

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

// ── Sliders → GradingState ──────────────────────────────────────────────────

/// Sync slider value changes to GradingState parameters.
///
/// Only mutates `GradingState` when the slider value actually differs from
/// the current param, preventing feedback loops with `sync_params_to_sliders`.
pub fn sync_sliders_to_params(
    sliders: Query<(&SliderValue, &ParamSlider), Changed<SliderValue>>,
    mut state: ResMut<GradingState>,
) {
    for (value, slider) in sliders.iter() {
        let current = read_param(&state, slider.0);
        if (current - value.0).abs() > f32::EPSILON {
            write_param(&mut state, slider.0, value.0);
            state.dirty = true;
        }
    }
}

// ── Wheels → GradingState ───────────────────────────────────────────────────

/// Observer: sync wheel value changes to GradingState lift/gamma/gain/offset.
///
/// Converts the 0..1 Vec2 position to \[R, G, B, Master\] channel adjustments:
/// - X axis: R-G balance (right = more red, left = more green)
/// - Y axis: B channel (down = more blue, up = less blue)
pub fn on_wheel_value_change(
    event: On<ValueChange<Vec2>>,
    wheels: Query<&WheelType>,
    mut state: ResMut<GradingState>,
) {
    let Ok(wheel_type) = wheels.get(event.source) else {
        return;
    };

    // Map 0..1 UI space to -1..1 offset space.
    let dx = (event.value.x - 0.5) * 2.0;
    let dy = (event.value.y - 0.5) * 2.0;

    let channels = match wheel_type {
        WheelType::Lift | WheelType::Offset => {
            // Additive channels (neutral = 0).
            let master = match wheel_type {
                WheelType::Lift => state.params.lift[3],
                _ => state.params.offset[3],
            };
            [dx, -dx, dy, master]
        }
        WheelType::Gamma | WheelType::Gain => {
            // Multiplicative channels (neutral = 1).
            let master = match wheel_type {
                WheelType::Gamma => state.params.gamma[3],
                _ => state.params.gain[3],
            };
            [1.0 + dx, 1.0 - dx, 1.0 + dy, master]
        }
    };

    match wheel_type {
        WheelType::Lift => state.params.lift = channels,
        WheelType::Gamma => state.params.gamma = channels,
        WheelType::Gain => state.params.gain = channels,
        WheelType::Offset => state.params.offset = channels,
    }
    state.dirty = true;
}

// ── GradingState → Sliders ──────────────────────────────────────────────────

/// Sync GradingState back to slider values when params change externally
/// (e.g. ResetGrade, AutoBalance).
pub fn sync_params_to_sliders(
    state: Res<GradingState>,
    sliders: Query<(Entity, &ParamSlider, &SliderValue)>,
    mut commands: Commands,
) {
    if !state.is_changed() {
        return;
    }
    for (entity, slider, current) in sliders.iter() {
        let target = read_param(&state, slider.0);
        if (current.0 - target).abs() > f32::EPSILON {
            commands.entity(entity).insert(SliderValue(target));
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
            WheelType::Lift => &state.params.lift,
            WheelType::Gamma => &state.params.gamma,
            WheelType::Gain => &state.params.gain,
            WheelType::Offset => &state.params.offset,
        };

        // Reverse the channel → Vec2 mapping.
        let (dx, dy) = match wheel_type {
            WheelType::Lift | WheelType::Offset => {
                // Additive: R = dx, G = -dx, B = dy → dx = (R-G)/2, dy = B
                ((channels[0] - channels[1]) / 2.0, channels[2])
            }
            WheelType::Gamma | WheelType::Gain => {
                // Multiplicative: R = 1+dx, G = 1-dx, B = 1+dy
                ((channels[0] - channels[1]) / 2.0, channels[2] - 1.0)
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
        if let Ok(mat_node) = q_material.get(inner_ent) {
            if let Some(mat) = materials.get_mut(mat_node.id()) {
                mat.cursor_x = dx;
                mat.cursor_y = dy;
            }
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
