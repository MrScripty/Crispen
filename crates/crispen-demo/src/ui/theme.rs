//! DaVinci Resolve-style dark theme constants.
//!
//! All colors, sizing, and spacing values used across the Crispen UI
//! are defined here so widgets share a consistent look.

use bevy::color::Color;

// ── Background colors ───────────────────────────────────────────────────────

/// Main application background.
pub const BG_DARK: Color = Color::srgb(0.114, 0.114, 0.125);

/// Panel / sidebar background.
pub const BG_PANEL: Color = Color::srgb(0.157, 0.157, 0.173);

/// Control surface background (wheels, sliders).
pub const BG_CONTROL: Color = Color::srgb(0.200, 0.200, 0.220);

// ── Text colors ─────────────────────────────────────────────────────────────

/// Primary text (labels, values).
pub const TEXT_PRIMARY: Color = Color::srgb(0.878, 0.878, 0.878);

/// Dimmed / secondary text.
pub const TEXT_DIM: Color = Color::srgb(0.502, 0.502, 0.502);

// ── Accent ──────────────────────────────────────────────────────────────────

/// Accent color for selected items and active controls.
pub const ACCENT: Color = Color::srgb(0.918, 0.584, 0.200);

// ── Sizing ──────────────────────────────────────────────────────────────────

/// Diameter (px) of a color wheel widget.
pub const WHEEL_SIZE: f32 = 200.0;

/// Height (px) of horizontal slider tracks.
pub const SLIDER_HEIGHT: f32 = 20.0;

/// Inner padding (px) applied to panels.
pub const PANEL_PADDING: f32 = 8.0;

// ── Typography ──────────────────────────────────────────────────────────────

/// Font size for control labels.
pub const FONT_SIZE_LABEL: f32 = 12.0;

/// Font size for numeric readouts.
pub const FONT_SIZE_VALUE: f32 = 11.0;
