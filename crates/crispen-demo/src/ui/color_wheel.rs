//! Color wheel widget (Lift / Gamma / Gain / Offset).
//!
//! Renders a DaVinci Resolve-style color wheel using a custom UiMaterial
//! shader with pointer-based drag interaction. Follows the bevy_feathers
//! `ColorPlane` pattern for observers, coordinate conversion, and
//! `ValueChange<Vec2>` emission.

use bevy::asset::embedded_asset;
use bevy::picking::{
    Pickable,
    events::{Cancel, Drag, DragEnd, DragStart, Pointer, Press},
};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui::{
    ComputedNode, ComputedUiRenderTargetInfo, InteractionDisabled, UiGlobalTransform, UiScale,
    UiTransform, Val2,
};
use bevy::ui_render::prelude::{MaterialNode, UiMaterial, UiMaterialPlugin};
use bevy::ui_widgets::ValueChange;

use super::theme;

// ── Constants ───────────────────────────────────────────────────────────────

const THUMB_SIZE: f32 = 10.0;

// ── Components ──────────────────────────────────────────────────────────────

/// Identifies which grading channel a color wheel controls.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WheelType {
    Lift,
    Gamma,
    Gain,
    Offset,
}

impl WheelType {
    /// Human-readable label for display below the wheel.
    pub fn label(self) -> &'static str {
        match self {
            Self::Lift => "LIFT",
            Self::Gamma => "GAMMA",
            Self::Gain => "GAIN",
            Self::Offset => "OFFSET",
        }
    }
}

/// Tracks whether the user is actively dragging within this wheel.
#[derive(Component, Default)]
struct ColorWheelDragState(bool);

/// Marker for the inner node that receives the `MaterialNode`.
#[derive(Component, Default)]
struct ColorWheelInner;

/// Marker for the positioned thumb dot.
#[derive(Component, Default)]
struct ColorWheelThumb;

// ── Material ────────────────────────────────────────────────────────────────

/// UiMaterial driving the color wheel fragment shader.
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct ColorWheelMaterial {
    /// Cursor X position, normalized to -1..1 from center.
    #[uniform(0)]
    pub cursor_x: f32,
    /// Cursor Y position, normalized to -1..1 from center.
    #[uniform(0)]
    pub cursor_y: f32,
    /// Master channel brightness overlay (0..1 typical).
    #[uniform(0)]
    pub master: f32,
}

impl Default for ColorWheelMaterial {
    fn default() -> Self {
        Self {
            cursor_x: 0.0,
            cursor_y: 0.0,
            master: 0.5,
        }
    }
}

impl UiMaterial for ColorWheelMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://crispen_demo/ui/shaders/color_wheel.wgsl".into()
    }
}

// ── Bundle ──────────────────────────────────────────────────────────────────

/// Spawn a color wheel widget for the given channel.
///
/// Emits [`ValueChange<Vec2>`] with values in 0..1 (center = 0.5, 0.5).
/// The outer container is sized to [`theme::WHEEL_SIZE`].
pub fn color_wheel(wheel_type: WheelType) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            width: Val::Px(theme::WHEEL_SIZE),
            height: Val::Px(theme::WHEEL_SIZE + 24.0),
            ..Default::default()
        },
        wheel_type,
        ColorWheelDragState::default(),
        children![
            // Inner node: receives MaterialNode and handles picking.
            (
                Node {
                    width: Val::Px(theme::WHEEL_SIZE),
                    height: Val::Px(theme::WHEEL_SIZE),
                    ..Default::default()
                },
                ColorWheelInner,
                children![
                    // Thumb dot: positioned absolutely, ignores picking.
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(50.0),
                            top: Val::Percent(50.0),
                            width: Val::Px(THUMB_SIZE),
                            height: Val::Px(THUMB_SIZE),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::MAX,
                            ..Default::default()
                        },
                        ColorWheelThumb,
                        BorderColor::all(Color::WHITE),
                        Outline {
                            width: Val::Px(1.0),
                            offset: Val::Px(0.0),
                            color: Color::BLACK,
                        },
                        Pickable::IGNORE,
                        UiTransform::from_translation(Val2::new(
                            Val::Percent(-50.0),
                            Val::Percent(-50.0),
                        )),
                    )
                ],
            ),
            // Label text below the wheel.
            (
                Text::new(wheel_type.label()),
                TextFont {
                    font_size: theme::FONT_SIZE_LABEL,
                    ..Default::default()
                },
                TextColor(theme::TEXT_DIM),
            ),
        ],
    )
}

// ── Observers (pointer interaction) ─────────────────────────────────────────

/// Convert a pointer position to a normalized 0..1 value within the inner node.
fn pointer_to_normalized(
    pointer_pos: Vec2,
    node: &ComputedNode,
    node_target: &ComputedUiRenderTargetInfo,
    transform: &UiGlobalTransform,
    ui_scale: f32,
) -> Vec2 {
    let local_pos = transform
        .try_inverse()
        .unwrap()
        .transform_point2(pointer_pos * node_target.scale_factor() / ui_scale);
    let pos = local_pos / node.size() + Vec2::splat(0.5);
    pos.clamp(Vec2::ZERO, Vec2::ONE)
}

fn on_pointer_press(
    mut press: On<Pointer<Press>>,
    q_wheels: Query<Has<InteractionDisabled>, With<WheelType>>,
    q_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<ColorWheelInner>,
    >,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    if let Ok((node, node_target, transform, parent)) = q_inner.get(press.entity)
        && let Ok(disabled) = q_wheels.get(parent.0)
    {
        press.propagate(false);
        if !disabled {
            let new_value = pointer_to_normalized(
                press.pointer_location.position,
                node,
                node_target,
                transform,
                ui_scale.0,
            );
            commands.trigger(ValueChange {
                source: parent.0,
                value: new_value,
            });
        }
    }
}

fn on_drag_start(
    mut drag_start: On<Pointer<DragStart>>,
    mut q_wheels: Query<(&mut ColorWheelDragState, Has<InteractionDisabled>), With<WheelType>>,
    q_inner: Query<&ChildOf, With<ColorWheelInner>>,
) {
    if let Ok(parent) = q_inner.get(drag_start.entity)
        && let Ok((mut state, disabled)) = q_wheels.get_mut(parent.0)
    {
        drag_start.propagate(false);
        if !disabled {
            state.0 = true;
        }
    }
}

fn on_drag(
    mut drag: On<Pointer<Drag>>,
    q_wheels: Query<(&ColorWheelDragState, Has<InteractionDisabled>), With<WheelType>>,
    q_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<ColorWheelInner>,
    >,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    if let Ok((node, node_target, transform, parent)) = q_inner.get(drag.entity)
        && let Ok((state, disabled)) = q_wheels.get(parent.0)
    {
        drag.propagate(false);
        if state.0 && !disabled {
            let new_value = pointer_to_normalized(
                drag.pointer_location.position,
                node,
                node_target,
                transform,
                ui_scale.0,
            );
            commands.trigger(ValueChange {
                source: parent.0,
                value: new_value,
            });
        }
    }
}

fn on_drag_end(
    mut drag_end: On<Pointer<DragEnd>>,
    mut q_wheels: Query<&mut ColorWheelDragState, With<WheelType>>,
    q_inner: Query<&ChildOf, With<ColorWheelInner>>,
) {
    if let Ok(parent) = q_inner.get(drag_end.entity)
        && let Ok(mut state) = q_wheels.get_mut(parent.0)
    {
        drag_end.propagate(false);
        state.0 = false;
    }
}

fn on_drag_cancel(
    drag_cancel: On<Pointer<Cancel>>,
    mut q_wheels: Query<&mut ColorWheelDragState, With<WheelType>>,
    q_inner: Query<&ChildOf, With<ColorWheelInner>>,
) {
    if let Ok(parent) = q_inner.get(drag_cancel.entity)
        && let Ok(mut state) = q_wheels.get_mut(parent.0)
    {
        state.0 = false;
    }
}

// ── Systems ─────────────────────────────────────────────────────────────────

/// Ensures each wheel's inner node has a `MaterialNode<ColorWheelMaterial>`,
/// and keeps cursor/master uniforms in sync with external param changes.
/// Also updates the thumb position.
fn update_wheel_material(
    q_wheels: Query<Entity, With<WheelType>>,
    q_children: Query<&Children>,
    q_material_node: Query<&MaterialNode<ColorWheelMaterial>>,
    mut q_node: Query<&mut Node>,
    mut materials: ResMut<Assets<ColorWheelMaterial>>,
    mut commands: Commands,
) {
    for wheel_ent in q_wheels.iter() {
        let Ok(children) = q_children.get(wheel_ent) else {
            continue;
        };
        // First child is the inner node.
        let Some(&inner_ent) = children.first() else {
            continue;
        };

        if q_material_node.get(inner_ent).is_err() {
            // First time: insert the material and center the thumb.
            let handle = materials.add(ColorWheelMaterial::default());
            commands.entity(inner_ent).insert(MaterialNode(handle));

            let Ok(inner_children) = q_children.get(inner_ent) else {
                continue;
            };
            let Some(&thumb_ent) = inner_children.first() else {
                continue;
            };
            let Ok(mut thumb_node) = q_node.get_mut(thumb_ent) else {
                continue;
            };
            thumb_node.left = Val::Percent(50.0);
            thumb_node.top = Val::Percent(50.0);
        }
    }
}

/// Updates the thumb position when a `ValueChange<Vec2>` is received
/// for a wheel entity. Value is in 0..1 space.
fn update_wheel_thumb(
    event: On<ValueChange<Vec2>>,
    q_wheels: Query<Entity, With<WheelType>>,
    q_children: Query<&Children>,
    mut q_node: Query<&mut Node>,
) {
    let Ok(wheel_ent) = q_wheels.get(event.source) else {
        return;
    };
    let Ok(children) = q_children.get(wheel_ent) else {
        return;
    };
    let Some(&inner_ent) = children.first() else {
        return;
    };
    let Ok(inner_children) = q_children.get(inner_ent) else {
        return;
    };
    let Some(&thumb_ent) = inner_children.first() else {
        return;
    };
    let Ok(mut thumb_node) = q_node.get_mut(thumb_ent) else {
        return;
    };
    thumb_node.left = Val::Percent(event.value.x * 100.0);
    thumb_node.top = Val::Percent(event.value.y * 100.0);
}

// ── Plugin ──────────────────────────────────────────────────────────────────

/// Registers the color wheel UiMaterial, observers, and update systems.
pub struct ColorWheelPlugin;

impl Plugin for ColorWheelPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/color_wheel.wgsl");
        app.add_plugins(UiMaterialPlugin::<ColorWheelMaterial>::default());
        app.add_systems(PostUpdate, update_wheel_material);
        app.add_observer(on_pointer_press)
            .add_observer(on_drag_start)
            .add_observer(on_drag)
            .add_observer(on_drag_end)
            .add_observer(on_drag_cancel)
            .add_observer(update_wheel_thumb);
    }
}
