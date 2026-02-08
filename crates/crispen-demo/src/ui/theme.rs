//! DaVinci Resolve-style dark theme constants.
//!
//! All colors, sizing, and spacing values used across the Crispen UI
//! are defined here so widgets share a consistent look.

use bevy::color::Color;

// ── Background colors ───────────────────────────────────────────────────────

/// Main application background.
pub const BG_DARK: Color = Color::srgb(0.118, 0.118, 0.118);

/// Panel / sidebar background.
pub const BG_PANEL: Color = Color::srgb(0.176, 0.176, 0.176);

/// Control surface background (wheels, sliders).
pub const BG_CONTROL: Color = Color::srgb(0.22, 0.22, 0.22);

// ── Text colors ─────────────────────────────────────────────────────────────

/// Primary text (labels, values).
pub const TEXT_PRIMARY: Color = Color::srgb(0.85, 0.85, 0.85);

/// Dimmed / secondary text.
pub const TEXT_DIM: Color = Color::srgb(0.55, 0.55, 0.55);

// ── Accent ──────────────────────────────────────────────────────────────────

/// Accent color for selected items and active controls.
pub const ACCENT: Color = Color::srgb(0.95, 0.55, 0.094);

// ── Slider colors ───────────────────────────────────────────────────────────

/// Slider track background.
pub const SLIDER_TRACK: Color = Color::srgb(0.25, 0.25, 0.25);

/// Slider thumb (grab handle).
pub const SLIDER_THUMB: Color = Color::srgb(0.7, 0.7, 0.7);

// ── Sizing ──────────────────────────────────────────────────────────────────

/// Diameter (px) of a color wheel widget.
pub const WHEEL_SIZE: f32 = 140.0;

/// Height (px) of horizontal slider tracks.
pub const SLIDER_HEIGHT: f32 = 18.0;

/// Inner padding (px) applied to panels.
pub const PANEL_PADDING: f32 = 8.0;

// ── Typography ──────────────────────────────────────────────────────────────

/// Font size for control labels.
pub const FONT_SIZE_LABEL: f32 = 11.0;

/// Font size for numeric readouts.
pub const FONT_SIZE_VALUE: f32 = 10.0;
