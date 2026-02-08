//! Primaries panel (Lift / Gamma / Gain / Offset wheels + slider bars).
//!
//! Layout matches DaVinci Resolve's Primaries panel:
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │  Temp  │  Tint  │  Contrast │  Pivot  │  Mid Detail            │
//! ├────────┼────────┼───────────┼─────────┼────────────────────────┤
//! │  LIFT  │  GAMMA │   GAIN    │  OFFSET │                        │
//! │(wheel) │(wheel) │ (wheel)   │(wheel)  │                        │
//! ├────────┼────────┼───────────┼─────────┼────────────────────────┤
//! │ Shadows│Highlights│Saturation│  Hue    │  Luma Mix              │
//! └──────────────────────────────────────────────────────────────────┘
//! ```

use bevy::prelude::*;

use super::color_wheel::{color_wheel, WheelType};
use super::components::{param_default, param_range, param_step, spawn_param_slider, ParamId};
use super::theme;

/// Spawn the primaries panel as a child of the given parent.
pub fn spawn_primaries_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(theme::PANEL_PADDING)),
                row_gap: Val::Px(6.0),
                width: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(theme::BG_PANEL),
        ))
        .with_children(|panel| {
            spawn_top_sliders(panel);
            spawn_wheels_row(panel);
            spawn_bottom_sliders(panel);
        });
}

/// Convenience wrapper that passes range/default/step from the `ParamId`.
fn slider(parent: &mut ChildSpawnerCommands, label: &str, id: ParamId) {
    spawn_param_slider(
        parent,
        label,
        id,
        param_range(id),
        param_default(id),
        param_step(id),
    );
}

fn spawn_top_sliders(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(12.0),
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|row| {
            slider(row, "TEMP", ParamId::Temperature);
            slider(row, "TINT", ParamId::Tint);
            slider(row, "CONTRAST", ParamId::Contrast);
            slider(row, "PIVOT", ParamId::Pivot);
            slider(row, "MID DETAIL", ParamId::MidtoneDetail);
        });
}

fn spawn_wheels_row(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceEvenly,
            align_items: AlignItems::Center,
            width: Val::Percent(100.0),
            padding: UiRect::axes(Val::Px(0.0), Val::Px(4.0)),
            ..default()
        })
        .with_children(|row| {
            for wheel_type in [
                WheelType::Lift,
                WheelType::Gamma,
                WheelType::Gain,
                WheelType::Offset,
            ] {
                row.spawn(color_wheel(wheel_type));
            }
        });
}

fn spawn_bottom_sliders(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(12.0),
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|row| {
            slider(row, "SHADOWS", ParamId::Shadows);
            slider(row, "HIGHLIGHTS", ParamId::Highlights);
            slider(row, "SATURATION", ParamId::Saturation);
            slider(row, "HUE", ParamId::Hue);
            slider(row, "LUMA MIX", ParamId::LumaMix);
        });
}
