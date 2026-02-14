//! Input forwarding — routes Bevy mouse/keyboard events to the CEF backend.

mod hotkeys;
mod keyboard;
mod mouse;

use bevy::prelude::*;

/// Tracks the current mouse position in both window and webview coordinates.
#[derive(Resource, Default)]
pub struct MouseState {
    /// Window-space X (logical pixels).
    pub window_x: f32,
    /// Window-space Y (logical pixels).
    pub window_y: f32,
    /// Last time a mouse move was forwarded to CEF.
    pub last_move_sent: Option<std::time::Instant>,
}

/// Plugin that registers all input-forwarding systems.
pub struct InputForwardingPlugin;

impl Plugin for InputForwardingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MouseState>().add_systems(
            PreUpdate,
            (
                mouse::track_mouse_position,
                mouse::forward_mouse_buttons,
                mouse::forward_mouse_scroll,
                keyboard::forward_keyboard,
                hotkeys::handle_devtools_hotkey,
            ),
        );
    }
}
