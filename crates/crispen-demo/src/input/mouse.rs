//! Mouse event forwarding to CEF.

use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::prelude::*;
use bevy::window::CursorMoved;
use crispen_frontend_core::{CompositeBackend, MouseButton as FcMouseButton, MouseEvent};
use std::time::{Duration, Instant};

use super::MouseState;
use crate::cef_bridge::CefFrontendResource;

const THROTTLE: Duration = Duration::from_millis(16); // ~60 fps

/// Track the cursor and forward move events (throttled).
pub fn track_mouse_position(
    mut state: ResMut<MouseState>,
    mut cursor: MessageReader<CursorMoved>,
    webview: Option<NonSendMut<CefFrontendResource>>,
) {
    let Some(mut wv) = webview else {
        cursor.clear();
        return;
    };

    let mut moved = false;
    for ev in cursor.read() {
        state.window_x = ev.position.x;
        state.window_y = ev.position.y;
        moved = true;
    }
    if !moved {
        return;
    }

    let now = Instant::now();
    if let Some(last) = state.last_move_sent {
        if now.duration_since(last) < THROTTLE {
            return;
        }
    }

    wv.backend.send_mouse_event(MouseEvent::Move {
        x: state.window_x,
        y: state.window_y,
    });
    state.last_move_sent = Some(now);
}

/// Forward mouse button presses/releases.
pub fn forward_mouse_buttons(
    mut events: MessageReader<MouseButtonInput>,
    mouse: Res<MouseState>,
    webview: Option<NonSendMut<CefFrontendResource>>,
) {
    let Some(mut wv) = webview else {
        events.clear();
        return;
    };

    for ev in events.read() {
        let Some(btn) = convert_button(ev.button) else { continue };
        let me = if ev.state.is_pressed() {
            MouseEvent::ButtonDown { button: btn, x: mouse.window_x, y: mouse.window_y }
        } else {
            MouseEvent::ButtonUp { button: btn, x: mouse.window_x, y: mouse.window_y }
        };
        wv.backend.send_mouse_event(me);
    }
}

/// Forward scroll wheel events.
pub fn forward_mouse_scroll(
    mut events: MessageReader<MouseWheel>,
    mouse: Res<MouseState>,
    webview: Option<NonSendMut<CefFrontendResource>>,
) {
    let Some(mut wv) = webview else {
        events.clear();
        return;
    };

    for ev in events.read() {
        let (dx, dy) = match ev.unit {
            bevy::input::mouse::MouseScrollUnit::Line => (ev.x * 40.0, ev.y * 40.0),
            bevy::input::mouse::MouseScrollUnit::Pixel => (ev.x, ev.y),
        };
        wv.backend.send_mouse_event(MouseEvent::Scroll {
            delta_x: dx,
            delta_y: -dy, // invert Y for web conventions
            x: mouse.window_x,
            y: mouse.window_y,
        });
    }
}

fn convert_button(b: bevy::input::mouse::MouseButton) -> Option<FcMouseButton> {
    match b {
        bevy::input::mouse::MouseButton::Left => Some(FcMouseButton::Left),
        bevy::input::mouse::MouseButton::Right => Some(FcMouseButton::Right),
        bevy::input::mouse::MouseButton::Middle => Some(FcMouseButton::Middle),
        _ => None,
    }
}
