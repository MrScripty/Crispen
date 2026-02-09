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

/// Image viewport background.
pub const BG_VIEWER: Color = Color::srgb(0.06, 0.06, 0.06);

// ── Text colors ─────────────────────────────────────────────────────────────

/// Primary text (labels, values).
pub const TEXT_PRIMARY: Color = Color::srgb(0.85, 0.85, 0.85);

/// Dimmed / secondary text.
pub const TEXT_DIM: Color = Color::srgb(0.55, 0.55, 0.55);

// ── Accent ──────────────────────────────────────────────────────────────────

/// Accent color for selected items and active controls.
pub const ACCENT: Color = Color::srgb(0.95, 0.55, 0.094);

// ── Border ──────────────────────────────────────────────────────────────────

/// Subtle panel border color.
pub const BORDER_SUBTLE: Color = Color::srgb(0.26, 0.26, 0.26);

// ── Sizing ──────────────────────────────────────────────────────────────────

/// Diameter (px) of a dial / rotary knob widget.
pub const DIAL_SIZE: f32 = 40.0;

/// Diameter (px) of a primary color wheel.
pub const WHEEL_SIZE: f32 = 132.0;

/// Horizontal gap (px) between adjacent color wheels.
pub const WHEEL_GAP: f32 = 10.0;

/// Width (px) reserved for each dial slot in the top/bottom rows.
pub const DIAL_SLOT_WIDTH: f32 = 78.0;

/// Width (px) of the 4-wheel cluster.
pub const WHEEL_GROUP_WIDTH: f32 = WHEEL_SIZE * 4.0 + WHEEL_GAP * 3.0;

/// Width (px) of the vectorscope panel shown beside primary wheels.
pub const VECTORSCOPE_PANEL_WIDTH: f32 = 172.0;

/// Width (px) of top/bottom dial rows, centered around wheel edge anchors.
pub const WHEEL_DIAL_ROW_WIDTH: f32 = WHEEL_GROUP_WIDTH + DIAL_SLOT_WIDTH;

/// Width (px) of the primaries section content block.
pub const PRIMARIES_SECTION_WIDTH: f32 = WHEEL_DIAL_ROW_WIDTH;

/// Width (px) of the hue-vs-curves section content block.
pub const HUE_CURVES_SECTION_WIDTH: f32 = 316.0;

/// Minimum width (px) of the scopes section content block.
pub const SCOPES_SECTION_MIN_WIDTH: f32 = VECTORSCOPE_PANEL_WIDTH + 16.0;

/// Horizontal gap (px) between bottom panel sections.
pub const BOTTOM_SECTION_GAP: f32 = 16.0;

/// Additional top margin (px) applied to the top dial row.
pub const TOP_DIAL_ROW_MARGIN_TOP: f32 = 6.0;

/// Additional top margin (px) applied to the bottom dial row (negative moves up).
pub const BOTTOM_DIAL_ROW_MARGIN_TOP: f32 = -6.0;

/// Inner padding (px) applied to panels.
pub const PANEL_PADDING: f32 = 8.0;

/// Bottom panel height (px), similar to Resolve primaries module.
pub const PRIMARIES_PANEL_HEIGHT: f32 = 340.0;

/// Top toolbar height (px).
pub const TOOLBAR_HEIGHT: f32 = 36.0;

/// Width (px) of the collapsible OFX side panel.
pub const OFX_PANEL_WIDTH: f32 = 280.0;

/// Width (px) of toolbar dropdown controls.
pub const TOOLBAR_DROPDOWN_WIDTH: f32 = 180.0;

/// Active background for toggle buttons.
pub const BG_TOGGLE_ACTIVE: Color = Color::srgb(0.30, 0.30, 0.30);

/// Background color for the hue-vs-curves plot area.
pub const CURVE_PLOT_BG: Color = Color::srgb(0.14, 0.14, 0.14);

/// Grid line color in the hue-vs-curves plot.
pub const CURVE_GRID_LINE: Color = Color::srgb(0.23, 0.23, 0.23);

/// Center neutral line color in the hue-vs-curves plot.
pub const CURVE_NEUTRAL_LINE: Color = Color::srgb(0.32, 0.32, 0.32);

// ── Typography ──────────────────────────────────────────────────────────────

/// Font size for control labels.
pub const FONT_SIZE_LABEL: f32 = 11.0;

/// Font size for numeric readouts.
pub const FONT_SIZE_VALUE: f32 = 10.0;
