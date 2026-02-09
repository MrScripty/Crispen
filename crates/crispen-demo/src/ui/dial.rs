//! Rotary dial / knob widget for scalar parameter controls.
//!
//! Renders a DaVinci Resolve-style dial using a custom UiMaterial shader.
//! Vertical drag interaction: drag up to increase, drag down to decrease.

use bevy::asset::embedded_asset;
use bevy::picking::{
    Pickable,
    events::{Cancel, Drag, DragEnd, DragStart, Pointer, Press},
};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui::{InteractionDisabled, UiTransform, Val2};
use bevy::ui_render::prelude::{MaterialNode, UiMaterial, UiMaterialPlugin};

use super::components::ParamId;
use super::theme;

// ── Constants ───────────────────────────────────────────────────────────────

/// Pixels of vertical drag for a full min→max sweep.
const DRAG_PIXELS_FULL_RANGE: f32 = 200.0;

// ── Components ──────────────────────────────────────────────────────────────

/// Current dial value.
#[derive(Component, Debug, Clone, Copy)]
pub struct DialValue(pub f32);

/// Value range (min, max).
#[derive(Component, Debug, Clone, Copy)]
pub struct DialRange {
    pub min: f32,
    pub max: f32,
}

/// Step increment for snapping.
#[derive(Component, Debug, Clone, Copy)]
pub struct DialStep(pub f32);

/// Links a dial to a grading parameter.
#[derive(Component, Debug, Clone, Copy)]
pub struct ParamDial(pub ParamId);

/// Whether the dial label is rendered above or below the knob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialLabelPosition {
    Above,
    Below,
}

/// Marker on the inner node that receives the `MaterialNode`.
#[derive(Component, Default)]
pub struct DialInner;

/// Text entity displaying the dial's numeric value, linked to the dial entity.
#[derive(Component)]
pub struct DialValueLabel(pub Entity);

/// Tracks drag state for vertical-drag interaction.
#[derive(Component, Default)]
struct DialDragState {
    active: bool,
    start_y: f32,
    start_value: f32,
}

// ── Material ────────────────────────────────────────────────────────────────

/// UiMaterial driving the dial knob fragment shader.
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct DialMaterial {
    /// Normalized value 0..1 mapped to the 270° arc sweep.
    #[uniform(0)]
    pub value_norm: f32,
    /// 1.0 when the user is actively dragging.
    #[uniform(0)]
    pub is_active: f32,
}

impl Default for DialMaterial {
    fn default() -> Self {
        Self {
            value_norm: 0.5,
            is_active: 0.0,
        }
    }
}

impl UiMaterial for DialMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://crispen_demo/ui/shaders/dial.wgsl".into()
    }
}

// ── Spawn ───────────────────────────────────────────────────────────────────

/// Spawn a labeled dial with a numeric readout.
///
/// Layout (vertical):
/// ```text
/// ┌──────────┐
/// │  LABEL   │
/// │   (O)    │  ← dial circle
/// │  0.00    │
/// └──────────┘
/// ```
pub fn spawn_param_dial(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    param_id: ParamId,
    range: (f32, f32),
    default_val: f32,
    step: f32,
    label_position: DialLabelPosition,
) {
    parent
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(2.0),
            width: Val::Px(theme::DIAL_SLOT_WIDTH),
            ..default()
        })
        .with_children(|col| {
            if label_position == DialLabelPosition::Above {
                col.spawn((
                    Text::new(label),
                    TextFont {
                        font_size: theme::FONT_SIZE_LABEL,
                        ..default()
                    },
                    TextColor(theme::TEXT_DIM),
                ));
            }

            // Dial container — captures pointer events and holds drag state.
            let dial_id = col
                .spawn((
                    Node {
                        width: Val::Px(theme::DIAL_SIZE),
                        height: Val::Px(theme::DIAL_SIZE),
                        ..Default::default()
                    },
                    DialInner,
                    DialValue(default_val),
                    DialRange {
                        min: range.0,
                        max: range.1,
                    },
                    DialStep(step),
                    ParamDial(param_id),
                    DialDragState {
                        active: false,
                        start_y: 0.0,
                        start_value: default_val,
                    },
                ))
                .id();

            // Value text centered inside the dial.
            let mut value_id = None;
            col.commands().entity(dial_id).with_children(|dial| {
                value_id = Some(
                    dial.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(50.0),
                            top: Val::Percent(50.0),
                            ..Default::default()
                        },
                        Text::new(format!("{default_val:.2}")),
                        TextFont {
                            font_size: theme::FONT_SIZE_VALUE,
                            ..default()
                        },
                        TextColor(theme::TEXT_PRIMARY),
                        Pickable::IGNORE,
                        UiTransform::from_translation(Val2::new(
                            Val::Percent(-50.0),
                            Val::Percent(-50.0),
                        )),
                    ))
                    .id(),
                );
            });
            let value_id = value_id.expect("dial value text child should be spawned");

            col.commands()
                .entity(value_id)
                .insert(DialValueLabel(dial_id));

            if label_position == DialLabelPosition::Below {
                col.spawn((
                    Text::new(label),
                    TextFont {
                        font_size: theme::FONT_SIZE_LABEL,
                        ..default()
                    },
                    TextColor(theme::TEXT_DIM),
                ));
            }
        });
}

// ── Helpers ─────────────────────────────────────────────────────────────────

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

// ── Observers (pointer interaction) ─────────────────────────────────────────

fn on_dial_press(
    mut press: On<Pointer<Press>>,
    mut q_dials: Query<(&mut DialDragState, &DialValue, Has<InteractionDisabled>), With<DialInner>>,
) {
    if let Ok((mut state, value, disabled)) = q_dials.get_mut(press.entity) {
        press.propagate(false);
        if !disabled {
            state.start_y = press.pointer_location.position.y;
            state.start_value = value.0;
        }
    }
}

fn on_dial_drag_start(
    mut drag_start: On<Pointer<DragStart>>,
    mut q_dials: Query<(&mut DialDragState, &DialValue, Has<InteractionDisabled>), With<DialInner>>,
) {
    if let Ok((mut state, value, disabled)) = q_dials.get_mut(drag_start.entity) {
        drag_start.propagate(false);
        if !disabled {
            state.active = true;
            state.start_y = drag_start.pointer_location.position.y;
            state.start_value = value.0;
        }
    }
}

fn on_dial_drag(
    mut drag: On<Pointer<Drag>>,
    mut q_dials: Query<
        (&mut DialDragState, &mut DialValue, &DialRange, &DialStep),
        With<DialInner>,
    >,
    mut materials: ResMut<Assets<DialMaterial>>,
    q_material: Query<&MaterialNode<DialMaterial>>,
) {
    if let Ok((state, mut value, range, step)) = q_dials.get_mut(drag.entity) {
        drag.propagate(false);
        if !state.active {
            return;
        }
        let delta_y = state.start_y - drag.pointer_location.position.y;
        let sensitivity = (range.max - range.min) / DRAG_PIXELS_FULL_RANGE;
        let raw = state.start_value + delta_y * sensitivity;
        let snapped = snap_to_step(raw, step.0, range.min).clamp(range.min, range.max);

        if (value.0 - snapped).abs() > f32::EPSILON {
            value.0 = snapped;
        }

        // Update material uniform immediately for visual feedback.
        if let Ok(mat_node) = q_material.get(drag.entity)
            && let Some(mat) = materials.get_mut(mat_node.id())
        {
            mat.value_norm = normalize_value(snapped, range.min, range.max);
            mat.is_active = 1.0;
        }
    }
}

fn on_dial_drag_end(
    mut drag_end: On<Pointer<DragEnd>>,
    mut q_dials: Query<&mut DialDragState, With<DialInner>>,
    mut materials: ResMut<Assets<DialMaterial>>,
    q_material: Query<&MaterialNode<DialMaterial>>,
) {
    if let Ok(mut state) = q_dials.get_mut(drag_end.entity) {
        drag_end.propagate(false);
        state.active = false;

        if let Ok(mat_node) = q_material.get(drag_end.entity)
            && let Some(mat) = materials.get_mut(mat_node.id())
        {
            mat.is_active = 0.0;
        }
    }
}

fn on_dial_drag_cancel(
    drag_cancel: On<Pointer<Cancel>>,
    mut q_dials: Query<&mut DialDragState, With<DialInner>>,
    mut materials: ResMut<Assets<DialMaterial>>,
    q_material: Query<&MaterialNode<DialMaterial>>,
) {
    if let Ok(mut state) = q_dials.get_mut(drag_cancel.entity) {
        state.active = false;

        if let Ok(mat_node) = q_material.get(drag_cancel.entity)
            && let Some(mat) = materials.get_mut(mat_node.id())
        {
            mat.is_active = 0.0;
        }
    }
}

// ── Systems ─────────────────────────────────────────────────────────────────

/// Lazily insert `MaterialNode<DialMaterial>` on dial inner nodes.
fn update_dial_material(
    q_dials: Query<(Entity, &DialValue, &DialRange), With<DialInner>>,
    q_material_node: Query<&MaterialNode<DialMaterial>>,
    mut materials: ResMut<Assets<DialMaterial>>,
    mut commands: Commands,
) {
    for (entity, value, range) in q_dials.iter() {
        if q_material_node.get(entity).is_err() {
            let norm = normalize_value(value.0, range.min, range.max);
            let handle = materials.add(DialMaterial {
                value_norm: norm,
                is_active: 0.0,
            });
            commands.entity(entity).insert(MaterialNode(handle));
        }
    }
}

/// Update value labels when `DialValue` changes.
pub fn update_dial_visuals(
    dials: Query<(Entity, &DialValue, &DialRange), Changed<DialValue>>,
    mut labels: Query<(&DialValueLabel, &mut Text)>,
    q_material: Query<&MaterialNode<DialMaterial>>,
    mut materials: ResMut<Assets<DialMaterial>>,
) {
    for (dial_ent, value, range) in dials.iter() {
        // Update material.
        if let Ok(mat_node) = q_material.get(dial_ent)
            && let Some(mat) = materials.get_mut(mat_node.id())
        {
            mat.value_norm = normalize_value(value.0, range.min, range.max);
        }
    }

    // Update text labels.
    for (label, mut text) in labels.iter_mut() {
        if let Ok((_, value, _)) = dials.get(label.0) {
            **text = format!("{:.2}", value.0);
        }
    }
}

// ── Plugin ──────────────────────────────────────────────────────────────────

/// Registers the dial UiMaterial, observers, and update systems.
pub struct DialPlugin;

impl Plugin for DialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/dial.wgsl");
        app.add_plugins(UiMaterialPlugin::<DialMaterial>::default());
        app.add_systems(PostUpdate, update_dial_material);
        app.add_observer(on_dial_press)
            .add_observer(on_dial_drag_start)
            .add_observer(on_dial_drag)
            .add_observer(on_dial_drag_end)
            .add_observer(on_dial_drag_cancel);
    }
}
