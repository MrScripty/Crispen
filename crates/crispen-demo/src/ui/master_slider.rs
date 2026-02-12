//! Horizontal master-level slider below each primary color wheel.
//!
//! Controls the master (luminance) channel — index `[3]` of the
//! lift / gamma / gain / offset arrays in `GradingParams`.
//! Horizontal drag interaction: drag right to increase, left to decrease.
//! Double-click resets to the identity value.

use bevy::asset::embedded_asset;
use bevy::picking::events::{Cancel, Click, Drag, DragEnd, DragStart, Pointer, Press};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui::InteractionDisabled;
use bevy::ui_render::prelude::{MaterialNode, UiMaterial, UiMaterialPlugin};
use std::time::{Duration, Instant};

use super::color_wheel::WheelType;
use super::theme;

// ── Constants ───────────────────────────────────────────────────────────────

/// Pixels of horizontal drag for a full min→max sweep.
const DRAG_PIXELS_FULL_RANGE: f32 = 300.0;

/// Max time between two primary clicks to treat as a double-click reset.
const DOUBLE_CLICK_MAX_GAP: Duration = Duration::from_millis(350);

// ── Components ──────────────────────────────────────────────────────────────

/// Links a master slider to its wheel type.
#[derive(Component, Debug, Clone, Copy)]
pub struct MasterSliderWheel(pub WheelType);

/// Current master slider value.
#[derive(Component, Debug, Clone, Copy)]
pub struct MasterSliderValue(pub f32);

/// Value range (min, max).
#[derive(Component, Debug, Clone, Copy)]
pub struct MasterSliderRange {
    pub min: f32,
    pub max: f32,
}

/// Default value for double-click reset.
#[derive(Component, Debug, Clone, Copy)]
struct MasterSliderDefault(f32);

/// Marker for the master slider node.
#[derive(Component, Default)]
pub struct MasterSliderInner;

/// Tracks drag state for horizontal-drag interaction.
#[derive(Component, Default)]
struct MasterSliderDragState {
    active: bool,
    start_x: f32,
    start_value: f32,
}

/// Double-click detection state.
#[derive(Component, Default)]
struct MasterSliderClickState {
    last_primary_click_at: Option<Instant>,
}

// ── Material ────────────────────────────────────────────────────────────────

/// UiMaterial driving the master slider fragment shader.
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct MasterSliderMaterial {
    /// Normalized value 0..1 mapped to horizontal position.
    #[uniform(0)]
    pub value_norm: f32,
    /// Normalized center / default position 0..1.
    #[uniform(0)]
    pub center_norm: f32,
    /// 1.0 when the user is actively dragging.
    #[uniform(0)]
    pub is_active: f32,
}

impl Default for MasterSliderMaterial {
    fn default() -> Self {
        Self {
            value_norm: 0.5,
            center_norm: 0.5,
            is_active: 0.0,
        }
    }
}

impl UiMaterial for MasterSliderMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://crispen_demo/ui/shaders/master_slider.wgsl".into()
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Return `(min, max, default)` for the master channel of a given wheel type.
pub fn master_params(wheel: WheelType) -> (f32, f32, f32) {
    match wheel {
        WheelType::Lift => (-1.0, 1.0, 0.0),
        WheelType::Gamma => (0.0, 4.0, 1.0),
        WheelType::Gain => (0.0, 4.0, 1.0),
        WheelType::Offset => (-1.0, 1.0, 0.0),
    }
}

fn normalize_value(value: f32, min: f32, max: f32) -> f32 {
    if (max - min).abs() < f32::EPSILON {
        return 0.5;
    }
    ((value - min) / (max - min)).clamp(0.0, 1.0)
}

fn snap_to_step(value: f32, step: f32, min: f32) -> f32 {
    if step <= f32::EPSILON {
        return value;
    }
    min + ((value - min) / step).round() * step
}

// ── Bundle ──────────────────────────────────────────────────────────────────

/// Spawn a horizontal master-level slider for the given wheel type.
pub fn master_slider(wheel_type: WheelType) -> impl Bundle {
    let (min, max, default) = master_params(wheel_type);
    (
        Node {
            width: Val::Px(theme::WHEEL_SIZE),
            height: Val::Px(theme::MASTER_SLIDER_HEIGHT),
            ..Default::default()
        },
        MasterSliderInner,
        MasterSliderWheel(wheel_type),
        MasterSliderValue(default),
        MasterSliderRange { min, max },
        MasterSliderDefault(default),
        MasterSliderDragState {
            active: false,
            start_x: 0.0,
            start_value: default,
        },
        MasterSliderClickState::default(),
    )
}

// ── Observers (pointer interaction) ─────────────────────────────────────────

fn on_slider_press(
    mut press: On<Pointer<Press>>,
    mut q_sliders: Query<
        (
            &mut MasterSliderDragState,
            &MasterSliderValue,
            Has<InteractionDisabled>,
        ),
        With<MasterSliderInner>,
    >,
) {
    if let Ok((mut drag, value, disabled)) = q_sliders.get_mut(press.entity) {
        press.propagate(false);
        if !disabled {
            drag.start_x = press.pointer_location.position.x;
            drag.start_value = value.0;
        }
    }
}

fn on_slider_drag_start(
    mut drag_start: On<Pointer<DragStart>>,
    mut q_sliders: Query<
        (
            &mut MasterSliderDragState,
            &MasterSliderValue,
            Has<InteractionDisabled>,
        ),
        With<MasterSliderInner>,
    >,
    q_material: Query<&MaterialNode<MasterSliderMaterial>>,
    mut materials: ResMut<Assets<MasterSliderMaterial>>,
) {
    if let Ok((mut drag, value, disabled)) = q_sliders.get_mut(drag_start.entity) {
        drag_start.propagate(false);
        if !disabled {
            drag.active = true;
            drag.start_x = drag_start.pointer_location.position.x;
            drag.start_value = value.0;

            if let Ok(mat_node) = q_material.get(drag_start.entity)
                && let Some(mat) = materials.get_mut(mat_node.id())
            {
                mat.is_active = 1.0;
            }
        }
    }
}

fn on_slider_drag(
    mut drag: On<Pointer<Drag>>,
    mut q_sliders: Query<
        (
            &MasterSliderDragState,
            &mut MasterSliderValue,
            &MasterSliderRange,
        ),
        With<MasterSliderInner>,
    >,
    q_material: Query<&MaterialNode<MasterSliderMaterial>>,
    mut materials: ResMut<Assets<MasterSliderMaterial>>,
) {
    if let Ok((state, mut value, range)) = q_sliders.get_mut(drag.entity) {
        drag.propagate(false);
        if !state.active {
            return;
        }
        let delta_x = drag.pointer_location.position.x - state.start_x;
        let sensitivity = (range.max - range.min) / DRAG_PIXELS_FULL_RANGE;
        let raw = state.start_value + delta_x * sensitivity;
        let step = (range.max - range.min) / 400.0;
        let snapped = snap_to_step(raw, step, range.min).clamp(range.min, range.max);

        if (value.0 - snapped).abs() > f32::EPSILON {
            value.0 = snapped;
        }

        if let Ok(mat_node) = q_material.get(drag.entity)
            && let Some(mat) = materials.get_mut(mat_node.id())
        {
            mat.value_norm = normalize_value(snapped, range.min, range.max);
            mat.is_active = 1.0;
        }
    }
}

fn on_slider_drag_end(
    mut drag_end: On<Pointer<DragEnd>>,
    mut q_sliders: Query<&mut MasterSliderDragState, With<MasterSliderInner>>,
    q_material: Query<&MaterialNode<MasterSliderMaterial>>,
    mut materials: ResMut<Assets<MasterSliderMaterial>>,
) {
    if let Ok(mut state) = q_sliders.get_mut(drag_end.entity) {
        drag_end.propagate(false);
        state.active = false;

        if let Ok(mat_node) = q_material.get(drag_end.entity)
            && let Some(mat) = materials.get_mut(mat_node.id())
        {
            mat.is_active = 0.0;
        }
    }
}

fn on_slider_drag_cancel(
    drag_cancel: On<Pointer<Cancel>>,
    mut q_sliders: Query<&mut MasterSliderDragState, With<MasterSliderInner>>,
    q_material: Query<&MaterialNode<MasterSliderMaterial>>,
    mut materials: ResMut<Assets<MasterSliderMaterial>>,
) {
    if let Ok(mut state) = q_sliders.get_mut(drag_cancel.entity) {
        state.active = false;

        if let Ok(mat_node) = q_material.get(drag_cancel.entity)
            && let Some(mat) = materials.get_mut(mat_node.id())
        {
            mat.is_active = 0.0;
        }
    }
}

#[allow(clippy::type_complexity)]
fn on_slider_click(
    mut click: On<Pointer<Click>>,
    mut q_sliders: Query<
        (
            &mut MasterSliderValue,
            &MasterSliderDefault,
            &MasterSliderRange,
            &mut MasterSliderClickState,
            Has<InteractionDisabled>,
        ),
        With<MasterSliderInner>,
    >,
    q_material: Query<&MaterialNode<MasterSliderMaterial>>,
    mut materials: ResMut<Assets<MasterSliderMaterial>>,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    if let Ok((mut value, default_value, range, mut click_state, disabled)) =
        q_sliders.get_mut(click.entity)
    {
        click.propagate(false);
        if disabled {
            return;
        }

        let now = Instant::now();
        let is_double_click = click_state
            .last_primary_click_at
            .is_some_and(|last| now.duration_since(last) <= DOUBLE_CLICK_MAX_GAP);
        click_state.last_primary_click_at = Some(now);

        if !is_double_click {
            return;
        }

        // Consume this pair so a rapid third click doesn't immediately re-trigger.
        click_state.last_primary_click_at = None;

        if (value.0 - default_value.0).abs() <= f32::EPSILON {
            return;
        }

        value.0 = default_value.0;

        if let Ok(mat_node) = q_material.get(click.entity)
            && let Some(mat) = materials.get_mut(mat_node.id())
        {
            mat.value_norm = normalize_value(default_value.0, range.min, range.max);
        }
    }
}

// ── Systems ─────────────────────────────────────────────────────────────────

/// Lazily insert `MaterialNode<MasterSliderMaterial>` on slider nodes.
fn update_slider_material(
    q_sliders: Query<
        (
            Entity,
            &MasterSliderValue,
            &MasterSliderRange,
            &MasterSliderWheel,
        ),
        With<MasterSliderInner>,
    >,
    q_material_node: Query<&MaterialNode<MasterSliderMaterial>>,
    mut materials: ResMut<Assets<MasterSliderMaterial>>,
    mut commands: Commands,
) {
    for (entity, value, range, wheel) in q_sliders.iter() {
        if q_material_node.get(entity).is_err() {
            let (_, _, default) = master_params(wheel.0);
            let norm = normalize_value(value.0, range.min, range.max);
            let center = normalize_value(default, range.min, range.max);
            let handle = materials.add(MasterSliderMaterial {
                value_norm: norm,
                center_norm: center,
                is_active: 0.0,
            });
            commands.entity(entity).insert(MaterialNode(handle));
        }
    }
}

// ── Plugin ──────────────────────────────────────────────────────────────────

/// Registers the master slider UiMaterial, observers, and update systems.
pub struct MasterSliderPlugin;

impl Plugin for MasterSliderPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/master_slider.wgsl");
        app.add_plugins(UiMaterialPlugin::<MasterSliderMaterial>::default());
        app.add_systems(PostUpdate, update_slider_material);
        app.add_observer(on_slider_press)
            .add_observer(on_slider_click)
            .add_observer(on_slider_drag_start)
            .add_observer(on_slider_drag)
            .add_observer(on_slider_drag_end)
            .add_observer(on_slider_drag_cancel);
    }
}
