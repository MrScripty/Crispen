//! Frontend core abstractions for Crispen.
//!
//! Defines the [`CompositeBackend`] trait that abstracts over different UI
//! rendering backends (CEF, WebKitGTK, etc.).  Input event types and capture
//! results are also defined here to avoid coupling the backend crate to
//! domain-specific IPC types.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

// ── Capture ──────────────────────────────────────────────────────

/// Result of capturing the UI framebuffer.
#[derive(Debug, Clone)]
pub enum CaptureResult {
    /// RGBA pixel data (owned) with dimensions.
    Rgba(Vec<u8>, u32, u32),
    /// BGRA pixel data (shared via `Arc`) with dimensions.
    Bgra(Arc<Vec<u8>>, u32, u32),
}

// ── Errors ───────────────────────────────────────────────────────

/// Errors that can occur in frontend operations.
#[derive(Debug, thiserror::Error)]
pub enum FrontendError {
    #[error("failed to send message to UI: {0}")]
    SendFailed(String),

    #[error("backend is not ready")]
    NotReady,

    #[error("backend error: {0}")]
    Backend(String),
}

// ── Input events ─────────────────────────────────────────────────

/// Mouse event forwarded from Bevy to the webview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseEvent {
    Move { x: f32, y: f32 },
    ButtonDown { button: MouseButton, x: f32, y: f32 },
    ButtonUp { button: MouseButton, x: f32, y: f32 },
    Scroll { delta_x: f32, delta_y: f32, x: f32, y: f32 },
}

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Keyboard input event forwarded from Bevy to the webview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardEvent {
    pub key: String,
    pub pressed: bool,
    pub modifiers: Modifiers,
}

/// Keyboard modifier keys state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

// ── Backend trait ────────────────────────────────────────────────

/// Trait for UI rendering backends that can be composited into the Bevy scene.
///
/// IPC payloads are exchanged as raw JSON strings so that this crate does not
/// depend on domain-specific types (e.g. `GradingParams`).
pub trait CompositeBackend {
    /// Poll the backend for events and updates.  Call once per frame.
    fn poll(&mut self);

    /// Whether the backend is ready for capture.
    fn is_ready(&self) -> bool;

    /// Capture the current framebuffer if it has changed since the last call.
    fn capture_if_dirty(&mut self) -> Option<CaptureResult>;

    /// Current surface size in physical pixels.
    fn size(&self) -> (u32, u32);

    /// Resize the backend surface.
    fn resize(&mut self, width: u32, height: u32);

    /// Forward a mouse event to the backend.
    fn send_mouse_event(&mut self, event: MouseEvent);

    /// Forward a keyboard event to the backend.
    fn send_keyboard_event(&mut self, event: KeyboardEvent);

    /// Send a serialised JSON message to the UI.
    fn send_to_ui(&mut self, json: String) -> Result<(), FrontendError>;

    /// Try to receive a serialised JSON message from the UI (non-blocking).
    fn try_recv_from_ui(&mut self) -> Option<String>;

    /// Open developer tools for debugging (CEF only).
    fn show_dev_tools(&self) {}
}
