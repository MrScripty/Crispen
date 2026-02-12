//! Zoom and pan navigation for the image viewer.
//!
//! Scroll-wheel zooms centered on the cursor, middle-click drag pans,
//! double-click or Home key resets to fit.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::{
    Pickable,
    events::{Cancel, Click, Drag, DragEnd, DragStart, Pointer},
    pointer::PointerButton,
};
use bevy::prelude::*;
use bevy::ui::ComputedUiRenderTargetInfo;
use bevy::window::PrimaryWindow;
use std::time::{Duration, Instant};

// ── Constants ───────────────────────────────────────────────────────────────

const ZOOM_MIN: f32 = 0.1;
const ZOOM_MAX: f32 = 20.0;
/// Multiplicative zoom per scroll-wheel line tick.
const ZOOM_FACTOR: f32 = 1.1;
/// Maximum gap between two primary clicks to treat as double-click reset.
const DOUBLE_CLICK_MAX_GAP: Duration = Duration::from_millis(350);

// ── Resource ────────────────────────────────────────────────────────────────

/// Shared zoom/pan state applied to all viewer image wrappers.
#[derive(Resource)]
pub struct ViewerTransform {
    pub zoom: f32,
    pub pan: Vec2,
    /// Aspect ratio (width / height) of the loaded image. `None` until an
    /// image is loaded; used to letter/pillar-box the viewer content.
    pub image_aspect_ratio: Option<f32>,
    /// Previous pointer position during a middle-button drag.
    drag_prev_pos: Option<Vec2>,
    /// Timestamp of the last primary click on the viewer (for double-click).
    last_click_at: Option<Instant>,
}

impl Default for ViewerTransform {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            image_aspect_ratio: None,
            drag_prev_pos: None,
            last_click_at: None,
        }
    }
}

// ── Components ──────────────────────────────────────────────────────────────

/// Marker on the viewer frame node (the bordered viewport area).
/// Used to query frame size for applying the transform.
#[derive(Component)]
pub struct ViewerFrame;

/// Marker on the absolutely-positioned wrapper node that holds each `ImageNode`.
/// Pointer events bubble here from the image child; zoom/pan is applied to
/// this node's `Node` properties each frame.
#[derive(Component)]
pub struct ViewerImageWrapper;

/// Convenience constant: add to overlay elements (labels) so pointer events
/// pass through to the image wrapper below.
pub const PICKABLE_IGNORE: Pickable = Pickable::IGNORE;

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Test whether `cursor_pos` (logical window coordinates) falls inside a frame.
fn cursor_in_frame(
    cursor_pos: Vec2,
    transform: &UiGlobalTransform,
    node: &ComputedNode,
    node_target: &ComputedUiRenderTargetInfo,
    ui_scale: f32,
) -> bool {
    let local = transform
        .try_inverse()
        .unwrap()
        .transform_point2(cursor_pos * node_target.scale_factor() / ui_scale);
    let half = node.size() / 2.0;
    local.x.abs() <= half.x && local.y.abs() <= half.y
}

/// Convert `cursor_pos` to local coordinates relative to the frame center.
fn cursor_local(
    cursor_pos: Vec2,
    transform: &UiGlobalTransform,
    node_target: &ComputedUiRenderTargetInfo,
    ui_scale: f32,
) -> Vec2 {
    transform
        .try_inverse()
        .unwrap()
        .transform_point2(cursor_pos * node_target.scale_factor() / ui_scale)
}

// ── Observers (pointer interaction) ─────────────────────────────────────────

pub fn on_viewer_drag_start(
    mut ev: On<Pointer<DragStart>>,
    mut state: ResMut<ViewerTransform>,
    wrappers: Query<(), With<ViewerImageWrapper>>,
) {
    if ev.button != PointerButton::Middle {
        return;
    }
    if wrappers.get(ev.entity).is_ok() {
        ev.propagate(false);
        state.drag_prev_pos = Some(ev.pointer_location.position);
    }
}

pub fn on_viewer_drag(
    mut ev: On<Pointer<Drag>>,
    mut state: ResMut<ViewerTransform>,
    wrappers: Query<(), With<ViewerImageWrapper>>,
) {
    if ev.button != PointerButton::Middle {
        return;
    }
    if wrappers.get(ev.entity).is_ok() {
        ev.propagate(false);
        let current = ev.pointer_location.position;
        if let Some(prev) = state.drag_prev_pos {
            state.pan += current - prev;
        }
        state.drag_prev_pos = Some(current);
    }
}

pub fn on_viewer_drag_end(
    mut ev: On<Pointer<DragEnd>>,
    mut state: ResMut<ViewerTransform>,
    wrappers: Query<(), With<ViewerImageWrapper>>,
) {
    if wrappers.get(ev.entity).is_ok() {
        ev.propagate(false);
        state.drag_prev_pos = None;
    }
}

pub fn on_viewer_drag_cancel(
    ev: On<Pointer<Cancel>>,
    mut state: ResMut<ViewerTransform>,
    wrappers: Query<(), With<ViewerImageWrapper>>,
) {
    if wrappers.get(ev.entity).is_ok() {
        state.drag_prev_pos = None;
    }
}

pub fn on_viewer_click(
    ev: On<Pointer<Click>>,
    mut state: ResMut<ViewerTransform>,
    wrappers: Query<(), With<ViewerImageWrapper>>,
) {
    if ev.button != PointerButton::Primary {
        return;
    }
    if wrappers.get(ev.entity).is_ok() {
        let now = Instant::now();
        let is_double = state
            .last_click_at
            .is_some_and(|last| now.duration_since(last) <= DOUBLE_CLICK_MAX_GAP);
        state.last_click_at = Some(now);

        if is_double {
            // Consume the pair so a rapid third click doesn't re-trigger.
            state.last_click_at = None;
            state.zoom = 1.0;
            state.pan = Vec2::ZERO;
        }
    }
}

// ── Systems ─────────────────────────────────────────────────────────────────

/// Zoom on scroll-wheel when the cursor is over a viewer frame.
pub fn handle_viewer_scroll(
    mut scroll: MessageReader<MouseWheel>,
    mut state: ResMut<ViewerTransform>,
    frames: Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            &ComputedUiRenderTargetInfo,
        ),
        With<ViewerFrame>,
    >,
    ui_scale: Res<UiScale>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    // Accumulate scroll across all events this frame.
    let mut total = 0.0_f32;
    for ev in scroll.read() {
        total += match ev.unit {
            MouseScrollUnit::Line => ev.y,
            MouseScrollUnit::Pixel => ev.y / 50.0,
        };
    }
    if total.abs() < f32::EPSILON {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Find the frame the cursor is currently inside.
    let hovered = frames.iter().find(|(node, transform, target)| {
        cursor_in_frame(cursor_pos, transform, node, target, ui_scale.0)
    });
    let Some((_, frame_transform, frame_target)) = hovered else {
        return;
    };

    let old_zoom = state.zoom;
    let factor = ZOOM_FACTOR.powf(total);
    let new_zoom = (old_zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);

    // Cursor-centered zoom: adjust pan so the point under the cursor stays
    // fixed on screen.
    let local = cursor_local(cursor_pos, frame_transform, frame_target, ui_scale.0);
    let zoom_ratio = new_zoom / old_zoom;
    state.pan = local * (1.0 - zoom_ratio) + state.pan * zoom_ratio;
    state.zoom = new_zoom;
}

/// Reset zoom/pan on Home key.
pub fn reset_viewer_transform(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<ViewerTransform>) {
    if keys.just_pressed(KeyCode::Home) {
        state.zoom = 1.0;
        state.pan = Vec2::ZERO;
    }
}

/// Apply `ViewerTransform` to every `ViewerImageWrapper` node.
///
/// The wrapper uses `position_type: Absolute` inside its parent frame.
/// Width/height are set to fit the image within the frame while preserving
/// its aspect ratio, then scaled by the current zoom level. Left/top pixel
/// offsets centre the fitted image and apply the pan.
pub fn apply_viewer_transform(
    state: Res<ViewerTransform>,
    frames: Query<&ComputedNode, With<ViewerFrame>>,
    mut wrappers: Query<(&mut Node, &ChildOf), With<ViewerImageWrapper>>,
) {
    let zoom = state.zoom;
    for (mut node, parent) in &mut wrappers {
        let Ok(frame_node) = frames.get(parent.0) else {
            continue;
        };
        let fs = frame_node.size();
        if fs.x < 1.0 || fs.y < 1.0 {
            continue;
        }

        // Compute the fraction of the frame the image should occupy at zoom 1
        // so that it fits without distortion (letter/pillar-boxing).
        let (w_frac, h_frac) = if let Some(ar) = state.image_aspect_ratio {
            let frame_ar = fs.x / fs.y;
            if ar > frame_ar {
                // Image wider than frame → width-limited.
                (1.0, frame_ar / ar)
            } else {
                // Image taller than frame → height-limited.
                (ar / frame_ar, 1.0)
            }
        } else {
            (1.0, 1.0)
        };

        node.width = Val::Percent(zoom * w_frac * 100.0);
        node.height = Val::Percent(zoom * h_frac * 100.0);
        node.left = Val::Px(fs.x * (1.0 - zoom * w_frac) / 2.0 + state.pan.x);
        node.top = Val::Px(fs.y * (1.0 - zoom * h_frac) / 2.0 + state.pan.y);
    }
}
